use crate::{
    domain::{Error, Result, hash},
    project::Project,
};
use globset::{Glob, GlobSetBuilder};
use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};
#[derive(Debug, Clone)]
pub struct VerifiedFile {
    pub path: String,
    pub text: String,
    pub hash: String,
    pub size_bytes: u64,
}
#[derive(Debug, Clone, Serialize)]
pub struct Inventory {
    pub paths: Vec<String>,
    pub skipped: Vec<serde_json::Value>,
}
fn security_globs(p: &Project) -> Result<globset::GlobSet> {
    let mut b = GlobSetBuilder::new();
    for s in [
        "**/.git/**",
        "**/.env",
        "**/.env.*",
        "**/*.pem",
        "**/*.key",
        "**/credentials*",
    ]
    .iter()
    .map(|s| s.to_string())
    .chain(p.config.policy.exclude.iter().cloned())
    {
        b.add(Glob::new(&s).map_err(|_| Error::new("INVALID_CONFIG", "Invalid security glob", 2))?);
    }
    b.build()
        .map_err(|_| Error::new("INVALID_CONFIG", "Invalid security policy", 2))
}
pub fn policy_allows(p: &Project, path: &str) -> Result<()> {
    let rel = Path::new(path);
    if path.is_empty()
        || path.contains('\\')
        || rel.is_absolute()
        || rel.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::new("PATH_OUTSIDE_ROOT", "Invalid relative path", 5));
    }
    if rel.components().any(|c| c.as_os_str() == ".git") || security_globs(p)?.is_match(path) {
        return Err(Error::new("POLICY_DENIED", "Excluded by current policy", 5));
    }
    Ok(())
}
pub fn authorize(p: &Project, path: &str) -> Result<PathBuf> {
    policy_allows(p, path)?;
    let rel = Path::new(path);
    if path.is_empty()
        || path.contains('\\')
        || rel.is_absolute()
        || rel.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::new(
            "PATH_OUTSIDE_ROOT",
            "Only project-relative paths without traversal are accepted",
            5,
        ));
    }
    if rel.components().any(|c| c.as_os_str() == ".git") || security_globs(p)?.is_match(path) {
        return Err(Error::new(
            "POLICY_DENIED",
            "Path excluded by security policy",
            5,
        ));
    }
    let mut resolved = p.root.clone();
    for c in rel.components() {
        resolved.push(c);
        if fs::symlink_metadata(&resolved)?.file_type().is_symlink() {
            return Err(Error::new("POLICY_DENIED", "Symlink traversal denied", 5));
        }
    }
    if !fs::canonicalize(&resolved)?.starts_with(fs::canonicalize(&p.root)?) {
        return Err(Error::new("PATH_OUTSIDE_ROOT", "Path outside project", 5));
    }
    Ok(resolved)
}
pub fn read(p: &Project, path: &str) -> Result<VerifiedFile> {
    for _ in 0..3 {
        let _resolved = authorize(p, path)?;
        #[cfg(unix)]
        let mut f = secure_open(p, path)?;
        #[cfg(not(unix))]
        let mut f = fs::File::open(&_resolved)?;
        let before = f.metadata()?;
        if !before.is_file() {
            return Err(Error::new("INVALID_ARGUMENT", "Expected a regular file", 2));
        }
        if before.len() > p.config.index.max_file_bytes {
            return Err(Error::new(
                "FILE_TOO_LARGE",
                "File exceeds configured reader limit",
                3,
            ));
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut f)
            .take(p.config.index.max_file_bytes + 1)
            .read_to_end(&mut bytes)?;
        let after = f.metadata()?;
        let reopened = fs::metadata(authorize(p, path)?)?;
        let mut stable = before.len() == after.len()
            && after.len() == reopened.len()
            && before.modified().ok() == after.modified().ok()
            && after.modified().ok() == reopened.modified().ok();
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            stable &= before.ino() == after.ino()
                && after.ino() == reopened.ino()
                && before.dev() == reopened.dev()
                && before.mtime_nsec() == after.mtime_nsec();
        }
        if !stable {
            continue;
        }
        if bytes.len() as u64 > p.config.index.max_file_bytes {
            return Err(Error::new(
                "FILE_TOO_LARGE",
                "File exceeds configured reader limit",
                3,
            ));
        }
        if bytes.contains(&0) {
            return Err(Error::new(
                "UNSUPPORTED_ENCODING",
                "Binary input is unsupported",
                3,
            ));
        }
        let file_hash = hash(&bytes);
        let text = String::from_utf8(bytes)
            .map_err(|_| Error::new("UNSUPPORTED_ENCODING", "Only UTF-8 text is supported", 3))?;
        return Ok(VerifiedFile {
            path: path.into(),
            size_bytes: text.len() as u64,
            text,
            hash: file_hash,
        });
    }
    Err(Error::new(
        "CONCURRENT_MODIFICATION",
        "File changed repeatedly while reading",
        4,
    ))
}
pub fn redact(text: &str) -> (String, bool) {
    redact_span(text, 0, text.len())
}
pub fn redact_span(text: &str, start: usize, end: usize) -> (String, bool) {
    if start > end
        || !text.is_char_boundary(start)
        || !text.is_char_boundary(end)
        || end > text.len()
    {
        return ("[REDACTED: INVALID RANGE]".into(), true);
    }
    static PATTERNS: std::sync::LazyLock<Vec<regex::Regex>> = std::sync::LazyLock::new(|| {
        [
        r"(?i)\b(?:sk-|ghp_|github_pat_|AKIA)[A-Za-z0-9_\-]{12,}",
        r#"(?i)(?:password|api[_-]?key|secret|token)\s*[:=]\s*["']?[^\s"',;]{8,}"#,
        r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----[\s\S]*?(?:-----END (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|$)",
    ].into_iter().map(|p|regex::Regex::new(p).expect("fixed secret masking pattern")).collect()
    });
    let mut ranges = Vec::new();
    for re in PATTERNS.iter() {
        for m in re.find_iter(text) {
            if m.start() < end && m.end() > start {
                ranges.push((m.start().max(start), m.end().min(end)));
            }
        }
    }
    ranges.sort();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (a, b) in ranges {
        if let Some(last) = merged.last_mut()
            && last.1 >= a
        {
            last.1 = last.1.max(b);
            continue;
        }
        merged.push((a, b));
    }
    let mut result = String::new();
    let mut cursor = start;
    for (a, b) in &merged {
        result.push_str(&text[cursor..*a]);
        result.push_str("[REDACTED]");
        cursor = *b;
    }
    result.push_str(&text[cursor..end]);
    (result, !merged.is_empty())
}
pub fn inventory(p: &Project, include_ignored: bool) -> Result<Inventory> {
    let mut walker = ignore::WalkBuilder::new(&p.root);
    walker
        .follow_links(false)
        .hidden(true)
        .git_ignore(!include_ignored)
        .git_global(!include_ignored)
        .git_exclude(!include_ignored)
        .require_git(false)
        .add_custom_ignore_filename(".pctxignore");
    walker.add_ignore(p.root.join(".pctx/ignore"));
    let mut paths = Vec::new();
    let mut skipped = Vec::new();
    for entry in walker.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                skipped.push(serde_json::json!({"reason":"walk_error"}));
                continue;
            }
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Some(path) = entry
            .path()
            .strip_prefix(&p.root)
            .ok()
            .and_then(|s| s.to_str())
        else {
            skipped.push(serde_json::json!({"reason":"unsupported_filename"}));
            continue;
        };
        let path = path.replace(std::path::MAIN_SEPARATOR, "/");
        if path.starts_with(".toolchain/")
            || path.starts_with("target/")
            || path.starts_with(".pctx-dev-data/")
        {
            continue;
        }
        match authorize(p, &path) {
            Ok(_) => paths.push(path),
            Err(e) if e.exit == 5 => {}
            Err(e) => skipped.push(serde_json::json!({"path":path,"reason":e.code})),
        }
    }
    for area in ["rules", "decisions", "handoffs"] {
        let dir = p.root.join(".pctx").join(area);
        if !dir.exists() {
            continue;
        }
        for entry in ignore::WalkBuilder::new(dir)
            .follow_links(false)
            .hidden(false)
            .build()
            .flatten()
        {
            if entry.file_type().is_some_and(|t| t.is_file())
                && let Some(path) = entry
                    .path()
                    .strip_prefix(&p.root)
                    .ok()
                    .and_then(|s| s.to_str())
                && authorize(p, path).is_ok()
            {
                paths.push(path.into());
            }
        }
    }
    for path in [".pctx/config.toml", ".pctx/glossary.toml"] {
        if p.root.join(path).is_file() && authorize(p, path).is_ok() {
            paths.push(path.into());
        }
    }
    paths.sort();
    paths.dedup();
    Ok(Inventory { paths, skipped })
}
pub fn manifest(p: &Project) -> Result<std::collections::BTreeMap<String, String>> {
    let inventory = inventory(p, false)?;
    if !inventory.skipped.is_empty() {
        return Err(Error::new(
            "PARTIAL_RESULT",
            "Manifest inventory is incomplete",
            3,
        ));
    }
    let mut map = std::collections::BTreeMap::new();
    for path in inventory.paths {
        if redact(&path).1 {
            return Err(Error::new(
                "PARTIAL_RESULT",
                "Sensitive manifest identity excluded",
                3,
            ));
        }
        let f = read(p, &path)?;
        map.insert(path, f.hash);
    }
    Ok(map)
}

#[cfg(unix)]
pub(crate) fn secure_open(p: &Project, path: &str) -> Result<fs::File> {
    use std::{
        ffi::CString,
        os::fd::{AsRawFd, FromRawFd},
    };
    let mut dir = fs::File::open(&p.root)?;
    let components = Path::new(path).components().collect::<Vec<_>>();
    for (i, c) in components.iter().enumerate() {
        use std::os::unix::ffi::OsStrExt;
        let name = CString::new(c.as_os_str().as_bytes())
            .map_err(|_| Error::new("INVALID_ARGUMENT", "NUL in path", 2))?;
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if i + 1 < components.len() {
                libc::O_DIRECTORY
            } else {
                0
            };
        // Directory descriptors pin each verified component against symlink replacement.
        let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(Error::new(
                "POLICY_DENIED",
                "Path changed or cannot be opened safely",
                5,
            ));
        }
        dir = unsafe { fs::File::from_raw_fd(fd) };
    }
    Ok(dir)
}

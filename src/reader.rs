use crate::{
    domain::{Error, Result},
    project::Project,
};
use globset::{Glob, GlobSetBuilder};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    sync::{Arc, LazyLock, Mutex},
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
// Cache only compiled pattern programs, never a path's authorization or source result.
// Exact current exclusion vectors are keys; a policy edit cannot reuse an older program.
static SECURITY_GLOBS: LazyLock<Mutex<BTreeMap<Vec<String>, Arc<globset::GlobSet>>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));
fn security_globs(exclusions: &[String]) -> Result<Arc<globset::GlobSet>> {
    {
        let cache = SECURITY_GLOBS.lock().map_err(|_| {
            Error::new(
                "POLICY_UNAVAILABLE",
                "Security pattern cache unavailable",
                7,
            )
        })?;
        if let Some(program) = cache.get(exclusions) {
            return Ok(Arc::clone(program));
        }
    }
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
    .chain(exclusions.iter().cloned())
    {
        b.add(Glob::new(&s).map_err(|_| Error::new("INVALID_CONFIG", "Invalid security glob", 2))?);
    }
    let program = Arc::new(
        b.build()
            .map_err(|_| Error::new("INVALID_CONFIG", "Invalid security policy", 2))?,
    );
    let mut cache = SECURITY_GLOBS.lock().map_err(|_| {
        Error::new(
            "POLICY_UNAVAILABLE",
            "Security pattern cache unavailable",
            7,
        )
    })?;
    if cache.len() >= 8 {
        cache.clear();
    }
    cache.insert(exclusions.to_vec(), Arc::clone(&program));
    Ok(program)
}
pub(crate) fn validate_exclusions(exclusions: &[String]) -> Result<()> {
    security_globs(exclusions)?;
    Ok(())
}
pub(crate) fn validate_policy(p: &Project) -> Result<()> {
    p.check_deadline()?;
    let result = validate_exclusions(&p.config.policy.exclude);
    p.check_deadline()?;
    result
}
// Cooperative checks surround native filesystem operations. A syscall itself
// can still block: no detached worker or hard filesystem latency claim.
fn checked_fs<T>(p: &Project, operation: impl FnOnce() -> std::io::Result<T>) -> Result<T> {
    p.check_deadline()?;
    let result = operation();
    p.check_deadline()?;
    Ok(result?)
}
pub fn policy_allows(p: &Project, path: &str) -> Result<()> {
    p.check_deadline()?;
    let rel = Path::new(path);
    if path.is_empty()
        || path.contains('\\')
        || rel.is_absolute()
        || rel.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::new("PATH_OUTSIDE_ROOT", "Invalid relative path", 5));
    }
    if rel.components().any(|c| c.as_os_str() == ".git")
        || security_globs(&p.config.policy.exclude)?.is_match(path)
    {
        return Err(Error::new("POLICY_DENIED", "Excluded by current policy", 5));
    }
    p.check_deadline()?;
    Ok(())
}
pub(crate) fn validate_root(p: &Project) -> Result<()> {
    p.check_deadline()?;
    let opened = anchored_root(&p.root, &p.root_anchor, p.deadline);
    p.check_deadline()?;
    opened?;
    Ok(())
}
/// Final identity gate for already collected timeout partials. This does not
/// reopen source content, renew the budget or permit further query work.
pub(crate) fn validate_partial_root(p: &Project) -> Result<()> {
    anchored_root(&p.root, &p.root_anchor, None)?;
    Ok(())
}
pub fn authorize(p: &Project, path: &str) -> Result<PathBuf> {
    policy_allows(p, path)?;
    validate_root(p)?;
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
    if rel.components().any(|c| c.as_os_str() == ".git")
        || security_globs(&p.config.policy.exclude)?.is_match(path)
    {
        return Err(Error::new(
            "POLICY_DENIED",
            "Path excluded by security policy",
            5,
        ));
    }
    let mut resolved = p.root.clone();
    for c in rel.components() {
        resolved.push(c);
        if checked_fs(p, || fs::symlink_metadata(&resolved))?
            .file_type()
            .is_symlink()
        {
            return Err(Error::new("POLICY_DENIED", "Symlink traversal denied", 5));
        }
    }
    if !checked_fs(p, || fs::canonicalize(&resolved))?
        .starts_with(checked_fs(p, || fs::canonicalize(&p.root))?)
    {
        return Err(Error::new("PATH_OUTSIDE_ROOT", "Path outside project", 5));
    }
    validate_root(p)?;
    Ok(resolved)
}
pub fn read(p: &Project, path: &str) -> Result<VerifiedFile> {
    read_with_identity(p, path).map(|(file, _)| file)
}

pub(crate) fn read_with_identity(
    p: &Project,
    path: &str,
) -> Result<(VerifiedFile, same_file::Handle)> {
    p.check_deadline()?;
    for _ in 0..3 {
        p.check_deadline()?;
        #[cfg(unix)]
        policy_allows(p, path)?;
        #[cfg(not(unix))]
        authorize(p, path)?;
        let mut f = secure_open(p, path)?;
        let before = checked_fs(p, || f.metadata())?;
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
        use sha2::{Digest, Sha256};
        let mut bytes = Vec::new();
        let mut digest = Sha256::new();
        let mut source = Read::by_ref(&mut f).take(p.config.index.max_file_bytes.saturating_add(1));
        let mut chunk = [0u8; 65536];
        loop {
            let count = checked_fs(p, || source.read(&mut chunk))?;
            if count == 0 {
                break;
            }
            digest.update(&chunk[..count]);
            p.check_deadline()?;
            bytes.extend_from_slice(&chunk[..count]);
        }
        let after = checked_fs(p, || f.metadata())?;
        // Reopen through the anchored no-follow traversal, not an unchecked
        // pathname stat after authorization.
        #[cfg(unix)]
        policy_allows(p, path)?;
        #[cfg(not(unix))]
        authorize(p, path)?;
        let reopened_file = secure_open(p, path)?;
        let reopened = checked_fs(p, || reopened_file.metadata())?;
        let identity = same_file::Handle::from_file(checked_fs(p, || f.try_clone())?)?;
        let same_instance = identity == same_file::Handle::from_file(reopened_file)?;
        p.check_deadline()?;
        let stable = same_instance
            && before.len() == after.len()
            && after.len() == reopened.len()
            && before.modified().ok() == after.modified().ok()
            && after.modified().ok() == reopened.modified().ok();
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let stable = stable
                && before.ino() == after.ino()
                && after.ino() == reopened.ino()
                && before.dev() == reopened.dev()
                && before.mtime_nsec() == after.mtime_nsec();
            if !stable {
                continue;
            }
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
        p.check_deadline()?;
        let binary = bytes.contains(&0);
        p.check_deadline()?;
        if binary {
            return Err(Error::new(
                "UNSUPPORTED_ENCODING",
                "Binary input is unsupported",
                3,
            ));
        }
        let file_hash = format!("{:x}", digest.finalize());
        p.check_deadline()?;
        let text = String::from_utf8(bytes);
        p.check_deadline()?;
        let text = text
            .map_err(|_| Error::new("UNSUPPORTED_ENCODING", "Only UTF-8 text is supported", 3))?;
        return Ok((
            VerifiedFile {
                path: path.into(),
                size_bytes: text.len() as u64,
                text,
                hash: file_hash,
            },
            identity,
        ));
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
    validate_policy(p)?;
    validate_root(p)?;
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
        p.check_deadline()?;
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
            Err(e) if e.code == "TIMEOUT" => return Err(e),
            Err(e) if e.exit == 5 => {}
            Err(e) => skipped.push(serde_json::json!({"path":path,"reason":e.code})),
        }
    }
    for area in ["rules", "decisions", "handoffs"] {
        p.check_deadline()?;
        let dir = p.root.join(".pctx").join(area);
        let exists = dir.exists();
        p.check_deadline()?;
        if !exists {
            continue;
        }
        for entry in ignore::WalkBuilder::new(dir)
            .follow_links(false)
            .hidden(false)
            .build()
        {
            p.check_deadline()?;
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    skipped.push(serde_json::json!({"reason":"walk_error"}));
                    continue;
                }
            };
            if entry.file_type().is_some_and(|t| t.is_file())
                && let Some(path) = entry
                    .path()
                    .strip_prefix(&p.root)
                    .ok()
                    .and_then(|s| s.to_str())
            {
                match authorize(p, path) {
                    Ok(_) => paths.push(path.into()),
                    Err(e) if e.code == "TIMEOUT" => return Err(e),
                    Err(_) => {}
                }
            }
        }
    }
    for path in [".pctx/config.toml", ".pctx/glossary.toml"] {
        p.check_deadline()?;
        let is_file = p.root.join(path).is_file();
        p.check_deadline()?;
        if is_file {
            match authorize(p, path) {
                Ok(_) => paths.push(path.into()),
                Err(e) if e.code == "TIMEOUT" => return Err(e),
                Err(_) => {}
            }
        }
    }
    p.check_deadline()?;
    paths.sort();
    paths.dedup();
    validate_root(p)?;
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
        p.check_deadline()?;
        let sensitive = redact(&path).1;
        p.check_deadline()?;
        if sensitive {
            return Err(Error::new(
                "PARTIAL_RESULT",
                "Sensitive manifest identity excluded",
                3,
            ));
        }
        let f = read(p, &path)?;
        map.insert(path, f.hash);
    }
    p.check_deadline()?;
    Ok(map)
}

fn physical_root(root: &Path) -> PathBuf {
    let root = root.to_owned();
    // Only fixed system aliases used by macOS tempfile are recognized.
    #[cfg(target_os = "macos")]
    {
        for (alias, physical) in [("/var", "/private/var"), ("/tmp", "/private/tmp")] {
            if let Ok(tail) = root.strip_prefix(alias)
                && fs::read_link(alias)
                    .is_ok_and(|target| Path::new("/").join(target) == Path::new(physical))
            {
                return Path::new(physical).join(tail);
            }
        }
    }
    root
}

fn root_changed() -> Error {
    Error::new(
        "POLICY_DENIED",
        "Project root or ancestor instance changed",
        5,
    )
}

// A bounded traversal opens each directory once and verifies the actual handle.
// Capture and validation share traversal rules but production always validates
// against an immutable RootAnchor, never a path-based authorization cache.
fn walk_root(
    root: &Path,
    mut inspect: impl FnMut(usize, &fs::File) -> Result<()>,
) -> Result<fs::File> {
    let root = physical_root(root);
    if !root.is_absolute() || root.components().count() > 256 {
        return Err(Error::new(
            "PATH_OUTSIDE_ROOT",
            "Invalid or excessively deep project root",
            5,
        ));
    }
    #[cfg(unix)]
    {
        use std::{
            ffi::CString,
            os::{
                fd::{AsRawFd, FromRawFd},
                unix::ffi::OsStrExt,
            },
        };
        let mut dir = fs::File::open("/")?;
        inspect(0, &dir)?;
        let mut index = 1;
        for component in root.components() {
            match component {
                Component::RootDir => continue,
                Component::Normal(name) => {
                    let name = CString::new(name.as_bytes())
                        .map_err(|_| Error::new("INVALID_ARGUMENT", "NUL in root path", 2))?;
                    let fd = unsafe {
                        libc::openat(
                            dir.as_raw_fd(),
                            name.as_ptr(),
                            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                        )
                    };
                    if fd < 0 {
                        return Err(root_changed());
                    }
                    dir = unsafe { fs::File::from_raw_fd(fd) };
                    inspect(index, &dir)?;
                    index += 1;
                }
                _ => {
                    return Err(Error::new(
                        "PATH_OUTSIDE_ROOT",
                        "Invalid project root component",
                        5,
                    ));
                }
            }
        }
        Ok(dir)
    }
    #[cfg(not(unix))]
    {
        let mut paths: Vec<_> = root.ancestors().map(Path::to_owned).collect();
        paths.reverse();
        let mut last = None;
        for (index, path) in paths.iter().enumerate() {
            let dir = non_unix_open(path, true)?;
            inspect(index, &dir)?;
            last = Some(dir);
        }
        last.ok_or_else(root_changed)
    }
}

pub(crate) fn capture_root(root: &Path) -> Result<Vec<same_file::Handle>> {
    let mut handles = Vec::new();
    walk_root(root, |_, file| {
        handles.push(same_file::Handle::from_file(file.try_clone()?)?);
        Ok(())
    })?;
    Ok(handles)
}

fn anchored_root(
    root: &Path,
    anchor: &crate::project::RootAnchor,
    deadline: Option<crate::deadline::Deadline>,
) -> Result<fs::File> {
    let mut count = 0;
    let file = walk_root(root, |index, file| {
        if let Some(deadline) = deadline {
            deadline.check()?;
        }
        anchor.verify(index, file)?;
        count += 1;
        if let Some(deadline) = deadline {
            deadline.check()?;
        }
        Ok(())
    })?;
    if count != anchor.len() {
        return Err(root_changed());
    }
    Ok(file)
}

pub(crate) fn validate_anchor(root: &Path, anchor: &crate::project::RootAnchor) -> Result<()> {
    anchored_root(root, anchor, None)?;
    Ok(())
}

#[cfg(not(unix))]
fn non_unix_open(path: &Path, directory: bool) -> Result<fs::File> {
    let before = fs::symlink_metadata(path)?;
    if before.file_type().is_symlink() {
        return Err(root_changed());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        // Reject all reparse points, including junctions; do not claim NT-relative
        // handle traversal. Parent-component replacement is checked before/after.
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        if before.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(root_changed());
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let actual = file.metadata()?;
        if actual.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || (directory && !actual.is_dir())
        {
            return Err(root_changed());
        }
        Ok(file)
    }
    #[cfg(not(windows))]
    {
        let file = fs::File::open(path)?;
        if directory && !file.metadata()?.is_dir() {
            return Err(root_changed());
        }
        Ok(file)
    }
}

pub(crate) fn anchored_open(
    root: &Path,
    anchor: &crate::project::RootAnchor,
    path: &str,
) -> Result<fs::File> {
    anchored_open_deadline(root, anchor, path, None)
}
pub(crate) fn anchored_open_deadline(
    root: &Path,
    anchor: &crate::project::RootAnchor,
    path: &str,
    deadline: Option<crate::deadline::Deadline>,
) -> Result<fs::File> {
    let check = || deadline.map(|d| d.check()).unwrap_or(Ok(()));
    check()?;
    let relative = Path::new(path);
    if path.is_empty()
        || path.contains('\\')
        || relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::new(
            "PATH_OUTSIDE_ROOT",
            "Invalid anchored relative path",
            5,
        ));
    }
    let mut dir = anchored_root(root, anchor, deadline)?;
    check()?;
    #[cfg(unix)]
    {
        use std::{
            ffi::CString,
            os::fd::{AsRawFd, FromRawFd},
            os::unix::ffi::OsStrExt,
        };
        let components = relative.components().collect::<Vec<_>>();
        for (i, c) in components.iter().enumerate() {
            check()?;
            let name = CString::new(c.as_os_str().as_bytes())
                .map_err(|_| Error::new("INVALID_ARGUMENT", "NUL in path", 2))?;
            // Regular files ignore O_NONBLOCK; a FIFO must never block admission
            // before the caller can reject its non-regular file type.
            let flags = libc::O_RDONLY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | if i + 1 < components.len() {
                    libc::O_DIRECTORY
                } else {
                    libc::O_NONBLOCK
                };
            let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
            let native_error = (fd < 0).then(std::io::Error::last_os_error);
            if fd >= 0 {
                dir = unsafe { fs::File::from_raw_fd(fd) };
            }
            check()?;
            if let Some(error) = native_error {
                if error.kind() == std::io::ErrorKind::NotFound {
                    return Err(error.into());
                }
                return Err(Error::new(
                    "POLICY_DENIED",
                    "Path changed or cannot be opened safely",
                    5,
                ));
            }
        }
        // The opened chain is pinned; reject a replaced named root before return.
        anchored_root(root, anchor, deadline)?;
        check()?;
        Ok(dir)
    }
    #[cfg(not(unix))]
    {
        let mut current = root.to_owned();
        let components: Vec<_> = relative.components().collect();
        for (i, component) in components.iter().enumerate() {
            check()?;
            current.push(component);
            let opened = non_unix_open(&current, i + 1 < components.len());
            check()?;
            dir = opened?;
        }
        anchored_root(root, anchor, deadline)?;
        check()?;
        Ok(dir)
    }
}

pub(crate) fn secure_open(p: &Project, path: &str) -> Result<fs::File> {
    p.check_deadline()?;
    let opened = anchored_open_deadline(&p.root, &p.root_anchor, path, p.deadline);
    p.check_deadline()?;
    opened
}

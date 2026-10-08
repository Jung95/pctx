use crate::domain::{Error, Result, hash, id};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub project: ProjectConfig,
    #[serde(default)]
    pub index: IndexConfig,
    #[serde(default)]
    pub policy: PolicyConfig,
    #[serde(default)]
    pub search: toml::Table,
    #[serde(default)]
    pub context: toml::Table,
    #[serde(default)]
    pub roles: toml::Table,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexConfig {
    pub max_file_bytes: u64,
    pub languages: Vec<String>,
    pub follow_symlinks: bool,
}
impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            max_file_bytes: 1048576,
            languages: vec![
                "python".into(),
                "javascript".into(),
                "jsx".into(),
                "typescript".into(),
                "tsx".into(),
            ],
            follow_symlinks: false,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PolicyConfig {
    pub exclude: Vec<String>,
    pub network: String,
    pub persist_source: bool,
}
impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            exclude: vec![],
            network: "deny".into(),
            persist_source: false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Project {
    pub root: PathBuf,
    pub data_dir: PathBuf,
    pub workspace_dir: PathBuf,
    pub control_dir: PathBuf,
    pub project_id: String,
    pub workspace_id: String,
    pub coordination_id: String,
    pub config: Config,
}
#[derive(Serialize, Deserialize, Clone)]
struct Binding {
    project_id: String,
    workspace_id: String,
    coordination_id: String,
}
#[derive(Default, Serialize, Deserialize)]
struct Registry {
    #[serde(default)]
    roots: BTreeMap<String, Binding>,
    #[serde(default)]
    common_dirs: BTreeMap<String, String>,
}

pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
pub fn atomic_write(path: &Path, bytes: &[u8], overwrite: bool) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("INVALID_ARGUMENT", "Output needs a parent directory", 2))?;
    if !parent.is_dir() {
        return Err(Error::new("IO_ERROR", "Output parent does not exist", 7));
    }
    if fs::symlink_metadata(path).is_ok() && !overwrite {
        return Err(Error::new("REVISION_CONFLICT", "Output already exists", 9));
    }
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::new("POLICY_DENIED", "Symlink output denied", 5));
    }
    let temp = parent.join(format!(".pctx-{}.tmp", id("write")));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut f = options.open(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        if overwrite {
            fs::rename(&temp, path)?;
        } else {
            fs::hard_link(&temp, path)?;
            fs::remove_file(&temp)?;
        }
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn data_dir() -> Result<PathBuf> {
    if let Some(d) = std::env::var_os("PCTX_DATA_DIR") {
        return Ok(PathBuf::from(d));
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| {
            Error::new(
                "INVALID_CONFIG",
                "User home unavailable; set PCTX_DATA_DIR",
                2,
            )
        })?;
    #[cfg(target_os = "macos")]
    let d = PathBuf::from(home).join("Library/Application Support/pctx");
    #[cfg(target_os = "windows")]
    let d = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from(home).join("AppData/Local"))
        .join("pctx");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let d = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from(home).join(".local/share"))
        .join("pctx");
    Ok(d)
}
pub fn detect_root(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return fs::canonicalize(p).map_err(Into::into);
    }
    let cwd = std::env::current_dir()?;
    for p in cwd.ancestors() {
        if p.join(".pctx/config.toml").is_file() {
            return Ok(p.to_owned());
        }
    }
    if let Ok(o) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&cwd)
        .output()
        && o.status.success()
    {
        return fs::canonicalize(String::from_utf8_lossy(&o.stdout).trim()).map_err(Into::into);
    }
    Ok(cwd)
}
impl Project {
    pub fn init(root: &Path) -> Result<Self> {
        let root = fs::canonicalize(root)?;
        let dir = root.join(".pctx");
        if fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::new(
                "POLICY_DENIED",
                "Linked project configuration denied",
                5,
            ));
        }
        fs::create_dir_all(&dir)?;
        if !dir.join("config.toml").exists() {
            let c = Config {
                schema_version: 1,
                project: ProjectConfig {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: root
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("project")
                        .into(),
                },
                index: Default::default(),
                policy: Default::default(),
                search: Default::default(),
                context: Default::default(),
                roles: Default::default(),
            };
            atomic_write(
                &dir.join("config.toml"),
                toml::to_string_pretty(&c)
                    .map_err(|_| Error::new("INVALID_CONFIG", "Cannot serialize configuration", 2))?
                    .as_bytes(),
                false,
            )?;
        }
        Self::load_binding(&root, true)
    }
    pub fn open(root: &Path) -> Result<Self> {
        Self::load_binding(root, false)
    }
    fn load_binding(root: &Path, register: bool) -> Result<Self> {
        let root = fs::canonicalize(root)?;
        for p in [root.join(".pctx"), root.join(".pctx/config.toml")] {
            if fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(Error::new(
                    "POLICY_DENIED",
                    "Linked configuration denied",
                    5,
                ));
            }
        }
        let source = fs::read_to_string(root.join(".pctx/config.toml"))
            .map_err(|_| Error::new("NOT_INITIALIZED", "Run pctx init first", 6))?;
        let config: Config = toml::from_str(&source).map_err(|_| {
            Error::new("INVALID_CONFIG", "Invalid project TOML or unknown field", 2)
        })?;
        let config = effective_config(config)?;
        if config.schema_version != 1
            || config.policy.persist_source
            || config.index.follow_symlinks
            || !["deny", "configured"].contains(&config.policy.network.as_str())
            || uuid::Uuid::parse_str(&config.project.id).is_err()
        {
            return Err(Error::new(
                "INVALID_CONFIG",
                "Unsupported schema, persistence, symlink, network or project ID setting",
                2,
            ));
        }
        let data_dir = data_dir()?;
        if register {
            private_dir(&data_dir)?;
        }
        let data_dir = fs::canonicalize(&data_dir).map_err(|_| {
            Error::new(
                "NOT_INITIALIZED",
                "Local data directory missing; run pctx init",
                6,
            )
        })?;
        let _registry_lock = if register {
            use fs2::FileExt;
            let lock = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(data_dir.join("registry.lock"))?;
            lock.lock_exclusive()?;
            Some(lock)
        } else {
            None
        };
        let registry_path = data_dir.join("registry.json");
        let mut registry: Registry = match fs::read(&registry_path) {
            Ok(b) => serde_json::from_slice(&b)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Registry::default(),
            Err(e) => return Err(e.into()),
        };
        let root_key = root.to_string_lossy().to_string();
        let binding = match registry
            .roots
            .get(&root_key)
            .filter(|b| b.project_id == config.project.id)
        {
            Some(b) => b.clone(),
            None if register => {
                let common = std::process::Command::new("git")
                    .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
                    .current_dir(&root)
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| format!("{}:{}", config.project.id, s.trim()));
                let coordination_id = common
                    .as_ref()
                    .and_then(|key| registry.common_dirs.get(key))
                    .cloned()
                    .unwrap_or_else(|| id("COORD"));
                if let Some(key) = common {
                    registry.common_dirs.insert(key, coordination_id.clone());
                }
                let b = Binding {
                    project_id: config.project.id.clone(),
                    workspace_id: id("WS"),
                    coordination_id,
                };
                registry.roots.insert(root_key, b.clone());
                atomic_write(&registry_path, &serde_json::to_vec(&registry)?, true)?;
                b
            }
            None => {
                return Err(Error::new(
                    "NOT_INITIALIZED",
                    "Workspace is not registered; run pctx init",
                    6,
                ));
            }
        };
        let workspace_dir = data_dir.join("workspaces").join(&binding.workspace_id);
        let control_dir = data_dir.join("controls").join(&binding.coordination_id);
        if register {
            private_dir(&workspace_dir)?;
            private_dir(&control_dir)?;
        }
        Ok(Self {
            root,
            data_dir,
            workspace_dir,
            control_dir,
            project_id: binding.project_id,
            workspace_id: binding.workspace_id,
            coordination_id: binding.coordination_id,
            config,
        })
    }
    pub fn policy_hash(&self) -> String {
        hash(serde_json::to_vec(&self.config.policy).unwrap_or_default())
    }
    pub fn index_db(&self) -> PathBuf {
        self.workspace_dir.join("index.sqlite3")
    }
    pub fn control_db(&self) -> PathBuf {
        self.control_dir.join("control.sqlite3")
    }
    pub fn connect(&self, control: bool) -> Result<Connection> {
        use fs2::FileExt;
        let path = if control {
            self.control_db()
        } else {
            self.index_db()
        };
        let parent = path
            .parent()
            .ok_or_else(|| Error::new("IO_ERROR", "Database location has no parent", 7))?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(parent.join("connection-init.lock"))?;
        let begin = std::time::Instant::now();
        loop {
            if lock.try_lock_exclusive().is_ok() {
                break;
            }
            if begin.elapsed() > Duration::from_secs(5) {
                return Err(Error::new(
                    "INDEX_BUSY",
                    "Database initialization is busy",
                    7,
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > 1 {
            return Err(Error::new(
                "DB_SCHEMA_TOO_NEW",
                "Database schema is newer than this binary",
                7,
            ));
        }
        if rusqlite::version_number() < 3051003 {
            return Err(Error::new(
                "CAPABILITY_UNAVAILABLE",
                "SQLite WAL fix requires version 3.51.3 or newer",
                6,
            ));
        }
        let mode: String = db.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            db.pragma_update(None, "journal_mode", "WAL")?;
        }
        Ok(db)
    }
}

/// Publish a handoff through pinned directory descriptors; repository symlink swaps
/// must not redirect writes outside the authorized project.
pub fn write_handoff(p: &Project, name: &str, bytes: &[u8], overwrite: bool) -> Result<()> {
    #[cfg(unix)]
    {
        use std::{
            ffi::CString,
            os::fd::{AsRawFd, FromRawFd},
        };
        let parent = crate::reader::secure_open(p, ".pctx")?;
        let sub = CString::new("handoffs").unwrap();
        let r = unsafe { libc::mkdirat(parent.as_raw_fd(), sub.as_ptr(), 0o700) };
        if r != 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists {
            return Err(std::io::Error::last_os_error().into());
        }
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                sub.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(Error::new(
                "POLICY_DENIED",
                "Linked handoff directory denied",
                5,
            ));
        }
        let dir = unsafe { fs::File::from_raw_fd(fd) };
        let final_name = CString::new(format!("{name}.md"))
            .map_err(|_| Error::new("INVALID_ARGUMENT", "Invalid handoff name", 2))?;
        let temp = CString::new(format!(".{}.tmp", id("handoff"))).unwrap();
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                temp.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let mut file = unsafe { fs::File::from_raw_fd(fd) };
        let result = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            let r = if overwrite {
                unsafe {
                    libc::renameat(
                        dir.as_raw_fd(),
                        temp.as_ptr(),
                        dir.as_raw_fd(),
                        final_name.as_ptr(),
                    )
                }
            } else {
                unsafe {
                    libc::linkat(
                        dir.as_raw_fd(),
                        temp.as_ptr(),
                        dir.as_raw_fd(),
                        final_name.as_ptr(),
                        0,
                    )
                }
            };
            if r != 0 {
                return Err(Error::new(
                    "REVISION_CONFLICT",
                    "Handoff publication failed or output exists",
                    9,
                ));
            }
            dir.sync_all()?;
            Ok(())
        })();
        unsafe {
            libc::unlinkat(dir.as_raw_fd(), temp.as_ptr(), 0);
        }
        result
    }
    #[cfg(not(unix))]
    {
        let dir = p.root.join(".pctx/handoffs");
        if fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::new(
                "POLICY_DENIED",
                "Linked handoff directory denied",
                5,
            ));
        }
        private_dir(&dir)?;
        atomic_write(&dir.join(format!("{name}.md")), bytes, overwrite)
    }
}

fn effective_config(mut project: Config) -> Result<Config> {
    let user_path = std::env::var_os("PCTX_USER_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/pctx/config.toml"))
        });
    if let Some(path) = user_path.filter(|p| p.exists()) {
        let source = fs::read_to_string(path)?;
        let table: toml::Table = toml::from_str(&source)
            .map_err(|_| Error::new("INVALID_CONFIG", "Invalid user TOML", 2))?;
        if table.keys().any(|k| {
            ![
                "schema_version",
                "index",
                "search",
                "context",
                "roles",
                "policy",
            ]
            .contains(&k.as_str())
        }) {
            return Err(Error::new(
                "INVALID_CONFIG",
                "Unknown user configuration section",
                2,
            ));
        }
        for (key, target) in [
            ("search", &mut project.search),
            ("context", &mut project.context),
            ("roles", &mut project.roles),
        ] {
            if let Some(user) = table.get(key).and_then(toml::Value::as_table) {
                for (k, v) in user {
                    target.entry(k.clone()).or_insert(v.clone());
                }
            }
        }
        if let Some(policy) = table.get("policy") {
            let user: PolicyConfig = policy
                .clone()
                .try_into()
                .map_err(|_| Error::new("INVALID_CONFIG", "Invalid user policy", 2))?;
            project.policy.exclude.extend(user.exclude);
            project.policy.exclude.sort();
            project.policy.exclude.dedup();
            if user.network == "deny" {
                project.policy.network = "deny".into();
            }
            if user.persist_source {
                return Err(Error::new(
                    "INVALID_CONFIG",
                    "Source index persistence unsupported",
                    2,
                ));
            }
        }
    }
    if let Ok(size) = std::env::var("PCTX_MAX_FILE_BYTES") {
        project.index.max_file_bytes = size
            .parse()
            .map_err(|_| Error::new("INVALID_CONFIG", "Invalid PCTX_MAX_FILE_BYTES", 2))?;
    }
    if project.index.max_file_bytes == 0 || project.index.max_file_bytes > 64 * 1024 * 1024 {
        return Err(Error::new(
            "INVALID_CONFIG",
            "File limit must be between 1 byte and 64 MiB",
            2,
        ));
    }
    for (table, allowed) in [
        (
            &project.search,
            &["default_limit", "max_candidates", "case"][..],
        ),
        (
            &project.context,
            &["default_budget_bytes", "max_snippet_lines", "profile"][..],
        ),
    ] {
        if table.keys().any(|k| !allowed.contains(&k.as_str())) {
            return Err(Error::new(
                "INVALID_CONFIG",
                "Unknown search/context option",
                2,
            ));
        }
    }
    Ok(project)
}

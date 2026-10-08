use crate::{
    deadline::Deadline,
    domain::{Error, Result, hash, id},
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
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
/// Immutable identity of the root and every ancestor opened for this project.
/// Handles remain alive across clones so a deleted directory identity cannot be
/// recycled while an application still holds authority for it.
#[derive(Clone, Debug)]
pub struct RootAnchor {
    handles: std::sync::Arc<[same_file::Handle]>,
    #[cfg(unix)]
    identities: std::sync::Arc<[(u64, u64)]>,
}
impl RootAnchor {
    /// Bind an existing directory for project loading or explicit fixture construction.
    /// Capturing a replacement is not a way to refresh a loaded Project's authority.
    pub fn capture(root: &Path) -> Result<Self> {
        let handles = crate::reader::capture_root(root)?;
        #[cfg(unix)]
        let identities = {
            use std::os::unix::fs::MetadataExt;
            handles
                .iter()
                .map(|handle| {
                    let metadata = handle.as_file().metadata()?;
                    Ok((metadata.dev(), metadata.ino()))
                })
                .collect::<Result<Vec<_>>>()?
        };
        Ok(Self {
            handles: handles.into(),
            #[cfg(unix)]
            identities: identities.into(),
        })
    }
    pub(crate) fn verify(&self, index: usize, file: &fs::File) -> Result<()> {
        #[cfg(unix)]
        let matches = {
            use std::os::unix::fs::MetadataExt;
            let metadata = file.metadata()?;
            self.identities.get(index) == Some(&(metadata.dev(), metadata.ino()))
        };
        #[cfg(not(unix))]
        let matches = {
            let actual = same_file::Handle::from_file(file.try_clone()?)?;
            self.handles.get(index) == Some(&actual)
        };
        if !matches {
            return Err(Error::new(
                "POLICY_DENIED",
                "Project root or ancestor instance changed",
                5,
            ));
        }
        Ok(())
    }
    pub(crate) fn len(&self) -> usize {
        self.handles.len()
    }
}
#[derive(Clone, Debug)]
pub struct Project {
    /// Finite request budget; absent for long-running child supervision.
    pub deadline: Option<crate::deadline::Deadline>,
    pub root: PathBuf,
    pub root_anchor: RootAnchor,
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
#[cfg(windows)]
fn windows_publish_file(
    source: &fs::File,
    directory: &fs::File,
    path: &Path,
    overwrite: bool,
) -> Result<()> {
    use std::{
        mem::{MaybeUninit, offset_of, size_of},
        os::windows::{ffi::OsStrExt, io::AsRawHandle},
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_RENAME_INFO, FILE_RENAME_INFO_0, FileRenameInfoEx, SetFileInformationByHandle,
    };
    let name = path
        .file_name()
        .ok_or_else(|| Error::new("INVALID_ARGUMENT", "Output needs a filename", 2))?;
    let name: Vec<u16> = name.encode_wide().take(32768).collect();
    if name.is_empty() || name.len() >= 32768 || name.contains(&0) || name.contains(&(b':' as u16))
    {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Invalid bounded output filename",
            2,
        ));
    }
    let name_bytes = name
        .len()
        .checked_mul(size_of::<u16>())
        .ok_or_else(|| Error::new("INVALID_ARGUMENT", "Output filename too long", 2))?;
    // Allocate native-header-aligned, fully initialized backing storage, including
    // a whole header PLUS the filename payload. Never extend FileName[1] through
    // a Rust array reference and never cast an alignment-1 Vec<u8> into a header.
    let bytes = size_of::<FILE_RENAME_INFO>() + name_bytes;
    let units = bytes.div_ceil(size_of::<FILE_RENAME_INFO>());
    let mut storage: Vec<MaybeUninit<FILE_RENAME_INFO>> =
        (0..units).map(|_| MaybeUninit::zeroed()).collect();
    let base = storage.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    // FILE_RENAME_REPLACE_IF_EXISTS=1, FILE_RENAME_POSIX_SEMANTICS=2. Existing
    // target handles remain valid and subsequent opens see the source identity.
    // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information
    // https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info
    unsafe {
        base.write(FILE_RENAME_INFO {
            Anonymous: FILE_RENAME_INFO_0 {
                Flags: if overwrite { 1 | 2 } else { 0 },
            },
            RootDirectory: directory.as_raw_handle(),
            FileNameLength: name_bytes as u32,
            FileName: [0],
        });
        let payload = base
            .cast::<u8>()
            .add(offset_of!(FILE_RENAME_INFO, FileName))
            .cast::<u16>();
        std::ptr::copy_nonoverlapping(name.as_ptr(), payload, name.len());
        if SetFileInformationByHandle(
            source.as_raw_handle(),
            FileRenameInfoEx,
            base.cast(),
            bytes as u32,
        ) == 0
        {
            let error = std::io::Error::last_os_error();
            if !overwrite && error.kind() == std::io::ErrorKind::AlreadyExists {
                return Err(Error::new("REVISION_CONFLICT", "Output already exists", 9));
            }
            return Err(error.into());
        }
    }
    // Source data was WRITE_THROUGH + sync_all before publication. Do not claim
    // directory-fsync or reboot/crash durability, and never copy across volumes,
    // delete the old target first, or silently downgrade unsupported filesystems.
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
    #[cfg(windows)]
    let directory = {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE,
        };
        // Hold the target directory identity without delete sharing and rename to
        // a relative leaf through that handle. Staging creation still resolves a
        // pathname; this is not a complete NT-relative ancestor traversal proof.
        let file = fs::OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES | FILE_TRAVERSE)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(parent)?;
        let metadata = file.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(Error::new(
                "POLICY_DENIED",
                "Linked output parent denied",
                5,
            ));
        }
        file
    };
    let temp = parent.join(format!(".pctx-{}.tmp", id("write")));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::{
                Foundation::GENERIC_WRITE,
                Storage::FileSystem::{DELETE, FILE_FLAG_WRITE_THROUGH, FILE_SHARE_READ},
            };
            // Renaming the owned staging identity requires DELETE source access.
            options
                .access_mode(GENERIC_WRITE | DELETE)
                .share_mode(FILE_SHARE_READ)
                .custom_flags(FILE_FLAG_WRITE_THROUGH);
        }
        let mut f = options.open(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        #[cfg(windows)]
        windows_publish_file(&f, &directory, path, overwrite)?;
        drop(f);
        #[cfg(not(windows))]
        {
            if overwrite {
                fs::rename(&temp, path)?;
            } else {
                fs::hard_link(&temp, path).map_err(|e| {
                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                        Error::new("REVISION_CONFLICT", "Output already exists", 9)
                    } else {
                        e.into()
                    }
                })?;
                fs::remove_file(&temp)?;
            }
            fs::File::open(parent)?.sync_all()?;
        }
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
fn lock_contended(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock
        || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
}
fn check_request(deadline: Option<Deadline>) -> Result<()> {
    deadline.map(Deadline::check).unwrap_or(Ok(()))
}
fn git_query(
    root: &Path,
    args: &[&str],
    deadline: Option<Deadline>,
) -> Result<std::process::Output> {
    check_request(deadline)?;
    let deadline = match deadline {
        Some(d) => d,
        None => Deadline::from_millis(10000)?,
    };
    let mut command = std::process::Command::new("git");
    command
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.pager=cat",
        ])
        .args(args)
        .current_dir(root)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_PAGER", "cat");
    crate::query_process::output(command, deadline, 1024 * 1024)
}
pub fn detect_root(explicit: Option<&Path>) -> Result<PathBuf> {
    detect_root_with_deadline(explicit, None)
}
pub fn detect_root_with_deadline(
    explicit: Option<&Path>,
    deadline: Option<Deadline>,
) -> Result<PathBuf> {
    check_request(deadline)?;
    if let Some(p) = explicit {
        let root = fs::canonicalize(p)?;
        check_request(deadline)?;
        return Ok(root);
    }
    let cwd = std::env::current_dir()?;
    for p in cwd.ancestors() {
        check_request(deadline)?;
        if p.join(".pctx/config.toml").is_file() {
            check_request(deadline)?;
            return Ok(p.to_owned());
        }
    }
    match git_query(&cwd, &["rev-parse", "--show-toplevel"], deadline) {
        Ok(o) if o.status.success() => {
            let root = fs::canonicalize(String::from_utf8_lossy(&o.stdout).trim())?;
            check_request(deadline)?;
            Ok(root)
        }
        Err(e)
            if matches!(
                e.code.as_str(),
                "TIMEOUT" | "PARTIAL_RESULT" | "CAPABILITY_UNAVAILABLE"
            ) =>
        {
            Err(e)
        }
        _ => {
            check_request(deadline)?;
            Ok(cwd)
        }
    }
}
impl Project {
    pub fn check_deadline(&self) -> Result<()> {
        self.deadline
            .map(|deadline| deadline.check())
            .unwrap_or(Ok(()))
    }
    pub fn remaining(&self, maximum: Duration) -> Result<Duration> {
        self.deadline
            .map(|deadline| deadline.remaining().map(|left| left.min(maximum)))
            .unwrap_or(Ok(maximum))
    }
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
        Self::load_binding(&root, true, None)
    }
    pub fn open(root: &Path) -> Result<Self> {
        Self::open_with_deadline(root, None)
    }
    pub fn open_with_deadline(root: &Path, deadline: Option<Deadline>) -> Result<Self> {
        Self::load_binding(root, false, deadline)
    }
    fn load_binding(root: &Path, register: bool, deadline: Option<Deadline>) -> Result<Self> {
        check_request(deadline)?;
        let root = fs::canonicalize(root)?;
        check_request(deadline)?;
        // Capture before loading policy, then read through that same authority.
        let root_anchor = RootAnchor::capture(&root)?;
        check_request(deadline)?;
        let mut config_file =
            crate::reader::anchored_open(&root, &root_anchor, ".pctx/config.toml").map_err(
                |e| {
                    if e.code == "IO_ERROR" {
                        Error::new("NOT_INITIALIZED", "Run pctx init first", 6)
                    } else {
                        e
                    }
                },
            )?;
        check_request(deadline)?;
        let config_metadata = config_file.metadata()?;
        if !config_metadata.is_file() || config_metadata.len() > 1024 * 1024 {
            return Err(Error::new(
                "INVALID_CONFIG",
                "Project configuration must be a bounded regular file",
                2,
            ));
        }
        let mut source = String::new();
        std::io::Read::by_ref(&mut config_file)
            .take(1024 * 1024 + 1)
            .read_to_string(&mut source)?;
        check_request(deadline)?;
        if source.len() > 1024 * 1024 {
            return Err(Error::new(
                "INVALID_CONFIG",
                "Project configuration exceeds size limit",
                2,
            ));
        }
        crate::reader::validate_anchor(&root, &root_anchor)?;
        let config_reopened =
            crate::reader::anchored_open(&root, &root_anchor, ".pctx/config.toml")?;
        if same_file::Handle::from_file(config_file.try_clone()?)?
            != same_file::Handle::from_file(config_reopened)?
        {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Project configuration changed while loading",
                4,
            ));
        }
        let config: Config = toml::from_str(&source).map_err(|_| {
            Error::new("INVALID_CONFIG", "Invalid project TOML or unknown field", 2)
        })?;
        check_request(deadline)?;
        let config = effective_config(config, deadline)?;
        check_request(deadline)?;
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
        check_request(deadline)?;
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
            if let Some(deadline) = deadline {
                let started = std::time::Instant::now();
                loop {
                    deadline.check()?;
                    match lock.try_lock_exclusive() {
                        Ok(()) => break,
                        Err(e) if lock_contended(&e) => {}
                        Err(e) => return Err(e.into()),
                    }
                    if started.elapsed() >= Duration::from_secs(5) {
                        return Err(Error::new(
                            "INDEX_BUSY",
                            "Registry initialization is busy",
                            7,
                        ));
                    }
                    std::thread::sleep(deadline.remaining()?.min(Duration::from_millis(5)));
                }
            } else {
                lock.lock_exclusive()?;
            }
            Some(lock)
        } else {
            None
        };
        check_request(deadline)?;
        let registry_path = data_dir.join("registry.json");
        let mut registry: Registry = match fs::read(&registry_path) {
            Ok(b) => serde_json::from_slice(&b)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Registry::default(),
            Err(e) => return Err(e.into()),
        };
        check_request(deadline)?;
        let root_key = root.to_string_lossy().to_string();
        let binding = match registry
            .roots
            .get(&root_key)
            .filter(|b| b.project_id == config.project.id)
        {
            Some(b) => b.clone(),
            None if register => {
                let common = match git_query(
                    &root,
                    &["rev-parse", "--path-format=absolute", "--git-common-dir"],
                    deadline,
                ) {
                    Ok(o) if o.status.success() => String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| format!("{}:{}", config.project.id, s.trim())),
                    Err(e)
                        if matches!(
                            e.code.as_str(),
                            "TIMEOUT" | "PARTIAL_RESULT" | "CAPABILITY_UNAVAILABLE"
                        ) =>
                    {
                        return Err(e);
                    }
                    _ => None,
                };
                check_request(deadline)?;
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
                crate::reader::validate_anchor(&root, &root_anchor)?;
                check_request(deadline)?;
                atomic_write(&registry_path, &serde_json::to_vec(&registry)?, true)?;
                check_request(deadline)?;
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
        check_request(deadline)?;
        if register {
            private_dir(&workspace_dir)?;
            private_dir(&control_dir)?;
        }
        crate::reader::validate_anchor(&root, &root_anchor)?;
        check_request(deadline)?;
        Ok(Self {
            deadline,
            root,
            root_anchor,
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
    fn database_phase<T>(
        &self,
        db: &Connection,
        call: impl FnOnce() -> rusqlite::Result<T>,
    ) -> Result<T> {
        db.busy_timeout(self.remaining(Duration::from_secs(5))?)?;
        let result = call();
        self.check_deadline()?;
        result.map_err(Into::into)
    }
    pub fn connect(&self, control: bool) -> Result<Connection> {
        self.check_deadline()?;
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
            self.check_deadline()?;
            match lock.try_lock_exclusive() {
                Ok(()) => break,
                Err(e) if lock_contended(&e) => {}
                Err(e) => return Err(e.into()),
            }
            if begin.elapsed() > Duration::from_secs(5) {
                return Err(Error::new(
                    "INDEX_BUSY",
                    "Database initialization is busy",
                    7,
                ));
            }
            std::thread::sleep(self.remaining(Duration::from_millis(5))?);
        }
        self.check_deadline()?;
        let db = Connection::open(path)?;
        if let Some(deadline) = self.deadline {
            db.progress_handler(1000, Some(move || deadline.check().is_err()))?;
        }
        self.check_deadline()?;
        self.database_phase(&db, || db.pragma_update(None, "foreign_keys", "ON"))?;
        let version: i64 = self.database_phase(&db, || {
            db.pragma_query_value(None, "user_version", |r| r.get(0))
        })?;
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
        let mode: String = self.database_phase(&db, || {
            db.pragma_query_value(None, "journal_mode", |r| r.get(0))
        })?;
        if !mode.eq_ignore_ascii_case("wal") {
            self.database_phase(&db, || db.pragma_update(None, "journal_mode", "WAL"))?;
        }
        self.check_deadline()?;
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

fn effective_config(mut project: Config, deadline: Option<Deadline>) -> Result<Config> {
    check_request(deadline)?;
    let user_path = std::env::var_os("PCTX_USER_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/pctx/config.toml"))
        });
    if let Some(path) = user_path.filter(|p| p.exists()) {
        check_request(deadline)?;
        let source = fs::read_to_string(path)?;
        check_request(deadline)?;
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
    crate::reader::validate_exclusions(&project.policy.exclude)?;
    check_request(deadline)?;
    Ok(project)
}

//! Bounded explicit task-document input under the existing request deadline.
//!
//! Stdin belongs to this synchronous request. Calls through this module are
//! serialized; callers must not concurrently read stdin through another API.
//! Unix dup shares file-status flags, so nonblocking mode is restored before
//! returning, with an RAII fallback on errors/panic. No reader worker survives
//! expiry. Regular disk I/O remains cooperatively bounded, not hard-cancelled.
use crate::{
    deadline::Deadline,
    domain::{Error, Result, hash},
    project::{Project, RootAnchor},
    reader,
};
use std::{
    fs::{self, File, Metadata},
    io::Read,
    path::{Component, Path, PathBuf},
    sync::{Mutex, TryLockError},
    time::Duration,
};

pub const MAX_TASK_BYTES: usize = 1024 * 1024;
static STDIN_OWNER: Mutex<()> = Mutex::new(());

/// Read an explicit regular file under its caller's byte cap and existing
/// optional deadline. A metadata check alone cannot bound growth during reading.
/// This treats '-' as a literal filename; stdin is a separate transport.
pub fn bounded_file_bytes(
    path: &Path,
    limit: usize,
    deadline: Option<Deadline>,
) -> Result<Vec<u8>> {
    bounded_file_bytes_observed(path, limit, deadline, || Ok(()))
}
fn bounded_file_bytes_observed(
    path: &Path,
    limit: usize,
    deadline: Option<Deadline>,
    observe: impl FnOnce() -> Result<()>,
) -> Result<Vec<u8>> {
    let check = || deadline.map_or(Ok(()), Deadline::check);
    check()?;
    limit
        .checked_add(1)
        .ok_or_else(|| Error::new("INVALID_ARGUMENT", "Input limit exceeds supported range", 2))?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path)?;
    check()?;
    let metadata = file.metadata()?;
    check()?;
    if !metadata.is_file() {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Input must be a regular file",
            2,
        ));
    }
    let overflow = || Error::new("FILE_TOO_LARGE", "Explicit input exceeds byte limit", 2);
    if metadata.len() > limit as u64 {
        return Err(overflow());
    }
    observe()?;
    bounded_reader_bytes(&mut file, limit, deadline, |_| Ok(()))
}

/// Shared chunk loop for already-admitted regular files. Callers retain their
/// own authority and metadata gates; no request budget is created here.
pub(crate) fn bounded_reader_bytes(
    file: &mut impl Read,
    limit: usize,
    deadline: Option<Deadline>,
    mut observe: impl FnMut(usize) -> Result<()>,
) -> Result<Vec<u8>> {
    let check = || deadline.map_or(Ok(()), Deadline::check);
    check()?;
    let bound = limit
        .checked_add(1)
        .ok_or_else(|| Error::new("INVALID_ARGUMENT", "Input limit exceeds supported range", 2))?;
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        check()?;
        let capacity = chunk.len().min(bound - bytes.len());
        let read = file.read(&mut chunk[..capacity]);
        if let Ok(count) = read {
            observe(count)?;
        }
        check()?;
        match read {
            Ok(0) => return Ok(bytes),
            Ok(count) => {
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.len() > limit {
                    return Err(Error::new(
                        "FILE_TOO_LARGE",
                        "Explicit input exceeds byte limit",
                        2,
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
}

fn too_large() -> Error {
    Error::new("FILE_TOO_LARGE", "Task document exceeds limit", 2)
}
#[cfg(not(unix))]
fn unsupported() -> Error {
    Error::new(
        "CAPABILITY_UNAVAILABLE",
        "This task input transport has no verified deadline-aware reader",
        6,
    )
}
fn regular(file: &File, check_size: bool) -> Result<()> {
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Task document must be a regular file",
            2,
        ));
    }
    if check_size && metadata.len() > MAX_TASK_BYTES as u64 {
        return Err(too_large());
    }
    Ok(())
}

fn read_disk(file: &mut File, deadline: Deadline) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        deadline.check()?;
        let capacity = chunk.len().min(MAX_TASK_BYTES + 1 - bytes.len());
        let read = file.read(&mut chunk[..capacity]);
        deadline.check()?;
        match read {
            Ok(0) => {
                deadline.check()?;
                return Ok(bytes);
            }
            Ok(count) => {
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.len() > MAX_TASK_BYTES {
                    return Err(too_large());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
}

/// A bounded task input and its retained, private source authority.
/// Provenance deliberately contains no filesystem names or document text.
pub struct TaskDocument {
    pub text: String,
    raw_hash: String,
    deadline: Deadline,
    proof: Option<TaskFileProof>,
}
struct TaskFileProof {
    original: PathBuf,
    canonical: PathBuf,
    alias: Vec<(PathBuf, AliasStamp)>,
    authority: TaskAuthority,
    identity: same_file::Handle,
    stamp: FileStamp,
}
enum TaskAuthority {
    Project {
        project: Box<Project>,
        relative: String,
    },
    External {
        parent: PathBuf,
        anchor: RootAnchor,
        leaf: String,
    },
}
#[derive(PartialEq, Eq)]
struct FileStamp {
    length: u64,
    modified: Option<std::time::SystemTime>,
}
impl FileStamp {
    fn new(m: &Metadata) -> Self {
        Self {
            length: m.len(),
            modified: m.modified().ok(),
        }
    }
}
#[derive(PartialEq, Eq)]
struct AliasStamp {
    directory: bool,
    symlink: bool,
    target: Option<PathBuf>,
    #[cfg(unix)]
    identity: (u64, u64),
    #[cfg(windows)]
    identity: (u32, u64),
}
fn changed() -> Error {
    Error::new("CONCURRENT_MODIFICATION", "Task input authority changed", 4)
}
fn unsafe_input() -> Error {
    Error::new(
        "POLICY_DENIED",
        "Task input alias is not an admitted project source",
        5,
    )
}
fn canonical(path: &Path, deadline: Deadline) -> Result<PathBuf> {
    deadline.check()?;
    let value = fs::canonicalize(path);
    deadline.check()?;
    Ok(value?)
}
fn alias_stamps(path: &Path, deadline: Deadline) -> Result<Vec<(PathBuf, AliasStamp)>> {
    let mut current = PathBuf::new();
    let mut out = Vec::new();
    for component in path.components() {
        deadline.check()?;
        current.push(component);
        // A Windows drive prefix alone is not an absolute directory authority.
        if !current.is_absolute() {
            continue;
        }
        let m = fs::symlink_metadata(&current)?;
        #[cfg(windows)]
        let symlink = {
            use std::os::windows::fs::MetadataExt;
            m.file_type().is_symlink() || m.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let symlink = m.file_type().is_symlink();
        let target = if symlink {
            Some(fs::read_link(&current)?)
        } else {
            None
        };
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            (m.dev(), m.ino())
        };
        #[cfg(windows)]
        let identity = {
            use std::os::windows::fs::MetadataExt;
            (m.file_attributes(), m.creation_time())
        };
        out.push((
            current.clone(),
            AliasStamp {
                directory: m.is_dir(),
                symlink,
                target,
                #[cfg(any(unix, windows))]
                identity,
            },
        ));
        deadline.check()?;
    }
    Ok(out)
}
impl TaskAuthority {
    fn open(&self, deadline: Deadline) -> Result<File> {
        match self {
            Self::Project { project, relative } => reader::secure_open(project, relative),
            Self::External {
                parent,
                anchor,
                leaf,
            } => reader::anchored_open_deadline(parent, anchor, leaf, Some(deadline)),
        }
    }
    fn read(&self, deadline: Deadline) -> Result<(String, same_file::Handle, FileStamp)> {
        self.read_observed(deadline, |_| Ok(()))
    }
    fn read_observed(
        &self,
        deadline: Deadline,
        mut observe: impl FnMut(&str) -> Result<()>,
    ) -> Result<(String, same_file::Handle, FileStamp)> {
        deadline.check()?;
        // Pin identity before reading. Reopening afterwards prevents an atomic
        // replacement, including one containing identical bytes, from rebinding it.
        let mut file = self.open(deadline)?;
        regular(&file, true)?;
        let stamp = FileStamp::new(&file.metadata()?);
        let identity = same_file::Handle::from_file(file.try_clone()?)?;
        observe("pinned")?;
        let text = match self {
            Self::Project { project, relative } => {
                let (source, read_identity) = reader::read_with_identity(project, relative)?;
                observe("read")?;
                if identity != read_identity {
                    return Err(changed());
                }
                source.text
            }
            Self::External { .. } => {
                String::from_utf8(read_disk(&mut file, deadline)?).map_err(|_| {
                    Error::new("UNSUPPORTED_ENCODING", "Task document must be UTF-8", 2)
                })?
            }
        };
        let reopened = self.open(deadline)?;
        regular(&reopened, true)?;
        let after = FileStamp::new(&file.metadata()?);
        let current = FileStamp::new(&reopened.metadata()?);
        let current_identity = same_file::Handle::from_file(reopened)?;
        deadline.check()?;
        if identity != current_identity || stamp != after || stamp != current {
            return Err(changed());
        }
        Ok((text, identity, stamp))
    }
}
impl TaskDocument {
    pub fn provenance(&self) -> serde_json::Value {
        let kind = match self.proof.as_ref().map(|p| &p.authority) {
            None => "stdin",
            Some(TaskAuthority::Project { .. }) => "project_file",
            Some(TaskAuthority::External { .. }) => "external_file",
        };
        serde_json::json!({"kind":kind,"hash":self.raw_hash,"replayable":self.proof.is_some()})
    }
    /// Checks the original input authority, never rereading stdin or following
    /// the original alias to open a newly selected file. Delivery-policy
    /// revalidation remains the caller's responsibility.
    pub fn revalidate(&self, p: &Project) -> Result<()> {
        self.deadline.check()?;
        p.check_deadline()?;
        let mut current_project = p.clone();
        current_project.deadline = Some(self.deadline);
        reader::validate_root(&current_project)?;
        if let Some(proof) = &self.proof {
            if canonical(&proof.original, self.deadline)? != proof.canonical
                || alias_stamps(&proof.original, self.deadline)? != proof.alias
            {
                return Err(changed());
            }
            if let TaskAuthority::Project { project, .. } = &proof.authority
                && (project.root != p.root
                    || project.project_id != p.project_id
                    || project.workspace_id != p.workspace_id)
            {
                return Err(changed());
            }
            let (text, identity, stamp) = proof.authority.read(self.deadline)?;
            if identity != proof.identity
                || stamp != proof.stamp
                || hash(text.as_bytes()) != self.raw_hash
            {
                return Err(changed());
            }
        }
        self.deadline.check()?;
        p.check_deadline()
    }
}
/// Explicit task-file input with project-source delivery admission before body
/// access. External explicit input is pinned independently of project policy.
pub fn project_task_document(
    p: &Project,
    path: &str,
    mut before_project_read: impl FnMut(&str) -> Result<()>,
) -> Result<TaskDocument> {
    let deadline = match p.deadline {
        Some(deadline) => deadline,
        None => Deadline::from_millis(10_000)?,
    };
    deadline.check()?;
    if path == "-" {
        let text = task_document(path, deadline)?;
        return Ok(TaskDocument {
            raw_hash: hash(text.as_bytes()),
            text,
            deadline,
            proof: None,
        });
    }
    let mut bounded_project = p.clone();
    bounded_project.deadline = Some(deadline);
    reader::validate_root(&bounded_project)?;
    let original = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };
    deadline.check()?;
    let root = canonical(&p.root, deadline)?;
    let lexical_project = original.starts_with(&p.root) || original.starts_with(&root);
    if lexical_project {
        let base = if original.starts_with(&p.root) {
            &p.root
        } else {
            &root
        };
        let relative = original.strip_prefix(base).map_err(|_| unsafe_input())?;
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(unsafe_input());
        }
        let mut current = base.clone();
        for part in relative.components() {
            current.push(part);
            deadline.check()?;
            let m = fs::symlink_metadata(&current)?;
            #[cfg(windows)]
            let link = {
                use std::os::windows::fs::MetadataExt;
                m.file_type().is_symlink() || m.file_attributes() & 0x400 != 0
            };
            #[cfg(not(windows))]
            let link = m.file_type().is_symlink();
            if link {
                return Err(unsafe_input());
            }
        }
    }
    let target = canonical(&original, deadline)?;
    if lexical_project && !target.starts_with(&root) {
        return Err(unsafe_input());
    }
    let alias = alias_stamps(&original, deadline)?;
    let authority = if let Ok(relative) = target.strip_prefix(&root) {
        let relative = relative
            .to_str()
            .ok_or_else(unsafe_input)?
            .replace('\\', "/");
        deadline.check()?;
        before_project_read(&relative)?;
        deadline.check()?;
        let mut captured = p.clone();
        captured.deadline = Some(deadline);
        captured.config.index.max_file_bytes = captured
            .config
            .index
            .max_file_bytes
            .min(MAX_TASK_BYTES as u64);
        TaskAuthority::Project {
            project: Box::new(captured),
            relative,
        }
    } else {
        let parent = target.parent().ok_or_else(unsafe_input)?.to_path_buf();
        let leaf = target
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(unsafe_input)?
            .to_string();
        deadline.check()?;
        let anchor = RootAnchor::capture(&parent)?;
        deadline.check()?;
        TaskAuthority::External {
            parent,
            anchor,
            leaf,
        }
    };
    let (text, identity, stamp) = authority.read(deadline)?;
    let doc = TaskDocument {
        raw_hash: hash(text.as_bytes()),
        text,
        deadline,
        proof: Some(TaskFileProof {
            original,
            canonical: target,
            alias,
            authority,
            identity,
            stamp,
        }),
    };
    doc.revalidate(p)?;
    Ok(doc)
}

/// Existing explicit regular-file input transport, without interpreting "-" as stdin.
/// The caller retains its original request clock and encoding/domain rules.
pub(crate) fn explicit_file_bytes(path: &Path, deadline: Deadline) -> Result<Vec<u8>> {
    deadline.check()?;
    let bytes = file_bytes(path, deadline)?;
    deadline.check()?;
    Ok(bytes)
}

/// One bounded, serialized stdin transport shared by finite explicit inputs.
/// The caller owns the deadline and maps byte/encoding policy to its domain.
pub(crate) fn read_stdin(deadline: Deadline) -> Result<Vec<u8>> {
    deadline.check()?;
    let _owner = loop {
        deadline.check()?;
        match STDIN_OWNER.try_lock() {
            Ok(owner) => break owner,
            Err(TryLockError::WouldBlock) => {
                std::thread::sleep(deadline.remaining()?.min(Duration::from_millis(1)))
            }
            Err(TryLockError::Poisoned(_)) => {
                return Err(Error::new(
                    "SOURCE_UNAVAILABLE",
                    "Task input ownership unavailable",
                    7,
                ));
            }
        }
    };
    let bytes = stdin_bytes(deadline);
    deadline.check()?;
    bytes
}

pub fn task_document(path: &str, deadline: Deadline) -> Result<String> {
    deadline.check()?;
    let bytes = if path == "-" {
        read_stdin(deadline)
    } else {
        file_bytes(Path::new(path), deadline)
    };
    deadline.check()?;
    let text = String::from_utf8(bytes?);
    deadline.check()?;
    text.map_err(|_| Error::new("UNSUPPORTED_ENCODING", "Task document must be UTF-8", 2))
}

#[cfg(unix)]
fn file_bytes(path: &Path, deadline: Deadline) -> Result<Vec<u8>> {
    use std::os::unix::fs::OpenOptionsExt;
    deadline.check()?;
    // O_NONBLOCK prevents a FIFO pathname from blocking during open; metadata
    // rejects it before any source read. Explicit regular-file symlinks retain
    // the pre-existing task-file semantics, unlike project source authority.
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)?;
    deadline.check()?;
    regular(&file, true)?;
    read_disk(&mut file, deadline)
}

#[cfg(unix)]
struct StdinFlags {
    file: File,
    original: i32,
    restored: bool,
}
#[cfg(unix)]
impl StdinFlags {
    fn restore(&mut self) -> Result<()> {
        use std::os::fd::AsRawFd;
        if unsafe { libc::fcntl(self.file.as_raw_fd(), libc::F_SETFL, self.original) } == -1 {
            return Err(std::io::Error::last_os_error().into());
        }
        self.restored = true;
        Ok(())
    }
}
#[cfg(unix)]
impl Drop for StdinFlags {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.restore();
        }
    }
}

#[cfg(unix)]
fn stdin_bytes(deadline: Deadline) -> Result<Vec<u8>> {
    use std::os::fd::{AsRawFd, FromRawFd};
    let fd = unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_DUPFD_CLOEXEC, 3) };
    if fd == -1 {
        return Err(std::io::Error::last_os_error().into());
    }
    // Duplicate adoption transfers ownership of only the successful new FD.
    let file = unsafe { File::from_raw_fd(fd) };
    let original = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if original == -1 {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut input = StdinFlags {
        file,
        original,
        restored: false,
    };
    if unsafe { libc::fcntl(fd, libc::F_SETFL, original | libc::O_NONBLOCK) } == -1 {
        return Err(std::io::Error::last_os_error().into());
    }
    let result = (|| -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        let mut chunk = [0; 8192];
        loop {
            deadline.check()?;
            let capacity = chunk.len().min(MAX_TASK_BYTES + 1 - bytes.len());
            match input.file.read(&mut chunk[..capacity]) {
                Ok(0) => {
                    deadline.check()?;
                    return Ok(bytes);
                }
                Ok(count) => {
                    bytes.extend_from_slice(&chunk[..count]);
                    if bytes.len() > MAX_TASK_BYTES {
                        return Err(too_large());
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.into()),
            }
            let mut poll = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let remaining = deadline.remaining()?;
            let wait = remaining
                .as_millis()
                .saturating_add(u128::from(remaining.subsec_nanos() % 1_000_000 != 0))
                .min(i32::MAX as u128) as i32;
            let observed = unsafe { libc::poll(&mut poll, 1, wait) };
            if observed < 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted
            {
                return Err(std::io::Error::last_os_error().into());
            }
            if observed > 0 && poll.revents & libc::POLLNVAL != 0 {
                return Err(Error::new(
                    "SOURCE_UNAVAILABLE",
                    "Task input descriptor invalidated",
                    7,
                ));
            }
        }
    })();
    // A failed restoration is an input failure, never a successful context.
    input.restore()?;
    result
}

#[cfg(windows)]
fn file_bytes(path: &Path, deadline: Deadline) -> Result<Vec<u8>> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_DISK, GetFileType};
    let mut file = File::open(path)?;
    deadline.check()?;
    if unsafe { GetFileType(file.as_raw_handle().cast()) } != FILE_TYPE_DISK {
        return Err(unsupported());
    }
    regular(&file, true)?;
    read_disk(&mut file, deadline)
}

#[cfg(windows)]
fn stdin_bytes(deadline: Deadline) -> Result<Vec<u8>> {
    use std::{
        os::windows::io::{AsRawHandle, FromRawHandle},
        ptr,
    };
    use windows_sys::Win32::{
        Foundation::{
            DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_BROKEN_PIPE, ERROR_MORE_DATA,
            GetLastError,
        },
        Storage::FileSystem::{FILE_TYPE_DISK, FILE_TYPE_PIPE, GetFileType, ReadFile},
        System::{Pipes::PeekNamedPipe, Threading::GetCurrentProcess},
    };
    deadline.check()?;
    let stdin = std::io::stdin();
    let mut duplicate = ptr::null_mut();
    if unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            stdin.as_raw_handle().cast(),
            GetCurrentProcess(),
            &mut duplicate,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut file = unsafe { File::from_raw_handle(duplicate.cast()) };
    let kind = unsafe { GetFileType(duplicate) };
    if kind == FILE_TYPE_DISK {
        regular(&file, false)?;
        return read_disk(&mut file, deadline);
    }
    if kind != FILE_TYPE_PIPE {
        return Err(unsupported());
    }
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        deadline.check()?;
        let mut available = 0;
        if unsafe {
            PeekNamedPipe(
                duplicate,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                &mut available,
                ptr::null_mut(),
            )
        } == 0
        {
            if unsafe { GetLastError() } == ERROR_BROKEN_PIPE {
                deadline.check()?;
                return Ok(bytes);
            }
            return Err(std::io::Error::last_os_error().into());
        }
        if available == 0 {
            std::thread::sleep(deadline.remaining()?.min(Duration::from_millis(5)));
            continue;
        }
        let capacity = chunk
            .len()
            .min(available as usize)
            .min(MAX_TASK_BYTES + 1 - bytes.len());
        let mut count = 0;
        let success = unsafe {
            ReadFile(
                duplicate,
                chunk.as_mut_ptr(),
                capacity as u32,
                &mut count,
                ptr::null_mut(),
            )
        };
        if success == 0 {
            let code = unsafe { GetLastError() };
            if code == ERROR_BROKEN_PIPE {
                deadline.check()?;
                return Ok(bytes);
            }
            if code != ERROR_MORE_DATA || count == 0 {
                return Err(std::io::Error::from_raw_os_error(code as i32).into());
            }
        }
        if count == 0 {
            return Err(Error::new(
                "SOURCE_UNAVAILABLE",
                "Task input read made no progress",
                7,
            ));
        }
        bytes.extend_from_slice(&chunk[..count as usize]);
        if bytes.len() > MAX_TASK_BYTES {
            return Err(too_large());
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn file_bytes(_: &Path, _: Deadline) -> Result<Vec<u8>> {
    Err(unsupported())
}
#[cfg(not(any(unix, windows)))]
fn stdin_bytes(_: Deadline) -> Result<Vec<u8>> {
    Err(unsupported())
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use crate::project::{Config, ProjectConfig};

    #[test]
    fn phase_explicit_file_expiry_after_actual_chunk_prevents_further_reads() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("input");
        let body = vec![b'x'; 65536];
        fs::write(&path, &body).unwrap();
        let mut file = fs::File::open(&path).unwrap();
        let deadline = Deadline::from_millis(200).unwrap();
        let mut reads = 0;
        let error = bounded_reader_bytes(&mut file, body.len(), Some(deadline), |count| {
            assert_eq!(count, 8192);
            reads += 1;
            while let Ok(left) = deadline.remaining() {
                std::thread::sleep(left.min(Duration::from_millis(5)));
            }
            Ok(())
        })
        .unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("TIMEOUT", 7));
        assert_eq!(reads, 1);
        assert_eq!(fs::read(&path).unwrap(), body);
    }
    #[test]
    fn explicit_file_cap_detects_growth_after_metadata_at_each_caller_limit() {
        for limit in [65536, 256 * 1024, 1024 * 1024] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("input");
            fs::write(&path, vec![b'x'; limit]).unwrap();
            assert_eq!(bounded_file_bytes(&path, limit, None).unwrap().len(), limit);
            let error = bounded_file_bytes_observed(&path, limit, None, || {
                use std::io::Write;
                fs::OpenOptions::new()
                    .append(true)
                    .open(&path)?
                    .write_all(b"growth")?;
                Ok(())
            })
            .unwrap_err();
            assert_eq!((error.code.as_str(), error.exit), ("FILE_TOO_LARGE", 2));
            assert_eq!(fs::metadata(&path).unwrap().len(), (limit + 6) as u64);
        }
    }

    #[test]
    fn explicit_file_cap_rejects_large_archive_before_read_and_keeps_original_expiry() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("archive");
        fs::File::create(&path).unwrap().set_len(268435457).unwrap();
        let error = bounded_file_bytes_observed(&path, 268435456, None, || {
            panic!("Oversize archive reached body")
        })
        .unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("FILE_TOO_LARGE", 2));
        let expired = Deadline::from_instant(std::time::Instant::now() - Duration::from_secs(1));
        let original = expired.instant();
        let error = bounded_file_bytes(&temp.path().join("missing"), 1, Some(expired)).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("TIMEOUT", 7));
        assert_eq!(expired.instant(), original);
        fs::write(&path, b"input").unwrap();
        let deadline = Deadline::from_millis(1).unwrap();
        let original = deadline.instant();
        let error = bounded_file_bytes_observed(&path, 16, Some(deadline), || {
            std::thread::sleep(Duration::from_millis(5));
            Ok(())
        })
        .unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("TIMEOUT", 7));
        assert_eq!(deadline.instant(), original);
    }

    #[test]
    fn explicit_file_transport_preserves_literal_dash_and_regular_file_symlink_semantics() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("-");
        fs::write(&path, b"literal filename").unwrap();
        assert_eq!(
            bounded_file_bytes(&path, 32, None).unwrap(),
            b"literal filename"
        );
        let error = bounded_file_bytes(temp.path(), 32, None).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        #[cfg(unix)]
        {
            let alias = temp.path().join("alias");
            std::os::unix::fs::symlink(&path, &alias).unwrap();
            assert_eq!(
                bounded_file_bytes(&alias, 32, None).unwrap(),
                b"literal filename"
            );
            let fifo = temp.path().join("fifo");
            let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            let error = bounded_file_bytes(&fifo, 32, None).unwrap_err();
            assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        }
    }

    #[test]
    fn project_read_identity_rejects_replace_and_restore_even_with_same_bytes() {
        for replacement_text in ["different bytes", "original bytes"] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let deadline = Deadline::from_millis(10_000).unwrap();
            let project = Project {
                deadline: Some(deadline),
                root_anchor: RootAnchor::capture(&root).unwrap(),
                data_dir: root.join("data"),
                workspace_dir: root.join("workspace"),
                control_dir: root.join("control"),
                root: root.clone(),
                project_id: "identity-fixture".into(),
                workspace_id: "ws".into(),
                coordination_id: "coord".into(),
                config: Config {
                    schema_version: 1,
                    project: ProjectConfig {
                        id: "identity-fixture".into(),
                        name: "fixture".into(),
                    },
                    index: Default::default(),
                    policy: Default::default(),
                    search: Default::default(),
                    context: Default::default(),
                    roles: Default::default(),
                },
            };
            let path = root.join("task.txt");
            let original = root.join("original.txt");
            let replacement = root.join("replacement.txt");
            fs::write(&path, "original bytes").unwrap();
            fs::write(&replacement, replacement_text).unwrap();
            let authority = TaskAuthority::Project {
                project: Box::new(project),
                relative: "task.txt".into(),
            };
            let mut phases = Vec::new();
            let result = authority.read_observed(deadline, |phase| {
                phases.push(phase.to_owned());
                match phase {
                    "pinned" => {
                        fs::rename(&path, &original)?;
                        fs::rename(&replacement, &path)?;
                    }
                    "read" => {
                        fs::rename(&path, &replacement)?;
                        fs::rename(&original, &path)?;
                    }
                    _ => unreachable!(),
                }
                Ok(())
            });
            assert_eq!(phases, ["pinned", "read"]);
            assert_eq!(result.err().unwrap().code, "CONCURRENT_MODIFICATION");
            assert_eq!(fs::read_to_string(&path).unwrap(), "original bytes");
        }
    }
}

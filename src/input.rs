//! Bounded explicit task-document input under the existing request deadline.
//!
//! Stdin belongs to this synchronous request. Calls through this module are
//! serialized; callers must not concurrently read stdin through another API.
//! Unix dup shares file-status flags, so nonblocking mode is restored before
//! returning, with an RAII fallback on errors/panic. No reader worker survives
//! expiry. Regular disk I/O remains cooperatively bounded, not hard-cancelled.
use crate::{
    deadline::Deadline,
    domain::{Error, Result},
};
use std::{
    fs::File,
    io::Read,
    sync::{Mutex, TryLockError},
    time::Duration,
};

pub const MAX_TASK_BYTES: usize = 1024 * 1024;
static STDIN_OWNER: Mutex<()> = Mutex::new(());

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

pub fn task_document(path: &str, deadline: Deadline) -> Result<String> {
    deadline.check()?;
    let bytes = if path == "-" {
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
        stdin_bytes(deadline)
    } else {
        file_bytes(path, deadline)
    };
    deadline.check()?;
    let text = String::from_utf8(bytes?);
    deadline.check()?;
    text.map_err(|_| Error::new("UNSUPPORTED_ENCODING", "Task document must be UTF-8", 2))
}

#[cfg(unix)]
fn file_bytes(path: &str, deadline: Deadline) -> Result<Vec<u8>> {
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
fn file_bytes(path: &str, deadline: Deadline) -> Result<Vec<u8>> {
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
fn file_bytes(_: &str, _: Deadline) -> Result<Vec<u8>> {
    Err(unsupported())
}
#[cfg(not(any(unix, windows)))]
fn stdin_bytes(_: Deadline) -> Result<Vec<u8>> {
    Err(unsupported())
}

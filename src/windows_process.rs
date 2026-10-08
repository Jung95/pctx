//! Windows containment primitives, with suspended, atomically assigned process admission.
//!
//! A named-job receipt is not a lease-release proof. An empty accounting snapshot
//! must be combined with a sealed admission phase and durable guardian ownership.
//! SID privacy is not authentication against other processes of the same user,
//! administrators, or a forged same-user receipt; guardian sealing remains required.
//! Closing these handles does not kill children: neither breakaway nor
//! KILL_ON_JOB_CLOSE is enabled. Explicit cancellation still requires a later zero
//! ActiveProcesses observation. No PID-only identity or missing-job inference exists.
//!
//! Native assumptions: Microsoft job-objects and QueryInformationJobObject docs:
//! https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
//! https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-queryinformationjobobject
//! https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-terminatejobobject
//! Creation FILETIME is an unsigned 64-bit identity component, not a boot identity:
//! https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes
#![cfg(windows)]

use std::{
    ffi::c_void,
    io,
    mem::{size_of, size_of_val},
    ptr,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, FILETIME, GetLastError, HANDLE, LocalFree},
    Security::{
        ACCESS_ALLOWED_ACE, ACL,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetSecurityInfo, SDDL_REVISION_1, SE_KERNEL_OBJECT,
        },
        DACL_SECURITY_INFORMATION, GetAce, GetSecurityDescriptorControl, GetTokenInformation,
        OWNER_SECURITY_INFORMATION, PSID, SE_DACL_PROTECTED, SECURITY_ATTRIBUTES, TOKEN_QUERY,
        TOKEN_USER, TokenUser,
    },
    System::{
        JobObjects::{
            CreateJobObjectW, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicAccountingInformation,
            JobObjectExtendedLimitInformation, OpenJobObjectW, QueryInformationJobObject,
            SetInformationJobObject, TerminateJobObject,
        },
        Threading::{
            GetCurrentProcess, GetProcessId, GetProcessTimes, OpenProcess, OpenProcessToken,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};

// Documented JOB_OBJECT_ALL_ACCESS = 0x1f001f (no undocumented rights).
// https://learn.microsoft.com/en-us/windows/win32/procthread/job-object-security-and-access-rights
// Numeric constants keep the dependency surface limited to the native APIs used.
const JOB_ALL_ACCESS: u32 = 0x001f_001f;
const JOB_QUERY: u32 = 0x0004;
const READ_CONTROL_RIGHT: u32 = 0x0002_0000;
const JOB_PREFIX: &str = "Local\\PCTX-";

#[derive(Debug)]
struct Handle(HANDLE);
impl Handle {
    // Only successful native constructors call this. Never accepts external handles.
    fn from_native(raw: HANDLE) -> io::Result<Self> {
        if raw.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn sid_string(sid: PSID) -> io::Result<String> {
    let mut raw = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut raw) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let allocation = LocalAllocation(raw.cast());
    let mut len = 0;
    // Windows emits a short NUL-terminated SID. Bound even this OS-owned string.
    while len < 1024 && unsafe { *raw.add(len) } != 0 {
        len += 1;
    }
    if len == 1024 {
        return Err(io::Error::other("unbounded SID representation"));
    }
    let result = String::from_utf16(unsafe { std::slice::from_raw_parts(raw, len) })
        .map_err(|_| io::Error::other("invalid SID representation"));
    drop(allocation);
    result
}
/// The current process token's user, not an environment-supplied account name.
pub fn current_user_sid() -> io::Result<String> {
    let mut token = ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = Handle::from_native(token)?;
    let mut needed = 0;
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut needed);
    }
    if needed < size_of::<TOKEN_USER>() as u32 || needed > 65536 {
        return Err(io::Error::other("invalid token-user size"));
    }
    // Alignment matters: TOKEN_USER contains a native pointer.
    let mut data = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            data.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let user = unsafe { &*data.as_ptr().cast::<TOKEN_USER>() };
    sid_string(user.User.Sid)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobIdentity {
    pub name: String,
    pub owner_sid: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainmentObservation {
    Live {
        active_processes: u32,
    },
    /// A successful instantaneous accounting query, never a standalone release proof.
    EmptyProven,
    Unknown {
        reason: &'static str,
        win32_error: Option<u32>,
    },
}
fn unknown(reason: &'static str, error: io::Error) -> ContainmentObservation {
    ContainmentObservation::Unknown {
        reason,
        win32_error: error.raw_os_error().map(|v| v as u32),
    }
}
fn observe(handle: &Handle) -> ContainmentObservation {
    if let Err(error) = validate_limits(handle) {
        return unknown("job_limits_invalid", error);
    }
    let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
    let mut returned = 0;
    if unsafe {
        QueryInformationJobObject(
            handle.0,
            JobObjectBasicAccountingInformation,
            (&mut info as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
            size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
            &mut returned,
        )
    } == 0
    {
        return unknown("accounting_query_failed", io::Error::last_os_error());
    }
    if returned as usize != size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() {
        return ContainmentObservation::Unknown {
            reason: "incomplete_accounting",
            win32_error: None,
        };
    }
    match info.ActiveProcesses {
        0 => ContainmentObservation::EmptyProven,
        active_processes => ContainmentObservation::Live { active_processes },
    }
}

/// A newly created empty job, admitting at most one root process.
/// There is no external raw-handle adoption API.
#[derive(Debug)]
pub struct PreparedJob {
    handle: Handle,
    identity: JobIdentity,
    admission_started: bool,
}
impl PreparedJob {
    pub fn create() -> io::Result<Self> {
        let owner_sid = current_user_sid()?;
        let identity = JobIdentity {
            name: format!("{JOB_PREFIX}{}", uuid::Uuid::new_v4()),
            owner_sid,
        };
        let sddl = wide(&format!(
            "O:{}D:P(A;;0x{JOB_ALL_ACCESS:x};;;{})",
            identity.owner_sid, identity.owner_sid
        ));
        let mut sd = ptr::null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let allocation = LocalAllocation(sd);
        let attrs = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };
        let name = wide(&identity.name);
        let raw = unsafe { CreateJobObjectW(&attrs, name.as_ptr()) };
        let last_error = unsafe { GetLastError() };
        let handle = Handle::from_native(raw)?;
        drop(allocation);
        if last_error == ERROR_ALREADY_EXISTS {
            return Err(io::Error::other("job name already exists"));
        }
        // Explicit zero limits: no breakaway or kill-on-close, including silent breakaway.
        let limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        if unsafe {
            SetInformationJobObject(
                handle.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        validate_security(&handle, &identity.owner_sid)?;
        validate_limits(&handle)?;
        Ok(Self {
            handle,
            identity,
            admission_started: false,
        })
    }
    pub fn identity(&self) -> &JobIdentity {
        &self.identity
    }
    pub fn observe(&self) -> ContainmentObservation {
        observe(&self.handle)
    }
    /// Explicit cancellation only. Zero accounting is checked after termination.
    /// A bounded timeout leaves Live/Unknown intact; it never grants release.
    pub fn cancel_and_observe(&self, exit_code: u32, wait: Duration) -> ContainmentObservation {
        if unsafe { TerminateJobObject(self.handle.0, exit_code) } == 0 {
            return unknown("termination_failed", io::Error::last_os_error());
        }
        let deadline = Instant::now() + wait.min(Duration::from_secs(30));
        loop {
            let observed = self.observe();
            if !matches!(observed, ContainmentObservation::Live { .. })
                || Instant::now() >= deadline
            {
                return observed;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
fn validate_limits(handle: &Handle) -> io::Result<()> {
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    let mut returned = 0;
    if unsafe {
        QueryInformationJobObject(
            handle.0,
            JobObjectExtendedLimitInformation,
            (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            &mut returned,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    if returned as usize != size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>()
        || limits.BasicLimitInformation.LimitFlags != 0
    {
        return Err(io::Error::other("unexpected job limits"));
    }
    Ok(())
}
fn validate_security(handle: &Handle, expected_sid: &str) -> io::Result<()> {
    let mut owner = ptr::null_mut();
    let mut dacl: *mut ACL = ptr::null_mut();
    let mut sd = ptr::null_mut();
    let error = unsafe {
        GetSecurityInfo(
            handle.0,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut dacl,
            ptr::null_mut(),
            &mut sd,
        )
    };
    if error != 0 {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    let _allocation = LocalAllocation(sd);
    let mut control = 0;
    let mut revision = 0;
    if owner.is_null()
        || sid_string(owner)? != expected_sid
        || unsafe { GetSecurityDescriptorControl(sd, &mut control, &mut revision) } == 0
        || control & SE_DACL_PROTECTED == 0
        || dacl.is_null()
        || unsafe { (*dacl).AceCount } != 1
    {
        return Err(io::Error::other("job security identity mismatch"));
    }
    let mut raw_ace = ptr::null_mut();
    if unsafe { GetAce(dacl, 0, &mut raw_ace) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let ace = unsafe { &*raw_ace.cast::<ACCESS_ALLOWED_ACE>() };
    // ACCESS_ALLOWED_ACE_TYPE = 0; disallow all inheritance/audit/object ACE variants.
    if ace.Header.AceType != 0
        || ace.Header.AceFlags != 0
        || ace.Mask != JOB_ALL_ACCESS
        || sid_string(ptr::addr_of!(ace.SidStart).cast_mut().cast())? != expected_sid
    {
        return Err(io::Error::other("job DACL is not private"));
    }
    Ok(())
}

/// Reopened query-only job. It cannot cancel or change limits/security.
#[derive(Debug)]
pub struct JobObserver {
    handle: Handle,
}
impl JobObserver {
    pub fn reopen(identity: &JobIdentity) -> Result<Self, ContainmentObservation> {
        let Some(suffix) = identity.name.strip_prefix(JOB_PREFIX) else {
            return Err(ContainmentObservation::Unknown {
                reason: "invalid_job_name",
                win32_error: None,
            });
        };
        if uuid::Uuid::parse_str(suffix).is_err() {
            return Err(ContainmentObservation::Unknown {
                reason: "invalid_job_name",
                win32_error: None,
            });
        }
        let sid = current_user_sid().map_err(|e| unknown("user_identity_unavailable", e))?;
        if sid != identity.owner_sid {
            return Err(ContainmentObservation::Unknown {
                reason: "foreign_owner",
                win32_error: None,
            });
        }
        let name = wide(&identity.name);
        let handle = Handle::from_native(unsafe {
            OpenJobObjectW(JOB_QUERY | READ_CONTROL_RIGHT, 0, name.as_ptr())
        })
        .map_err(|e| unknown("job_open_failed", e))?;
        validate_security(&handle, &sid).map_err(|e| unknown("job_security_invalid", e))?;
        validate_limits(&handle).map_err(|e| unknown("job_limits_invalid", e))?;
        Ok(Self { handle })
    }
    pub fn observe(&self) -> ContainmentObservation {
        observe(&self.handle)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub creation_filetime: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityObservation {
    Matches,
    Mismatch { observed: ProcessIdentity },
    Unknown { win32_error: Option<u32> },
}
/// An owned, query-only process handle pins the process object across PID reuse.
#[derive(Debug)]
pub struct OwnedProcess {
    handle: Handle,
}
impl OwnedProcess {
    pub fn open(pid: u32) -> io::Result<Self> {
        let handle =
            Handle::from_native(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) })?;
        Ok(Self { handle })
    }
    pub fn identity(&self) -> io::Result<ProcessIdentity> {
        let pid = unsafe { GetProcessId(self.handle.0) };
        if pid == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        if unsafe {
            GetProcessTimes(
                self.handle.0,
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(ProcessIdentity {
            pid,
            creation_filetime: (u64::from(creation.dwHighDateTime) << 32)
                | u64::from(creation.dwLowDateTime),
        })
    }
    pub fn compare(&self, expected: ProcessIdentity) -> IdentityObservation {
        match self.identity() {
            Ok(observed) if observed == expected => IdentityObservation::Matches,
            Ok(observed) => IdentityObservation::Mismatch { observed },
            Err(e) => IdentityObservation::Unknown {
                win32_error: e.raw_os_error().map(|v| v as u32),
            },
        }
    }
}

/// Explicit native launch inputs. The caller must validate executable/script
/// fingerprints and cwd policy before admission; this module does not grant trust.
/// Arguments target the Microsoft CRT quoting convention, not cmd.exe syntax.
#[derive(Debug)]
pub struct NativeSpawnRequest {
    pub executable: std::path::PathBuf,
    pub args: Vec<std::ffi::OsString>,
    pub environment: std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
    pub cwd: std::path::PathBuf,
}

fn os_wide(value: &std::ffi::OsStr) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let units: Vec<u16> = value.encode_wide().take(65537).collect();
    if units.len() > 65536 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "launch value exceeds admission budget",
        ));
    }
    if units.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "NUL in launch input",
        ));
    }
    Ok(units)
}
fn command_line(request: &NativeSpawnRequest) -> io::Result<Vec<u16>> {
    let mut out = Vec::new();
    let executable = os_wide(request.executable.as_os_str())?;
    if executable.contains(&u16::from(b'"')) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "quote in executable path",
        ));
    }
    out.push(u16::from(b'"'));
    out.extend(executable);
    out.push(u16::from(b'"'));
    // https://learn.microsoft.com/en-us/cpp/c-language/parsing-c-command-line-arguments
    // Quote every argument, doubling trailing backslashes and backslashes before quotes.
    for arg in &request.args {
        out.extend([u16::from(b' '), u16::from(b'"')]);
        let mut slashes = 0;
        for unit in os_wide(arg)? {
            if unit == u16::from(b'\\') {
                slashes += 1;
                continue;
            }
            if unit == u16::from(b'"') {
                out.extend(std::iter::repeat_n(u16::from(b'\\'), slashes * 2 + 1));
            } else {
                out.extend(std::iter::repeat_n(u16::from(b'\\'), slashes));
            }
            slashes = 0;
            out.push(unit);
        }
        out.extend(std::iter::repeat_n(u16::from(b'\\'), slashes * 2));
        out.push(u16::from(b'"'));
        if out.len() >= 32767 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "command line exceeds native bound",
            ));
        }
    }
    if out.len() >= 32767 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "command line exceeds native bound",
        ));
    }
    out.push(0);
    Ok(out)
}
fn environment_block(request: &NativeSpawnRequest) -> io::Result<Vec<u16>> {
    let mut entries = std::collections::BTreeMap::new();
    let mut total_units = 2usize;
    for (name, value) in &request.environment {
        // Explicit environment: no parent inheritance or hidden drive-current-dir entries.
        // Restrict names to ASCII for deterministic Windows case-insensitive ordering.
        let name = name.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "non-ASCII environment name")
        })?;
        if name.is_empty() || !name.is_ascii() || name.contains(['=', '\0']) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid environment name",
            ));
        }
        let value = os_wide(value)?;
        total_units = total_units
            .saturating_add(name.len())
            .saturating_add(value.len())
            .saturating_add(2);
        if total_units > 65536 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "environment exceeds admission budget",
            ));
        }
        if entries
            .insert(name.to_ascii_uppercase(), (name, value))
            .is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "duplicate case-insensitive environment name",
            ));
        }
    }
    let mut block = Vec::new();
    for (name, value) in entries.values() {
        block.extend(name.encode_utf16());
        block.push(u16::from(b'='));
        block.extend(value);
        block.push(0);
        // Module budget, not a claim about the OS's Unicode environment maximum.
        if block.len() > 65535 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "environment exceeds admission budget",
            ));
        }
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    Ok(block)
}
fn pipe() -> io::Result<(Handle, Handle)> {
    use windows_sys::Win32::System::Pipes::CreatePipe;
    let attrs = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: ptr::null_mut(),
        bInheritHandle: 1,
    };
    let mut read = ptr::null_mut();
    let mut write = ptr::null_mut();
    if unsafe { CreatePipe(&mut read, &mut write, &attrs, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((Handle::from_native(read)?, Handle::from_native(write)?))
}
fn noninheritable(handle: &Handle) -> io::Result<()> {
    use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
    if unsafe { SetHandleInformation(handle.0, HANDLE_FLAG_INHERIT, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn duplicate_owned(handle: &Handle) -> io::Result<Handle> {
    use windows_sys::Win32::Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle};
    let mut raw = ptr::null_mut();
    if unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle.0,
            GetCurrentProcess(),
            &mut raw,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Handle::from_native(raw)
}
fn pipe_file(handle: Handle) -> std::fs::File {
    use std::os::windows::io::FromRawHandle;
    let owned = std::mem::ManuallyDrop::new(handle);
    // Private ownership transfer from a successful CreatePipe, never external adoption.
    unsafe { std::fs::File::from_raw_handle(owned.0) }
}
struct Attributes {
    storage: Vec<usize>,
    initialized: bool,
}
impl Attributes {
    fn pointer(&mut self) -> windows_sys::Win32::System::Threading::LPPROC_THREAD_ATTRIBUTE_LIST {
        self.storage.as_mut_ptr().cast()
    }
    fn new() -> io::Result<Self> {
        use windows_sys::Win32::System::Threading::InitializeProcThreadAttributeList;
        let mut bytes = 0;
        unsafe {
            InitializeProcThreadAttributeList(ptr::null_mut(), 2, 0, &mut bytes);
        }
        if bytes == 0 || bytes > 65536 {
            return Err(io::Error::other("invalid native attribute-list size"));
        }
        let mut attrs = Self {
            storage: vec![0; bytes.div_ceil(size_of::<usize>())],
            initialized: false,
        };
        if unsafe { InitializeProcThreadAttributeList(attrs.pointer(), 2, 0, &mut bytes) } == 0 {
            // DeleteProcThreadAttributeList requires successfully initialized storage.
            return Err(io::Error::last_os_error());
        }
        attrs.initialized = true;
        Ok(attrs)
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                windows_sys::Win32::System::Threading::DeleteProcThreadAttributeList(
                    self.pointer(),
                );
            }
        }
    }
}
impl PreparedJob {
    /// Windows 10+/Server 2016+: atomic JOB_LIST assignment plus CREATE_SUSPENDED.
    /// https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute
    /// No shell, PATH search, fallback unassigned process, or automatic resume.
    /// An error is not a lease-release proof: caller must retain guardian/lease
    /// ownership until an independent observation proves admission is closed/empty.
    pub fn spawn_suspended(&mut self, request: &NativeSpawnRequest) -> io::Result<SuspendedChild> {
        use windows_sys::Win32::System::Threading::{
            CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
            EXTENDED_STARTUPINFO_PRESENT, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION, STARTF_USESTDHANDLES,
            STARTUPINFOEXW, STARTUPINFOW, UpdateProcThreadAttribute,
        };
        if self.admission_started || self.observe() != ContainmentObservation::EmptyProven {
            return Err(io::Error::other("job admission unavailable"));
        }
        if !request.executable.is_absolute()
            || !request.cwd.is_absolute()
            || !request
                .executable
                .extension()
                .is_some_and(|e| e.to_str().is_some_and(|e| e.eq_ignore_ascii_case("exe")))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "explicit absolute executable and cwd required",
            ));
        }
        let mut command = command_line(request)?;
        let environment = environment_block(request)?;
        let mut application = os_wide(request.executable.as_os_str())?;
        application.push(0);
        let mut cwd = os_wide(request.cwd.as_os_str())?;
        cwd.push(0);
        let (stdout_read, stdout_write) = pipe()?;
        let (stderr_read, stderr_write) = pipe()?;
        let (stdin_read, stdin_write) = pipe()?;
        noninheritable(&stdout_read)?;
        noninheritable(&stderr_read)?;
        drop(stdin_write); // Child stdin is a valid read handle already at EOF.
        let job = duplicate_owned(&self.handle)?;
        let jobs = [job.0];
        let handles = [stdin_read.0, stdout_write.0, stderr_write.0];
        // Values must outlive the attribute list (Microsoft Update... contract).
        let mut attrs = Attributes::new()?;
        for (kind, raw, bytes) in [
            (
                PROC_THREAD_ATTRIBUTE_JOB_LIST,
                jobs.as_ptr().cast::<c_void>(),
                size_of_val(&jobs),
            ),
            (
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                handles.as_ptr().cast::<c_void>(),
                size_of_val(&handles),
            ),
        ] {
            if unsafe {
                UpdateProcThreadAttribute(
                    attrs.pointer(),
                    0,
                    kind as usize,
                    raw,
                    bytes,
                    ptr::null_mut(),
                    ptr::null(),
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        let startup = STARTUPINFOEXW {
            StartupInfo: STARTUPINFOW {
                cb: size_of::<STARTUPINFOEXW>() as u32,
                dwFlags: STARTF_USESTDHANDLES,
                hStdInput: stdin_read.0,
                hStdOutput: stdout_write.0,
                hStdError: stderr_write.0,
                ..STARTUPINFOW::default()
            },
            lpAttributeList: attrs.pointer(),
        };
        let mut info = PROCESS_INFORMATION::default();
        self.admission_started = true;
        let success = unsafe {
            CreateProcessW(
                application.as_ptr(),
                command.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                1,
                CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT,
                environment.as_ptr().cast(),
                cwd.as_ptr(),
                &startup.StartupInfo,
                &mut info,
            )
        };
        let creation_error = io::Error::last_os_error();
        drop(attrs);
        drop(stdin_read);
        drop(stdout_write);
        drop(stderr_write);
        if success == 0 {
            return Err(creation_error);
        }
        // Successful CreateProcessW returns owned real process/thread handles.
        let process = OwnedProcess {
            handle: Handle(info.hProcess),
        };
        let child = SuspendedChild {
            process,
            thread: Some(Handle(info.hThread)),
            job: Some(job),
            stdout: Some(pipe_file(stdout_read)),
            stderr: Some(pipe_file(stderr_read)),
            resume_attempted: false,
            cleanup_confirmed: false,
        };
        child.verify_membership()?;
        Ok(child)
    }
}

/// Owned suspended process and capture pipes. Drop requests cancellation; it
/// never authorizes resource release. Unknown cleanup retains the job handle.
/// Pipe readers must be drained concurrently by the bounded capture layer.
#[derive(Debug)]
pub struct SuspendedChild {
    process: OwnedProcess,
    thread: Option<Handle>,
    job: Option<Handle>,
    stdout: Option<std::fs::File>,
    stderr: Option<std::fs::File>,
    resume_attempted: bool,
    cleanup_confirmed: bool,
}
impl SuspendedChild {
    fn verify_membership(&self) -> io::Result<()> {
        let mut member = 0;
        if unsafe {
            windows_sys::Win32::System::JobObjects::IsProcessInJob(
                self.process.handle.0,
                self.job.as_ref().expect("owned job").0,
                &mut member,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if member == 0 {
            return Err(io::Error::other("atomic job assignment not observed"));
        }
        Ok(())
    }
    pub fn identity(&self) -> io::Result<ProcessIdentity> {
        self.process.identity()
    }
    pub fn take_stdout(&mut self) -> Option<std::fs::File> {
        self.stdout.take()
    }
    pub fn take_stderr(&mut self) -> Option<std::fs::File> {
        self.stderr.take()
    }
    /// # Safety
    /// Caller must first persist and verify durable guardian ACK bound to this
    /// exact process creation identity, job receipt and still-owned resource lease.
    /// This native primitive cannot validate that external protocol itself.
    /// Isolated native fixtures with no production lease may test resume while
    /// retaining owning job/process handles and ensuring observed cancellation;
    /// such tests do not certify durable guardian attachment.
    pub unsafe fn resume_after_guardian_ack(&mut self) -> io::Result<()> {
        if self.resume_attempted || self.cleanup_confirmed {
            return Err(io::Error::other("child admission already resolved"));
        }
        self.verify_membership()?;
        let job = self.job.as_ref().expect("owned job");
        validate_limits(job)?;
        self.resume_attempted = true;
        let previous = unsafe {
            windows_sys::Win32::System::Threading::ResumeThread(
                self.thread.as_ref().expect("suspended thread").0,
            )
        };
        if previous == u32::MAX {
            let error = io::Error::last_os_error();
            self.reject_and_observe(1, Duration::from_secs(1));
            return Err(error);
        }
        if previous != 1 {
            // Unexpected suspend state cannot be certified. Request cleanup immediately.
            self.reject_and_observe(1, Duration::from_secs(1));
            return Err(io::Error::other("unexpected primary thread suspend count"));
        }
        self.thread.take();
        Ok(())
    }
    /// Returns an unsigned exit code only after the process handle is signaled.
    /// STILL_ACTIVE=259 is never used as a liveness heuristic.
    pub fn wait_exit(&self, wait: Duration) -> io::Result<Option<u32>> {
        use windows_sys::Win32::{
            Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
            System::Threading::{GetExitCodeProcess, WaitForSingleObject},
        };
        let result = unsafe {
            WaitForSingleObject(self.process.handle.0, wait.as_millis().min(30000) as u32)
        };
        if result == WAIT_TIMEOUT {
            return Ok(None);
        }
        if result != WAIT_OBJECT_0 {
            return Err(io::Error::last_os_error());
        }
        let mut code = 0;
        if unsafe { GetExitCodeProcess(self.process.handle.0, &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Some(code))
    }
    pub fn observe_containment(&mut self) -> ContainmentObservation {
        let observed = observe(self.job.as_ref().expect("owned job"));
        if observed == ContainmentObservation::EmptyProven {
            match self.wait_exit(Duration::ZERO) {
                Ok(Some(_)) => self.cleanup_confirmed = true,
                Ok(None) => {
                    return ContainmentObservation::Unknown {
                        reason: "root_not_exited",
                        win32_error: None,
                    };
                }
                Err(e) => return unknown("root_exit_unavailable", e),
            }
        }
        observed
    }
    pub fn reject_and_observe(&mut self, exit_code: u32, wait: Duration) -> ContainmentObservation {
        self.resume_attempted = true; // Rejection permanently closes admission, even if cleanup is unknown.
        let deadline = Instant::now() + wait.min(Duration::from_secs(30));
        let handle = self.job.as_ref().expect("owned job");
        let terminated = unsafe { TerminateJobObject(handle.0, exit_code) };
        let error = io::Error::last_os_error();
        // Also terminate the owned root if a membership observation failed.
        unsafe {
            windows_sys::Win32::System::Threading::TerminateProcess(
                self.process.handle.0,
                exit_code,
            );
        }
        if terminated == 0 {
            return unknown("termination_failed", error);
        }
        let _ = self.wait_exit(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(1)),
        );
        loop {
            let observed = self.observe_containment();
            if !matches!(observed, ContainmentObservation::Live { .. })
                || Instant::now() >= deadline
            {
                return observed;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for SuspendedChild {
    fn drop(&mut self) {
        if !self.cleanup_confirmed
            && self.reject_and_observe(1, Duration::from_secs(1))
                != ContainmentObservation::EmptyProven
        {
            // Never turn unknown containment into a last-handle-close inference.
            // Guardian/lease release remains the caller's responsibility.
            if let Some(job) = self.job.take() {
                std::mem::forget(job);
            }
        }
    }
}

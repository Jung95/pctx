//! Windows containment primitives, deliberately without process creation or assignment.
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
    mem::size_of,
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

/// A newly created empty job. No public handle adoption, assignment, or launch API.
#[derive(Debug)]
pub struct PreparedJob {
    handle: Handle,
    identity: JobIdentity,
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
        Ok(Self { handle, identity })
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

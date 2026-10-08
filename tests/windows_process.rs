//! Native Windows API fixtures only. Unix runs contain no simulated assertions.
#![cfg(windows)]
use pctx::windows_process::{
    ContainmentObservation, IdentityObservation, JobIdentity, JobObserver, OwnedProcess,
    PreparedJob, ProcessIdentity, current_user_sid,
};
use std::{mem::size_of, ptr, time::Duration};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ACCESS_DENIED, HANDLE, LocalFree},
    Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        SECURITY_ATTRIBUTES,
    },
    System::{JobObjects::CreateJobObjectW, Threading::GetCurrentProcessId},
};

struct NativeFixture(HANDLE);
impl Drop for NativeFixture {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn fixture_job(sddl: &str) -> (NativeFixture, JobIdentity) {
    let identity = JobIdentity {
        name: format!("Local\\PCTX-{}", uuid::Uuid::new_v4()),
        owner_sid: current_user_sid().unwrap(),
    };
    let sddl: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    let name: Vec<u16> = identity.name.encode_utf16().chain(Some(0)).collect();
    let mut sd = ptr::null_mut();
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                ptr::null_mut(),
            )
        },
        0
    );
    let attrs = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd,
        bInheritHandle: 0,
    };
    let raw = unsafe { CreateJobObjectW(&attrs, name.as_ptr()) };
    // Capture immediately: LocalFree may overwrite the thread last-error value.
    // https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
    let creation_error = std::io::Error::last_os_error();
    unsafe {
        LocalFree(sd);
    }
    assert!(
        !raw.is_null(),
        "native fixture job creation failed: {}",
        creation_error
    );
    (NativeFixture(raw), identity)
}

#[test]
fn native_empty_job_reopens_with_private_security_and_zero_limits() {
    let job = PreparedJob::create().unwrap();
    assert_eq!(job.observe(), ContainmentObservation::EmptyProven);
    let observer = JobObserver::reopen(job.identity()).unwrap();
    assert_eq!(observer.observe(), ContainmentObservation::EmptyProven);
    // Exercise explicit native termination and subsequent accounting, not an
    // inference from successful TerminateJobObject alone. No process is launched.
    assert_eq!(
        job.cancel_and_observe(17, Duration::from_millis(100)),
        ContainmentObservation::EmptyProven
    );
}

#[test]
fn missing_job_is_unknown_never_empty() {
    let identity = JobIdentity {
        name: format!("Local\\PCTX-{}", uuid::Uuid::new_v4()),
        owner_sid: current_user_sid().unwrap(),
    };
    assert!(matches!(
        JobObserver::reopen(&identity),
        Err(ContainmentObservation::Unknown {
            reason: "job_open_failed",
            win32_error: Some(_)
        })
    ));
}

#[test]
fn foreign_receipt_and_malformed_names_do_not_open() {
    let job = PreparedJob::create().unwrap();
    let mut identity = job.identity().clone();
    identity.owner_sid = "S-1-5-18".to_owned();
    // Do not assume the test account is not SYSTEM.
    if identity.owner_sid == current_user_sid().unwrap() {
        identity.owner_sid = "S-1-0-0".to_owned();
    }
    assert!(matches!(
        JobObserver::reopen(&identity),
        Err(ContainmentObservation::Unknown {
            reason: "foreign_owner",
            ..
        })
    ));
    identity.name = "Local\\PCTX-invalid\0ignored".to_owned();
    assert!(matches!(
        JobObserver::reopen(&identity),
        Err(ContainmentObservation::Unknown {
            reason: "invalid_job_name",
            ..
        })
    ));
}

#[test]
fn broad_native_dacl_cannot_impersonate_private_job() {
    let sid = current_user_sid().unwrap();
    let (_native, identity) = fixture_job(&format!("O:{sid}D:P(A;;GA;;;WD)"));
    assert!(matches!(
        JobObserver::reopen(&identity),
        Err(ContainmentObservation::Unknown {
            reason: "job_security_invalid",
            ..
        })
    ));
}

#[test]
fn native_access_denial_is_unknown() {
    let sid = current_user_sid().unwrap();
    // The creator receives its creation handle; an empty DACL denies a fresh
    // query open. Owners may read security but are not granted JOB_OBJECT_QUERY.
    let (_native, identity) = fixture_job(&format!("O:{sid}D:P"));
    assert!(matches!(
        JobObserver::reopen(&identity),
        Err(ContainmentObservation::Unknown {
            reason: "job_open_failed",
            win32_error: Some(ERROR_ACCESS_DENIED)
        })
    ));
}

#[test]
fn query_handle_retains_object_then_final_drop_destroys_empty_job() {
    let job = PreparedJob::create().unwrap();
    let identity = job.identity().clone();
    let observer = JobObserver::reopen(&identity).unwrap();
    drop(job);
    assert_eq!(observer.observe(), ContainmentObservation::EmptyProven);
    let second = JobObserver::reopen(&identity).unwrap();
    drop(second);
    drop(observer);
    assert!(matches!(
        JobObserver::reopen(&identity),
        Err(ContainmentObservation::Unknown {
            reason: "job_open_failed",
            ..
        })
    ));
}

#[test]
fn owned_process_pins_unsigned_creation_identity_and_detects_mismatch() {
    let pid = unsafe { GetCurrentProcessId() };
    let process = OwnedProcess::open(pid).unwrap();
    let first = process.identity().unwrap();
    assert_eq!(first.pid, pid);
    assert!(first.creation_filetime > u32::MAX as u64);
    assert_eq!(first, process.identity().unwrap());
    assert_eq!(process.compare(first), IdentityObservation::Matches);
    assert_eq!(
        process.compare(ProcessIdentity {
            pid,
            creation_filetime: first.creation_filetime ^ 1
        }),
        IdentityObservation::Mismatch { observed: first }
    );
    assert!(OwnedProcess::open(0).is_err());
    // Dropping one independently opened handle must not invalidate another.
    let second = OwnedProcess::open(pid).unwrap();
    drop(process);
    assert_eq!(second.identity().unwrap(), first);
}

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

fn launch_fixture(
    temp: &tempfile::TempDir,
    mode: &str,
    payload: &[&str],
) -> pctx::windows_process::NativeSpawnRequest {
    use std::{collections::BTreeMap, ffi::OsString};
    let args: Vec<OsString> = ["--exact", "--nocapture", "--", "fixture_child_entry"]
        .into_iter()
        .chain(payload.iter().copied())
        .map(OsString::from)
        .collect();
    let expected: Vec<String> = args
        .iter()
        .map(|s| s.to_str().unwrap().to_owned())
        .collect();
    let mut environment: BTreeMap<OsString, OsString> = std::env::vars_os()
        .filter(|(k, _)| {
            k.to_str()
                .is_some_and(|k| k.is_ascii() && !k.contains('=') && !k.is_empty())
        })
        .collect();
    // All fixture paths and values are generated test data, not command trust grants.
    environment.insert("PCTX_NATIVE_FIXTURE_MODE".into(), mode.into());
    environment.insert(
        "PCTX_NATIVE_FIXTURE_DIR".into(),
        temp.path().as_os_str().to_owned(),
    );
    environment.insert(
        "PCTX_NATIVE_FIXTURE_ARGS".into(),
        serde_json::to_string(&expected).unwrap().into(),
    );
    environment.insert("PCTX_NATIVE_FIXTURE_UNICODE".into(), "환경-雪-😀".into());
    pctx::windows_process::NativeSpawnRequest {
        executable: std::env::current_exe().unwrap(),
        args,
        environment,
        cwd: temp.path().to_owned(),
    }
}

// A real native child target in this test executable. The exact libtest filter
// means the payload arguments cannot recursively execute the parent fixtures.
#[test]
fn fixture_child_entry() {
    let Ok(mode) = std::env::var("PCTX_NATIVE_FIXTURE_MODE") else {
        return;
    };
    let dir = std::path::PathBuf::from(std::env::var_os("PCTX_NATIVE_FIXTURE_DIR").unwrap());
    if mode == "descendant" {
        std::fs::write(dir.join("descendant-marker"), "descendant ran").unwrap();
        std::thread::sleep(Duration::from_secs(60));
        return;
    }
    let actual: Vec<String> = std::env::args().skip(1).collect();
    let expected: Vec<String> =
        serde_json::from_str(&std::env::var("PCTX_NATIVE_FIXTURE_ARGS").unwrap()).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        std::env::current_dir().unwrap().canonicalize().unwrap(),
        dir.canonicalize().unwrap()
    );
    assert_eq!(
        std::env::var("PCTX_NATIVE_FIXTURE_UNICODE").unwrap(),
        "환경-雪-😀"
    );
    use std::io::Read;
    let mut input = String::new();
    assert_eq!(std::io::stdin().read_to_string(&mut input).unwrap(), 0);
    if mode == "spawn-descendant" {
        let descendant: std::os::windows::io::OwnedHandle =
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "fixture_child_entry", "--nocapture"])
                .env("PCTX_NATIVE_FIXTURE_MODE", "descendant")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap()
                .into();
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        while !dir.join("descendant-marker").exists() {
            if std::time::Instant::now() >= deadline {
                use std::os::windows::io::AsRawHandle;
                unsafe {
                    windows_sys::Win32::System::Threading::TerminateProcess(
                        descendant.as_raw_handle(),
                        1,
                    );
                    windows_sys::Win32::System::Threading::WaitForSingleObject(
                        descendant.as_raw_handle(),
                        1000,
                    );
                }
                panic!("descendant did not start");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Intentionally exit this root with a live descendant. Containment, not
        // root exit or pipe EOF, must govern the parent test's cleanup proof.
    }
    std::fs::write(
        dir.join("root-marker"),
        serde_json::to_vec(&actual).unwrap(),
    )
    .unwrap();
    if mode == "exit259" {
        std::process::exit(259);
    }
    println!("native fixture output: 雪");
    eprintln!("native fixture stderr");
}

fn fixture_ack_and_resume(
    child: &mut pctx::windows_process::SuspendedChild,
    job: &PreparedJob,
    temp: &tempfile::TempDir,
) {
    use std::io::Write;
    // Test-local durable receipt only; this does not certify production guardian
    // attachment. This isolated fixture owns no production resource lease.
    let identity = child.identity().unwrap();
    let mut receipt = std::fs::File::create(temp.path().join("fixture-admission-receipt")).unwrap();
    write!(
        receipt,
        "{} {} {}",
        job.identity().name,
        identity.pid,
        identity.creation_filetime
    )
    .unwrap();
    receipt.sync_all().unwrap();
    // SAFETY: controlled fixture; this test retains the owning job/process and
    // always observes native cleanup. Production must supply its real ACK first.
    unsafe {
        child.resume_after_guardian_ack().unwrap();
    }
}

#[test]
fn suspended_native_admission_roundtrips_crt_unicode_arguments_and_pipes() {
    use std::io::Read;
    let temp = tempfile::tempdir().unwrap();
    let request = launch_fixture(
        &temp,
        "echo",
        &[
            "",
            "white space",
            "quote\"inside",
            "backslash\\\"quote",
            "trailing\\\\",
            "雪😀",
        ],
    );
    let mut job = PreparedJob::create().unwrap();
    let mut child = job.spawn_suspended(&request).unwrap();
    assert_eq!(
        job.observe(),
        ContainmentObservation::Live {
            active_processes: 1
        }
    );
    std::thread::sleep(Duration::from_millis(100));
    assert!(!temp.path().join("root-marker").exists());
    assert_eq!(child.wait_exit(Duration::ZERO).unwrap(), None);
    let mut stdout = child.take_stdout().unwrap();
    let mut stderr = child.take_stderr().unwrap();
    fixture_ack_and_resume(&mut child, &job, &temp);
    assert_eq!(child.wait_exit(Duration::from_secs(15)).unwrap(), Some(0));
    let mut out = String::new();
    let mut err = String::new();
    stdout.read_to_string(&mut out).unwrap();
    stderr.read_to_string(&mut err).unwrap();
    assert!(out.contains("native fixture output: 雪"));
    assert!(err.contains("native fixture stderr"));
    assert!(temp.path().join("root-marker").exists());
    assert_eq!(
        child.observe_containment(),
        ContainmentObservation::EmptyProven
    );
    assert!(
        job.spawn_suspended(&request).is_err(),
        "one job cannot reopen admission after root completion"
    );
}

#[test]
fn rejection_of_suspended_child_observes_empty_without_executing_marker() {
    let temp = tempfile::tempdir().unwrap();
    let request = launch_fixture(&temp, "echo", &[]);
    let mut job = PreparedJob::create().unwrap();
    let mut child = job.spawn_suspended(&request).unwrap();
    assert_eq!(
        child.reject_and_observe(23, Duration::from_secs(15)),
        ContainmentObservation::EmptyProven
    );
    assert_eq!(child.wait_exit(Duration::ZERO).unwrap(), Some(23));
    assert!(!temp.path().join("root-marker").exists());
    // Cancellation resolved admission; it cannot subsequently resume.
    assert!(unsafe { child.resume_after_guardian_ack() }.is_err());
}

#[test]
fn descendant_stays_contained_after_root_exit_until_explicit_cancellation() {
    let temp = tempfile::tempdir().unwrap();
    let request = launch_fixture(&temp, "spawn-descendant", &[]);
    let mut job = PreparedJob::create().unwrap();
    let mut child = job.spawn_suspended(&request).unwrap();
    fixture_ack_and_resume(&mut child, &job, &temp);
    assert_eq!(child.wait_exit(Duration::from_secs(20)).unwrap(), Some(0));
    assert!(temp.path().join("descendant-marker").exists());
    assert!(matches!(
        child.observe_containment(),
        ContainmentObservation::Live {
            active_processes: 1..
        }
    ));
    assert_eq!(
        child.reject_and_observe(31, Duration::from_secs(15)),
        ContainmentObservation::EmptyProven
    );
}

#[test]
fn native_admission_rejects_ambiguous_environment_and_relative_executable() {
    let temp = tempfile::tempdir().unwrap();
    let mut request = launch_fixture(&temp, "echo", &[]);
    let mut job = PreparedJob::create().unwrap();
    request
        .environment
        .insert("pctx_duplicate".into(), "first".into());
    request
        .environment
        .insert("PCTX_DUPLICATE".into(), "second".into());
    assert!(job.spawn_suspended(&request).is_err());
    assert_eq!(job.observe(), ContainmentObservation::EmptyProven);
    request.executable = "implicit.exe".into();
    assert!(job.spawn_suspended(&request).is_err());
    assert!(!temp.path().join("root-marker").exists());
}

#[test]
fn signaled_native_exit_259_is_an_exit_code_not_liveness() {
    let temp = tempfile::tempdir().unwrap();
    let request = launch_fixture(&temp, "exit259", &[]);
    let mut job = PreparedJob::create().unwrap();
    let mut child = job.spawn_suspended(&request).unwrap();
    fixture_ack_and_resume(&mut child, &job, &temp);
    assert_eq!(child.wait_exit(Duration::from_secs(15)).unwrap(), Some(259));
    assert_eq!(
        child.observe_containment(),
        ContainmentObservation::EmptyProven
    );
}

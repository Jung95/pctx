//! Actual Windows keeper/parent/target processes. No simulated Unix coverage.
#![cfg(windows)]
use fs2::FileExt;
use pctx::{
    windows_guardian::{
        GuardianClient, GuardianConfiguration, GuardianLaunch, GuardianReceipt,
        provision_canonical_slot,
    },
    windows_process::{
        ContainmentObservation, JobIdentity, JobObserver, NativeSpawnRequest, OwnedProcess,
        PreparedJob,
    },
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn environment(mode: &str, directory: &Path) -> BTreeMap<OsString, OsString> {
    let mut env: BTreeMap<_, _> = std::env::vars_os()
        .filter(|(k, _)| k.to_str().is_some_and(|k| k.is_ascii() && !k.contains('=')))
        .collect();
    env.insert("PCTX_GUARDIAN_FIXTURE".into(), mode.into());
    env.insert(
        "PCTX_GUARDIAN_FIXTURE_DIR".into(),
        directory.as_os_str().to_owned(),
    );
    env
}
fn launch(directory: &Path) -> GuardianLaunch {
    let executable = std::env::current_exe().unwrap();
    let executable_hash = pctx::domain::hash(std::fs::read(&executable).unwrap());
    GuardianLaunch {
        executable,
        executable_hash,
        args: vec![
            "--exact".into(),
            "fixture_guardian_entry".into(),
            "--nocapture".into(),
        ],
        environment: environment("guardian", directory),
    }
}
fn setup(directory: &Path) -> (PreparedJob, GuardianConfiguration) {
    let slot = directory.join("canonical.lock");
    if !slot.exists() {
        provision_canonical_slot(&slot).unwrap();
    }
    let job = PreparedJob::create().unwrap();
    let parent = OwnedProcess::open(std::process::id())
        .unwrap()
        .identity()
        .unwrap();
    let config = GuardianConfiguration::prepare(
        directory,
        vec![slot],
        format!("JOB-{}", uuid::Uuid::new_v4()),
        pctx::domain::hash(b"fixture-profile"),
        parent,
        job.identity(),
    )
    .unwrap();
    (job, config)
}
fn request(directory: &Path, mode: &str) -> NativeSpawnRequest {
    NativeSpawnRequest {
        executable: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            "fixture_target_entry".into(),
            "--nocapture".into(),
        ],
        environment: environment(mode, directory),
        cwd: directory.to_owned(),
    }
}
fn canonical(directory: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.join("canonical.lock"))
        .unwrap()
}
fn wait_for(path: &Path, seconds: u64) {
    let end = Instant::now() + Duration::from_secs(seconds);
    while !path.exists() {
        assert!(
            Instant::now() < end,
            "fixture stage was not published: {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn wait_json<T: serde::de::DeserializeOwned>(path: &Path, timeout: Duration) -> T {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(value) = serde_json::from_slice(&bytes)
        {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "complete fixture receipt unavailable"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn registered_cli_entry_uses_owned_protocol_without_loading_project() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().canonicalize().unwrap();
    let (job, config) = setup(&directory);
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_pctx"));
    let launch = GuardianLaunch {
        executable_hash: pctx::domain::hash(std::fs::read(&executable).unwrap()),
        executable,
        args: vec!["__pctx-windows-guardian-v1".into()],
        environment: environment("cli", &directory),
    };
    let mut keeper = GuardianClient::start(config, &launch).unwrap();
    keeper.abort_before_spawn(job).unwrap();
    keeper.wait_released(Duration::from_secs(15)).unwrap();
    assert!(!directory.join(".pctx").exists());
    canonical(&directory).try_lock_exclusive().unwrap();
}
#[test]
fn fixture_guardian_entry() {
    if std::env::var("PCTX_GUARDIAN_FIXTURE").as_deref() != Ok("guardian") {
        return;
    }
    pctx::windows_guardian::run_from_stdio().unwrap();
}
#[test]
fn fixture_target_entry() {
    let Ok(mode) = std::env::var("PCTX_GUARDIAN_FIXTURE") else {
        return;
    };
    let directory = PathBuf::from(std::env::var_os("PCTX_GUARDIAN_FIXTURE_DIR").unwrap());
    if mode == "grandchild" {
        std::fs::write(directory.join("descendant-started"), b"native descendant").unwrap();
        std::thread::sleep(Duration::from_secs(60));
        return;
    }
    std::fs::write(directory.join("target-started"), b"native target").unwrap();
    if mode == "hold" {
        std::thread::sleep(Duration::from_secs(60));
        return;
    }
    if mode == "descendant-root" {
        let _owned_descendant: std::os::windows::io::OwnedHandle =
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "fixture_target_entry", "--nocapture"])
                .envs(environment("grandchild", &directory))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
                .into();
        wait_for(&directory.join("descendant-started"), 15);
        // Intentionally return with a live native descendant still inside the job.
    }
}
#[test]
fn native_keeper_durable_ack_single_use_and_descendant_bound_release() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().canonicalize().unwrap();
    let (mut job, config) = setup(&directory);
    let protocol_dir = config.protocol_dir.clone();
    let secret = serde_json::to_value(&config).unwrap()["nonce"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut keeper = GuardianClient::start(config, &launch(&directory)).unwrap();
    assert_eq!(keeper.ready().boot_id, None);
    assert!(canonical(&directory).try_lock_exclusive().is_err());
    let mut child = job
        .spawn_suspended(&request(&directory, "descendant-root"))
        .unwrap();
    assert!(!directory.join("target-started").exists());
    let ack = keeper.attach(&child).unwrap();
    assert!(ack.admission_sealed);
    assert!(!ack.containment_empty);
    let durable = std::fs::read_to_string(protocol_dir.join("ack.json")).unwrap();
    assert!(!durable.contains(&secret));
    assert_eq!(
        serde_json::from_str::<GuardianReceipt>(&durable).unwrap(),
        ack
    );
    assert!(
        keeper.attach(&child).is_err(),
        "nonce must be consumed exactly once"
    );
    // SAFETY: actual independent keeper owns the canonical slot and its identity-
    // bound ACK was flushed before this fixture allows any target instruction.
    unsafe {
        child.resume_after_guardian_ack().unwrap();
    }
    assert_eq!(child.wait_exit(Duration::from_secs(20)).unwrap(), Some(0));
    assert!(matches!(
        child.observe_containment(),
        ContainmentObservation::Live {
            active_processes: 1..
        }
    ));
    assert!(keeper.guardian_alive().unwrap());
    assert!(
        canonical(&directory).try_lock_exclusive().is_err(),
        "root exit must not release descendant ownership"
    );
    assert_eq!(
        child.reject_and_observe(7, Duration::from_secs(15)),
        ContainmentObservation::EmptyProven
    );
    let finished = keeper.wait_released(Duration::from_secs(15)).unwrap();
    assert!(finished.admission_sealed && finished.containment_empty);
    assert_eq!(finished.phase, "released");
    canonical(&directory).try_lock_exclusive().unwrap();
}
#[test]
fn sealed_empty_abort_consumes_admission_capability_before_unlock() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().canonicalize().unwrap();
    let (job, config) = setup(&directory);
    let mut keeper = GuardianClient::start(config, &launch(&directory)).unwrap();
    assert!(canonical(&directory).try_lock_exclusive().is_err());
    let ack = keeper.abort_before_spawn(job).unwrap();
    assert!(ack.admission_sealed && ack.containment_empty);
    assert_eq!(ack.child, None);
    keeper.wait_released(Duration::from_secs(15)).unwrap();
    canonical(&directory).try_lock_exclusive().unwrap();
}
#[test]
fn stale_parent_creation_identity_cannot_admit_a_keeper() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().canonicalize().unwrap();
    let (_job, mut config) = setup(&directory);
    config.parent.creation_filetime ^= 1;
    assert!(GuardianClient::start(config, &launch(&directory)).is_err());
    canonical(&directory).try_lock_exclusive().unwrap();
}
#[test]
fn fixture_parent_entry() {
    if std::env::var("PCTX_GUARDIAN_FIXTURE").as_deref() != Ok("parent") {
        return;
    }
    let directory = PathBuf::from(std::env::var_os("PCTX_GUARDIAN_FIXTURE_DIR").unwrap());
    let (mut job, config) = setup(&directory);
    let protocol = config.protocol_dir.clone();
    let mut keeper = GuardianClient::start(config, &launch(&directory)).unwrap();
    let mut target = job.spawn_suspended(&request(&directory, "hold")).unwrap();
    let ack = keeper.attach(&target).unwrap();
    // SAFETY: the actual keeper's durable ACK binds this suspended child and slot.
    unsafe {
        target.resume_after_guardian_ack().unwrap();
    }
    wait_for(&directory.join("target-started"), 15);
    let data = serde_json::json!({"job_name":job.identity().name,"job_owner_sid":job.identity().owner_sid,"guardian":ack.guardian,"protocol_dir":protocol,"admission_sealed":ack.admission_sealed});
    pctx::project::atomic_write(
        &directory.join("parent-stage.json"),
        &serde_json::to_vec(&data).unwrap(),
        false,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(60));
}
#[test]
fn native_parent_death_keeps_live_target_slot_until_job_empty() {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::{
        Foundation::WAIT_TIMEOUT,
        System::{
            JobObjects::{OpenJobObjectW, TerminateJobObject},
            Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, WaitForSingleObject},
        },
    };
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().canonicalize().unwrap();
    let mut parent = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "fixture_parent_entry", "--nocapture"])
        .envs(environment("parent", &directory))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for(&directory.join("parent-stage.json"), 40);
    let data: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("parent-stage.json")).unwrap())
            .unwrap();
    assert_eq!(data["admission_sealed"], true);
    let job = JobIdentity {
        name: data["job_name"].as_str().unwrap().into(),
        owner_sid: data["job_owner_sid"].as_str().unwrap().into(),
    };
    let observer = JobObserver::reopen(&job).unwrap();
    let guardian_pid = data["guardian"]["pid"].as_u64().unwrap() as u32;
    let guardian = OwnedProcess::open(guardian_pid).unwrap();
    assert_eq!(
        guardian.identity().unwrap().creation_filetime,
        data["guardian"]["creation_filetime"].as_u64().unwrap()
    );
    let raw = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | 0x100000,
            0,
            guardian_pid,
        )
    };
    assert!(!raw.is_null());
    let guardian_handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    parent.kill().unwrap();
    parent.wait().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(guardian_handle.as_raw_handle(), 0) },
        WAIT_TIMEOUT
    );
    assert!(matches!(
        observer.observe(),
        ContainmentObservation::Live {
            active_processes: 1..
        }
    ));
    assert!(
        canonical(&directory).try_lock_exclusive().is_err(),
        "killed parent must not unlock live target"
    );
    let name: Vec<u16> = job.name.encode_utf16().chain(Some(0)).collect();
    let raw = unsafe { OpenJobObjectW(8 | 4, 0, name.as_ptr()) };
    assert!(!raw.is_null());
    let cancellation = unsafe { OwnedHandle::from_raw_handle(raw) };
    assert_ne!(
        unsafe { TerminateJobObject(cancellation.as_raw_handle(), 19) },
        0
    );
    let end = Instant::now() + Duration::from_secs(20);
    while observer.observe() != ContainmentObservation::EmptyProven {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(10));
    }
    let protocol = PathBuf::from(data["protocol_dir"].as_str().unwrap());
    let receipt: GuardianReceipt =
        wait_json(&protocol.join("finished.json"), Duration::from_secs(20));
    assert!(receipt.admission_sealed && receipt.containment_empty);
    assert_eq!(receipt.boot_id, None);
    while canonical(&directory).try_lock_exclusive().is_err() {
        assert!(
            Instant::now() < end,
            "keeper did not release proven-empty slot"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn stop_empty_fixture_keeper(expected: pctx::windows_guardian::IdentityReceipt) {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::{
        Foundation::{FILETIME, WAIT_OBJECT_0},
        System::Threading::{
            GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, TerminateProcess,
            WaitForSingleObject,
        },
    };
    let raw = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | 0x100000 | 1,
            0,
            expected.pid,
        )
    };
    assert!(!raw.is_null());
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    assert_ne!(
        unsafe {
            GetProcessTimes(
                handle.as_raw_handle(),
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        },
        0
    );
    assert_eq!(
        (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime),
        expected.creation_filetime
    );
    // Explicit isolated fixture cleanup only, after both fixture jobs are proven
    // empty and no production resource or admission capability exists.
    assert_ne!(unsafe { TerminateProcess(handle.as_raw_handle(), 1) }, 0);
    assert_eq!(
        unsafe { WaitForSingleObject(handle.as_raw_handle(), 15000) },
        WAIT_OBJECT_0
    );
}
#[test]
fn wrong_job_attach_retains_unsealed_slot_even_when_observed_job_is_empty() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().canonicalize().unwrap();
    let (job_a, config) = setup(&directory);
    let protocol = config.protocol_dir.clone();
    let mut keeper = GuardianClient::start(config, &launch(&directory)).unwrap();
    let keeper_identity = keeper.ready().guardian;
    let mut cleanup = FixtureCleanup::new(keeper_identity, &[job_a.identity()]);
    let mut job_b = PreparedJob::create().unwrap();
    cleanup.add_job(job_b.identity());
    let mut wrong_child = job_b.spawn_suspended(&request(&directory, "hold")).unwrap();
    assert!(keeper.attach(&wrong_child).is_err());
    assert!(keeper.wait_released(Duration::from_millis(10)).is_err());
    assert!(!protocol.join("ack.json").exists());
    assert_eq!(job_a.observe(), ContainmentObservation::EmptyProven);
    assert!(
        canonical(&directory).try_lock_exclusive().is_err(),
        "empty accounting cannot replace sealed admission"
    );
    assert_eq!(
        wrong_child.reject_and_observe(3, Duration::from_secs(15)),
        ContainmentObservation::EmptyProven
    );
    drop(wrong_child);
    drop(job_b);
    drop(job_a);
    cleanup.cleanup();
    assert!(cleanup.complete);
    let end = Instant::now() + Duration::from_secs(5);
    while canonical(&directory).try_lock_exclusive().is_err() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(10));
    }
}

// Explicit test-only cleanup: never infer job emptiness from a timeout or a dead
// parent. Handles pin the actual fixture jobs while cancellation is observed.
struct FixtureCleanup {
    keeper: pctx::windows_guardian::IdentityReceipt,
    jobs: Vec<(JobObserver, std::os::windows::io::OwnedHandle)>,
    complete: bool,
}
impl FixtureCleanup {
    fn new(keeper: pctx::windows_guardian::IdentityReceipt, jobs: &[&JobIdentity]) -> Self {
        let mut cleanup = Self {
            keeper,
            jobs: Vec::new(),
            complete: false,
        };
        for identity in jobs {
            cleanup.add_job(identity);
        }
        cleanup
    }
    fn add_job(&mut self, identity: &JobIdentity) {
        use std::os::windows::io::{FromRawHandle, OwnedHandle};
        use windows_sys::Win32::System::JobObjects::OpenJobObjectW;
        let item = {
            let observer = JobObserver::reopen(identity).unwrap();
            let name: Vec<u16> = identity.name.encode_utf16().chain(Some(0)).collect();
            // The validated observer owns and pins the same named native object.
            let raw = unsafe { OpenJobObjectW(8 | 4, 0, name.as_ptr()) };
            assert!(!raw.is_null());
            (observer, unsafe { OwnedHandle::from_raw_handle(raw) })
        };
        self.jobs.push(item);
    }
    fn cleanup(&mut self) {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        if self.complete {
            return;
        }
        for (_, handle) in &self.jobs {
            // Cancellation alone never proves release eligibility.
            unsafe {
                TerminateJobObject(handle.as_raw_handle(), 23);
            }
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        while self
            .jobs
            .iter()
            .any(|(job, _)| job.observe() != ContainmentObservation::EmptyProven)
        {
            if Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Catch fixture assertions so cleanup during unwinding cannot double panic.
        self.complete = std::panic::catch_unwind(|| stop_empty_fixture_keeper(self.keeper)).is_ok();
    }
}
impl Drop for FixtureCleanup {
    fn drop(&mut self) {
        self.cleanup();
    }
}
#[test]
fn fixture_inert_keeper_entry() {
    if std::env::var("PCTX_GUARDIAN_FIXTURE").as_deref() != Ok("inert") {
        return;
    }
    let directory = PathBuf::from(std::env::var_os("PCTX_GUARDIAN_FIXTURE_DIR").unwrap());
    let identity = OwnedProcess::open(std::process::id())
        .unwrap()
        .identity()
        .unwrap();
    let receipt = pctx::windows_guardian::IdentityReceipt::from(identity);
    let pending = directory.join("inert-stage.pending");
    std::fs::write(&pending, serde_json::to_vec(&receipt).unwrap()).unwrap();
    std::fs::rename(pending, directory.join("inert-stage.json")).unwrap();
    // Intentionally never read stdin and never send a protocol receipt.
    std::thread::sleep(Duration::from_secs(60));
}
#[test]
fn native_no_reader_has_one_bounded_send_receive_deadline() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().canonicalize().unwrap();
    let (job, mut config) = setup(&directory);
    // Greater than ordinary anonymous pipe capacity, still below the 16KiB frame
    // bound. No real slots are acquired by this explicitly inert fixture.
    let long_slot = directory.join("x".repeat(230));
    config.canonical_slots = vec![long_slot; 32];
    assert!(serde_json::to_vec(&config).unwrap().len() > 8192);
    assert!(serde_json::to_vec(&config).unwrap().len() < 16384);
    let mut inert = launch(&directory);
    inert.args = vec![
        "--exact".into(),
        "fixture_inert_keeper_entry".into(),
        "--nocapture".into(),
    ];
    inert.environment = environment("inert", &directory);
    let started = Instant::now();
    let result = GuardianClient::start(config, &inert);
    let elapsed = started.elapsed();
    let identity = wait_json(&directory.join("inert-stage.json"), Duration::from_secs(5));
    let mut cleanup = FixtureCleanup::new(identity, &[job.identity()]);
    assert!(matches!(result, Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut));
    // Includes executable fingerprint/process creation overhead before IPC starts.
    assert!(
        elapsed < Duration::from_secs(12),
        "unbounded parent IPC wait: {elapsed:?}"
    );
    assert_eq!(job.observe(), ContainmentObservation::EmptyProven);
    // The timeout did not kill a keeper, invent release proof, or modify a ledger.
    assert_eq!(
        OwnedProcess::open(identity.pid)
            .unwrap()
            .identity()
            .unwrap()
            .creation_filetime,
        identity.creation_filetime
    );
    cleanup.cleanup();
    assert!(cleanup.complete);
}

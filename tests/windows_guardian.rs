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
    process::{Child, Command, Stdio},
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
    let directory = PathBuf::from(std::env::var_os("PCTX_GUARDIAN_FIXTURE_DIR").unwrap());
    let identity = OwnedProcess::open(std::process::id())
        .unwrap()
        .identity()
        .unwrap();
    publish_fixture_json(
        &directory.join("keeper-identity.json"),
        &serde_json::json!({
            "pid": identity.pid, "creation_filetime": identity.creation_filetime
        }),
    );
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
// Immutable phase files are independent of the atomic replacement API under test.
// No nonce, argv, environment, or capability is serialized into diagnostics.
fn publish_fixture_json(path: &Path, value: &serde_json::Value) {
    use std::io::Write;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .unwrap();
    file.write_all(&serde_json::to_vec(value).unwrap()).unwrap();
    file.sync_all().unwrap();
}
fn publish_parent_phase(
    directory: &Path,
    phase: &str,
    started: Instant,
    value: &mut serde_json::Value,
) {
    value["phase"] = serde_json::json!(phase);
    value["elapsed_ms"] = serde_json::json!(started.elapsed().as_millis() as u64);
    publish_fixture_json(&directory.join(format!("parent-{phase}.json")), value);
}
const PARENT_PHASES: &[&str] = &[
    "configured",
    "keeper-ready",
    "child-suspended",
    "admission-ack",
    "resumed",
    "target-started",
    "final-published",
];
fn bounded_json(path: &Path) -> Option<serde_json::Value> {
    use std::io::Read;
    let file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 64 * 1024 {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}
fn diagnostic_tail(path: &Path) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = File::open(path) else {
        return "unavailable".into();
    };
    let length = file.metadata().map(|m| m.len()).unwrap_or(0);
    if file
        .seek(SeekFrom::Start(length.saturating_sub(8192)))
        .is_err()
    {
        return "unavailable".into();
    }
    let mut bytes = Vec::new();
    if file.take(8192).read_to_end(&mut bytes).is_err() {
        return "unavailable".into();
    }
    pctx::reader::redact(&String::from_utf8_lossy(&bytes)).0
}
struct ParentFixtureCleanup {
    child: Option<Child>,
    directory: PathBuf,
    native: FixtureCleanup,
    job_known: bool,
    latest: serde_json::Value,
}
impl ParentFixtureCleanup {
    fn new(child: Child, directory: &Path) -> Self {
        Self {
            child: Some(child),
            directory: directory.into(),
            native: FixtureCleanup::pending(),
            job_known: false,
            latest: serde_json::Value::Null,
        }
    }
    fn refresh(&mut self) {
        for phase in PARENT_PHASES {
            if let Some(value) = bounded_json(&self.directory.join(format!("parent-{phase}.json")))
            {
                self.latest = value;
            }
        }
        if !self.job_known
            && let (Some(name), Some(sid)) = (
                self.latest["job_name"].as_str(),
                self.latest["job_owner_sid"].as_str(),
            )
        {
            let job = JobIdentity {
                name: name.into(),
                owner_sid: sid.into(),
            };
            self.job_known = self.native.try_add_job(&job);
        }
        if self.native.keeper.is_none()
            && let Some(value) = bounded_json(&self.directory.join("keeper-identity.json"))
            && let Ok(identity) = serde_json::from_value(value)
        {
            self.native.keeper = Some(identity);
        }
    }
    fn kill_and_reap(&mut self) -> std::io::Result<()> {
        if let Some(child) = self.child.as_mut() {
            if child.try_wait()?.is_none()
                && let Err(error) = child.kill()
                && child.try_wait()?.is_none()
            {
                return Err(error);
            }
            child.wait()?;
        }
        self.child = None;
        Ok(())
    }
    fn cleanup(&mut self) {
        // Pin the actual isolated job before removing its last parent-owned handle.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.refresh()));
        if self.kill_and_reap().is_err() {
            return;
        }
        // Recover an identity published while the helper was being stopped.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.refresh()));
        // A keeper must never be terminated without a known pinned job and EmptyProven.
        if self.native.keeper.is_some() && !self.job_known {
            return;
        }
        self.native.cleanup();
    }
}
impl Drop for ParentFixtureCleanup {
    fn drop(&mut self) {
        self.cleanup();
    }
}
fn wait_parent_stage(
    parent: &mut ParentFixtureCleanup,
    timeout: Duration,
) -> Result<serde_json::Value, String> {
    let started = Instant::now();
    loop {
        parent.refresh();
        if let Some(value) = bounded_json(&parent.directory.join("parent-stage.json")) {
            return Ok(value);
        }
        let status = parent
            .child
            .as_mut()
            .unwrap()
            .try_wait()
            .map_err(|e| format!("helper try_wait failed: {e}"))?;
        if status.is_some() || started.elapsed() >= timeout {
            return Err(format!(
                "parent-stage unavailable after {}ms; helper_status={status:?}; latest_phase={}; stdout_tail={:?}; stderr_tail={:?}",
                started.elapsed().as_millis(),
                parent.latest,
                diagnostic_tail(&parent.directory.join("parent-stdout.log")),
                diagnostic_tail(&parent.directory.join("parent-stderr.log"))
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn fixture_parent_entry() {
    if std::env::var("PCTX_GUARDIAN_FIXTURE").as_deref() != Ok("parent") {
        return;
    }
    let directory = PathBuf::from(std::env::var_os("PCTX_GUARDIAN_FIXTURE_DIR").unwrap());
    let (mut job, config) = setup(&directory);
    let protocol = config.protocol_dir.clone();
    let started = Instant::now();
    let mut phase = serde_json::json!({"job_name":job.identity().name,
        "job_owner_sid":job.identity().owner_sid,"protocol_dir":protocol});
    publish_parent_phase(&directory, "configured", started, &mut phase);
    let mut keeper = GuardianClient::start(config, &launch(&directory)).unwrap();
    phase["guardian"] = serde_json::to_value(keeper.ready().guardian).unwrap();
    publish_parent_phase(&directory, "keeper-ready", started, &mut phase);
    let mut target = job.spawn_suspended(&request(&directory, "hold")).unwrap();
    let child = target.identity().unwrap();
    phase["child"] =
        serde_json::json!({"pid":child.pid,"creation_filetime":child.creation_filetime});
    publish_parent_phase(&directory, "child-suspended", started, &mut phase);
    let ack = keeper.attach(&target).unwrap();
    phase["admission_sealed"] = serde_json::json!(ack.admission_sealed);
    publish_parent_phase(&directory, "admission-ack", started, &mut phase);
    // SAFETY: the actual keeper's durable ACK binds this suspended child and slot.
    unsafe {
        target.resume_after_guardian_ack().unwrap();
    }
    publish_parent_phase(&directory, "resumed", started, &mut phase);
    wait_for(&directory.join("target-started"), 15);
    publish_parent_phase(&directory, "target-started", started, &mut phase);
    let data = serde_json::json!({"job_name":job.identity().name,"job_owner_sid":job.identity().owner_sid,"guardian":ack.guardian,"protocol_dir":protocol,"admission_sealed":ack.admission_sealed});
    pctx::project::atomic_write(
        &directory.join("parent-stage.json"),
        &serde_json::to_vec(&data).unwrap(),
        false,
    )
    .unwrap();
    publish_parent_phase(&directory, "final-published", started, &mut phase);
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
    // Regular files cannot deadlock a helper on a full unread anonymous pipe.
    let stdout = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(directory.join("parent-stdout.log"))
        .unwrap();
    let stderr = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(directory.join("parent-stderr.log"))
        .unwrap();
    let parent = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "fixture_parent_entry", "--nocapture"])
        .envs(environment("parent", &directory))
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout.try_clone().unwrap()))
        .stderr(Stdio::from(stderr.try_clone().unwrap()))
        .spawn()
        .unwrap();
    let mut parent = ParentFixtureCleanup::new(parent, &directory);
    let data = match wait_parent_stage(&mut parent, Duration::from_secs(40)) {
        Ok(data) => data,
        Err(reason) => {
            // Keep on-disk diagnostics for the failing CI fixture, after safe cleanup.
            parent.cleanup();
            stdout.sync_all().unwrap();
            stderr.sync_all().unwrap();
            let retained = temp.keep();
            panic!(
                "{reason}; cleanup_job_pinned={}; cleanup_keeper_identity_known={}; cleanup_completed={}; fixture diagnostics retained at {}",
                parent.job_known,
                parent.native.keeper.is_some(),
                parent.native.complete,
                retained.display()
            );
        }
    };
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
    parent.kill_and_reap().unwrap();
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
    parent.native.complete = true;
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
    if unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } == WAIT_OBJECT_0 {
        return;
    }
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
    keeper: Option<pctx::windows_guardian::IdentityReceipt>,
    jobs: Vec<(JobObserver, std::os::windows::io::OwnedHandle)>,
    complete: bool,
}
impl FixtureCleanup {
    fn pending() -> Self {
        Self {
            keeper: None,
            jobs: Vec::new(),
            complete: false,
        }
    }
    fn new(keeper: pctx::windows_guardian::IdentityReceipt, jobs: &[&JobIdentity]) -> Self {
        let mut cleanup = Self::pending();
        cleanup.keeper = Some(keeper);
        for identity in jobs {
            cleanup.add_job(identity);
        }
        cleanup
    }
    fn add_job(&mut self, identity: &JobIdentity) {
        assert!(
            self.try_add_job(identity),
            "isolated fixture job could not be pinned"
        );
    }
    fn try_add_job(&mut self, identity: &JobIdentity) -> bool {
        use std::os::windows::io::{FromRawHandle, OwnedHandle};
        use windows_sys::Win32::System::JobObjects::OpenJobObjectW;
        let Ok(observer) = JobObserver::reopen(identity) else {
            return false;
        };
        let name: Vec<u16> = identity.name.encode_utf16().chain(Some(0)).collect();
        // The validated observer owns and pins the same named native object.
        let raw = unsafe { OpenJobObjectW(8 | 4, 0, name.as_ptr()) };
        if raw.is_null() {
            return false;
        }
        self.jobs
            .push((observer, unsafe { OwnedHandle::from_raw_handle(raw) }));
        true
    }
    fn cleanup(&mut self) {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        if self.complete || (self.keeper.is_some() && self.jobs.is_empty()) {
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
        if let Some(keeper) = self.keeper {
            self.complete = std::panic::catch_unwind(|| stop_empty_fixture_keeper(keeper)).is_ok();
        } else {
            self.complete = true;
        }
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

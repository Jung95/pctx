//! Registered checks exercise real CLI children; every fixture has private host slots.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
    host: PathBuf,
    task: String,
    run: String,
}
impl Fixture {
    fn new(script: &str, heavy: bool, timeout: Option<u64>) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        assert!(
            Command::new("git")
                .args(["-c", "init.templateDir=", "init", "--quiet"])
                .arg(&root)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_SYSTEM", "/dev/null")
                .status()
                .unwrap()
                .success()
        );
        let mut f = Self {
            root,
            data: temp.path().join("data"),
            host: temp.path().join("host"),
            _temp: temp,
            task: String::new(),
            run: String::new(),
        };
        f.ok(&["init"]);
        fs::write(f.root.join("fixture.sh"), script).unwrap();
        fs::write(f.root.join("code.rs"), "fn fixture() {}\n").unwrap();
        let profile = format!(
            "schema_version = 1\n[checks.unit]\nargv = ['/bin/sh', 'fixture.sh']\nreporter = 'pctx-json-v1'\nheavy = {heavy}\nresources = [{}]\n{}",
            if heavy { "'heavy-compute'" } else { "" },
            timeout
                .map(|t| format!("execution_timeout_ms = {t}\n"))
                .unwrap_or_default()
        );
        fs::write(f.root.join(".pctx/runner.toml"), profile).unwrap();
        let def = f.data.join("task.json");
        fs::write(&def,json!({"schema_version":1,"title":"Runner fixture","scope":["code.rs","fixture.sh"],"acceptance":[{"id":"behavior","description":"Actual evidence","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test","allowed_sources":["runner_observed"],"output_paths":["reports/**"]}]}).to_string()).unwrap();
        f.task = f.ok(&["task", "create", "--from-file", def.to_str().unwrap()])["data"]["task_id"]
            .as_str()
            .unwrap()
            .into();
        f.ok(&["agent", "register", "--name", "worker", "--kind", "agent"]);
        f.ok(&["task", "ready", &f.task]);
        f.ok(&["task", "assign", &f.task, "--agent", "worker"]);
        f.run = f.ok(&["task", "start", &f.task])["data"]["run_id"]
            .as_str()
            .unwrap()
            .into();
        f
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.args(["--root", self.root.to_str().unwrap(), "--format", "json"])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_HOST_RESOURCE_DIR", &self.host)
            .env("PCTX_ACTOR", "owner")
            .env("PCTX_RUNNER_DIAGNOSTICS", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env_remove("PCTX_RUN_CAPABILITY");
        c
    }
    fn output(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
    fn ok(&self, args: &[&str]) -> Value {
        let o = self.output(args);
        assert!(
            o.status.success(),
            "{args:?}: {} {}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).unwrap()
    }
    fn trust(&self) {
        let plan = self.ok(&[
            "runner",
            "check-plan",
            "--task-id",
            &self.task,
            "--key",
            "unit",
            "--run",
            &self.run,
        ]);
        self.ok(&[
            "runner",
            "trust",
            "--key",
            "unit",
            "--expect-hash",
            plan["data"]["fingerprint"].as_str().unwrap(),
        ]);
    }
    fn run_args(&self) -> Vec<&str> {
        vec![
            "runner",
            "check-run",
            "--task-id",
            &self.task,
            "--key",
            "unit",
            "--run",
            &self.run,
        ]
    }
    fn slot(&self) -> PathBuf {
        self.host.join("slots/exclusive-compute.json")
    }
    fn wait_child_process(&self, child: &mut std::process::Child) -> Value {
        use std::io::Read;
        let started = Instant::now();
        // The legacy route performs four mandatory executable fingerprint checks
        // before ACK, plus separate five-second READY and ATTACHED handshakes.
        // Eight seconds expired on native GitHub macOS with runner still alive.
        // Allow 10s protocol + 30s bounded admission/hash work + 5s publication;
        // this is a fixture allowance, not a product performance pass or weaker
        // attachment criterion. Stage diagnostics expose where that time went.
        let end = started + Duration::from_secs(45);
        loop {
            if let Ok(bytes) = fs::read(self.slot())
                && let Ok(v) = serde_json::from_slice::<Value>(&bytes)
                && v["pid"].as_u64().is_some()
                && v["guardian_attached"] == true
            {
                return v;
            }
            let exited = child.try_wait().unwrap();
            if exited.is_some() || Instant::now() >= end {
                if exited.is_none() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                let bounded = |pipe: &mut dyn Read| {
                    let mut bytes = vec![];
                    let _ = pipe.take(32768).read_to_end(&mut bytes);
                    pctx::reader::redact(&String::from_utf8_lossy(&bytes)).0
                };
                let stdout = child
                    .stdout
                    .take()
                    .map(|mut pipe| bounded(&mut pipe))
                    .unwrap_or_default();
                let stderr = child
                    .stderr
                    .take()
                    .map(|mut pipe| bounded(&mut pipe))
                    .unwrap_or_default();
                let latest = fs::read(self.slot())
                    .ok()
                    .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
                let slot=latest.map(|v|json!({"job_id":v["job_id"],"state":v["state"],"pid":v["pid"],"process_group":v["process_group"],"start_identity":v["start_identity"],"guardian_pid":v["guardian_pid"],"guardian_start":v["guardian_start"],"guardian_attached":v["guardian_attached"],"updated_at":v["updated_at"]}));
                panic!(
                    "Runner did not durably acknowledge guardian attachment; elapsed_ms={} status={exited:?} latest_slot={slot:?} stdout={stdout} bounded_stage_stderr={stderr}. Native boot/start identity must be observable; sandbox denial does not qualify as bridge support.",
                    started.elapsed().as_millis()
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn wait_child(&self) -> Value {
        let end = Instant::now() + Duration::from_secs(8);
        loop {
            if let Ok(bytes) = fs::read(self.slot())
                && let Ok(v) = serde_json::from_slice::<Value>(&bytes)
                && v["pid"].as_u64().is_some()
            {
                return v;
            }
            assert!(
                Instant::now() < end,
                "Runner never published child identity"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
fn report(tests: u64) -> String {
    let now = chrono::Utc::now().timestamp();
    format!(
        "printf '%s\\n' '{}'\n",
        json!({"schema_version":1,"check_key":"unit","producer":"isolated-fixture","source":"external_report","exit_code":0,"tests":tests,"passed":tests,"failed":0,"errors":0,"skipped":0,"result":"passed","started_at":now,"finished_at":now})
    )
}
#[test]
fn trusted_typed_check_is_real_evidence() {
    let f = Fixture::new(&report(1), false, None);
    f.trust();
    let v = f.ok(&f.run_args());
    assert_eq!(v["data"]["execution"]["child_exit_code"], 0);
    assert_ne!(v["data"]["evidence"]["result"], "unverified");
    assert!(v["data"]["execution"]["spawned"].as_bool().unwrap());
}
#[test]
fn unknown_and_zero_tests_do_not_pass() {
    for script in ["printf 'all tests passed\\n'\n".to_string(), report(0)] {
        let f = Fixture::new(&script, false, None);
        f.trust();
        let o = f.output(&f.run_args());
        if o.status.success() {
            let v: Value = serde_json::from_slice(&o.stdout).unwrap();
            assert_ne!(v["data"]["evidence"]["result"], "passed");
        } else {
            assert!(String::from_utf8_lossy(&o.stdout).contains("INVALID_ARGUMENT"));
        }
    }
}
#[test]
fn changed_script_and_agent_forged_trust_are_denied() {
    let f = Fixture::new(&report(1), false, None);
    let plan = f.ok(&[
        "runner",
        "check-plan",
        "--task-id",
        &f.task,
        "--key",
        "unit",
        "--run",
        &f.run,
    ]);
    let o = f
        .command(&[
            "runner",
            "trust",
            "--key",
            "unit",
            "--expect-hash",
            plan["data"]["fingerprint"].as_str().unwrap(),
        ])
        .env("PCTX_ACTOR", "agent")
        .output()
        .unwrap();
    assert!(!o.status.success());
    f.trust();
    fs::write(
        f.root.join("fixture.sh"),
        "touch reports/should-not-exist\n",
    )
    .unwrap();
    assert!(!f.output(&f.run_args()).status.success());
    assert!(!f.root.join("reports/should-not-exist").exists());
}
#[test]
fn cross_project_heavy_capacity_is_one() {
    let a = Fixture::new(&format!("sleep 2\n{}", report(1)), true, None);
    let mut b = Fixture::new(&report(1), true, None);
    b.host = a.host.clone();
    a.trust();
    b.trust();
    let mut first = a
        .command(&a.run_args())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    a.wait_child();
    let denied = b.output(&b.run_args());
    assert!(!denied.status.success());
    assert!(
        String::from_utf8_lossy(&denied.stdout).contains("RESOURCE_BUSY"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&denied.stdout),
        String::from_utf8_lossy(&denied.stderr)
    );
    assert!(first.wait().unwrap().success());
    assert!(!a.slot().exists());
}
#[test]
fn parent_exit_does_not_release_living_child() {
    let a = Fixture::new("sleep 30\n", true, None);
    let mut b = Fixture::new(&report(1), true, None);
    b.host = a.host.clone();
    a.trust();
    b.trust();
    let mut parent = a
        .command(&a.run_args())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let job = a.wait_child();
    parent.kill().unwrap();
    parent.wait().unwrap();
    let group = job["process_group"].as_i64().unwrap();
    assert_eq!(unsafe { libc::kill(-(group as i32), 0) }, 0);
    assert!(!b.output(&b.run_args()).status.success());
    assert!(a.slot().exists());
    unsafe {
        libc::kill(-(group as i32), libc::SIGKILL);
    } // Cleanup never treats a parent death as release evidence.
}
#[test]
fn timeout_is_observed_without_success_evidence() {
    let f = Fixture::new("sleep 5\n", true, Some(50));
    f.trust();
    let out = f.output(&f.run_args());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(7), "{v}");
    assert_eq!(v["status"], "error");
    assert_eq!(v["errors"][0]["code"], "TIMEOUT");
    assert_eq!(v["data"]["execution"]["spawned"], true);
    assert_ne!(v["data"]["execution"]["termination"], "exited");
    assert_ne!(v["data"]["evidence"]["result"], "passed");
}
#[test]
fn legacy_bridge_absence_prevents_execution() {
    let f = Fixture::new(&report(1), true, None);
    let path = f.root.join(".pctx/runner.toml");
    let text = fs::read_to_string(&path).unwrap() + "resource_backend = 'legacy'\n";
    fs::write(path, text).unwrap();
    f.trust();
    let o = f.output(&f.run_args());
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("CAPABILITY_UNAVAILABLE"));
    assert!(!f.slot().exists());
}
#[test]
fn source_mutation_during_check_is_stale() {
    let f = Fixture::new(&format!("sleep 1\n{}", report(1)), true, None);
    f.trust();
    let child = f
        .command(&f.run_args())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    f.wait_child();
    fs::write(f.root.join("code.rs"), "fn changed() {}\n").unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(
        o.status.success(),
        "{} {}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["data"]["evidence"]["result"], "stale");
}
#[test]
fn explicit_script_input_change_requires_new_trust() {
    let f = Fixture::new(&report(1), false, None);
    fs::write(f.root.join("dependency.txt"), "v1").unwrap();
    let path = f.root.join(".pctx/runner.toml");
    let text = fs::read_to_string(&path).unwrap() + "script_inputs = ['dependency.txt']\n";
    fs::write(path, text).unwrap();
    f.trust();
    fs::write(f.root.join("dependency.txt"), "v2").unwrap();
    assert!(!f.output(&f.run_args()).status.success());
}
// A finite synthetic legacy runner uses the documented canonical advisory mutex.
#[test]
fn legacy_lock_fixture() {
    use fs2::FileExt;
    let Some(path) = std::env::var_os("PCTX_TEST_LEGACY_LOCK") else {
        return;
    };
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let marker = std::env::var_os("PCTX_TEST_LEGACY_MARKER").unwrap();
    if std::env::var("PCTX_TEST_LEGACY_MODE").unwrap_or_default() == "try" {
        fs::write(
            marker,
            if file.try_lock_exclusive().is_ok() {
                "acquired"
            } else {
                "blocked"
            },
        )
        .unwrap();
        return;
    }
    file.lock_exclusive().unwrap();
    fs::write(marker, "acquired").unwrap();
    std::thread::sleep(Duration::from_secs(30));
}
fn bridge(f: &Fixture) -> PathBuf {
    let path = f.host.join("canonical.lock");
    fs::create_dir_all(&f.host).unwrap();
    fs::write(&path, "").unwrap();
    let profile = f.root.join(".pctx/runner.toml");
    let text = fs::read_to_string(&profile).unwrap()
        + &format!(
            "resource_backend = 'legacy'\n[checks.unit.bridge]\nprotocol = 'fs2-guardian-v1'\nlock_path = {:?}\n",
            fs::canonicalize(&path).unwrap().to_str().unwrap()
        );
    fs::write(profile, text).unwrap();
    path
}
fn legacy(path: &std::path::Path, marker: &std::path::Path, mode: &str) -> std::process::Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "legacy_lock_fixture", "--nocapture"])
        .env("PCTX_TEST_LEGACY_LOCK", path)
        .env("PCTX_TEST_LEGACY_MARKER", marker)
        .env("PCTX_TEST_LEGACY_MODE", mode)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}
fn wait_marker(path: &std::path::Path) {
    let end = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn canonical_legacy_and_pctx_exclude_each_other() {
    let f = Fixture::new(&format!("sleep 2\n{}", report(1)), true, None);
    let lock = bridge(&f);
    f.trust();
    let marker = f.host.join("legacy-started");
    let mut other = legacy(&lock, &marker, "hold");
    wait_marker(&marker);
    let denied = f.output(&f.run_args());
    assert!(!denied.status.success());
    assert!(
        String::from_utf8_lossy(&denied.stdout).contains("RESOURCE_BUSY"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&denied.stdout),
        String::from_utf8_lossy(&denied.stderr)
    );
    other.kill().unwrap();
    other.wait().unwrap();
    let mut check = f
        .command(&f.run_args())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    f.wait_child_process(&mut check);
    let marker = f.host.join("reciprocal");
    let mut other = legacy(&lock, &marker, "try");
    assert!(other.wait().unwrap().success());
    assert_eq!(fs::read_to_string(marker).unwrap(), "blocked");
    assert!(check.wait().unwrap().success());
}
#[test]
fn canonical_mutex_survives_main_parent_death() {
    let f = Fixture::new("sleep 30\n", true, None);
    let lock = bridge(&f);
    f.trust();
    let mut parent = f
        .command(&f.run_args())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let job = f.wait_child_process(&mut parent);
    parent.kill().unwrap();
    parent.wait().unwrap();
    let marker = f.host.join("still-held");
    let mut other = legacy(&lock, &marker, "try");
    assert!(other.wait().unwrap().success());
    assert_eq!(fs::read_to_string(marker).unwrap(), "blocked");
    let group = job["process_group"].as_i64().unwrap();
    unsafe {
        libc::kill(-(group as i32), libc::SIGKILL);
    }
    if let Some(guardian) = job["guardian_pid"].as_i64() {
        // Fixture cleanup after the entire execution group is killed; no product unlock inference.
        unsafe {
            libc::kill(guardian as i32, libc::SIGKILL);
        }
    }
}
#[test]
fn synthetic_low_memory_defers_heavy_admission() {
    let f = Fixture::new(&report(1), true, None);
    let path = f.root.join("memory.json");
    fs::write(&path,json!({"schema_version":1,"source":"fixture","available_bytes":1,"pressure":"normal","sampled_at":chrono::Utc::now().timestamp()}).to_string()).unwrap();
    let profile = f.root.join(".pctx/runner.toml");
    let text = fs::read_to_string(&profile).unwrap()
        + "[checks.unit.memory]\nsource = 'fixture'\nfixture_path = 'memory.json'\nminimum_available_bytes = 1024\n";
    fs::write(profile, text).unwrap();
    f.trust();
    let o = f.output(&f.run_args());
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("MEMORY_PRESSURE"));
    assert!(!f.slot().exists());
}
fn linked_provider(f: &Fixture) -> PathBuf {
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .args([
                    "-c",
                    "core.hooksPath=/dev/null",
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=fixture@example.invalid"
                ])
                .args(args)
                .current_dir(&f.root)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_SYSTEM", "/dev/null")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
                .success()
        );
    };
    git(&["add", "code.rs", "fixture.sh", ".pctx/config.toml"]);
    git(&["commit", "--quiet", "-m", "fixture"]);
    let target = f._temp.path().join("provider-worktree");
    git(&[
        "worktree",
        "add",
        "--detach",
        target.to_str().unwrap(),
        "HEAD",
    ]);
    let o = Command::new(env!("CARGO_BIN_EXE_pctx"))
        .args([
            "--root",
            target.to_str().unwrap(),
            "--format",
            "json",
            "init",
        ])
        .env("PCTX_DATA_DIR", &f.data)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let profile = format!(
        "schema_version = 1\n[checks.unit]\nargv = ['/bin/sh','fixture.sh']\nreporter = 'pctx-json-v1'\nresources = ['aux-agent']\n[checks.unit.auxiliary_provider]\nkind = 'local'\nworkspace = {:?}\nscope = ['code.rs']\n",
        fs::canonicalize(&target).unwrap().to_str().unwrap()
    );
    fs::write(f.root.join(".pctx/runner.toml"), profile).unwrap();
    target
}
#[test]
fn cloud_helper_is_only_durable_intent() {
    fn snapshot(root: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Option<Vec<u8>>> {
        fn walk(
            base: &std::path::Path,
            dir: &std::path::Path,
            values: &mut std::collections::BTreeMap<PathBuf, Option<Vec<u8>>>,
        ) {
            values.insert(dir.strip_prefix(base).unwrap().into(), None);
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                assert!(!entry.file_type().unwrap().is_symlink());
                if path.is_dir() {
                    walk(base, &path, values);
                } else {
                    values.insert(
                        path.strip_prefix(base).unwrap().into(),
                        Some(fs::read(path).unwrap()),
                    );
                }
            }
        }
        let mut values = std::collections::BTreeMap::new();
        walk(root, root, &mut values);
        values
    }
    let f = Fixture::new(&report(1), false, None);
    for mode in ["cloud", "native"] {
        let v = f.ok(&[
            "runner",
            "helper-request",
            "--task-id",
            &f.task,
            "--key",
            "unit",
            "--run",
            &f.run,
            "--mode",
            mode,
            "--scope",
            "code.rs",
        ]);
        assert_eq!(v["data"]["helper"]["state"], "queued_intent");
        assert_eq!(v["data"]["model_started"], false);
        assert_eq!(v["data"]["slot_allocated"], false);
        let id = v["data"]["helper"]["helper_id"].as_str().unwrap();
        let status = f.ok(&["runner", "helper-status", id]);
        assert_eq!(status["data"]["helper"]["started"], false);
        let before = snapshot(&f.data);
        let denied = f.output(&[
            "runner",
            "helper-request",
            "--task-id",
            &f.task,
            "--key",
            "unit",
            "--run",
            &f.run,
            "--mode",
            mode,
            "--scope",
            "../escape",
        ]);
        assert_eq!(denied.status.code(), Some(5));
        let v: Value = serde_json::from_slice(&denied.stdout).unwrap();
        assert_eq!(v["errors"][0]["code"], "PATH_OUTSIDE_ROOT");
        assert_eq!(snapshot(&f.data), before);
        assert!(!f.host.exists());
    }
}
#[test]
fn local_auxiliary_provider_needs_registered_linked_workspace_and_capacity() {
    let f = Fixture::new(&format!("sleep 2\n{}", report(1)), false, None);
    let _target = linked_provider(&f);
    f.trust();
    let args = [
        "runner",
        "helper-request",
        "--task-id",
        &f.task,
        "--key",
        "unit",
        "--run",
        &f.run,
    ];
    let child = f
        .command(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let slot = f.host.join("slots/aux-agent.json");
    let end = Instant::now() + Duration::from_secs(8);
    loop {
        if let Ok(bytes) = fs::read(&slot)
            && serde_json::from_slice::<Value>(&bytes).is_ok_and(|v| v["pid"].as_u64().is_some())
        {
            break;
        }
        assert!(Instant::now() < end, "Auxiliary provider did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
    let denied = f.output(&args);
    assert!(!denied.status.success());
    assert!(
        String::from_utf8_lossy(&denied.stdout).contains("RESOURCE_BUSY"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&denied.stdout),
        String::from_utf8_lossy(&denied.stderr)
    );
    let o = child.wait_with_output().unwrap();
    assert!(
        o.status.success(),
        "{} {}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["data"]["helper"]["started"], true);
    assert_eq!(v["data"]["helper"]["state"], "finished_observed");
    assert!(v["data"]["resources_released"].as_bool().unwrap());
    assert!(!slot.exists());
}
#[test]
fn killed_guardian_causes_negative_child_observation() {
    let f = Fixture::new("sleep 30\n", true, None);
    bridge(&f);
    f.trust();
    let mut child = f
        .command(&f.run_args())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let job = f.wait_child_process(&mut child);
    let guardian = job["guardian_pid"].as_i64().unwrap();
    unsafe {
        libc::kill(guardian as i32, libc::SIGKILL);
    }
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(7), "{o:?}");
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["status"], "error");
    assert_eq!(v["errors"][0]["code"], "RESOURCE_OWNER_UNKNOWN");
    assert_eq!(
        v["data"]["execution"]["pctx_error"],
        "RESOURCE_OWNER_UNKNOWN"
    );
    assert_ne!(v["data"]["evidence"]["result"], "passed");
}
#[test]
fn unknown_memory_requires_disclosed_exact_owner_override() {
    let f = Fixture::new(&report(1), true, None);
    fs::write(f.root.join("memory.json"),json!({"schema_version":1,"source":"fixture","available_bytes":null,"pressure":"unknown","sampled_at":chrono::Utc::now().timestamp()}).to_string()).unwrap();
    let path = f.root.join(".pctx/runner.toml");
    let text = fs::read_to_string(&path).unwrap()
        + "[checks.unit.memory]\nsource = 'fixture'\nfixture_path = 'memory.json'\n";
    fs::write(&path, &text).unwrap();
    f.trust();
    let denied = f.output(&f.run_args());
    assert!(!denied.status.success());
    assert!(String::from_utf8_lossy(&denied.stdout).contains("MEMORY_UNKNOWN"));
    fs::write(path, text + "unknown = 'owner_override'\n").unwrap();
    f.trust();
    let v = f.ok(&f.run_args());
    assert_eq!(v["data"]["memory_unknown_override"], true);
    assert_eq!(v["data"]["memory_admission"]["source"], "fixture");
    assert_eq!(
        v["data"]["memory_admission"]["available_bytes"],
        Value::Null
    );
}
#[test]
fn user_created_host_state_symlink_is_denied() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new(&report(1), true, None);
    f.trust();
    let outside = f._temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, &f.host).unwrap();
    let o = f.output(&f.run_args());
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("POLICY_DENIED"));
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}
#[test]
fn linked_slot_child_directory_cannot_redirect_state() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new(&report(1), true, None);
    f.trust();
    fs::create_dir(&f.host).unwrap();
    let outside = f._temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, f.host.join("slots")).unwrap();
    let o = f.output(&f.run_args());
    assert!(!o.status.success());
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[test]
fn inherited_canonical_descriptor_retains_lock_after_invalid_proof_and_parent_close() {
    use fs2::FileExt;
    use std::{
        io::{BufRead, BufReader},
        os::{fd::AsRawFd, unix::process::CommandExt},
    };
    struct Cleanup(std::process::Child);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let f = Fixture::new(&report(1), true, None);
    let lock_path = bridge(&f);
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    lock.try_lock_exclusive().unwrap();
    let fd = lock.as_raw_fd();
    let mut command = f.command(&[
        "runner",
        "bridge-guardian",
        "--fd",
        "197",
        "--lock-path",
        lock_path.to_str().unwrap(),
    ]);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(fd, 197) < 0 || libc::fcntl(197, libc::F_SETFD, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut guardian = Cleanup(command.spawn().unwrap());
    let mut pipe = guardian.0.stdout.take().unwrap();
    unsafe {
        let flags = libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL);
        assert!(
            flags >= 0
                && libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) >= 0
        );
    }
    let mut read = BufReader::new(&mut pipe);
    let mut lines = String::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !lines.contains("PCTX-GUARDIAN-REJECTED-PROOF-v1\n") {
        let mut line = String::new();
        match read.read_line(&mut line) {
            Ok(0) => panic!("Guardian ended without retaining inherited descriptor: {lines}"),
            Ok(_) => lines.push_str(&line),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
            Err(e) => panic!("Guardian protocol IO: {e}"),
        }
        assert!(
            lines.len() <= 512 && Instant::now() < deadline,
            "Bounded guardian protocol timeout: {lines}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(lines.starts_with("PCTX-GUARDIAN-READY-v1\n"));
    assert!(!lines.contains("PCTX-GUARDIAN-ATTACHED-v1"));
    drop(lock);
    assert!(guardian.0.try_wait().unwrap().is_none());
    let contender = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    assert!(
        contender.try_lock_exclusive().is_err(),
        "Invalid proof must retain the inherited canonical mutex after parent's descriptor closes"
    );
    // No execution was launched. Explicit fixture destruction is safe and must
    // release the last inherited reference, unlike product TTL reclamation.
    guardian.0.kill().unwrap();
    guardian.0.wait().unwrap();
    contender.try_lock_exclusive().unwrap();
}

fn submit_reviewed_evidence(f: &Fixture, check: &str) {
    f.ok(&[
        "task",
        "criterion",
        "accept",
        &f.task,
        "--criterion",
        "behavior",
        "--evidence",
        check,
    ]);
    f.ok(&["task", "submit", &f.task, "--run", &f.run]);
    f.ok(&["task", "review", &f.task, "--approve"]);
    assert_eq!(
        f.ok(&["task", "complete", &f.task, "--dry-run"])["data"]["passed"],
        true
    );
}

#[test]
fn native_pass_is_invalidated_by_external_executable_change_with_sources_unchanged() {
    use std::{io::Write, os::unix::fs::PermissionsExt};
    let f = Fixture::new(&report(1), false, None);
    let tool = f._temp.path().join("registered-shell");
    // Relocated macOS system binaries can be killed before exec due to signature trust.
    // Use a real private executable wrapper; its bytes remain outside source inventory.
    fs::write(&tool, "#!/bin/sh\nexec /bin/sh \"$@\"\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    // The executable is outside the workspace source inventory, but inside a private fixture.
    let tool = fs::canonicalize(tool).unwrap();
    fs::write(f.root.join(".pctx/runner.toml"), format!("schema_version = 1\n[checks.unit]\nargv = ['{}', 'fixture.sh']\nreporter = 'pctx-json-v1'\nheavy = false\nresources = []\n", tool.display())).unwrap();
    let source_before = fs::read(f.root.join("code.rs")).unwrap();
    let script_before = fs::read(f.root.join("fixture.sh")).unwrap();
    f.trust();
    let result = f.ok(&f.run_args());
    if result["data"]["evidence"]["result"] != "passed" {
        let diagnostics = tempfile::Builder::new()
            .prefix("pctx-native-tool-failure-")
            .tempdir_in("/tmp")
            .unwrap()
            .keep();
        fs::write(
            diagnostics.join("runner.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        if let Some(output) = result["data"]["execution"]["output_id"].as_str() {
            let full = f.output(&["output", "show", output, "--view", "full"]);
            fs::write(diagnostics.join("child-output.json"), &full.stdout).unwrap();
            fs::write(diagnostics.join("retrieval-stderr.txt"), &full.stderr).unwrap();
        }
        panic!(
            "Native initial pass failed; preserved child evidence at {}: {}",
            diagnostics.display(),
            result
        );
    }
    assert_eq!(result["data"]["evidence"]["result"], "passed");
    let check = result["data"]["check_id"].as_str().unwrap();
    let stored = f.ok(&["check", "show", check]);
    let receipt: Value = serde_json::from_str(stored["data"]["report"].as_str().unwrap()).unwrap();
    assert_eq!(
        receipt["environment_authority"],
        "trusted_local_runner_profile"
    );
    assert!(receipt["check_binding"]["execution_fingerprint"].is_string());
    submit_reviewed_evidence(&f, check);
    fs::OpenOptions::new()
        .append(true)
        .open(&tool)
        .unwrap()
        .write_all(b"# changed tool bytes\n")
        .unwrap();
    assert_eq!(fs::read(f.root.join("code.rs")).unwrap(), source_before);
    assert_eq!(fs::read(f.root.join("fixture.sh")).unwrap(), script_before);
    let gate = f.ok(&["task", "complete", &f.task, "--dry-run"]);
    assert_eq!(gate["data"]["passed"], false);
    assert!(
        gate["data"]["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "check:unit")
    );
    assert_eq!(
        f.output(&["task", "complete", &f.task]).status.code(),
        Some(10)
    );
    assert_eq!(
        f.ok(&["task", "show", &f.task])["data"]["checks"][0]["result"],
        "stale"
    );
}

#[test]
fn native_environment_change_or_missing_profile_invalidates_existing_pass() {
    for missing in [false, true] {
        let f = Fixture::new(&report(1), false, None);
        f.trust();
        let result = f.ok(&f.run_args());
        assert_eq!(result["data"]["evidence"]["result"], "passed");
        submit_reviewed_evidence(&f, result["data"]["check_id"].as_str().unwrap());
        let profile = f.root.join(".pctx/runner.toml");
        if missing {
            fs::remove_file(profile).unwrap();
        } else {
            let mut text = fs::read_to_string(&profile).unwrap();
            text.push_str("env_allowlist = ['CI']\n[checks.unit.env]\nCI = 'changed'\n");
            fs::write(&profile, text).unwrap();
            // Even newly approving the changed environment cannot bless an old execution.
            let plan = f.ok(&[
                "runner",
                "check-plan",
                "--task-id",
                &f.task,
                "--key",
                "unit",
            ]);
            f.ok(&[
                "runner",
                "trust",
                "--key",
                "unit",
                "--expect-hash",
                plan["data"]["fingerprint"].as_str().unwrap(),
            ]);
        }
        assert_eq!(
            f.ok(&["task", "complete", &f.task, "--dry-run"])["data"]["passed"],
            false
        );
    }
}

#[test]
fn profile_change_during_execution_is_stale_at_report_recording() {
    let script = format!(
        "{}printf '%s\\n' \"env_allowlist = ['CI']\" '[checks.unit.env]' \"CI = 'during-run'\" >> .pctx/runner.toml\n",
        report(1)
    );
    let f = Fixture::new(&script, false, None);
    f.trust();
    let result = f.ok(&f.run_args());
    assert_eq!(result["data"]["execution"]["child_exit_code"], 0);
    assert_eq!(result["data"]["evidence"]["result"], "stale");
}

#[test]
fn check_plan_with_finite_budget_inspects_registered_auxiliary_workspace_without_launch() {
    let f = Fixture::new(&report(1), false, None);
    let _target = linked_provider(&f);
    let output = f.output(&[
        "check",
        "plan",
        "--task-id",
        &f.task,
        "--key",
        "unit",
        "--timeout-ms",
        "10000",
    ]);
    assert!(output.status.success(), "{output:?}");
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["status"], "ok");
    assert_eq!(response["data"]["execution_started"], false);
    assert_eq!(response["data"]["resources"], json!(["aux-agent"]));
    assert!(response["data"]["script_hashes"]["fixture.sh"].is_string());
    assert!(
        !f.host.exists(),
        "A plan launched execution or acquired host resources"
    );
}

#[test]
fn finite_auxiliary_plan_refuses_fifo_metadata_without_waiting_or_launching() {
    use std::os::unix::ffi::OsStrExt;
    struct ChildGuard(Option<std::process::Child>);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if let Some(child) = &mut self.0 {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    let f = Fixture::new(&report(1), false, None);
    let target = linked_provider(&f);
    let text = fs::read_to_string(target.join(".git")).unwrap();
    let gitdir =
        fs::canonicalize(target.join(text.trim().strip_prefix("gitdir: ").unwrap())).unwrap();
    for name in ["commondir", "gitdir"] {
        let path = gitdir.join(name);
        let original = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        let native = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(native.as_ptr(), 0o600) }, 0);
        let started = Instant::now();
        let mut child = ChildGuard(Some(
            f.command(&[
                "check",
                "plan",
                "--task-id",
                &f.task,
                "--key",
                "unit",
                "--timeout-ms",
                "1000",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
        ));
        loop {
            if child.0.as_mut().unwrap().try_wait().unwrap().is_some() {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "Plan blocked opening {name} FIFO"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let output = child.0.take().unwrap().wait_with_output().unwrap();
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(9), "{name}: {response}");
        assert_eq!(response["errors"][0]["code"], "WORKSPACE_MISMATCH");
        assert!(!f.host.exists(), "Rejected plan acquired host resources");
        fs::remove_file(&path).unwrap();
        fs::write(&path, original).unwrap();
    }
}

#[test]
fn check_alias_preserves_nested_timeout_processing_error() {
    let f = Fixture::new("sleep 5\n", false, Some(50));
    f.trust();
    let out = f.output(&[
        "check",
        "run",
        "--task-id",
        &f.task,
        "--key",
        "unit",
        "--run",
        &f.run,
    ]);
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(7), "{v}");
    assert_eq!(v["status"], "error");
    assert_eq!(v["errors"][0]["code"], "TIMEOUT");
    assert_eq!(v["data"]["execution"]["spawned"], true);
    assert_eq!(v["data"]["execution"]["pctx_error"], "TIMEOUT");
    assert_ne!(v["data"]["evidence"]["result"], "passed");
}

#[test]
fn check_capture_partial_is_not_wrapper_success_in_either_route() {
    let f = Fixture::new(
        "printf x >> .pctx/fixture-invocations\nprintf '\\377\\n'\n",
        false,
        Some(5000),
    );
    f.trust();
    for argv in [
        f.run_args(),
        vec![
            "check",
            "run",
            "--task-id",
            &f.task,
            "--key",
            "unit",
            "--run",
            &f.run,
        ],
    ] {
        let out = f.output(&argv);
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(out.status.code(), Some(3), "{v}");
        assert_eq!(v["status"], "partial");
        assert_eq!(v["data"]["execution"]["spawned"], true);
        assert_eq!(v["data"]["execution"]["child_exit_code"], 0);
        assert_eq!(v["data"]["execution"]["capture_complete"], false);
        assert!(v["data"]["execution"]["pctx_error"].is_null());
        assert_ne!(v["data"]["evidence"]["result"], "passed");
    }
    assert_eq!(
        fs::read(f.root.join(".pctx/fixture-invocations")).unwrap(),
        b"xx"
    );
}

#[test]
fn observed_failed_check_is_distinct_from_processing_failure() {
    let now = chrono::Utc::now().timestamp();
    let report = json!({"schema_version":1,"check_key":"unit","producer":"isolated-fixture",
        "source":"external_report","exit_code":1,"tests":1,"passed":0,"failed":1,"errors":0,
        "skipped":0,"result":"failed","started_at":now,"finished_at":now});
    let f = Fixture::new(
        &format!("printf x >> .pctx/fixture-invocations\nprintf '%s\\n' '{report}'\nexit 1\n"),
        false,
        Some(5000),
    );
    f.trust();
    for argv in [
        f.run_args(),
        vec![
            "check",
            "run",
            "--task-id",
            &f.task,
            "--key",
            "unit",
            "--run",
            &f.run,
        ],
    ] {
        let out = f.output(&argv);
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(out.status.code(), Some(0), "{v}");
        assert_eq!(v["status"], "ok");
        assert_eq!(v["data"]["execution"]["child_exit_code"], 1);
        assert!(v["data"]["execution"]["pctx_error"].is_null());
        assert_eq!(v["data"]["evidence"]["result"], "failed");
    }
    assert_eq!(
        fs::read(f.root.join(".pctx/fixture-invocations")).unwrap(),
        b"xx"
    );
}

#[test]
fn local_helper_preserves_nested_timeout_and_partial_capture() {
    for (script, timeout, expected_exit, expected_status) in [
        ("sleep 5\n", 50, 7, "error"),
        (
            "printf x >> .pctx/fixture-invocations\nprintf '\\377\\n'\n",
            5000,
            3,
            "partial",
        ),
    ] {
        let f = Fixture::new(script, false, Some(timeout));
        let target = linked_provider(&f);
        let path = f.root.join(".pctx/runner.toml");
        let profile = fs::read_to_string(&path).unwrap().replace(
            "[checks.unit]\n",
            &format!("[checks.unit]\nexecution_timeout_ms = {timeout}\n"),
        );
        fs::write(path, profile).unwrap();
        f.trust();
        let out = f.output(&[
            "runner",
            "helper-request",
            "--task-id",
            &f.task,
            "--key",
            "unit",
            "--run",
            &f.run,
        ]);
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(out.status.code(), Some(expected_exit), "{v}");
        assert_eq!(v["status"], expected_status, "{v}");
        assert_eq!(v["data"]["execution"]["spawned"], true);
        assert_eq!(v["data"]["helper"]["started"], true);
        assert!(!f.host.join("slots/aux-agent.json").exists());
        if expected_exit == 7 {
            assert_eq!(v["errors"][0]["code"], "TIMEOUT");
            assert_eq!(v["data"]["execution"]["pctx_error"], "TIMEOUT");
        } else {
            assert_eq!(v["data"]["execution"]["child_exit_code"], 0);
            assert_eq!(v["data"]["execution"]["capture_complete"], false);
            assert!(v["data"]["execution"]["pctx_error"].is_null());
            assert_eq!(
                fs::read(target.join(".pctx/fixture-invocations")).unwrap(),
                b"x"
            );
        }
    }
}

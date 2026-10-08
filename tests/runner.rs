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
        let end = Instant::now() + Duration::from_secs(8);
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
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_string(&mut stdout);
                }
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                panic!(
                    "Runner did not durably acknowledge guardian attachment; status={exited:?} stdout={stdout} stderr={stderr}. Native boot/start identity must be observable; sandbox denial does not qualify as bridge support."
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
    let v = f.ok(&f.run_args());
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
    let f = Fixture::new(&report(1), false, None);
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
        "cloud",
        "--scope",
        "code.rs",
    ]);
    assert_eq!(v["data"]["helper"]["state"], "queued_intent");
    assert_eq!(v["data"]["model_started"], false);
    assert_eq!(v["data"]["slot_allocated"], false);
    let id = v["data"]["helper"]["helper_id"].as_str().unwrap();
    let status = f.ok(&["runner", "helper-status", id]);
    assert_eq!(status["data"]["helper"]["started"], false);
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
    assert!(
        o.status.success(),
        "{} {}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
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

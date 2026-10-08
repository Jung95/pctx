//! Linux-only spawn-boundary proof using an observed, stopped manifest reader.
#![cfg(target_os = "linux")]

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
    host: PathBuf,
    tool: PathBuf,
    task: String,
    run: String,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        let git = Command::new("git")
            .args(["-c", "init.templateDir=", "init", "--quiet"])
            .arg(&root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .unwrap();
        assert!(git.status.success(), "git init: {git:?}");
        let mut f = Self {
            root,
            data: temp.path().join("data"),
            host: temp.path().join("host"),
            tool: temp.path().join("registered-shell"),
            _temp: temp,
            task: String::new(),
            run: String::new(),
        };
        f.ok(&["init"]);
        fs::copy("/bin/sh", &f.tool).unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let report = json!({"schema_version":1,"check_key":"unit","producer":"spawn-boundary-fixture","source":"external_report","exit_code":0,"tests":1,"passed":1,"failed":0,"errors":0,"skipped":0,"result":"passed","started_at":now,"finished_at":now});
        fs::write(
            f.root.join("fixture.sh"),
            format!("printf 'started\\n' > child-started\nprintf '%s\\n' '{report}'\n"),
        )
        .unwrap();
        fs::write(f.root.join("code.rs"), "fn fixture() {}\n").unwrap();
        fs::write(f.root.join(".pctx/runner.toml"), format!(
            "schema_version = 1\n[checks.unit]\nargv = ['{}', 'fixture.sh']\nreporter = 'pctx-json-v1'\nheavy = false\nresources = ['docs-publish']\nexecution_timeout_ms = 2000\n",
            f.tool.display()
        )).unwrap();
        let def = f.data.join("task.json");
        fs::write(&def, json!({"schema_version":1,"title":"Spawn boundary fixture","scope":["code.rs","fixture.sh"],"acceptance":[{"id":"behavior","description":"Observed native execution","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test","allowed_sources":["runner_observed"],"output_paths":["reports/**"]}]}).to_string()).unwrap();
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
            .env("PCTX_MAX_FILE_BYTES", (64 * 1024 * 1024).to_string())
            .env("PCTX_ACTOR", "owner")
            .env("PCTX_RUNNER_DIAGNOSTICS", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env_remove("PCTX_RUN_CAPABILITY");
        c
    }

    fn ok(&self, args: &[&str]) -> Value {
        let o = self.command(args).output().unwrap();
        assert!(o.status.success(), "{args:?}: {}", diagnostic(&o));
        serde_json::from_slice(&o.stdout).unwrap()
    }
}

fn diagnostic(o: &Output) -> String {
    format!(
        "status={} stdout={} stderr={}",
        o.status,
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

// Never leave a stopped supervisor behind, including on assertion unwinding.
struct Supervised(Option<Child>);
impl Supervised {
    fn child(&mut self) -> &mut Child {
        self.0.as_mut().unwrap()
    }
    fn finish(&mut self, deadline: Instant) -> Output {
        while self.child().try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::yield_now();
        }
        if self.child().try_wait().unwrap().is_none() {
            let _ = unsafe { libc::kill(self.child().id() as i32, libc::SIGCONT) };
            self.child().kill().unwrap();
        }
        self.0.take().unwrap().wait_with_output().unwrap()
    }
}
impl Drop for Supervised {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = unsafe { libc::kill(child.id() as i32, libc::SIGCONT) };
            let _ = child.kill();
            if let Ok(output) = child.wait_with_output()
                && std::thread::panicking()
            {
                eprintln!("Spawn-boundary fixture cleanup: {}", diagnostic(&output));
            }
        }
    }
}

fn observed_fd(pid: u32, source: &Path) -> Option<PathBuf> {
    fs::read_dir(format!("/proc/{pid}/fd"))
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|fd| fs::read_link(fd).ok().as_deref() == Some(source))
}

fn stopped(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .is_some_and(|s| {
            s.lines().any(|line| {
                line.starts_with("State:") && line.split_whitespace().nth(1) == Some("T")
            })
        })
}

#[test]
fn external_native_tool_changed_during_final_manifest_never_spawns() {
    let f = Fixture::new();
    // Bounded ordinary source, not a FIFO or a production synchronization hook.
    // The size only makes the actual open FD easier to observe; elapsed time
    // cannot establish synchronization or make this test pass.
    let source = f.root.join("zzzz-final-manifest.txt");
    let mut file = fs::File::create(&source).unwrap();
    let chunk = [b'x'; 8192];
    for _ in 0..4096 {
        file.write_all(&chunk).unwrap();
    }
    file.sync_all().unwrap();
    drop(file);
    let source_hash = pctx::domain::hash(fs::read(&source).unwrap());
    let script_before = fs::read(f.root.join("fixture.sh")).unwrap();
    let code_before = fs::read(f.root.join("code.rs")).unwrap();
    let tool_before = pctx::domain::hash(fs::read(&f.tool).unwrap());
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
    f.ok(&[
        "runner",
        "trust",
        "--key",
        "unit",
        "--expect-hash",
        plan["data"]["fingerprint"].as_str().unwrap(),
    ]);
    let controls = fs::read_dir(f.data.join("controls"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("control.sqlite3"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    assert_eq!(controls.len(), 1);
    let db = Connection::open_with_flags(&controls[0], OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    db.busy_timeout(Duration::from_millis(10)).unwrap();
    let child = f
        .command(&[
            "runner",
            "check-run",
            "--task-id",
            &f.task,
            "--key",
            "unit",
            "--run",
            &f.run,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut supervisor = Supervised(Some(child));
    let pid = supervisor.child().id();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut check_id = None;
    let mut pinned_fd = None;
    while Instant::now() < deadline && supervisor.child().try_wait().unwrap().is_none() {
        if check_id.is_none() {
            // Check Begin hashes inputs before committing this row. Therefore
            // an FD observed after this durable row belongs to the subsequent
            // output::manifest, not the earlier check fingerprint scan.
            check_id = db
                .query_row(
                    "SELECT id FROM checks WHERE task=?1 AND key='unit' AND status='running'",
                    [&f.task],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .unwrap();
        }
        if check_id.is_some()
            && let Some(fd) = observed_fd(pid, &source)
        {
            assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGSTOP) }, 0);
            while !stopped(pid) && Instant::now() < deadline {
                std::thread::yield_now();
            }
            // Revalidate after the stop: an FD seen before signal delivery
            // could already have closed. Such a missed rendezvous never passes.
            if stopped(pid) && fs::read_link(&fd).ok().as_deref() == Some(source.as_path()) {
                pinned_fd = Some(fd);
            }
            break;
        }
        std::thread::yield_now();
    }
    if pinned_fd.is_none() {
        let _ = unsafe { libc::kill(pid as i32, libc::SIGCONT) };
        let o = supervisor.finish(deadline);
        panic!(
            "Could not pin final manifest FD; check_id={check_id:?}: {}",
            diagnostic(&o)
        );
    }
    // docs-publish exercises a real host slot without heavy-compute memory
    // admission or a guardian. Prove ownership existed before testing release.
    let slot: Value =
        serde_json::from_slice(&fs::read(f.host.join("slots/exclusive-compute.json")).unwrap())
            .unwrap();
    assert_eq!(slot["state"], "starting_or_unknown");
    assert!(slot["pid"].is_null());
    // ELF trailing bytes preserve a runnable image, so removing the final
    // binding check would execute the marker script rather than fail exec.
    let mut tool = fs::OpenOptions::new().append(true).open(&f.tool).unwrap();
    tool.write_all(b"\nspawn-boundary executable mutation\n")
        .unwrap();
    tool.sync_all().unwrap();
    drop(tool);
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGCONT) }, 0);
    let o = supervisor.finish(deadline);
    let details = diagnostic(&o);
    assert_eq!(o.status.code(), Some(9), "{details}");
    let response: Value = serde_json::from_slice(&o.stdout).expect(&details);
    assert!(
        response["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["code"] == "CONFIG_CHANGED"),
        "{details}"
    );
    assert!(
        !f.root.join("child-started").exists(),
        "Native child executed: {details}"
    );
    let (status, report): (String, String) = db
        .query_row(
            "SELECT status,report FROM checks WHERE id=?1",
            [check_id.unwrap()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "unverified", "{details}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["execution_started"], false, "{details}");
    assert_eq!(report["gate_evidence"], false, "{details}");
    assert_eq!(report["reason"], "CONFIG_CHANGED", "{details}");
    for (directory, state) in [
        (f.host.join("jobs"), "not_started"),
        (f.data.join("output-jobs"), "not_started"),
    ] {
        let entries = fs::read_dir(&directory)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1, "{directory:?}: {details}");
        let job: Value = serde_json::from_slice(&fs::read(&entries[0]).unwrap()).unwrap();
        assert_eq!(job["state"], state, "{job}: {details}");
        assert!(job["pid"].is_null(), "{job}: {details}");
        if directory.ends_with("output-jobs") {
            assert_eq!(job["spawned"], false, "{job}: {details}");
            assert_eq!(job["pctx_error"], "CONFIG_CHANGED", "{job}: {details}");
        } else {
            assert_eq!(job["job_id"], slot["job_id"], "{job}: {details}");
        }
    }
    assert_eq!(
        fs::read_dir(f.host.join("slots")).unwrap().count(),
        0,
        "{details}"
    );
    assert!(
        !f.data.join("outputs").exists(),
        "Unexpected child output artifact: {details}"
    );
    assert_eq!(fs::read(f.root.join("fixture.sh")).unwrap(), script_before);
    assert_eq!(fs::read(f.root.join("code.rs")).unwrap(), code_before);
    assert_eq!(pctx::domain::hash(fs::read(&source).unwrap()), source_hash);
    assert_ne!(pctx::domain::hash(fs::read(&f.tool).unwrap()), tool_before);
}

//! Actual CLI Ctrl-C boundaries; no in-process flag injection or rerun.
#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Fixture {
    temp: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let f = Self {
            temp: tempfile::tempdir().unwrap(),
        };
        let out = f.command(&["init"]).output().unwrap();
        assert!(out.status.success(), "{out:?}");
        f
    }
    fn path(&self, leaf: &str) -> PathBuf {
        self.temp.path().join(leaf)
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.current_dir(self.temp.path())
            .args(["--root", self.temp.path().to_str().unwrap()])
            .env("PCTX_DATA_DIR", self.path("data"))
            .env("PCTX_USER_CONFIG", self.path("absent"))
            .env("PCTX_HOST_RESOURCE_DIR", self.path("host"))
            .env("PCTX_ACTOR", "owner");
        if !args.contains(&"--format") {
            c.args(["--format", "json"]);
        }
        c.args(args);
        c
    }
}
fn wait_until(mut ready: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while !ready() {
        assert!(
            Instant::now() < end,
            "native fixture did not reach boundary"
        );
        thread::sleep(Duration::from_millis(10));
    }
}
fn cancel(mut child: Child) -> Output {
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    let end = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= end {
            child.kill().unwrap();
            let out = child.wait_with_output().unwrap();
            panic!("cancellation did not finish: {out:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
}
fn cancelled(out: &Output) -> Value {
    assert_eq!(out.status.code(), Some(130), "{out:?}");
    assert!(out.stderr.is_empty(), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["status"], "error");
    assert_eq!(v["errors"][0]["code"], "CANCELLED");
    v
}

#[test]
fn held_stdin_cancel_is_one_complete_json_and_creates_no_context() {
    let f = Fixture::new();
    let mut child = f
        .command(&["build", "--task-file", "-", "--timeout-ms", "30000"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Keep the pipe open: neither EOF nor timeout can produce this result.
    let stdin = child.stdin.take().unwrap();
    thread::sleep(Duration::from_millis(250));
    let out = cancel(child);
    drop(stdin);
    cancelled(&out);
    assert!(!f.path("data/outputs").exists());
}

#[test]
fn watch_cancel_keeps_existing_ndjson_frames_and_reports_cancelled() {
    let f = Fixture::new();
    let frames = f.path("frames");
    let child = f
        .command(&["--format", "ndjson", "board", "--watch"])
        .stdout(Stdio::from(fs::File::create(&frames).unwrap()))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_until(|| fs::metadata(&frames).is_ok_and(|m| m.len() > 0));
    let out = cancel(child);
    assert_eq!(out.status.code(), Some(130), "{out:?}");
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(error["type"], "error");
    assert_eq!(error["data"]["code"], "CANCELLED");
    let frames = fs::read_to_string(frames).unwrap();
    for frame in frames.lines() {
        let frame: Value = serde_json::from_str(frame).unwrap();
        assert_ne!(frame["type"], "error");
    }
    assert!(frames.contains("board_snapshot"));
}

#[test]
fn finite_query_cancel_reaps_owned_query_child_instead_of_waiting_for_timeout() {
    let f = Fixture::new();
    fs::create_dir(f.path(".git")).unwrap();
    let git = f.path("git");
    fs::write(&git, "#!/bin/sh\nfor arg do\ncase \"$arg\" in\nconfig) exit 1;;\nrev-parse) printf 'true\\n'; exit 0;;\nstatus) printf 'one\\n' >> query.count; printf '%s' \"$$\" > query.pid; exec /bin/sleep 30;;\nesac\ndone\nexit 1\n").unwrap();
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).unwrap();
    let path = std::env::join_paths(std::iter::once(f.temp.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .unwrap();
    let child = f
        .command(&["repo", "status", "--timeout-ms", "30000"])
        .env("PATH", path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_until(|| fs::read_to_string(f.path("query.pid")).is_ok_and(|p| p.parse::<i32>().is_ok()));
    let pid: i32 = fs::read_to_string(f.path("query.pid"))
        .unwrap()
        .parse()
        .unwrap();
    let out = cancel(child);
    cancelled(&out);
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "owned query child survived"
    );
    assert_eq!(fs::read_to_string(f.path("query.count")).unwrap(), "one\n");
}

#[test]
fn run_cancel_preserves_native_truth_artifact_and_one_execution_for_both_policies() {
    for policy in ["pctx", "child"] {
        let f = Fixture::new();
        let program = f.path("fixture");
        fs::write(&program, "#!/bin/sh\nprintf 'one\\n' >> invocations\nprintf '%s' \"$$\" > child.pid\nexec /bin/sleep 30\n").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let path = program.to_str().unwrap();
        let plan = f.command(&["trust", "plan", "--", path]).output().unwrap();
        assert!(plan.status.success(), "{plan:?}");
        let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
        let trust = f
            .command(&[
                "trust",
                "add",
                "--expect-hash",
                plan["data"]["fingerprint"].as_str().unwrap(),
                "--",
                path,
            ])
            .output()
            .unwrap();
        assert!(trust.status.success(), "{trust:?}");
        let child = f
            .command(&["run", "--exit-policy", policy, "--", path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        wait_until(|| {
            fs::read_to_string(f.path("child.pid")).is_ok_and(|p| p.parse::<i32>().is_ok())
        });
        let pid: i32 = fs::read_to_string(f.path("child.pid"))
            .unwrap()
            .parse()
            .unwrap();
        let out = cancel(child);
        let v = cancelled(&out);
        assert_eq!(v["data"]["spawned"], true);
        assert_eq!(v["data"]["termination"], "cancelled");
        assert_eq!(v["data"]["signal"], libc::SIGKILL);
        assert!(v["data"]["child_exit_code"].is_null());
        assert_eq!(v["data"]["pctx_error"], "CANCELLED");
        assert_eq!(v["data"]["raw_available"], true);
        assert_eq!(fs::read_to_string(f.path("invocations")).unwrap(), "one\n");
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "direct child survived");
        let output_id = v["data"]["output_id"].as_str().unwrap();
        let reread = f.command(&["output", "show", output_id]).output().unwrap();
        assert!(reread.status.success(), "{reread:?}");
        let reread: Value = serde_json::from_slice(&reread.stdout).unwrap();
        assert_eq!(reread["data"]["termination"], "cancelled");
        assert_eq!(reread["data"]["signal"], libc::SIGKILL);
        assert_eq!(reread["data"]["pctx_error"], "CANCELLED");
        assert_eq!(fs::read_to_string(f.path("invocations")).unwrap(), "one\n");
    }
}

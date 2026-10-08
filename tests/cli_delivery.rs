//! PCTX01 primary output delivery never becomes successful absence or a panic.
#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    io::Write,
    os::fd::FromRawFd,
    process::{Command, Output, Stdio},
};
struct Fixture {
    temp: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        Self {
            temp: tempfile::tempdir().unwrap(),
        }
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.current_dir(self.temp.path())
            .args(args)
            .env("PCTX_DATA_DIR", self.temp.path().join("data"))
            .env("PCTX_USER_CONFIG", self.temp.path().join("absent-config"));
        c
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
    fn unchanged(&self) {
        assert_eq!(fs::read_dir(self.temp.path()).unwrap().count(), 0);
    }
}
fn closed_pipe() -> Stdio {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let reader = unsafe { fs::File::from_raw_fd(fds[0]) };
    let writer = unsafe { fs::File::from_raw_fd(fds[1]) };
    drop(reader);
    Stdio::from(writer)
}
#[test]
fn undeliverable_help_and_version_return_io_exit_without_project_effects() {
    let f = Fixture::new();
    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["session", "attach", "--help"],
    ] {
        let normal = f.run(&args);
        assert_eq!(normal.status.code(), Some(0));
        assert!(!normal.stdout.is_empty() && normal.stderr.is_empty());
        let output = f.command(&args).stdout(closed_pipe()).output().unwrap();
        assert_eq!(output.status.code(), Some(7), "{args:?}: {output:?}");
        assert!(output.stderr.is_empty());
        f.unchanged();
    }
}
#[test]
fn undeliverable_stream_refusal_returns_io_exit_without_project_effects() {
    let f = Fixture::new();
    let args = ["--root", "missing", "--format", "ndjson", "init"];
    let normal = f.run(&args);
    assert_eq!(normal.status.code(), Some(2));
    assert!(normal.stdout.is_empty());
    let v: Value = serde_json::from_slice(&normal.stderr).unwrap();
    assert_eq!(v["type"], "error");
    assert_eq!(v["data"]["code"], "INVALID_ARGUMENT");
    let output = f.command(&args).stderr(closed_pipe()).output().unwrap();
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    assert!(output.stdout.is_empty());
    f.unchanged();
}
#[test]
fn undeliverable_native_hook_output_is_io_error_without_repeating_import() {
    let f = Fixture::new();
    fs::create_dir(f.temp.path().join("project")).unwrap();
    let init = f.run(&["--root", "project", "--format", "json", "init"]);
    assert!(init.status.success(), "{init:?}");
    let agent = f.run(&[
        "--root", "project", "--format", "json", "agent", "register", "--name", "fixture",
    ]);
    assert!(agent.status.success(), "{agent:?}");
    let agent: Value = serde_json::from_slice(&agent.stdout).unwrap();
    let agent = agent["data"]["agent_id"].as_str().unwrap();
    let invoke = |key: &str, broken: bool| {
        let mut c = f.command(&[
            "--root",
            "project",
            "adapter",
            "claude",
            "event",
            "--agent",
            agent,
            "--hook",
            "--idempotency-key",
            key,
        ]);
        c.stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(if broken {
                closed_pipe()
            } else {
                Stdio::piped()
            });
        let mut child = c.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(br#"{"hook_event_name":"PermissionDenied","session_id":"native-fixture"}"#)
            .unwrap();
        child.wait_with_output().unwrap()
    };
    let normal = invoke("same-event", false);
    assert_eq!(normal.status.code(), Some(0), "{normal:?}");
    assert_eq!(normal.stdout, b"{}\n");
    assert!(normal.stderr.is_empty());
    let output = invoke("same-event", true);
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    assert!(output.stderr.is_empty());
    let repeated = invoke("same-event", false);
    assert_eq!(repeated.status.code(), Some(0));
    assert_eq!(repeated.stdout, b"{}\n");
    let failed_first = invoke("failed-first", true);
    assert_eq!(failed_first.status.code(), Some(7), "{failed_first:?}");
    assert!(failed_first.stderr.is_empty());
    let recovered = invoke("failed-first", false);
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert_eq!(recovered.stdout, b"{}\n");
    let databases: Vec<_> = fs::read_dir(f.temp.path().join("data/controls"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("control.sqlite3"))
        .filter(|path| path.is_file())
        .collect();
    assert_eq!(databases.len(), 1);
    let db = rusqlite::Connection::open_with_flags(
        &databases[0],
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    for key in ["same-event", "failed-first"] {
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM adapter_receipts WHERE key=?1",
                [key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 1,
            "Delivery failure must not repeat import for {key}"
        );
    }
}
#[test]
fn failed_response_file_and_closed_diagnostic_are_io_error_without_overwrite() {
    let f = Fixture::new();
    let target = f.temp.path().join("existing.json");
    fs::write(&target, b"owner-content").unwrap();
    let args = [
        "--root",
        "missing",
        "--format",
        "json",
        "--output",
        target.to_str().unwrap(),
        "status",
    ];
    let normal = f.run(&args);
    assert!(!normal.status.success());
    assert!(normal.stdout.is_empty());
    let broken = f.command(&args).stderr(closed_pipe()).output().unwrap();
    assert_eq!(broken.status.code(), Some(7), "{broken:?}");
    assert!(broken.stdout.is_empty());
    assert_eq!(fs::read(target).unwrap(), b"owner-content");
    assert!(!f.temp.path().join("missing").exists());
    assert!(!f.temp.path().join("data").exists());
}

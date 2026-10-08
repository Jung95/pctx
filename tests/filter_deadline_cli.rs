//! Actual finite filter queries own stdin until completion or the original deadline.
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Child, ChildStdin, Command, Output, Stdio},
    time::{Duration, Instant},
};
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let f = Self {
            root: base.join("project"),
            data: base.join("data"),
            _temp: temp,
        };
        fs::create_dir(&f.root).unwrap();
        let output = f.command().arg("init").output().unwrap();
        assert!(output.status.success(), "{output:?}");
        fs::create_dir_all(f.root.join(".pctx/filters")).unwrap();
        fs::write(
            f.root.join(".pctx/filters/preview.toml"),
            r#"schema_version = 1
id = "preview"
version = "1"
priority = 1
[match]
program = "never-executed-fixture"
argv_prefix = []
stream = "both"
[parse]
kind = "lines"
[render]
max_bytes = 8192
keep_head_lines = 20
keep_tail_lines = 20
show_omission_counts = true
[[rules]]
op = "protect"
pattern = 'warning|error'
"#,
        )
        .unwrap();
        f
    }
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.args(["--format", "json", "--root"])
            .arg(&self.root)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_USER_CONFIG", self.root.join("absent-user-config"));
        c
    }
    fn apply(&self, budget: &str) -> Command {
        let mut c = self.command();
        c.args([
            "--timeout-ms",
            budget,
            "filter",
            "apply",
            "--filter",
            "preview",
            "--input",
            "-",
            "--child-exit",
            "1",
        ]);
        c
    }
    fn no_preview_effects(&self) {
        for path in [
            self.data.join("filter-bindings"),
            self.data.join("filter-fixture-reports"),
            self.data.join("outputs"),
            self.root.join("never-executed-fixture"),
        ] {
            assert!(!path.exists(), "Unexpected filter effect: {path:?}");
        }
    }
}
struct OwnedChild {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
}
impl OwnedChild {
    fn spawn(c: &mut Command) -> Self {
        let mut child = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        Self {
            child: Some(child),
            stdin,
        }
    }
    fn finish(mut self) -> Output {
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            if self.child.as_mut().unwrap().try_wait().unwrap().is_some() {
                break;
            }
            assert!(
                Instant::now() < end,
                "Finite stdin query exceeded owned three-second cleanup bound"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        self.stdin.take();
        self.child.take().unwrap().wait_with_output().unwrap()
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.stdin.take();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
#[test]
fn stalled_partial_stdin_times_out_instead_of_waiting_for_eof() {
    let f = Fixture::new();
    // Positive EOF control proves a real filter/claimed nonzero result without execution.
    let mut positive = OwnedChild::spawn(&mut f.apply("5000"));
    positive
        .stdin
        .as_mut()
        .unwrap()
        .write_all("warning 한글 E42\n".as_bytes())
        .unwrap();
    positive.stdin.take();
    let output = positive.finish();
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["child_exit_code"], 1);
    assert_eq!(value["data"]["execution_started"], false);
    assert_eq!(value["data"]["test_result"], "not_evaluated");
    assert!(value["data"]["records"].to_string().contains("E42"));
    let mut stalled = OwnedChild::spawn(&mut f.apply("250"));
    stalled
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"warning unfinished record")
        .unwrap();
    // Retain the writer: the original request must expire before EOF is possible.
    let output = stalled.finish();
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], "TIMEOUT");
    assert!(value["data"].is_null());
    f.no_preview_effects();
}
#[test]
fn finite_filter_stdin_limits_and_preflight_errors_keep_complete_envelopes() {
    let f = Fixture::new();
    for (body, code, exit) in [
        (vec![b'x'; 1024 * 1024 + 1], "FILTER_LIMIT_EXCEEDED", 3),
        (vec![0xff], "UNSUPPORTED_ENCODING", 3),
    ] {
        let path = f.root.join("stdin-bytes");
        fs::write(&path, body).unwrap();
        let output = f
            .apply("5000")
            .stdin(Stdio::from(fs::File::open(&path).unwrap()))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(exit), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["errors"][0]["code"], code);
        assert!(value["data"].is_null());
    }
    for args in [
        vec!["filter", "validate", ".pctx/filters/preview.toml"],
        vec![
            "filter",
            "apply",
            "--filter",
            "preview",
            "--input",
            "-",
            "--child-exit",
            "1",
        ],
        vec!["filter", "explain", "--", "missing-program"],
    ] {
        let missing = f.root.join("missing");
        let data = f.data.join("untouched");
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.args(["--format", "json", "--root"])
            .arg(&missing)
            .env("PCTX_DATA_DIR", &data)
            .args(["--timeout-ms", "0"])
            .args(args);
        let child = OwnedChild::spawn(&mut c);
        let output = child.finish();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["errors"][0]["code"], "INVALID_ARGUMENT");
        assert!(!missing.exists());
        assert!(!data.exists());
    }
    f.no_preview_effects();
}

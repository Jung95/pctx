//! Bounded actual-CLI watch tests; no detached subprocesses survive a fixture.
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("project");
        let data = t.path().join("data");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("code.py"), "def value(): return 1\n").unwrap();
        let f = Self {
            _temporary: t,
            root,
            data,
        };
        f.ok(&["init"]);
        f
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.args(["--root", self.root.to_str().unwrap()])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_ACTOR", "owner")
            .env_remove("PCTX_RUN_ID")
            .env_remove("PCTX_RUN_CAPABILITY");
        c
    }
    fn ok(&self, args: &[&str]) -> Value {
        let out = self
            .command(args)
            .args(["--format", "json"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn active(&self) -> (String, String) {
        let path = self.data.join("task.json");
        fs::write(&path,json!({"schema_version":1,"title":"Watch fixture","scope":["code.py"],"acceptance":[{"id":"AC1","description":"Observe","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
        let task =
            self.ok(&["task", "create", "--from-file", path.to_str().unwrap()])["data"]["task_id"]
                .as_str()
                .unwrap()
                .to_owned();
        self.ok(&[
            "agent",
            "register",
            "--name",
            "watch-worker",
            "--kind",
            "agent",
        ]);
        self.ok(&["task", "ready", &task]);
        self.ok(&["task", "assign", &task, "--agent", "watch-worker"]);
        let run = self.ok(&["task", "start", &task]);
        (
            run["data"]["run_id"].as_str().unwrap().to_owned(),
            run["data"]["lease_epoch"].to_string(),
        )
    }
    fn project(&self) -> pctx::project::Project {
        let old = std::env::var_os("PCTX_DATA_DIR");
        unsafe { std::env::set_var("PCTX_DATA_DIR", &self.data) };
        let p = pctx::project::Project::open(&self.root).unwrap();
        unsafe {
            match old {
                Some(v) => std::env::set_var("PCTX_DATA_DIR", v),
                None => std::env::remove_var("PCTX_DATA_DIR"),
            }
        }
        p
    }
    fn watch(&self, ndjson: bool) -> Running {
        let args = if ndjson {
            vec!["board", "--watch", "--format", "ndjson"]
        } else {
            vec!["board", "--watch"]
        };
        let mut child = self
            .command(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let pipe = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines() {
                match line {
                    Ok(line) => {
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Running {
            child,
            rx,
            reader: Some(reader),
        }
    }
}
struct Running {
    child: Child,
    rx: Receiver<String>,
    reader: Option<std::thread::JoinHandle<()>>,
}
impl Running {
    fn line(&self) -> String {
        self.rx
            .recv_timeout(Duration::from_secs(8))
            .expect("watch did not produce a frame before timeout")
    }
    fn frame(&self) -> Value {
        serde_json::from_str(&self.line()).expect("NDJSON line must be complete JSON")
    }
    fn observation(&self) -> Value {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            let frame = self.frame();
            if frame["type"] == "board_observation" {
                return frame;
            }
        }
        panic!("no ephemeral board observation");
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
// Environment is read only when Project::open resolves fixture paths; the lock serializes it.
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[test]
fn native_stream_control_values_preserve_frames_and_escape_compact_rows() {
    let _guard = ENV.lock().unwrap();
    let f = Fixture::new();
    let (run, _) = f.active();
    let controls: String = (0..=31)
        .chain(127..=159)
        .chain([0x200e, 0x200f])
        .chain(0x2028..=0x202e)
        .chain(0x2066..=0x2069)
        .map(|n| char::from_u32(n).unwrap())
        .collect();
    let stage = "phase\u{1b}\u{85}\u{202e}\u{2028}\n\ttail";
    let p = f.project();
    let db = pctx::work::connect(&p).unwrap();
    db.execute(
        "UPDATE runs SET stage=?1 WHERE id=?2",
        rusqlite::params![stage, run],
    )
    .unwrap();
    // Controlled persisted event, not a claim of normal domain publication.
    db.execute(
        "INSERT INTO events(entity,type,payload,created) VALUES(?1,?2,?3,1)",
        rusqlite::params![controls, controls, json!({"message":controls}).to_string()],
    )
    .unwrap();
    drop(db);
    for route in ["board", "activity"] {
        let out = f
            .command(&[route, "--format", "ndjson", "--no-color"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        assert!(out.stderr.is_empty());
        let frames: Vec<Value> = out
            .stdout
            .split(|b| *b == b'\n')
            .filter(|s| !s.is_empty())
            .map(|s| {
                let text = std::str::from_utf8(s).unwrap();
                for c in controls.chars() {
                    assert!(!text.contains(c));
                }
                serde_json::from_slice(s).unwrap()
            })
            .collect();
        if route == "board" {
            assert_eq!(frames[0]["data"]["tasks"][0]["run"]["stage"], stage);
        } else {
            assert!(frames.iter().any(|v| v["type"] == controls
                && v["entity_id"] == controls
                && v["data"]["message"] == controls));
        }
    }
    {
        let watch = f.watch(false);
        assert!(watch.line().starts_with("PCTX "));
        let _columns = watch.line();
        let row = watch.line();
        assert!(row.contains("phase") && row.contains("tail"));
        assert!(
            row.contains("\\u{001b}") && row.contains("\\u{0085}") && row.contains("\\u{202e}")
        );
        for c in controls.chars() {
            assert!(!row.contains(c));
        }
    }
    fs::write(p.control_db(), b"not sqlite").unwrap();
    let out = f
        .command(&["activity", "--format", "ndjson"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(7));
    assert!(out.stdout.is_empty());
    assert_eq!(out.stderr.iter().filter(|b| **b == b'\n').count(), 1);
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(error["type"], "error");
    assert!(error["data"]["code"].is_string());
}
#[test]
fn heartbeat_and_expiry_appear_as_ephemeral_observations() {
    let _guard = ENV.lock().unwrap();
    let f = Fixture::new();
    let (run, epoch) = f.active();
    let p = f.project();
    let db = pctx::work::connect(&p).unwrap();
    db.execute(
        "UPDATE runs SET last_seen=? WHERE id=?",
        rusqlite::params![chrono::Utc::now().timestamp() - 100, run],
    )
    .unwrap();
    let watch = f.watch(true);
    let initial = watch.frame();
    assert_eq!(initial["type"], "board_snapshot");
    assert_eq!(
        initial["data"]["tasks"][0]["run"]["activity_status"],
        "stale"
    );
    let cursor = initial["event_seq"].clone();
    let completion = initial["data"]["tasks"][0]["checklist"].clone();
    f.ok(&["agent", "heartbeat", "--run", &run, "--lease-epoch", &epoch]);
    let observed = watch.observation();
    assert!(observed["event_seq"].is_null());
    assert_eq!(observed["persistent_cursor"], cursor);
    assert_eq!(
        observed["data"]["tasks"][0]["run"]["activity_status"],
        "active"
    );
    assert_eq!(observed["data"]["tasks"][0]["checklist"], completion);
    db.execute(
        "UPDATE runs SET lease_until=? WHERE id=?",
        rusqlite::params![chrono::Utc::now().timestamp() - 1, run],
    )
    .unwrap();
    let expired = watch.observation();
    assert_eq!(
        expired["data"]["tasks"][0]["run"]["activity_status"],
        "lease_expired"
    );
    assert_eq!(expired["persistent_cursor"], cursor);
    assert_eq!(f.ok(&["board"])["data"]["as_of_seq"], cursor);
}
#[test]
fn compact_watch_renders_essential_columns_and_refreshes() {
    let _guard = ENV.lock().unwrap();
    let f = Fixture::new();
    f.active();
    let watch = f.watch(false);
    let header = watch.line();
    assert!(header.starts_with("PCTX "));
    let columns = watch.line();
    for word in [
        "ID",
        "STATE",
        "AGENT",
        "STAGE",
        "ACCEPTANCE",
        "CHECKS",
        "ACTIVITY",
    ] {
        assert!(columns.contains(word));
    }
    let task = watch.line();
    assert!(task.contains("in_progress"));
    assert!(task.contains("unit:"));
    assert!(watch.line().starts_with("PCTX "));
}
#[test]
fn finite_ndjson_drains_all_pages_and_resumes_after_cursor() {
    let _guard = ENV.lock().unwrap();
    let f = Fixture::new();
    let p = f.project();
    let db = pctx::work::connect(&p).unwrap();
    for i in 0..1005 {
        db.execute(
            "INSERT INTO events(entity,type,payload,created) VALUES('fixture','observed',?,1)",
            [json!({"i":i}).to_string()],
        )
        .unwrap();
    }
    let out = f
        .command(&["activity", "--format", "ndjson"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let frames: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(frames.len(), 1005);
    assert!(frames.windows(2).all(
        |pair| pair[0]["event_seq"].as_i64().unwrap() < pair[1]["event_seq"].as_i64().unwrap()
    ));
    let cursor = frames[499]["event_seq"].as_i64().unwrap().to_string();
    let out = f
        .command(&["activity", "--format", "ndjson", "--since-seq", &cursor])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap().lines().count(), 505);
}
#[test]
fn closed_stdout_returns_controlled_error_without_panic() {
    let _guard = ENV.lock().unwrap();
    let f = Fixture::new();
    f.active();
    let mut child = f
        .command(&["board", "--watch", "--format", "ndjson"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let deadline = Instant::now() + Duration::from_secs(8);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("watch did not stop after stdout closed");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut diagnostic = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut diagnostic)
        .unwrap();
    assert!(!status.success());
    assert_ne!(status.code(), Some(101));
    assert!(!diagnostic.contains("panicked"));
}

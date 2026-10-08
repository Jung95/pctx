//! Schedule query scope starts before discovery and cannot renew on SQL/bridge reads.
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
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
        f.ok(&["init"]);
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
    fn ok(&self, args: &[&str]) -> Value {
        let out = self.command().args(args).output().unwrap();
        assert!(out.status.success(), "{args:?} {out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn setup(&self) {
        let path = self.root.join("schedule-definition.json");
        fs::write(&path,json!({"schema_version":1,"namespace":"local","id":"digest","timezone":"Europe/Berlin","cadence":{"kind":"daily","at":"09:00"},"valid_from":"2024-01-01T00:00:00Z","job":"read_query","bridge":"manual","role":"assistant","recipient":"owner","topic":"digest"}).to_string()).unwrap();
        self.ok(&[
            "schedule",
            "add",
            "--from-file",
            path.to_str().unwrap(),
            "--idempotency-key",
            "fixture",
        ]);
    }
}
#[test]
fn schedule_read_budget_preflight_never_creates_missing_project() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("missing");
    let data = t.path().join("data");
    for args in [
        vec!["schedule", "list"],
        vec!["schedule", "plan", "--namespace", "local", "digest"],
        vec![
            "schedule",
            "inspect",
            "--namespace",
            "local",
            "digest",
            "--observe-native",
        ],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_pctx"))
            .args(["--format", "json", "--root"])
            .arg(&root)
            .env("PCTX_DATA_DIR", &data)
            .args(["--timeout-ms", "0"])
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{out:?}");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["errors"][0]["code"], "INVALID_ARGUMENT");
        assert!(!root.exists());
        assert!(!data.exists());
    }
}
#[test]
fn finite_schedule_queries_preserve_state_and_expire_under_sql_contention() {
    let f = Fixture::new();
    let empty = f.ok(&["--timeout-ms", "5000", "schedule", "list"]);
    assert!(empty["data"]["schedules"].as_array().unwrap().is_empty());
    f.setup();
    let listed = f.ok(&["--timeout-ms", "10000", "schedule", "list"]);
    assert_eq!(listed["data"]["schedules"].as_array().unwrap().len(), 1);
    let plan = f.ok(&[
        "--timeout-ms",
        "10000",
        "schedule",
        "plan",
        "--namespace",
        "local",
        "digest",
    ]);
    assert!(!f.root.join(".pctx/schedule-bridge").exists());
    assert!(!f.data.join("outputs").exists());
    let path = f.root.join("plan.json");
    fs::write(&path, plan["data"].to_string()).unwrap();
    f.ok(&[
        "schedule",
        "install",
        "--from-file",
        path.to_str().unwrap(),
        "--expect-hash",
        plan["data"]["plan_hash"].as_str().unwrap(),
    ]);
    let status = f.ok(&["status"]);
    let dbpath = status["data"]["local_storage"]["control"].as_str().unwrap();
    let db = rusqlite::Connection::open(dbpath).unwrap();
    let snapshot = || {
        let events: i64 = db
            .query_row("SELECT count(*) FROM schedule_events", [], |r| r.get(0))
            .unwrap();
        let state:String=db.query_row("SELECT state||metadata FROM schedule_installations WHERE namespace='local' AND schedule='digest'",[],|r|r.get(0)).unwrap();
        (events, state)
    };
    let before = snapshot();
    let inspected = f.ok(&[
        "--timeout-ms",
        "10000",
        "schedule",
        "inspect",
        "--namespace",
        "local",
        "digest",
    ]);
    assert_eq!(inspected["data"]["registration_performed"], false);
    assert_eq!(snapshot(), before);
    assert!(!f.data.join("outputs").exists());
    db.execute_batch(
        "PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE; BEGIN EXCLUSIVE;",
    )
    .unwrap();
    for args in [
        vec!["schedule", "list"],
        vec!["schedule", "plan", "--namespace", "local", "digest"],
        vec!["schedule", "inspect", "--namespace", "local", "digest"],
    ] {
        let start = Instant::now();
        let out = f
            .command()
            .args(["--timeout-ms", "100"])
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(7), "{args:?} {out:?}");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["errors"][0]["code"], "TIMEOUT");
        assert!(v["data"].is_null());
        assert!(start.elapsed() < Duration::from_secs(3));
    }
    db.execute_batch("ROLLBACK").unwrap();
    assert_eq!(snapshot(), before);
    assert!(!f.data.join("outputs").exists());
}

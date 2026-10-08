//! PCTX01-G05/G06: stored syntax is not caller syntax; busy is retryable.
use fs2::FileExt;
use pctx::{
    deadline::Deadline,
    project::{Config, Project, RootAnchor},
    watch, work,
};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

type Snapshot = BTreeMap<PathBuf, Option<Vec<u8>>>;

fn snapshot(root: &Path) -> Snapshot {
    fn walk(root: &Path, path: &Path, result: &mut Snapshot) {
        if !path.exists() {
            return;
        }
        assert!(!fs::symlink_metadata(path).unwrap().is_symlink());
        let key = path.strip_prefix(root).unwrap().to_path_buf();
        if path.is_dir() {
            result.insert(key, None);
            for entry in fs::read_dir(path).unwrap() {
                walk(root, &entry.unwrap().path(), result);
            }
        } else {
            result.insert(key, Some(fs::read(path).unwrap()));
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    root: PathBuf,
    data: PathBuf,
    db: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let mut f = Self {
            _temp: temp,
            root: base.join("project"),
            data: base.join("data"),
            base,
            db: PathBuf::new(),
        };
        fs::create_dir(&f.root).unwrap();
        for args in [vec!["init"], vec!["activity"]] {
            let out = f.run("json", &args);
            assert!(out.status.success(), "{out:?}");
        }
        let candidates: Vec<_> = snapshot(&f.data)
            .keys()
            .filter(|p| p.file_name().is_some_and(|n| n == "control.sqlite3"))
            .cloned()
            .collect();
        assert_eq!(candidates.len(), 1);
        f.db = f.data.join(&candidates[0]);
        f
    }
    fn run(&self, format: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(&self.base)
            .args(["--root"])
            .arg(&self.root)
            .args(["--format", format])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_USER_CONFIG", self.base.join("absent-config"))
            .env("PCTX_ACTOR", "owner")
            .env_remove("PCTX_RUN_CAPABILITY")
            .output()
            .unwrap()
    }
    fn project(&self) -> Project {
        let config: Config =
            toml::from_str(&fs::read_to_string(self.root.join(".pctx/config.toml")).unwrap())
                .unwrap();
        Project {
            deadline: None,
            root_anchor: RootAnchor::capture(&self.root).unwrap(),
            root: self.root.clone(),
            data_dir: self.data.clone(),
            workspace_dir: self.data.join("unused-workspace"),
            control_dir: self.db.parent().unwrap().to_path_buf(),
            project_id: config.project.id.clone(),
            workspace_id: "direct-fixture".into(),
            coordination_id: "direct-fixture".into(),
            config,
        }
    }
    fn insert(&self, payload: &str) -> i64 {
        let db = Connection::open(&self.db).unwrap();
        db.execute("INSERT INTO events(entity,type,payload,created) VALUES('fixture','fixture_event',?1,1)", [payload]).unwrap();
        db.last_insert_rowid()
    }
    fn state(&self) -> (Snapshot, Snapshot) {
        (snapshot(&self.root), snapshot(&self.data))
    }
}

#[test]
fn corrupted_activity_syntax_is_storage_failure_on_every_existing_transport() {
    let f = Fixture::new();
    f.insert("null");
    f.insert("{PCTX_CORRUPT_SENTINEL");
    let before = f.state();
    for format in ["json", "compact", "ndjson"] {
        let out = f.run(format, &["activity"]);
        assert_eq!(out.status.code(), Some(7), "{format}: {out:?}");
        if format == "json" || format == "compact" {
            let value: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(value["status"], "error");
            assert_eq!(value["errors"][0]["code"], "DB_CORRUPT");
            assert_eq!(value["errors"][0]["retryable"], false);
            assert_eq!(value["coverage"]["status"], "partial");
            assert!(value["data"].is_null());
            assert!(out.stderr.is_empty());
        } else if format == "ndjson" {
            assert!(out.stdout.is_empty(), "No half-decoded page may be emitted");
            let error: Value = serde_json::from_slice(&out.stderr).unwrap();
            assert_eq!(error["data"]["code"], "DB_CORRUPT");
        }
        assert!(!String::from_utf8_lossy(&out.stdout).contains("PCTX_CORRUPT_SENTINEL"));
        assert!(!String::from_utf8_lossy(&out.stderr).contains("PCTX_CORRUPT_SENTINEL"));
        assert_eq!(f.state(), before);
    }
    // Unsupported Markdown is refused before stored state is inspected.
    let out = f.run("markdown", &["activity"]);
    assert_eq!(out.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], "INVALID_ARGUMENT");
    assert_eq!(f.state(), before);
    let mut p = f.project();
    let error = work::activity(&p, 0).unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), ("DB_CORRUPT", 7));
    let error = watch::run(&p, false, 0, false, true).unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), ("DB_CORRUPT", 7));
    p.deadline = Some(Deadline::from_millis(1).unwrap());
    let original = p.deadline.unwrap().instant();
    std::thread::sleep(Duration::from_millis(5));
    for error in [
        work::activity(&p, 0).unwrap_err(),
        watch::run(&p, false, 0, false, true).unwrap_err(),
    ] {
        assert_eq!((error.code.as_str(), error.exit), ("TIMEOUT", 7));
        assert_eq!(p.deadline.unwrap().instant(), original);
    }
    assert_eq!(f.state(), before);
}

#[test]
fn valid_stored_json_values_and_later_page_failure_preserve_events_and_cursors() {
    let f = Fixture::new();
    let values = [
        json!(null),
        json!(true),
        json!(false),
        json!(42),
        json!(-1),
        json!(1.25),
        json!("line\n\u{2028}"),
        json!([1, null]),
        json!({"key":"value"}),
    ];
    let mut last = 0;
    for value in &values {
        last = f.insert(&value.to_string());
    }
    let before = f.state();
    let p = f.project();
    let page = work::activity(&p, 0).unwrap();
    assert_eq!(
        page["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["data"].clone())
            .collect::<Vec<_>>(),
        values
    );
    assert_eq!(page["next_cursor"], last);
    assert_eq!(page["has_more"], false);
    assert!(p.deadline.is_none());
    let out = f.run("ndjson", &["activity"]);
    assert!(out.status.success(), "{out:?}");
    let frames: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(frames.len(), values.len());
    for (frame, value) in frames.iter().zip(&values) {
        assert_eq!(&frame["data"], value);
    }
    assert_eq!(f.state(), before);

    for _ in values.len()..1000 {
        f.insert("true");
    }
    let corrupt_seq = f.insert("{");
    let before = f.state();
    let out = f.run("ndjson", &["activity"]);
    assert_eq!(
        out.status.code(),
        Some(7),
        "status={:?} stderr={:?} stdout_bytes={}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
        out.stdout.len()
    );
    let frames: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(frames.len(), 1000);
    assert_eq!(frames.last().unwrap()["event_seq"], corrupt_seq - 1);
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(error["data"]["code"], "DB_CORRUPT");
    let page = work::activity(&p, 0).unwrap();
    assert_eq!(page["next_cursor"], corrupt_seq - 1);
    assert_eq!(page["has_more"], true);
    assert_eq!(
        work::activity(&p, corrupt_seq - 1).unwrap_err().code,
        "DB_CORRUPT"
    );
    assert_eq!(
        work::activity(&p, corrupt_seq).unwrap()["next_cursor"],
        corrupt_seq
    );
    assert_eq!(f.state(), before);
}

#[test]
fn caller_json_syntax_remains_input_error_without_storage_effects() {
    let f = Fixture::new();
    let file = f.base.join("invalid.json");
    fs::write(&file, "{PCTX_CORRUPT_SENTINEL").unwrap();
    let before = f.state();
    let out = f.run(
        "json",
        &["task", "create", "--from-file", file.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], "INVALID_ARGUMENT");
    assert_eq!(value["errors"][0]["retryable"], false);
    assert!(!String::from_utf8_lossy(&out.stdout).contains("PCTX_CORRUPT_SENTINEL"));
    assert_eq!(f.state(), before);
}

#[test]
fn busy_admission_is_retryable_but_original_request_expiry_is_not() {
    let f = Fixture::new();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.db.parent().unwrap().join("connection-init.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    let before = f.state();
    for format in ["json", "compact"] {
        let out = f.run(format, &["task", "list"]);
        assert_eq!(out.status.code(), Some(7), "{out:?}");
        if format == "json" || format == "compact" {
            let value: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(value["errors"][0]["code"], "INDEX_BUSY");
            assert_eq!(value["errors"][0]["retryable"], true);
            assert!(out.stderr.is_empty());
        }
        let out = f.run(format, &["task", "list", "--timeout-ms", "20"]);
        assert_eq!(out.status.code(), Some(7), "{out:?}");
        if format == "json" || format == "compact" {
            let value: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(value["errors"][0]["code"], "TIMEOUT");
            assert_eq!(value["errors"][0]["retryable"], false);
            assert!(out.stderr.is_empty());
        }
        assert_eq!(f.state(), before);
    }
    let busy_path = f.base.join("busy.sqlite3");
    let holder = Connection::open(&busy_path).unwrap();
    holder
        .execute_batch("CREATE TABLE fixture(value); BEGIN IMMEDIATE")
        .unwrap();
    let contender = Connection::open(&busy_path).unwrap();
    contender.busy_timeout(Duration::from_millis(1)).unwrap();
    let native = contender.execute_batch("BEGIN IMMEDIATE").unwrap_err();
    assert_eq!(
        native.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy)
    );
    let error: pctx::domain::Error = native.into();
    assert_eq!(
        (error.code.as_str(), error.exit, error.retryable),
        ("INDEX_BUSY", 7, true)
    );
    holder.execute_batch("ROLLBACK").unwrap();

    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE fixture(value); INSERT INTO fixture VALUES(1),(2)")
        .unwrap();
    let mut statement = db.prepare("SELECT value FROM fixture").unwrap();
    let mut rows = statement.query([]).unwrap();
    assert!(rows.next().unwrap().is_some());
    let native = db.execute_batch("DROP TABLE fixture").unwrap_err();
    assert_eq!(
        native.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseLocked)
    );
    let error: pctx::domain::Error = native.into();
    assert_eq!(
        (error.code.as_str(), error.exit, error.retryable),
        ("INDEX_BUSY", 7, true)
    );
}

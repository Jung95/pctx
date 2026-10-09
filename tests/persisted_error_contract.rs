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

impl Fixture {
    fn update(&self, sql: &str, params: impl rusqlite::Params) -> rusqlite::Result<usize> {
        Connection::open(&self.db)?.execute(sql, params)
    }
    fn success(&self, args: &[&str]) -> Value {
        let out = self.run("json", args);
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["data"].clone()
    }
    fn create_task(&self, key: &str) -> (PathBuf, String, Value) {
        let file = self.base.join("definition.json");
        fs::write(&file, json!({"schema_version":1,"title":"Stored boundary","scope":["code.rs"],"acceptance":[],"checks":[]}).to_string()).unwrap();
        let value = self.success(&[
            "task",
            "create",
            "--from-file",
            file.to_str().unwrap(),
            "--idempotency-key",
            key,
        ]);
        (file, value["task_id"].as_str().unwrap().into(), value)
    }
    fn assert_storage_refusal(&self, args: &[&str]) {
        self.assert_storage_refusal_state(args, false);
    }
    fn control_rows(&self) -> BTreeMap<String, Vec<Vec<String>>> {
        Self::database_rows(&self.db)
    }
    fn database_rows(path: &Path) -> BTreeMap<String, Vec<Vec<String>>> {
        let db = Connection::open(path).unwrap();
        let tables: Vec<String> = db
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let mut result = BTreeMap::new();
        for table in std::iter::once("sqlite_master".to_string()).chain(tables) {
            let query = format!(
                "SELECT * FROM \"{}\" ORDER BY rowid",
                table.replace('"', "\"\"")
            );
            let mut statement = db.prepare(&query).unwrap();
            let columns = statement.column_count();
            let data = statement
                .query_map([], |row| {
                    (0..columns)
                        .map(|i| Ok(pctx::domain::hash(format!("{:?}", row.get_ref(i)?))))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            result.insert(table, data);
        }
        result
    }
    fn assert_storage_refusal_state(&self, args: &[&str], logical_sqlite: bool) {
        let state = || {
            let mut state = self.state();
            if logical_sqlite {
                // SQLite checkpoints can move existing WAL pages into the DB.
                // Compare every SQL/schema cell separately and all non-DB files.
                let database = self.db.strip_prefix(&self.data).unwrap();
                state.1.retain(|path, _| {
                    path != database
                        && path != &PathBuf::from(format!("{}-wal", database.display()))
                        && path != &PathBuf::from(format!("{}-shm", database.display()))
                });
            }
            state
        };
        let before_rows = logical_sqlite.then(|| self.control_rows());
        let before = state();
        for format in ["json", "compact"] {
            let out = self.run(format, args);
            assert_eq!(out.status.code(), Some(7), "{out:?}");
            let value: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(value["errors"][0]["code"], "DB_CORRUPT");
            assert_eq!(value["errors"][0]["retryable"], false);
            assert!(value["data"].is_null());
            assert!(out.stderr.is_empty());
            assert!(!String::from_utf8_lossy(&out.stdout).contains("PCTX_STORED_SENTINEL"));
            if let Some(rows) = &before_rows {
                assert!(
                    self.control_rows() == *rows,
                    "{args:?} changed stored SQL/schema cells"
                );
            }
            let after = state();
            let changed: Vec<_> = [&before.0, &before.1]
                .into_iter()
                .zip([&after.0, &after.1])
                .flat_map(|(left, right)| {
                    left.keys()
                        .chain(right.keys())
                        .filter(|key| left.get(*key) != right.get(*key))
                        .map(|key| key.display().to_string())
                        .collect::<std::collections::BTreeSet<_>>()
                })
                .collect();
            assert!(
                after == before,
                "{args:?} changed prepared paths: {changed:?}"
            );
        }
    }
}

#[test]
fn stored_task_fields_refuse_corruption_without_mutating_or_changing_input_errors() {
    let f = Fixture::new();
    let (file, id, _) = f.create_task("task-fields");
    let original: String = Connection::open(&f.db)
        .unwrap()
        .query_row("SELECT definition FROM tasks WHERE id=?1", [&id], |r| {
            r.get(0)
        })
        .unwrap();
    // Typed stored definition syntax and shape both originate in storage.
    for corrupt in [
        "{PCTX_STORED_SENTINEL",
        "{\"title\":false,\"PCTX_STORED_SENTINEL\":true}",
    ] {
        f.update(
            "UPDATE tasks SET definition=?1 WHERE id=?2",
            rusqlite::params![corrupt, id],
        )
        .unwrap();
        for args in [
            vec!["task", "show", &id],
            vec!["task", "list"],
            vec!["board"],
            vec!["task", "ready", &id],
        ] {
            f.assert_storage_refusal(&args);
        }
    }
    f.update(
        "UPDATE tasks SET definition=?1 WHERE id=?2",
        rusqlite::params![original, id],
    )
    .unwrap();
    for column in ["submission", "completion"] {
        f.update(
            &format!("UPDATE tasks SET {column}=?1 WHERE id=?2"),
            rusqlite::params!["{PCTX_STORED_SENTINEL", id],
        )
        .unwrap();
        f.assert_storage_refusal(&["task", "show", &id]);
        f.assert_storage_refusal(&["task", "list"]);
        let mut p = f.project();
        p.deadline = Some(Deadline::from_millis(1).unwrap());
        let original = p.deadline.unwrap().instant();
        std::thread::sleep(Duration::from_millis(5));
        let before = f.state();
        let error = work::execute(
            &p,
            &work::WorkCommand::Task {
                command: work::TaskCommand::Show { task: id.clone() },
            },
        )
        .unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("TIMEOUT", 7));
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
        f.update(
            &format!("UPDATE tasks SET {column}=NULL WHERE id=?1"),
            [&id],
        )
        .unwrap();
    }
    // Nullable storage and every JSON value category remain accepted, without new shape rules.
    for value in [
        json!(null),
        json!(true),
        json!(17),
        json!(1.5),
        json!("text\n"),
        json!([null]),
        json!({"id":"fixture"}),
    ] {
        f.update(
            "UPDATE tasks SET submission=?1,completion=?1 WHERE id=?2",
            rusqlite::params![value.to_string(), id],
        )
        .unwrap();
        let before = f.state();
        assert_eq!(f.success(&["task", "show", &id])["submission"], value);
        assert_eq!(
            f.success(&["task", "list"])["tasks"][0]["submission"],
            value
        );
        assert_eq!(f.state(), before);
    }
    f.update(
        "UPDATE tasks SET submission=NULL,completion=NULL WHERE id=?1",
        [&id],
    )
    .unwrap();
    assert!(f.success(&["task", "show", &id])["submission"].is_null());
    fs::write(&file, "{PCTX_STORED_SENTINEL").unwrap();
    let before = f.state();
    let out = f.run(
        "json",
        &["task", "create", "--from-file", file.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["errors"][0]["code"],
        "INVALID_ARGUMENT"
    );
    assert_eq!(f.state(), before);
}

#[test]
fn stored_create_receipt_preserves_replay_conflict_and_original_expiry() {
    let f = Fixture::new();
    let (file, _, first) = f.create_task("create-replay");
    let args = [
        "task",
        "create",
        "--from-file",
        file.to_str().unwrap(),
        "--idempotency-key",
        "create-replay",
    ];
    let before = f.state();
    assert_eq!(f.success(&args), first);
    assert_eq!(f.state(), before);
    f.update(
        "UPDATE receipts SET response=?1 WHERE key='create-replay'",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    f.assert_storage_refusal(&args);
    let mut p = f.project();
    p.deadline = Some(Deadline::from_millis(1).unwrap());
    let instant = p.deadline.unwrap().instant();
    std::thread::sleep(Duration::from_millis(5));
    let before = f.state();
    let error = work::execute(
        &p,
        &work::WorkCommand::Task {
            command: work::TaskCommand::Create {
                from_file: file.clone(),
                idempotency_key: Some("create-replay".into()),
            },
        },
    )
    .unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), ("TIMEOUT", 7));
    assert_eq!(p.deadline.unwrap().instant(), instant);
    assert_eq!(f.state(), before);
    for value in [
        json!(null),
        json!(false),
        json!(5),
        json!(1.25),
        json!("receipt\n"),
        json!([]),
        json!({"event":"synthetic"}),
    ] {
        f.update(
            "UPDATE receipts SET response=?1 WHERE key='create-replay'",
            [value.to_string()],
        )
        .unwrap();
        let before = f.state();
        assert_eq!(f.success(&args), value);
        assert_eq!(f.state(), before);
    }
    f.update(
        "UPDATE receipts SET response=?1 WHERE key='create-replay'",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    let mut definition: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    definition["title"] = json!("Different request");
    fs::write(&file, definition.to_string()).unwrap();
    let before = f.state();
    let out = f.run("json", &args);
    assert_eq!(out.status.code(), Some(9));
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["errors"][0]["code"],
        "IDEMPOTENCY_CONFLICT"
    );
    assert_eq!(f.state(), before);
}

#[test]
fn stored_report_receipt_preserves_exact_replay_before_lease_and_hash_conflict_before_decode() {
    let f = Fixture::new();
    let (_, id, _) = f.create_task("report-task");
    let agent = f.success(&["agent", "register", "--name", "fixture"])["agent_id"]
        .as_str()
        .unwrap()
        .to_owned();
    f.success(&["task", "ready", &id]);
    f.success(&["task", "assign", &id, "--agent", &agent]);
    let run = f.success(&["task", "start", &id]);
    let run_id = run["run_id"].as_str().unwrap();
    let epoch = run["lease_epoch"].to_string();
    let args = [
        "agent",
        "report",
        "--run",
        run_id,
        "--lease-epoch",
        &epoch,
        "--report-seq",
        "1",
        "--idempotency-key",
        "report-replay",
        "--stage",
        "implementing",
        "--summary",
        "fixture",
    ];
    let first = f.success(&args);
    f.update("UPDATE runs SET lease_until=0 WHERE id=?1", [run_id])
        .unwrap();
    let before = f.state();
    assert_eq!(f.success(&args), first);
    assert_eq!(f.state(), before);
    f.update(
        "UPDATE receipts SET response=?1 WHERE key='report-replay'",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    f.assert_storage_refusal(&args);
    let mut p = f.project();
    p.deadline = Some(Deadline::from_millis(1).unwrap());
    let instant = p.deadline.unwrap().instant();
    std::thread::sleep(Duration::from_millis(5));
    let before = f.state();
    let error = work::execute(
        &p,
        &work::WorkCommand::Agent {
            command: work::AgentCommand::Report {
                run: run_id.into(),
                lease_epoch: epoch.parse().unwrap(),
                report_seq: 1,
                idempotency_key: "report-replay".into(),
                stage: "implementing".into(),
                summary: "fixture".into(),
                estimate_percent: None,
            },
        },
    )
    .unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), ("TIMEOUT", 7));
    assert_eq!(p.deadline.unwrap().instant(), instant);
    assert_eq!(f.state(), before);
    for value in [
        json!(null),
        json!(true),
        json!(-5),
        json!(0.25),
        json!("receipt\n"),
        json!([null]),
        json!({"event":"synthetic"}),
    ] {
        f.update(
            "UPDATE receipts SET response=?1 WHERE key='report-replay'",
            [value.to_string()],
        )
        .unwrap();
        let before = f.state();
        assert_eq!(f.success(&args), value);
        assert_eq!(f.state(), before);
    }
    f.update(
        "UPDATE receipts SET response=?1 WHERE key='report-replay'",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    let mut different = args;
    different[13] = "changed-summary";
    let before = f.state();
    let out = f.run("json", &different);
    assert_eq!(out.status.code(), Some(9));
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["errors"][0]["code"],
        "IDEMPOTENCY_CONFLICT"
    );
    assert_eq!(f.state(), before);
    let mut fresh = args;
    fresh[9] = "fresh-key";
    let out = f.run("json", &fresh);
    assert_eq!(out.status.code(), Some(9));
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["errors"][0]["code"],
        "LEASE_EXPIRED"
    );
    assert_eq!(f.state(), before);
}

impl Fixture {
    fn index_db(&self) -> PathBuf {
        let paths: Vec<_> = snapshot(&self.data)
            .keys()
            .filter(|p| p.file_name().is_some_and(|n| n == "index.sqlite3"))
            .cloned()
            .collect();
        assert_eq!(paths.len(), 1);
        self.data.join(&paths[0])
    }
    fn indexed(&self) {
        fs::write(self.root.join("code.rs"), "fn fixture() {}\n").unwrap();
        self.success(&["index", "update"]);
    }
    fn prepare_file(&self, path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

#[test]
fn grouped_quota_observation_syntax_and_shape_are_storage_errors() {
    let f = Fixture::new();
    f.success(&["quota", "report"]);
    for payload in ["{PCTX_STORED_SENTINEL", "{}"] {
        f.update("INSERT OR REPLACE INTO quota_observations VALUES('observation','main','fixture','fixture','input_tokens','tokens','statusline',NULL,NULL,NULL,0,0,9999999999,?1,'synthetic')", [payload]).unwrap();
        for args in [
            vec!["quota", "report"],
            vec!["quota", "plan", "--pool", "main"],
            vec!["quota", "reconcile", "--pool", "main"],
        ] {
            f.assert_storage_refusal(&args);
        }
    }
    f.update("DELETE FROM quota_observations", []).unwrap();
    let file = f.base.join("bad-usage.json");
    fs::write(&file, "{}").unwrap();
    let out = f.run(
        "json",
        &[
            "quota",
            "ingest",
            "--from-file",
            file.to_str().unwrap(),
            "--idempotency-key",
            "input",
        ],
    );
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn grouped_decision_payloads_preserve_caller_errors_and_safe_storage_refusal() {
    let f = Fixture::new();
    let file = f.base.join("decision.json");
    fs::write(&file, json!({"schema_version":1,"reason":"Synthetic decision","action":{"schema_version":1,"kind":"local_modify","resource":"code.rs","environment":"development","scope":["code.rs"],"actor":"worker","amount":0,"reversible":true,"cost_known":true}}).to_string()).unwrap();
    let id =
        f.success(&["decision", "request", "--from-file", file.to_str().unwrap()])["decision_id"]
            .as_str()
            .unwrap()
            .to_owned();
    for field in ["action", "request"] {
        let original: String = Connection::open(&f.db)
            .unwrap()
            .query_row(
                &format!("SELECT {field} FROM ops_decisions WHERE id=?1"),
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        f.update(
            &format!("UPDATE ops_decisions SET {field}=?1 WHERE id=?2"),
            rusqlite::params!["{PCTX_STORED_SENTINEL", id],
        )
        .unwrap();
        f.assert_storage_refusal(&["decision", "show", &id]);
        f.assert_storage_refusal(&["owner", "queue"]);
        f.update(
            &format!("UPDATE ops_decisions SET {field}=?1 WHERE id=?2"),
            rusqlite::params![original, id],
        )
        .unwrap();
    }
    fs::write(&file, "{PCTX_STORED_SENTINEL").unwrap();
    let out = f.run(
        "json",
        &["decision", "request", "--from-file", file.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn grouped_schedule_stored_definition_and_receipt_preserve_replay_conflict() {
    let f = Fixture::new();
    let file = f.base.join("schedule.json");
    let mut definition = json!({"schema_version":1,"namespace":"fixture","id":"digest","timezone":"UTC","cadence":{"kind":"daily","at":"10:00"},"valid_from":"2024-01-01T00:00:00Z","job":"read_query","bridge":"manual","role":"assistant","recipient":"owner","topic":"digest","enabled":true,"misfire":"coalesce_latest"});
    fs::write(&file, definition.to_string()).unwrap();
    let args = [
        "schedule",
        "add",
        "--from-file",
        file.to_str().unwrap(),
        "--idempotency-key",
        "stored-schedule",
    ];
    let first = f.success(&args);
    assert_eq!(f.success(&args), first);
    // Prepare SQLite's WAL reader markers without decoding the damaged value.
    // This qualifies an already-open control store, not first-reader journal creation.
    let keeper = Connection::open(&f.db).unwrap();
    keeper
        .query_row("SELECT count(*) FROM schedule_definitions", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
    let original = definition.to_string();
    for corrupt in ["{PCTX_STORED_SENTINEL", "{}"] {
        f.update("UPDATE schedule_definitions SET definition=?1", [corrupt])
            .unwrap();
        keeper
            .query_row("SELECT count(*) FROM schedule_definitions", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap();
        f.assert_storage_refusal_state(&["schedule", "list", "--namespace", "fixture"], true);
        f.assert_storage_refusal_state(
            &["schedule", "plan", "--namespace", "fixture", "digest"],
            true,
        );
    }
    f.update("UPDATE schedule_definitions SET definition=?1", [original])
        .unwrap();
    f.update(
        "UPDATE schedule_receipts SET response=?1 WHERE key='stored-schedule'",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    keeper
        .query_row("SELECT count(*) FROM schedule_receipts", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
    f.assert_storage_refusal_state(&args, true);
    definition["topic"] = json!("changed");
    fs::write(&file, definition.to_string()).unwrap();
    let out = f.run("json", &args);
    assert_eq!(out.status.code(), Some(9));
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["errors"][0]["code"],
        "IDEMPOTENCY_CONFLICT"
    );
    fs::write(&file, "{PCTX_STORED_SENTINEL").unwrap();
    assert_eq!(f.run("json", &args).status.code(), Some(2));
}

#[test]
fn grouped_index_metadata_and_checksum_matched_checkpoint_json_are_storage_errors() {
    let f = Fixture::new();
    f.indexed();
    f.success(&["checkpoint", "create", "--name", "fixture"]);
    let index = f.index_db();
    let corrupt = "{PCTX_STORED_SENTINEL";
    Connection::open(&index)
        .unwrap()
        .execute(
            "UPDATE checkpoints SET manifest=?1,manifest_hash=?2",
            rusqlite::params![corrupt, pctx::domain::hash(corrupt)],
        )
        .unwrap();
    f.assert_storage_refusal(&["checkpoint", "list"]);
    for payload in [corrupt, "{}"] {
        Connection::open(&index)
            .unwrap()
            .execute("UPDATE file_versions SET metadata=?1", [payload])
            .unwrap();
        f.assert_storage_refusal(&["outline", "code.rs"]);
    }
    // Pure invalid arguments still win before corrupt index access.
    assert_eq!(
        f.run("json", &["read", "code.rs", "--lines", "0:1"])
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn grouped_registry_pack_output_and_helper_metadata_refuse_without_publication() {
    let f = Fixture::new();
    f.indexed();
    let plan =
        f.db.parent()
            .unwrap()
            .join("pack-plans/PACKPLAN-fixture.json");
    let output = f.base.join("must-not-publish.tar");
    let registry = f.data.join("registry.json");
    let reg: Value = serde_json::from_slice(&fs::read(&registry).unwrap()).unwrap();
    let workspace = reg["roots"][f.root.to_str().unwrap()]["workspace_id"]
        .as_str()
        .unwrap();
    let artifact = f
        .data
        .join("outputs")
        .join(workspace)
        .join("OUT-fixture.json");
    let helper = f
        .index_db()
        .parent()
        .unwrap()
        .join("helpers/HELP-fixture.json");
    for bytes in [b"{PCTX_STORED_SENTINEL".as_slice(), b"{}".as_slice()] {
        for path in [&plan, &artifact, &helper] {
            f.prepare_file(path, bytes);
        }
        f.assert_storage_refusal(&[
            "pack",
            "create",
            "--plan",
            "PACKPLAN-fixture",
            "--expect-hash",
            &"a".repeat(64),
            "--output",
            output.to_str().unwrap(),
        ]);
        assert!(!output.exists());
        f.assert_storage_refusal(&["output", "show", "OUT-fixture"]);
        f.assert_storage_refusal(&["runner", "helper-status", "HELP-fixture"]);
    }
    let original = fs::read(&registry).unwrap();
    fs::write(&registry, "{PCTX_STORED_SENTINEL").unwrap();
    f.assert_storage_refusal(&["status"]);
    fs::write(&registry, original).unwrap();
    assert_eq!(
        f.run(
            "json",
            &[
                "pack",
                "create",
                "--plan",
                "../bad",
                "--expect-hash",
                &"a".repeat(64),
                "--output",
                output.to_str().unwrap()
            ]
        )
        .status
        .code(),
        Some(2)
    );
}

#[test]
fn grouped_context_selection_and_capsule_keep_identity_conflict_before_storage_decode() {
    let f = Fixture::new();
    f.indexed();
    let (_, task, _) = f.create_task("context-storage");
    let agent = f.success(&["agent", "register", "--name", "context-fixture"])["agent_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let session = f.success(&[
        "session",
        "attach",
        "--agent",
        &agent,
        "--runtime",
        "manual",
    ])["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let full = f.success(&[
        "context",
        "get",
        "--task-id",
        &task,
        "--session",
        &session,
        "--scope",
        "code.rs",
        "--budget-bytes",
        "50000",
    ]);
    let context = full["context_id"].as_str().unwrap();
    let delta = [
        "context",
        "get",
        "--task-id",
        &task,
        "--session",
        &session,
        "--mode",
        "delta",
        "--since",
        context,
        "--scope",
        "code.rs",
        "--budget-bytes",
        "50000",
    ];
    let original: String = Connection::open(&f.db)
        .unwrap()
        .query_row(
            "SELECT selection FROM pctx_context_emissions WHERE id=?1",
            [context],
            |r| r.get(0),
        )
        .unwrap();
    f.update(
        "UPDATE pctx_context_emissions SET selection=?1 WHERE id=?2",
        rusqlite::params!["{PCTX_STORED_SENTINEL", context],
    )
    .unwrap();
    let out = f.run("json", &delta);
    assert_eq!(out.status.code(), Some(9));
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["errors"][0]["code"],
        "BASELINE_MISMATCH"
    );
    f.success(&[
        "context",
        "ack",
        context,
        "--session",
        &session,
        "--epoch",
        "1",
    ]);
    for corrupt in ["{PCTX_STORED_SENTINEL", "[]"] {
        f.update(
            "UPDATE pctx_context_emissions SET selection=?1 WHERE id=?2",
            rusqlite::params![corrupt, context],
        )
        .unwrap();
        f.assert_storage_refusal(&delta);
    }
    f.update(
        "UPDATE pctx_context_emissions SET selection=?1 WHERE id=?2",
        rusqlite::params![original, context],
    )
    .unwrap();
    assert_eq!(f.success(&delta)["mode"], "delta");
    f.success(&["session", "suspend", "--session", &session]);
    f.success(&["session", "reconcile", "--session", &session]);
    f.update(
        "UPDATE pctx_session_capsules SET metadata=?1",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    f.assert_storage_refusal(&["session", "reconcile", "--session", &session]);
}

#[test]
fn grouped_broker_saved_snapshot_preserves_parser_priority() {
    let f = Fixture::new();
    f.indexed();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&f.root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap()
            .success()
    );
    f.success(&["repo", "status"]);
    let control = &f.db;
    let original: String = Connection::open(control)
        .unwrap()
        .query_row("SELECT value FROM broker_snapshots", [], |r| r.get(0))
        .unwrap();
    Connection::open(control)
        .unwrap()
        .execute(
            "UPDATE broker_snapshots SET value=?1",
            ["{PCTX_STORED_SENTINEL"],
        )
        .unwrap();
    f.assert_storage_refusal(&["repo", "status"]);
    assert_eq!(
        f.run("json", &["repo", "status", "--fields", "invalid-field"])
            .status
            .code(),
        Some(2)
    );
    Connection::open(control)
        .unwrap()
        .execute("UPDATE broker_snapshots SET value=?1", [original])
        .unwrap();
    f.success(&["repo", "status"]);
}

fn assert_safe_error(out: &Output, exit: i32, code: &str) {
    assert_eq!(out.status.code(), Some(exit));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], code);
    assert!(out.stderr.is_empty());
    for bytes in [&out.stdout, &out.stderr] {
        assert!(!String::from_utf8_lossy(bytes).contains("PCTX_STORED_SENTINEL"));
    }
}
impl Fixture {
    fn active_run(&self) -> (String, String, String) {
        self.indexed();
        let (_, task, _) = self.create_task("authority-task");
        let agent = self.success(&["agent", "register", "--name", "authority-fixture"])["agent_id"]
            .as_str()
            .unwrap()
            .to_owned();
        self.success(&["task", "ready", &task]);
        self.success(&["task", "assign", &task, "--agent", &agent]);
        let run = self.success(&["task", "start", &task])["run_id"]
            .as_str()
            .unwrap()
            .to_owned();
        (task, agent, run)
    }
    fn actor_output(
        &self,
        format: &str,
        args: &[&str],
        agent: &str,
        run: &str,
        token: &str,
    ) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(&self.base)
            .args(["--root"])
            .arg(&self.root)
            .args(["--format", format])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_USER_CONFIG", self.base.join("absent-config"))
            .env("PCTX_ACTOR", agent)
            .env("PCTX_RUN_ID", run)
            .env("PCTX_RUN_CAPABILITY", token)
            .output()
            .unwrap()
    }
    fn unchanged_after(&self, before: &(Snapshot, Snapshot)) {
        let after = self.state();
        // Never print snapshots: this fixture can contain synthetic private credentials.
        assert!(after == *before, "Prepared filesystem state changed");
    }
}

#[test]
fn grouped_private_credentials_keep_missing_mismatch_and_revoked_priority() {
    let f = Fixture::new();
    let (_, agent, run) = f.active_run();
    f.success(&["owner", "queue"]);
    let paths: Vec<_> = snapshot(&f.data)
        .keys()
        .filter(|p| {
            p.parent().is_some_and(|p| p.ends_with("credentials"))
                && p.file_name()
                    .is_some_and(|n| n == format!("{run}.json").as_str())
        })
        .map(|p| f.data.join(p))
        .collect();
    assert_eq!(paths.len(), 1);
    let path = &paths[0];
    let original = fs::read(path).unwrap();
    let credential: Value = serde_json::from_slice(&original).unwrap();
    let token = credential["capability"].as_str().unwrap();
    let epoch = credential["lease_epoch"].to_string();
    let input = f.base.join("authority-message.json");
    fs::write(&input, json!({"schema_version":1,"type":"notice","topic":"fixture","body":"Synthetic","idempotency_key":"authority"}).to_string()).unwrap();
    let report = [
        "agent",
        "report",
        "--run",
        &run,
        "--lease-epoch",
        &epoch,
        "--report-seq",
        "1",
        "--idempotency-key",
        "authority-report",
        "--stage",
        "implementing",
        "--summary",
        "fixture",
    ];
    let message = [
        "message",
        "send",
        "--from-file",
        input.to_str().unwrap(),
        "--to-role",
        "fixture",
    ];
    for raw in ["{PCTX_STORED_SENTINEL", "null", "{}"] {
        fs::write(path, raw).unwrap();
        let expected = if raw.starts_with('{') && raw != "{}" {
            (7, "DB_CORRUPT")
        } else {
            (5, "POLICY_DENIED")
        };
        let before = f.state();
        for args in [&report[..], &message[..]] {
            for format in ["json", "compact"] {
                let out = f.actor_output(format, args, &agent, &run, token);
                assert_safe_error(&out, expected.0, expected.1);
                assert!(!String::from_utf8_lossy(&out.stdout).contains(token));
                f.unchanged_after(&before);
            }
        }
    }
    fs::remove_file(path).unwrap();
    let before = f.state();
    for args in [&report[..], &message[..]] {
        assert_safe_error(
            &f.actor_output("json", args, &agent, &run, token),
            5,
            "POLICY_DENIED",
        );
        f.unchanged_after(&before);
    }
    fs::write(path, "{PCTX_STORED_SENTINEL").unwrap();
    f.update("UPDATE runs SET lease_until=0 WHERE id=?1", [&run])
        .unwrap();
    let before = f.state();
    for (args, code) in [
        (&report[..], "LEASE_EXPIRED"),
        (&message[..], "LEASE_REVOKED"),
    ] {
        assert_safe_error(&f.actor_output("json", args, &agent, &run, token), 9, code);
        f.unchanged_after(&before);
    }
    fs::write(path, &original).unwrap();
}

#[test]
fn grouped_message_queue_payloads_and_receipts_keep_recipient_and_replay_priority() {
    let f = Fixture::new();
    let agent = f.success(&["agent", "register", "--name", "message-fixture"])["agent_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let session = f.success(&[
        "session",
        "attach",
        "--agent",
        &agent,
        "--runtime",
        "manual",
    ])["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let other = f.success(&[
        "session",
        "attach",
        "--agent",
        &agent,
        "--runtime",
        "manual",
    ])["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let file = f.base.join("message.json");
    let message = json!({"schema_version":1,"type":"notice","topic":"fixture","body":"Synthetic","idempotency_key":"message-replay"});
    fs::write(&file, message.to_string()).unwrap();
    let send = [
        "message",
        "send",
        "--from-file",
        file.to_str().unwrap(),
        "--to-session",
        &session,
    ];
    let first = f.success(&send);
    let id = first["message_id"].as_str().unwrap();
    let db = Connection::open(&f.db).unwrap();
    let original: String = db
        .query_row("SELECT payload FROM ops_messages WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    let receipt: String = db
        .query_row("SELECT response FROM ops_receipts", [], |r| r.get(0))
        .unwrap();
    drop(db);
    for raw in ["{PCTX_STORED_SENTINEL", "{}"] {
        f.update(
            "UPDATE ops_messages SET payload=?1 WHERE id=?2",
            rusqlite::params![raw, id],
        )
        .unwrap();
        for args in [
            vec!["inbox", "read", "--session", &session],
            vec!["message", "ack", id, "--session", &session],
        ] {
            f.assert_storage_refusal(&args);
        }
        let before = f.state();
        assert_safe_error(
            &f.run("json", &["message", "ack", id, "--session", &other]),
            5,
            "POLICY_DENIED",
        );
        f.unchanged_after(&before);
    }
    f.update(
        "UPDATE ops_messages SET payload=?1 WHERE id=?2",
        rusqlite::params![original, id],
    )
    .unwrap();
    f.update(
        "UPDATE ops_receipts SET response=?1",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    f.assert_storage_refusal(&send);
    let mut changed = message.clone();
    changed["body"] = json!("Different");
    fs::write(&file, changed.to_string()).unwrap();
    let before = f.state();
    assert_safe_error(&f.run("json", &send), 9, "IDEMPOTENCY_CONFLICT");
    f.unchanged_after(&before);
    fs::write(&file, message.to_string()).unwrap();
    for value in [json!(null), json!([]), json!({"synthetic":true})] {
        f.update("UPDATE ops_receipts SET response=?1", [value.to_string()])
            .unwrap();
        assert_eq!(f.success(&send), value);
    }
    // json_extract selects a valid JSON object whose typed Message decoding fails.
    f.update("DELETE FROM ops_receipts", []).unwrap();
    f.update(
        "UPDATE ops_messages SET payload=?1",
        [json!({"idempotency_key":"message-replay"}).to_string()],
    )
    .unwrap();
    f.assert_storage_refusal(&send);
    f.update("UPDATE ops_messages SET payload=?1", [message.to_string()])
        .unwrap();
    assert_eq!(f.success(&send), first);
    f.update("UPDATE ops_receipts SET response=?1", [receipt])
        .unwrap();
}

#[test]
fn grouped_quota_receipt_and_prior_observation_decoders_preserve_conflict() {
    let f = Fixture::new();
    f.indexed();
    let (_, task, _) = f.create_task("quota-attribution");
    let agent = f.success(&["agent", "register", "--name", "quota-fixture"])["agent_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let session = f.success(&[
        "session",
        "attach",
        "--agent",
        &agent,
        "--runtime",
        "manual",
        "--account-pool",
        "main",
    ])["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let file = f.base.join("observations.json");
    let observation = json!({"observation_id":"first","pool_id":"main","provider":"fixture","model":"fixture","metric":"input_tokens","unit":"tokens","source":"provider_request","collector":"fixture","source_revision":"1","observed_at":"2026-01-01T01:00:00Z","window_id":"window","window_start":"2026-01-01T00:00:00Z","window_end":"2027-01-01T00:00:00Z","status":"actual","amount":10,"kind":"request","request_id":"request-fixture","task_id":task,"session_id":session,"context_epoch":1});
    let batch = json!({"schema_version":1,"observations":[observation]});
    fs::write(&file, batch.to_string()).unwrap();
    let ingest = [
        "quota",
        "ingest",
        "--from-file",
        file.to_str().unwrap(),
        "--idempotency-key",
        "quota-replay",
    ];
    let first = f.success(&ingest);
    f.update(
        "UPDATE quota_receipts SET response=?1",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    f.assert_storage_refusal(&ingest);
    let mut changed = batch.clone();
    changed["observations"][0]["amount"] = json!(11);
    fs::write(&file, changed.to_string()).unwrap();
    let before = f.state();
    assert_safe_error(&f.run("json", &ingest), 9, "IDEMPOTENCY_CONFLICT");
    f.unchanged_after(&before);
    fs::write(&file, batch.to_string()).unwrap();
    f.update("UPDATE quota_receipts SET response=?1", [first.to_string()])
        .unwrap();
    assert_eq!(f.success(&ingest), first);
    let mut next = observation.clone();
    next["observation_id"] = json!("second");
    fs::write(
        &file,
        json!({"schema_version":1,"observations":[next]}).to_string(),
    )
    .unwrap();
    f.update("DELETE FROM quota_receipts", []).unwrap();
    f.update(
        "UPDATE quota_observations SET payload=?1",
        [json!({"request_id":"request-fixture"}).to_string()],
    )
    .unwrap();
    f.assert_storage_refusal(&ingest);
    f.update("DELETE FROM quota_observations", []).unwrap();
    let mut cumulative = observation.clone();
    cumulative["kind"] = json!("cumulative");
    cumulative["source"] = json!("statusline");
    cumulative["request_id"] = Value::Null;
    cumulative["session_id"] = json!(session);
    cumulative["context_epoch"] = json!(1);
    cumulative["counter_epoch"] = json!("counter");
    // The prior row is selected by columns, before decoding its typed payload.
    f.update("INSERT INTO quota_observations VALUES('prior','main','fixture','fixture','input_tokens','tokens','statusline',?1,1,'counter',0,0,9999999999,?2,'synthetic')", rusqlite::params![session,"{}"]).unwrap();
    fs::write(
        &file,
        json!({"schema_version":1,"observations":[cumulative]}).to_string(),
    )
    .unwrap();
    f.assert_storage_refusal(&ingest);
}

#[cfg(unix)]
#[test]
fn grouped_private_execution_trust_refuses_corruption_before_spawn() {
    let f = Fixture::new();
    let plan = f.success(&[
        "trust",
        "plan",
        "--",
        "/usr/bin/printf",
        "PCTX_CHILD_WAS_STARTED",
    ]);
    let fingerprint = plan["fingerprint"].as_str().unwrap();
    f.success(&[
        "trust",
        "add",
        "--expect-hash",
        fingerprint,
        "--",
        "/usr/bin/printf",
        "PCTX_CHILD_WAS_STARTED",
    ]);
    let path = f.data.join(format!("trust/executions/{fingerprint}.json"));
    let original = fs::read(&path).unwrap();
    let run = ["run", "--", "/usr/bin/printf", "PCTX_CHILD_WAS_STARTED"];
    for raw in ["{PCTX_STORED_SENTINEL", "{}"] {
        fs::write(&path, raw).unwrap();
        let before = f.state();
        for format in ["json", "compact"] {
            let out = f.run(format, &run);
            assert_safe_error(&out, 7, "DB_CORRUPT");
            let value: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(value["data"]["spawned"], false);
            f.unchanged_after(&before);
        }
        assert!(
            !String::from_utf8_lossy(&f.run("json", &run).stdout)
                .contains("PCTX_CHILD_WAS_STARTED")
        );
    }
    let mut binding: Value = serde_json::from_slice(&original).unwrap();
    binding["workspace_id"] = json!("different-workspace");
    fs::write(&path, binding.to_string()).unwrap();
    let before = f.state();
    assert_safe_error(&f.run("json", &run), 9, "CONFIG_CHANGED");
    f.unchanged_after(&before);
    fs::remove_file(&path).unwrap();
    let before = f.state();
    assert_safe_error(&f.run("json", &run), 5, "OWNER_DECISION_REQUIRED");
    f.unchanged_after(&before);
}

#[cfg(unix)]
#[test]
fn grouped_runner_trust_and_task_decode_do_not_mask_storage_failure_as_owner_permission() {
    let f = Fixture::new();
    f.indexed();
    fs::write(f.root.join("fixture.sh"), "touch launched\n").unwrap();
    fs::write(f.root.join(".pctx/runner.toml"), "schema_version=1\n[checks.unit]\nargv=['/bin/sh','fixture.sh']\nreporter='pctx-json-v1'\nheavy=false\nresources=[]\n").unwrap();
    let file = f.base.join("runner-task.json");
    fs::write(&file, json!({"schema_version":1,"title":"Runner storage","scope":["code.rs","fixture.sh"],"checks":[{"key":"unit","kind":"test","allowed_sources":["runner_observed"],"output_paths":[]}],"acceptance":[]}).to_string()).unwrap();
    let task = f.success(&["task", "create", "--from-file", file.to_str().unwrap()])["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let args = ["runner", "check-plan", "--task-id", &task, "--key", "unit"];
    let plan = f.success(&args);
    assert!(
        plan["blocked_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "owner_binding_required")
    );
    let fingerprint = plan["fingerprint"].as_str().unwrap();
    f.success(&[
        "runner",
        "trust",
        "--key",
        "unit",
        "--expect-hash",
        fingerprint,
    ]);
    assert!(
        f.success(&args)["blocked_reasons"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let path = f.data.join(format!("trust/runners/{fingerprint}.json"));
    let original = fs::read(&path).unwrap();
    fs::write(&path, "{PCTX_STORED_SENTINEL").unwrap();
    f.assert_storage_refusal(&args);
    fs::write(&path, "null").unwrap();
    assert!(
        f.success(&args)["blocked_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "owner_binding_required")
    );
    fs::write(&path, &original).unwrap();
    let definition: String = Connection::open(&f.db)
        .unwrap()
        .query_row("SELECT definition FROM tasks WHERE id=?1", [&task], |r| {
            r.get(0)
        })
        .unwrap();
    for raw in ["{PCTX_STORED_SENTINEL", "{}"] {
        f.update(
            "UPDATE tasks SET definition=?1 WHERE id=?2",
            rusqlite::params![raw, task],
        )
        .unwrap();
        f.assert_storage_refusal(&args);
    }
    f.update(
        "UPDATE tasks SET definition=?1 WHERE id=?2",
        rusqlite::params![definition, task],
    )
    .unwrap();
    assert!(
        f.success(&args)["blocked_reasons"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!f.root.join("launched").exists());
}

#[test]
fn grouped_job_and_helper_consumers_refuse_corrupt_metadata_without_releasing_slots() {
    let f = Fixture::new();
    f.indexed();
    // Prepare all directories normally created by the mutation facade before snapshot.
    let host = f.data.join("host-resources");
    fs::create_dir_all(host.join("jobs")).unwrap();
    fs::create_dir_all(host.join("slots")).unwrap();
    let job = host.join("jobs/JOB-fixture.json");
    let slot = host.join("slots/exclusive-compute.json");
    let helper_dir = f.index_db().parent().unwrap().join("helpers");
    let helper = helper_dir.join("HELP-fixture.json");
    let workspace = f
        .index_db()
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let h = json!({"schema_version":1,"helper_id":"HELP-fixture","task_id":"synthetic","run_id":"synthetic","mode":"local","scope":[],"workspace":workspace,"state":"prepared","job_id":"JOB-fixture","output_id":"OUT-fixture","started":false,"created_at":0});
    for raw in ["{PCTX_STORED_SENTINEL", "{}"] {
        f.prepare_file(&job, raw.as_bytes());
        f.prepare_file(&slot, raw.as_bytes());
        f.prepare_file(&helper, h.to_string().as_bytes());
        for args in [
            vec!["runner", "resource-status"],
            vec!["runner", "job-cancel", "JOB-fixture"],
            vec!["runner", "helper-status", "HELP-fixture"],
            vec!["runner", "helper-cancel", "HELP-fixture"],
            vec![
                "runner",
                "helper-release",
                "HELP-fixture",
                "--evidence",
                "OUT-fixture",
            ],
        ] {
            f.assert_storage_refusal(&args);
            assert!(slot.exists());
        }
        // Existing evidence policy precedes reading corrupt job metadata.
        let before = f.state();
        assert_safe_error(
            &f.run(
                "json",
                &[
                    "runner",
                    "helper-release",
                    "HELP-fixture",
                    "--evidence",
                    "OUT-other",
                ],
            ),
            5,
            "POLICY_DENIED",
        );
        f.unchanged_after(&before);
        f.prepare_file(&helper, raw.as_bytes());
        f.assert_storage_refusal(&["runner", "helper-cancel", "HELP-fixture"]);
    }
}

#[test]
fn grouped_paired_index_cache_checkpoint_get_and_registry_shape_decoders() {
    let f = Fixture::new();
    f.indexed();
    let checkpoint = f.success(&["checkpoint", "create", "--name", "paired"]);
    let mut p = f.project();
    p.workspace_dir = f.index_db().parent().unwrap().to_path_buf();
    p.workspace_id = p
        .workspace_dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let id = checkpoint["id"].as_str().unwrap();
    let corrupt = "{PCTX_STORED_SENTINEL";
    Connection::open(f.index_db())
        .unwrap()
        .execute(
            "UPDATE checkpoints SET manifest=?1,manifest_hash=?2",
            rusqlite::params![corrupt, pctx::domain::hash(corrupt)],
        )
        .unwrap();
    let before = f.state();
    let error = pctx::storage::checkpoint_get(&p, id).unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), ("DB_CORRUPT", 7));
    f.unchanged_after(&before);
    Connection::open(f.index_db())
        .unwrap()
        .execute("UPDATE checkpoints SET manifest_hash='different'", [])
        .unwrap();
    let error = pctx::storage::checkpoint_get(&p, id).unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARCHIVE", 7));
    for raw in [corrupt, "{}"] {
        Connection::open(f.index_db())
            .unwrap()
            .execute("UPDATE file_versions SET metadata=?1", [raw])
            .unwrap();
        let index = f.index_db();
        let state = || {
            let mut state = f.state();
            let relative = index.strip_prefix(&f.data).unwrap();
            state.1.retain(|path, _| {
                path != relative
                    && path != &PathBuf::from(format!("{}-wal", relative.display()))
                    && path != &PathBuf::from(format!("{}-shm", relative.display()))
            });
            state
        };
        let before_rows = Fixture::database_rows(&index);
        let before = state();
        for format in ["json", "compact"] {
            assert_safe_error(&f.run(format, &["index", "update"]), 7, "DB_CORRUPT");
            assert!(
                Fixture::database_rows(&index) == before_rows,
                "Logical index/schema changed"
            );
            assert!(state() == before, "Non-index files changed");
        }
    }
    let registry = f.data.join("registry.json");
    let original = fs::read(&registry).unwrap();
    for raw in ["{}", "[]"] {
        fs::write(&registry, raw).unwrap();
        let before = f.state();
        assert_safe_error(&f.run("json", &["status"]), 6, "NOT_INITIALIZED");
        f.unchanged_after(&before);
    }
    fs::write(&registry, "null").unwrap();
    f.assert_storage_refusal(&["status"]);
    fs::write(&registry, original).unwrap();
}

#[test]
fn grouped_remaining_schedule_storage_branches_preserve_state_and_input_priority() {
    let f = Fixture::new();
    let file = f.base.join("remaining-schedule.json");
    let definition = json!({"schema_version":1,"namespace":"remaining","id":"digest","timezone":"UTC","cadence":{"kind":"daily","at":"10:00"},"valid_from":"2024-01-01T00:00:00Z","job":"read_query","bridge":"manual","role":"assistant","recipient":"owner","topic":"digest","enabled":true,"misfire":"coalesce_latest"});
    fs::write(&file, definition.to_string()).unwrap();
    f.success(&[
        "schedule",
        "add",
        "--from-file",
        file.to_str().unwrap(),
        "--idempotency-key",
        "remaining-schedule",
    ]);
    let pause = [
        "schedule",
        "pause",
        "--namespace",
        "remaining",
        "digest",
        "--reason",
        "fixture",
        "--expect-revision",
        "1",
    ];
    for raw in ["{PCTX_STORED_SENTINEL", "{}"] {
        f.update("UPDATE schedule_definitions SET definition=?1", [raw])
            .unwrap();
        f.assert_storage_refusal_state(&pause, true);
    }
    f.update(
        "UPDATE schedule_definitions SET definition=?1",
        [definition.to_string()],
    )
    .unwrap();
    let reconcile = [
        "schedule",
        "reconcile",
        "--namespace",
        "remaining",
        "--at",
        "2024-01-02T10:01:00Z",
    ];
    f.success(&reconcile);
    let original: String = Connection::open(&f.db)
        .unwrap()
        .query_row("SELECT metadata FROM schedule_occurrences", [], |r| {
            r.get(0)
        })
        .unwrap();
    f.update(
        "UPDATE schedule_occurrences SET metadata=?1",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    f.assert_storage_refusal_state(&reconcile, true);
    // Existing retained terminal occurrences accept arbitrary JSON Values; do not invent a schema.
    for value in [json!(null), json!([]), json!({"synthetic":true})] {
        f.update(
            "UPDATE schedule_occurrences SET state='failed',metadata=?1",
            [value.to_string()],
        )
        .unwrap();
        assert_eq!(f.success(&reconcile)["schedules"][0]["occurrence"], value);
    }
    f.update(
        "UPDATE schedule_occurrences SET state='planned',metadata=?1",
        [original],
    )
    .unwrap();
    let inspect = ["schedule", "inspect", "--namespace", "remaining", "digest"];
    assert_safe_error(&f.run("json", &inspect), 6, "CAPABILITY_UNVERIFIED");
    f.update("INSERT INTO schedule_installations VALUES('remaining','digest','synthetic','fixture','synthetic','synthetic','unknown_restored',?1,0)", ["{PCTX_STORED_SENTINEL"]).unwrap();
    f.assert_storage_refusal_state(&inspect, true);
    for value in [json!(null), json!([]), json!({"plan_hash":"synthetic"})] {
        f.update(
            "UPDATE schedule_installations SET metadata=?1",
            [value.to_string()],
        )
        .unwrap();
        let observed = f.success(&inspect);
        assert_eq!(observed["stored_state"], "unknown_restored");
        assert_eq!(observed["registration_performed"], false);
        assert_eq!(observed["requires_reapproval"], true);
    }
    f.update("INSERT INTO schedule_runs(namespace,schedule,revision,occurrence,attempt,state,result,started) VALUES('remaining','digest',1,'synthetic',1,'running',?1,0)", ["{PCTX_STORED_SENTINEL"]).unwrap();
    let recover = [
        "schedule",
        "recover",
        "--namespace",
        "remaining",
        "digest",
        "--revision",
        "1",
        "--occurrence",
        "synthetic",
        "--attempt",
        "1",
        "--reason",
        "fixture",
    ];
    f.assert_storage_refusal_state(&recover, true);
    for value in [json!(null), json!([]), json!({})] {
        f.update("UPDATE schedule_runs SET result=?1", [value.to_string()])
            .unwrap();
        let before = f.control_rows();
        assert_safe_error(&f.run("json", &recover), 10, "RESOURCE_OWNER_UNKNOWN");
        assert_eq!(f.control_rows(), before);
    }
    // Pure required-reason validation wins before the malformed saved recovery result.
    f.update(
        "UPDATE schedule_runs SET result=?1",
        ["{PCTX_STORED_SENTINEL"],
    )
    .unwrap();
    let mut invalid = recover;
    invalid[invalid.len() - 1] = "";
    assert_safe_error(&f.run("json", &invalid), 2, "INVALID_ARGUMENT");
}

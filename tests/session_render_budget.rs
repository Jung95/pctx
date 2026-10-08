//! CLI final-render capacity and semantic representation receipts; no model/run.
use pctx::domain::hash;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Reap(Child);
impl Drop for Reap {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
    task: String,
    agent: String,
}
impl Fixture {
    fn run(&self, args: &[&str]) -> (i32, Vec<u8>, Value) {
        let mut stdout = tempfile::tempfile().unwrap();
        let mut stderr = tempfile::tempfile().unwrap();
        let mut child = Reap(
            Command::new(env!("CARGO_BIN_EXE_pctx"))
                .args(["--root"])
                .arg(&self.root)
                .args(["--format", "json"])
                .args(args)
                .env("PCTX_DATA_DIR", &self.data)
                .env("PCTX_ACTOR", "owner")
                .stdin(Stdio::null())
                .stdout(Stdio::from(stdout.try_clone().unwrap()))
                .stderr(Stdio::from(stderr.try_clone().unwrap()))
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "isolated CLI exceeded fixture deadline"
            );
            std::thread::sleep(Duration::from_millis(2));
        };
        stdout.seek(SeekFrom::Start(0)).unwrap();
        stderr.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        stdout.take(128 * 1024 + 1).read_to_end(&mut bytes).unwrap();
        let mut errors = String::new();
        stderr.take(128 * 1024).read_to_string(&mut errors).unwrap();
        assert!(bytes.len() <= 128 * 1024);
        let value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("{args:?}: invalid JSON {e}; stderr={errors}"));
        (status.code().unwrap(), bytes, value)
    }
    fn ok(&self, args: &[&str]) -> Value {
        let (exit, _, value) = self.run(args);
        assert_eq!(exit, 0, "{args:?}: {value}");
        assert_eq!(value["status"], "ok");
        value
    }
    fn attach(&self) -> String {
        self.ok(&[
            "session",
            "attach",
            "--agent",
            &self.agent,
            "--runtime",
            "manual",
        ])["data"]["session_id"]
            .as_str()
            .unwrap()
            .into()
    }
    fn get(&self, session: &str, budget: &str) -> (i32, Vec<u8>, Value) {
        self.run(&[
            "context",
            "get",
            "--task-id",
            &self.task,
            "--session",
            session,
            "--scope",
            "auth.py",
            "--budget-bytes",
            budget,
        ])
    }
    fn db(&self) -> rusqlite::Connection {
        let control = fs::read_dir(self.data.join("controls"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        rusqlite::Connection::open(control.join("control.sqlite3")).unwrap()
    }
}
fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let mut f = Fixture {
        _temp: temp,
        root: base.join("project"),
        data: base.join("data"),
        task: String::new(),
        agent: String::new(),
    };
    fs::create_dir(&f.root).unwrap();
    fs::write(f.root.join("auth.py"), "def auth(): return 1\n").unwrap();
    f.ok(&["init"]);
    fs::create_dir_all(f.root.join(".pctx/rules")).unwrap();
    // Valid UTF-8 C1 characters survive masking; terminal-safe JSON expands each
    // two-byte U+0085 into six bytes. This is data, never execution evidence.
    fs::write(f.root.join(".pctx/rules/required.md"), format!("---\nschema_version: 1\nid: render-capacity\nrequired: true\nscope: [auth.py]\n---\nPreserve mandatory content {}\n", "\u{0085}".repeat(300))).unwrap();
    let definition = base.join("task.json");
    fs::write(&definition, json!({"schema_version":1,"title":"Context representation fixture",
        "scope":["auth.py"],"acceptance":[{"id":"retained","description":"Source references retained","evidence_check_keys":["unit"]}],
        "checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
    f.task = f.ok(&[
        "task",
        "create",
        "--from-file",
        definition.to_str().unwrap(),
    ])["data"]["task_id"]
        .as_str()
        .unwrap()
        .into();
    f.agent = f.ok(&[
        "agent",
        "register",
        "--name",
        "render-fixture",
        "--kind",
        "agent",
    ])["data"]["agent_id"]
        .as_str()
        .unwrap()
        .into();
    f
}
#[test]
fn terminal_escape_overflow_is_rejected_before_context_emission_or_event() {
    let f = fixture();
    let probe_session = f.attach();
    let (exit, rendered, probe) = f.get(&probe_session, "20000");
    assert_eq!(exit, 0, "{probe}");
    let raw_len = serde_json::to_vec(&probe).unwrap().len() + 1;
    assert!(rendered.len() > raw_len + 1000);
    let session = f.attach();
    assert_eq!(session.len(), probe_session.len());
    let budget = raw_len + 32; // Fixed-length timestamps/IDs; small envelope slack.
    assert!(budget < rendered.len());
    let db = f.db();
    let counts = || {
        (db.query_row("SELECT count(*) FROM pctx_context_emissions WHERE session=?1", [&session], |r| r.get::<_, i64>(0)).unwrap(),
         db.query_row("SELECT count(*) FROM pctx_session_events WHERE session=?1 AND kind='context_emitted'", [&session], |r| r.get::<_, i64>(0)).unwrap())
    };
    assert_eq!(counts(), (0, 0));
    let (exit, _, failure) = f.get(&session, &budget.to_string());
    assert_eq!(exit, 8, "{failure}");
    assert_eq!(failure["errors"][0]["code"], "BUDGET_TOO_SMALL");
    assert_eq!(counts(), (0, 0));
    let (exit, bytes, success) = f.get(&session, "20000");
    assert_eq!(exit, 0, "{success}");
    assert!(
        serde_json::to_vec(&success).unwrap().len() < budget,
        "fixture must establish raw JSON fits while final representation does not"
    );
    assert!(bytes.len() > budget);
}
#[test]
fn exact_final_budget_and_representation_receipts_require_explicit_ack() {
    let f = fixture();
    let probe_session = f.attach();
    let (exit, probe_bytes, probe) = f.get(&probe_session, "20000");
    assert_eq!(exit, 0, "{probe}");
    let session = f.attach();
    assert_eq!(session.len(), probe_session.len());
    let budget = probe_bytes.len();
    let (exit, bytes, full) = f.get(&session, &budget.to_string());
    assert_eq!(exit, 0, "{full}");
    assert_eq!(
        bytes.len(),
        budget,
        "complete final document includes newline and escapes"
    );
    assert_eq!(bytes.last(), Some(&b'\n'));
    let packet = &full["data"];
    assert_eq!(packet["serializer"], "minimal-context-v2");
    let context = packet["context_id"].as_str().unwrap();
    let db = f.db();
    let selection: String = db
        .query_row(
            "SELECT selection FROM pctx_context_emissions WHERE id=?1",
            [context],
            |r| r.get(0),
        )
        .unwrap();
    let metadata: Value = serde_json::from_str(&selection).unwrap();
    assert_eq!(metadata["task"]["representation"], "metadata");
    assert_eq!(metadata["file:auth.py"]["representation"], "reference");
    assert!(
        metadata["file:auth.py"]
            .get("delivered_text_hash")
            .is_none()
    );
    assert_eq!(
        metadata["file:.pctx/rules/required.md"]["representation"],
        "full_span"
    );
    for body in packet["added"].as_array().unwrap() {
        let key = if body["kind"] == "task" {
            "task".to_owned()
        } else {
            format!("file:{}", body["path"].as_str().unwrap())
        };
        let stored = &metadata[&key];
        assert_eq!(
            stored["delivered_body_hash"],
            hash(serde_json::to_vec(body).unwrap())
        );
        assert_eq!(stored["body_hash_format"], "canonical-json-v1");
        if let Some(text) = body["text"].as_str() {
            assert_eq!(stored["delivered_text_hash"], hash(text.as_bytes()));
            assert_eq!(stored["delivered_text_byte_range"], json!([0, text.len()]));
        }
    }
    assert!(!selection.contains("Preserve mandatory content"));
    assert!(!selection.contains("def auth"));
    let ack_count: i64 = db
        .query_row(
            "SELECT count(*) FROM pctx_context_acks WHERE session=?1",
            [&session],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ack_count, 0, "successful stdout never acknowledges receipt");
    let delta_args = [
        "context",
        "get",
        "--task-id",
        &f.task,
        "--session",
        &session,
        "--scope",
        "auth.py",
        "--mode",
        "delta",
        "--since",
        context,
        "--budget-bytes",
        "20000",
    ];
    let (exit, _, failure) = f.run(&delta_args);
    assert_eq!(exit, 9, "{failure}");
    assert_eq!(failure["errors"][0]["code"], "BASELINE_MISMATCH");
    let epoch = packet["context_epoch"].as_i64().unwrap().to_string();
    let ack = f.ok(&[
        "context",
        "ack",
        context,
        "--session",
        &session,
        "--epoch",
        &epoch,
    ]);
    assert_eq!(ack["data"]["acknowledged"], true);
    assert_eq!(ack["data"]["understanding_proven"], false);
    let delta = f.ok(&delta_args);
    assert_eq!(delta["data"]["context_id"], context);
    for key in ["added", "changed", "removed"] {
        assert!(delta["data"][key].as_array().unwrap().is_empty());
    }
    let after: String = db
        .query_row(
            "SELECT selection FROM pctx_context_emissions WHERE id=?1",
            [context],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(after, selection);
    // An isolated historical serializer fixture remains data, not a migration
    // claim. Existing acknowledged v1 baselines must fail the v2 comparison.
    db.execute("INSERT INTO pctx_context_emissions SELECT 'CTX-legacy-fixture',session,epoch,task,policy,scope,'minimal-context-v1',content_hash,selection,created FROM pctx_context_emissions WHERE id=?1", [context]).unwrap();
    db.execute("INSERT INTO pctx_context_acks SELECT session,epoch,'CTX-legacy-fixture',provenance,created FROM pctx_context_acks WHERE context=?1", [context]).unwrap();
    let (exit, _, old) = f.run(&[
        "context",
        "get",
        "--task-id",
        &f.task,
        "--session",
        &session,
        "--scope",
        "auth.py",
        "--mode",
        "delta",
        "--since",
        "CTX-legacy-fixture",
        "--budget-bytes",
        "20000",
    ]);
    assert_eq!(exit, 9, "{old}");
    assert_eq!(old["errors"][0]["code"], "BASELINE_MISMATCH");
    let schema: i64 = db
        .query_row("SELECT version FROM pctx_session_schema", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        schema, 1,
        "semantic serializer bump does not invent a DB migration"
    );
}

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
    // The now-visible budget.limit has fewer decimal digits than 20000.
    // Stabilize its width, then keep the exact complete-document assertion.
    let (exit, normalized, _) = f.get(&probe_session, &probe_bytes.len().to_string());
    assert_eq!(exit, 0);
    let budget = normalized.len();
    let (exit, bytes, full) = f.get(&session, &budget.to_string());
    assert_eq!(exit, 0, "{full}");
    assert_eq!(
        bytes.len(),
        budget,
        "complete final document includes newline and escapes"
    );
    assert_eq!(bytes.last(), Some(&b'\n'));
    let packet = &full["data"];
    assert_eq!(packet["serializer"], "adaptive-context-v3");
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
    assert_eq!(metadata["file:auth.py"]["representation"], "full_span");
    assert!(
        metadata["file:auth.py"]
            .get("delivered_text_hash")
            .is_some()
    );
    assert!(metadata["file:auth.py"].get("range").is_some());
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

#[test]
fn original_cli_budget_bounds_writer_admission_without_context_or_epoch_writes() {
    let f = fixture();
    let session = f.attach();
    let (exit, _, packet) = f.get(&session, "20000");
    assert_eq!(exit, 0, "{packet}");
    let context = packet["data"]["context_id"].as_str().unwrap();
    let db = f.db();
    let counts = || {
        [
            "pctx_sessions",
            "pctx_context_emissions",
            "pctx_context_acks",
            "pctx_session_events",
            "pctx_session_capsules",
        ]
        .map(|table| {
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap()
        })
    };
    let before = counts();
    db.execute_batch("BEGIN IMMEDIATE").unwrap();
    for args in [
        vec![
            "--timeout-ms",
            "100",
            "context",
            "get",
            "--task-id",
            &f.task,
            "--session",
            &session,
            "--scope",
            "auth.py",
            "--budget-bytes",
            "20000",
        ],
        vec![
            "--timeout-ms",
            "100",
            "context",
            "ack",
            context,
            "--session",
            &session,
            "--epoch",
            "1",
        ],
        vec![
            "--timeout-ms",
            "100",
            "session",
            "boundary",
            "--session",
            &session,
        ],
    ] {
        let begin = Instant::now();
        let (exit, _, failure) = f.run(&args);
        assert_eq!(exit, 7, "{args:?}: {failure}");
        // Busy admission can finish just before the deadline's final fraction;
        // neither case may renew the wait to the default five seconds.
        assert!(
            matches!(
                failure["errors"][0]["code"].as_str(),
                Some("TIMEOUT" | "INDEX_BUSY")
            ),
            "{failure}"
        );
        assert!(
            begin.elapsed() < Duration::from_secs(1),
            "request renewed its budget"
        );
        assert_eq!(counts(), before);
    }
    db.execute_batch("ROLLBACK").unwrap();
    assert_eq!(counts(), before);
    let epoch: i64 = db
        .query_row(
            "SELECT epoch FROM pctx_sessions WHERE id=?1",
            [&session],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(epoch, 1);
    let (_, _, current) = f.get(&session, "20000");
    assert_eq!(current["status"], "ok", "{current}");
}

#[test]
fn adaptive_delta_resends_unreceived_body_and_reuses_acknowledged_receipts() {
    let f = fixture();
    let source = format!(
        "def auth(\n    credential: str,\n):\n{}",
        ("    # preserved full body payload ".to_owned() + &"payload ".repeat(18) + "\n")
            .repeat(70)
    );
    fs::write(f.root.join("auth.py"), &source).unwrap();
    // A lexical candidate in Build's wider universe must not escape ContextGet scope.
    fs::write(
        f.root.join("representation.py"),
        "def outside():\n    return 'OFF_SCOPE_BODY'\n",
    )
    .unwrap();
    let session = f.attach();
    let (exit, full_bytes, full) = f.get(&session, "64000");
    assert_eq!(exit, 0, "{full}");
    let body = full["data"]["added"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["path"] == "auth.py")
        .unwrap();
    assert_eq!(body["representation"], "full_span");
    let budget = full_bytes.len() - serde_json::to_vec(body).unwrap().len() + 1000;
    let (exit, narrow_bytes, narrow) = f.get(&session, &budget.to_string());
    assert_eq!(exit, 0, "{narrow}");
    assert!(narrow_bytes.len() <= budget);
    assert_eq!(narrow["data"]["budget"]["used"], narrow_bytes.len());
    assert!(
        !String::from_utf8(narrow_bytes)
            .unwrap()
            .contains("representation.py")
    );
    let signature = narrow["data"]["added"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["path"] == "auth.py")
        .unwrap();
    assert_eq!(signature["representation"], "signature");
    assert_eq!(signature["body_omitted"], true);
    let range = &signature["signatures"][0]["range"];
    let start = range["start_byte"].as_u64().unwrap() as usize;
    let end = range["end_byte"].as_u64().unwrap() as usize;
    assert_eq!(signature["signatures"][0]["content"], source[start..end]);
    assert!(!signature.to_string().contains("preserved full body"));
    assert!(narrow["data"]["added"].as_array().unwrap().iter().any(|v| {
        v["kind"] == "required_rule"
            && v["text"]
                .as_str()
                .unwrap()
                .contains("Preserve mandatory content")
    }));
    let context = narrow["data"]["context_id"].as_str().unwrap();
    let run_delta = |budget: &str| {
        f.run(&[
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
            budget,
        ])
    };
    let ok_delta = |budget: &str| {
        let (exit, _, value) = run_delta(budget);
        assert_eq!(exit, 0, "{value}");
        value
    };
    let (exit, _, refused) = run_delta("64000");
    assert_eq!(exit, 9, "{refused}");
    assert_eq!(refused["errors"][0]["code"], "BASELINE_MISMATCH");
    f.ok(&[
        "context",
        "ack",
        context,
        "--session",
        &session,
        "--epoch",
        "1",
    ]);
    let db = f.db();
    let counts = || {
        [
            "pctx_context_emissions",
            "pctx_context_acks",
            "pctx_session_events",
        ]
        .map(|table| {
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap()
        })
    };
    let before = counts();
    let tight = budget.to_string();
    let unchanged = ok_delta(&tight);
    assert_eq!(unchanged["data"]["context_id"], context);
    assert_eq!(unchanged["data"]["unchanged"], true);
    assert!(unchanged["data"]["changed"].as_array().unwrap().is_empty());
    // Repeated query/ack must not grow either the receipt or event ledgers.
    ok_delta(&tight);
    f.ok(&[
        "context",
        "ack",
        context,
        "--session",
        &session,
        "--epoch",
        "1",
    ]);
    assert_eq!(counts(), before);
    let upgraded = ok_delta("64000");
    let changed = upgraded["data"]["changed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["path"] == "auth.py")
        .unwrap();
    assert_eq!(changed["representation"], "full_span");
    assert_eq!(changed["text"], source);
    assert_eq!(upgraded["data"]["unchanged"], false);
    assert_ne!(upgraded["data"]["context_id"], context);
    // The previously delivered full packet was never acked, so its body must
    // still be sent when upgrading the explicitly acknowledged signature.
    assert_eq!(upgraded["data"]["context_id"], full["data"]["context_id"]);
    let stored: String = db
        .query_row(
            "SELECT selection FROM pctx_context_emissions WHERE id=?1",
            [context],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!stored.contains("credential: str"));
    assert!(!stored.contains("preserved full body"));
    let metadata: Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(
        metadata["file:auth.py"]["signature_ranges"][0]["range"],
        *range
    );
    assert_eq!(
        metadata["file:auth.py"]["delivered_body_hash"],
        hash(serde_json::to_vec(signature).unwrap())
    );
    // Existing v2 data survives the semantic boundary, but cannot authorize v3 delta.
    db.execute("INSERT INTO pctx_context_emissions SELECT 'CTX-v2-fixture',session,epoch,task,policy,scope,'minimal-context-v2',content_hash,selection,created FROM pctx_context_emissions WHERE id=?1", [context]).unwrap();
    db.execute("INSERT INTO pctx_context_acks SELECT session,epoch,'CTX-v2-fixture',provenance,created FROM pctx_context_acks WHERE context=?1", [context]).unwrap();
    let (exit, _, legacy) = f.run(&[
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
        "CTX-v2-fixture",
        "--budget-bytes",
        "64000",
    ]);
    assert_eq!(exit, 9, "{legacy}");
    assert_eq!(legacy["errors"][0]["code"], "BASELINE_MISMATCH");
}

#[test]
fn omitted_sources_are_not_acknowledged_and_later_delivery_is_explicit() {
    let f = fixture();
    fs::write(f.root.join(".pctx/rules/required.md"),
        "---\nschema_version: 1\nid: global-policy\nrequired: true\nscope: ['**']\n---\nKeep the required policy in every full context.\n").unwrap();
    fs::create_dir(f.root.join("selected")).unwrap();
    for i in 0..20 {
        fs::write(
            f.root.join(format!("selected/file_{i:02}.py")),
            format!(
                "def selected_{i}(value: int):\n{}",
                ("    # optional source ".to_owned() + &"payload ".repeat(10) + "\n").repeat(20)
            ),
        )
        .unwrap();
    }
    let session = f.attach();
    let run = |mode: &str, since: Option<&str>, budget: &str| {
        let mut args = vec![
            "context",
            "get",
            "--task-id",
            &f.task,
            "--session",
            &session,
            "--scope",
            "selected",
            "--mode",
            mode,
            "--budget-bytes",
            budget,
        ];
        if let Some(id) = since {
            args.extend(["--since", id]);
        }
        let (exit, bytes, value) = f.run(&args);
        assert_eq!(exit, 0, "{mode}/{budget}: {value}");
        assert!(bytes.len() <= budget.parse::<usize>().unwrap());
        assert_eq!(value["data"]["budget"]["used"], bytes.len());
        value
    };
    let small = run("full", None, "8000");
    assert_eq!(small["data"]["selection_plan"]["selection_complete"], false);
    assert!(
        small["data"]["selection_plan"]["omitted_count"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(small["data"]["added"].as_array().unwrap().iter().any(|v| {
        v["required"] == true
            && v["text"]
                .as_str()
                .unwrap()
                .contains("Keep the required policy")
    }));
    let id = small["data"]["context_id"].as_str().unwrap();
    let omitted = small["data"]["selection_plan"]["omissions"]
        .as_array()
        .unwrap();
    assert!(
        !omitted.is_empty(),
        "fixture must preserve explicit omitted paths"
    );
    let db = f.db();
    let stored: String = db
        .query_row(
            "SELECT selection FROM pctx_context_emissions WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    let metadata: Value = serde_json::from_str(&stored).unwrap();
    for entry in omitted {
        assert!(
            metadata
                .get(format!("file:{}", entry["path"].as_str().unwrap()))
                .is_none()
        );
    }
    let ack = f.ok(&[
        "context",
        "ack",
        id,
        "--session",
        &session,
        "--epoch",
        "1",
        "--provenance",
        "transport-receipt",
    ]);
    assert_eq!(ack["data"]["receipt_reused"], false);
    let reused = f.ok(&[
        "context",
        "ack",
        id,
        "--session",
        &session,
        "--epoch",
        "1",
        "--provenance",
        "explicit-agent",
    ]);
    assert_eq!(reused["data"]["receipt_reused"], true);
    assert_eq!(
        reused["data"]["provenance"], "transport-receipt",
        "reuse preserves recorded provenance"
    );
    let expanded = run("delta", Some(id), "64000");
    assert_eq!(
        expanded["data"]["selection_plan"]["selection_complete"],
        true
    );
    for entry in omitted {
        assert!(
            expanded["data"]["added"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["path"] == entry["path"]
                    && v["representation"] == "full_span"
                    && v["text"].as_str().unwrap().contains("optional source"))
        );
    }
    assert!(!stored.contains("optional source"));
    assert!(!stored.contains("Keep the required policy"));
    let expanded_id = expanded["data"]["context_id"].as_str().unwrap();
    f.ok(&[
        "context",
        "ack",
        expanded_id,
        "--session",
        &session,
        "--epoch",
        "1",
    ]);
    for i in 0..20 {
        fs::write(
            f.root.join(format!("selected/file_{i:02}.py")),
            format!(
                "def changed_{i}(value: int):\n{}",
                ("    # current revision ".to_owned() + &"changed ".repeat(10) + "\n").repeat(20)
            ),
        )
        .unwrap();
    }
    // Keep capacity for every prior selection tombstone as well as changed bodies.
    let changed = run("delta", Some(expanded_id), "20000");
    assert!(!changed["data"]["changed"].as_array().unwrap().is_empty());
    assert!(
        !changed["data"]["invalidated"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for tombstone in changed["data"]["invalidated"].as_array().unwrap() {
        assert_eq!(tombstone["reason"], "no_longer_selected");
        assert_eq!(tombstone["tombstone"], true);
        assert!(
            f.root
                .join(tombstone["previous"]["path"].as_str().unwrap())
                .is_file()
        );
    }
    assert!(
        changed["data"]["removed"].as_array().unwrap().is_empty(),
        "budget omission is not source deletion"
    );
}

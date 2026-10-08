//! Native CLI owner-bound policy mutation and reporting-only control boundaries.
use serde_json::{Value, json};
use std::{fs, process::Command};
struct Fixture {
    temp: tempfile::TempDir,
    root: std::path::PathBuf,
    data: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        let data = temp.path().join("data");
        let f = Self { temp, root, data };
        f.ok("alice", &["init"]);
        let config = f.root.join(".pctx/config.toml");
        let mut text = fs::read_to_string(&config).unwrap();
        text.push_str("\n[[policy.source_topics]]\nscope=['**']\ntopics=[]\n");
        fs::write(config, text).unwrap();
        f
    }
    fn run(&self, principal: &str, args: &[&str]) -> (i32, Value) {
        let o = Command::new(env!("CARGO_BIN_EXE_pctx"))
            .arg("--root")
            .arg(&self.root)
            .args(["--format", "json"])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_ACTOR", "owner")
            .env("PCTX_OWNER_PRINCIPAL", principal)
            .env_remove("PCTX_RUN_ID")
            .env_remove("PCTX_RUN_CAPABILITY")
            .env(
                "GIT_CONFIG_GLOBAL",
                self.temp.path().join("empty.gitconfig"),
            )
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        (
            o.status.code().unwrap(),
            serde_json::from_slice(&o.stdout).unwrap_or_else(|e| {
                panic!("{args:?}: {e}: {}", String::from_utf8_lossy(&o.stderr))
            }),
        )
    }
    fn ok(&self, p: &str, args: &[&str]) -> Value {
        let (n, v) = self.run(p, args);
        assert_eq!(n, 0, "{v}");
        v["data"].clone()
    }
    fn input(&self, name: &str, v: Value) -> String {
        let p = self.temp.path().join(name);
        fs::write(&p, v.to_string()).unwrap();
        p.to_str().unwrap().to_owned()
    }
    fn restrictions(&self) -> Value {
        self.ok("alice", &["role", "list"])["restrictions"].clone()
    }
}
#[test]
fn original_owner_only_release_and_stale_revision_never_mutates_policy() {
    let f = Fixture::new();
    let created = f.ok(
        "alice",
        &["role", "pause", "legal", "--reason", "authored pause"],
    );
    let id = created["restriction_id"].as_str().unwrap();
    let revision = created["revision"].as_i64().unwrap();
    let (n, _) = f.run(
        "bob",
        &["role", "resume", "legal", "--reason", "wrong owner"],
    );
    assert_eq!(n, 5);
    let stale=f.input("stale.json",json!({"schema_version":1,"id":id,"expected_revision":revision+1,"owner_evidence":"fixture:owner-answer"}));
    let (n, _) = f.run("alice", &["policy", "release", "--from-file", &stale]);
    assert_eq!(n, 9);
    let current=f.input("current.json",json!({"schema_version":1,"id":id,"expected_revision":revision,"owner_evidence":"fixture:owner-answer"}));
    f.ok("alice", &["policy", "release", "--from-file", &current]);
    let listed = f.restrictions();
    assert_eq!(listed[0]["original_owner"], "alice");
    assert_eq!(listed[0]["active"], false);
    let (n, _) = f.run(
        "bob",
        &[
            "role",
            "pause",
            "legal",
            "--reason",
            "overwrite original owner",
        ],
    );
    assert_eq!(n, 5);
}
#[test]
fn explicitly_authored_reporting_precedence_scope_and_revocation() {
    let f = Fixture::new();
    let restriction = f.ok(
        "alice",
        &[
            "role",
            "pause",
            "legal",
            "--topic",
            "quiet",
            "--recipient",
            "owner",
            "--reason",
            "topic restriction",
        ],
    );
    let now = pctx::domain::now();
    let base = json!({"schema_version":1,"restriction_refs":[{"id":restriction["restriction_id"],"revision":restriction["revision"],"precedence":"exception"}],"role":"legal","recipient":"owner","topic":"quiet","source_scope":["src/**"],"payload_hash":pctx::domain::hash("scoped-report-marker"),"category":"security","not_before":now-1,"expires_at":now+600,"reason":"bounded reporting exception","owner_evidence":"fixture:original-owner"});
    let path = f.input("exception.json", base.clone());
    let (n, _) = f.run("bob", &["policy", "exception-record", "--from-file", &path]);
    assert_eq!(n, 5);
    let exception = f.ok(
        "alice",
        &["policy", "exception-record", "--from-file", &path],
    );
    let report = json!({"schema_version":1,"role":"legal","recipient":"owner","topic":"quiet","category":"security","source_paths":["src/check.rs"],"payload_hash":pctx::domain::hash("scoped-report-marker")});
    let path = f.input("report.json", report.clone());
    let allowed = f.ok(
        "alice",
        &["policy", "report-evaluate", "--from-file", &path],
    );
    assert_eq!(allowed["state"], "allowed");
    let mut wrong_body = report.clone();
    wrong_body["payload_hash"] = json!(pctx::domain::hash("unapproved-body-marker"));
    let path = f.input("wrong-body.json", wrong_body);
    assert_eq!(
        f.ok(
            "alice",
            &["policy", "report-evaluate", "--from-file", &path]
        )["state"],
        "held"
    );
    let mut excluded = report.clone();
    excluded["source_paths"] = json!([".env"]);
    let path = f.input("excluded.json", excluded);
    let held = f.ok(
        "alice",
        &["policy", "report-evaluate", "--from-file", &path],
    );
    assert_eq!(held["state"], "held");
    assert_eq!(held["reason_code"], "SOURCE_POLICY_DENIED");
    assert!(!held.to_string().contains(".env"));
    let mut outside = report.clone();
    outside["source_paths"] = json!(["private.txt"]);
    let path = f.input("outside.json", outside);
    assert_eq!(
        f.ok(
            "alice",
            &["policy", "report-evaluate", "--from-file", &path]
        )["state"],
        "held"
    );
    let revoke=f.input("revoke.json",json!({"schema_version":1,"id":exception["exception_id"],"expected_revision":exception["revision"],"owner_evidence":"fixture:revocation"}));
    f.ok(
        "alice",
        &["policy", "exception-revoke", "--from-file", &revoke],
    );
    let path = f.input("report.json", report.clone());
    assert_eq!(
        f.ok(
            "alice",
            &["policy", "report-evaluate", "--from-file", &path]
        )["state"],
        "held"
    );
    let mut undefined = base;
    undefined["restriction_refs"][0]["precedence"] = Value::Null;
    let path = f.input("undefined.json", undefined);
    f.ok(
        "alice",
        &["policy", "exception-record", "--from-file", &path],
    );
    let path = f.input("report.json", report);
    let held = f.ok(
        "alice",
        &["policy", "report-evaluate", "--from-file", &path],
    );
    assert_eq!(held["state"], "held");
    assert!(!held.to_string().contains("quiet"));
}

#[test]
fn reporting_exception_delivers_mailbox_only_and_revocation_blocks_ack() {
    let f = Fixture::new();
    let agent = f.ok("alice", &["agent", "register", "--name", "report-worker"])["agent_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let sid = f.ok(
        "alice",
        &[
            "session",
            "attach",
            "--agent",
            &agent,
            "--runtime",
            "manual",
            "--role",
            "legal",
        ],
    )["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let restriction = f.ok(
        "alice",
        &[
            "role",
            "pause",
            "legal",
            "--topic",
            "quiet",
            "--recipient",
            &agent,
            "--reason",
            "reporting restriction",
        ],
    );
    let message=f.input("message.json",json!({"schema_version":1,"type":"security","topic":"quiet","body":"scoped-report-marker","priority":0,"idempotency_key":"scoped-report","reporting":{"category":"security","source_paths":["src/check.rs"]}}));
    let payload_hash = f.ok(
        "alice",
        &["policy", "report-fingerprint", "--from-file", &message],
    )["payload_hash"]
        .clone();
    let queued = f.ok(
        "alice",
        &[
            "message",
            "send",
            "--from-file",
            &message,
            "--to-session",
            &sid,
        ],
    );
    let before = f.ok("alice", &["inbox", "read", "--session", &sid]);
    assert_eq!(before["messages"].as_array().unwrap().len(), 0);
    assert!(!before.to_string().contains("scoped-report-marker"));
    let now = pctx::domain::now();
    let path=f.input("exception.json",json!({"schema_version":1,"restriction_refs":[{"id":restriction["restriction_id"],"revision":restriction["revision"],"precedence":"exception"}],"role":"legal","recipient":agent,"topic":"quiet","source_scope":["src/**"],"payload_hash":payload_hash,"category":"security","not_before":now-1,"expires_at":now+600,"reason":"scoped security report","owner_evidence":"fixture:owner"}));
    let exception = f.ok(
        "alice",
        &["policy", "exception-record", "--from-file", &path],
    );
    let read = f.ok("alice", &["inbox", "read", "--session", &sid]);
    assert_eq!(
        read["messages"][0]["message"]["body"],
        "scoped-report-marker"
    );
    assert!(read["messages"][0]["delivery_policy_hash"].is_string());
    assert_eq!(read["model_woken"], false);
    let mut forged: Value = serde_json::from_slice(&fs::read(&message).unwrap()).unwrap();
    forged["idempotency_key"] = json!("forged-report");
    forged["evidence_refs"] = json!(["hidden-topic-title-marker"]);
    forged["correlation_id"] = json!("hidden-correlation-marker");
    let path = f.input("forged-message.json", forged);
    f.ok(
        "alice",
        &[
            "message",
            "send",
            "--from-file",
            &path,
            "--to-session",
            &sid,
        ],
    );
    let read = f.ok("alice", &["inbox", "read", "--session", &sid]);
    assert!(!read.to_string().contains("hidden-topic-title-marker"));
    assert!(!read.to_string().contains("hidden-correlation-marker"));
    let ordinary=f.input("ordinary.json",json!({"schema_version":1,"type":"notice","topic":"quiet","body":"withheld-ordinary-marker","priority":0,"idempotency_key":"ordinary"}));
    f.ok(
        "alice",
        &[
            "message",
            "send",
            "--from-file",
            &ordinary,
            "--to-session",
            &sid,
        ],
    );
    let read = f.ok("alice", &["inbox", "read", "--session", &sid]);
    assert!(!read.to_string().contains("withheld-ordinary-marker"));
    let path=f.input("revoke.json",json!({"schema_version":1,"id":exception["exception_id"],"expected_revision":exception["revision"],"owner_evidence":"fixture:revocation"}));
    f.ok(
        "alice",
        &["policy", "exception-revoke", "--from-file", &path],
    );
    let (n, v) = f.run(
        "alice",
        &[
            "message",
            "ack",
            queued["message_id"].as_str().unwrap(),
            "--session",
            &sid,
        ],
    );
    assert_eq!(n, 5, "{v}");
    let after = f.ok("alice", &["inbox", "read", "--session", &sid]);
    assert!(after["messages"].as_array().unwrap().is_empty());
}

#[test]
fn legacy_owner_is_unknown_until_explicit_attestation_and_agent_run_cannot_claim_owner() {
    let f = Fixture::new();
    f.ok(
        "alice",
        &["role", "pause", "legal", "--reason", "legacy fixture"],
    );
    let control = fs::read_dir(f.data.join("controls"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let db = rusqlite::Connection::open(control.join("control.sqlite3")).unwrap();
    db.execute_batch("DROP TABLE ops_reporting_exceptions; DROP TABLE ops_restrictions; DROP TABLE ops_policy_schema;").unwrap();
    drop(db);
    let legacy = f.restrictions();
    assert!(legacy[0]["original_owner"].is_null());
    assert_eq!(legacy[0]["active"], true);
    let (n, _) = f.run(
        "alice",
        &["role", "resume", "legal", "--reason", "cannot guess owner"],
    );
    assert_eq!(n, 5);
    let path=f.input("attest.json",json!({"schema_version":1,"id":legacy[0]["id"],"expected_revision":legacy[0]["revision"],"original_owner":"alice","owner_evidence":"fixture:legacy-owner-attestation"}));
    f.ok("alice", &["policy", "attest-owner", "--from-file", &path]);
    f.ok(
        "alice",
        &[
            "role",
            "resume",
            "legal",
            "--reason",
            "attested original owner",
        ],
    );
    let output = Command::new(env!("CARGO_BIN_EXE_pctx"))
        .arg("--root")
        .arg(&f.root)
        .args([
            "--format",
            "json",
            "role",
            "pause",
            "legal",
            "--reason",
            "forged owner",
        ])
        .env("PCTX_DATA_DIR", &f.data)
        .env("PCTX_ACTOR", "owner")
        .env("PCTX_OWNER_PRINCIPAL", "alice")
        .env("PCTX_RUN_ID", "agent-run-evidence")
        .env_remove("PCTX_RUN_CAPABILITY")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(f.restrictions()[0]["active"], false);
}

#[test]
fn reporting_pause_exception_expires_without_releasing_pause() {
    let f = Fixture::new();
    let pause = f.ok(
        "alice",
        &["role", "pause", "legal", "--reason", "delivery pause"],
    );
    let now = pctx::domain::now();
    let expiry = now + 3;
    let path=f.input("pause-report.json",json!({"schema_version":1,"restriction_refs":[{"id":pause["restriction_id"],"revision":pause["revision"],"precedence":"exception"}],"role":"legal","recipient":"owner","topic":"incident","source_scope":["src/**"],"payload_hash":pctx::domain::hash("authorized incident"),"category":"permission_incident","not_before":now-1,"expires_at":expiry,"reason":"report without resuming work","owner_evidence":"fixture:owner"}));
    f.ok(
        "alice",
        &["policy", "exception-record", "--from-file", &path],
    );
    let report=f.input("pause-evaluate.json",json!({"schema_version":1,"role":"legal","recipient":"owner","topic":"incident","source_paths":["src/check.rs"],"payload_hash":pctx::domain::hash("authorized incident"),"category":"permission_incident"}));
    assert_eq!(
        f.ok(
            "alice",
            &["policy", "report-evaluate", "--from-file", &report]
        )["state"],
        "allowed"
    );
    assert_eq!(f.restrictions()[0]["active"], true);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while pctx::domain::now() < expiry {
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        f.ok(
            "alice",
            &["policy", "report-evaluate", "--from-file", &report]
        )["state"],
        "held"
    );
    assert_eq!(f.restrictions()[0]["active"], true);
}

#[test]
fn report_source_topics_require_explicit_coverage_and_unknown_sources_hold() {
    let f = Fixture::new();
    let config = f.root.join(".pctx/config.toml");
    let mut text = fs::read_to_string(&config).unwrap();
    text.push_str("\n[[policy.source_topics]]\nscope=['src/**']\ntopics=['medical']\n");
    fs::write(&config, text).unwrap();
    let packet = f.ok(
        "alice",
        &[
            "role",
            "pause",
            "legal",
            "--topic",
            "quiet",
            "--recipient",
            "owner",
            "--reason",
            "packet restriction",
        ],
    );
    let source = f.ok(
        "alice",
        &[
            "role",
            "pause",
            "legal",
            "--topic",
            "medical",
            "--recipient",
            "owner",
            "--reason",
            "source restriction",
        ],
    );
    let report = json!({"schema_version":1,"role":"legal","recipient":"owner","topic":"quiet","category":"security","source_paths":["src/check.rs"],"payload_hash":pctx::domain::hash("authorized report projection")});
    let now = pctx::domain::now();
    let mut exception = json!({"schema_version":1,"restriction_refs":[{"id":packet["restriction_id"],"revision":packet["revision"],"precedence":"exception"}],"role":"legal","recipient":"owner","topic":"quiet","source_scope":["src/**"],"payload_hash":report["payload_hash"],"category":"security","not_before":now-1,"expires_at":now+600,"reason":"exact packet exception","owner_evidence":"fixture:owner"});
    let path = f.input("packet-exception.json", exception.clone());
    f.ok(
        "alice",
        &["policy", "exception-record", "--from-file", &path],
    );
    let path = f.input("source-report.json", report.clone());
    let held = f.ok(
        "alice",
        &["policy", "report-evaluate", "--from-file", &path],
    );
    assert_eq!(held["state"], "held");
    assert!(!held.to_string().contains("medical"));
    exception["restriction_refs"].as_array_mut().unwrap().push(json!({"id":source["restriction_id"],"revision":source["revision"],"precedence":"exception"}));
    let path = f.input("all-source-exception.json", exception);
    f.ok(
        "alice",
        &["policy", "exception-record", "--from-file", &path],
    );
    let path = f.input("source-report.json", report.clone());
    assert_eq!(
        f.ok(
            "alice",
            &["policy", "report-evaluate", "--from-file", &path]
        )["state"],
        "allowed"
    );
    // Removing source assignments cannot turn unknown sources into public input.
    let text = fs::read_to_string(&config).unwrap();
    let marker = text.find("\n[[policy.source_topics]]").unwrap();
    fs::write(&config, &text[..marker]).unwrap();
    let path = f.input("unknown-source.json", report);
    let held = f.ok(
        "alice",
        &["policy", "report-evaluate", "--from-file", &path],
    );
    assert_eq!(held["state"], "held");
    assert_eq!(held["reason_code"], "SOURCE_TOPIC_REQUIRED");
    assert!(!held.to_string().contains("src/check.rs"));
}

#[test]
fn managed_source_topic_change_blocks_previously_allowed_report_and_ack() {
    let f = Fixture::new();
    let agent = f.ok("alice", &["agent", "register", "--name", "topic-worker"])["agent_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let sid = f.ok(
        "alice",
        &[
            "session",
            "attach",
            "--agent",
            &agent,
            "--runtime",
            "manual",
            "--role",
            "legal",
        ],
    )["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let source_path = ".pctx/decisions/report-source.md";
    fs::create_dir_all(f.root.join(".pctx/decisions")).unwrap();
    let source = f.root.join(source_path);
    let write_source = |topics: &str| {
        fs::write(&source,format!("---\nid: report-source\nstatus: accepted\ndate: '2026-10-08'\nscope: [code.py]\ntopics: {topics}\n---\nsource-body-marker\n")).unwrap()
    };
    write_source("[]");
    let packet = f.ok(
        "alice",
        &[
            "role",
            "pause",
            "legal",
            "--topic",
            "quiet",
            "--recipient",
            &agent,
            "--reason",
            "packet restriction",
        ],
    );
    f.ok(
        "alice",
        &[
            "role",
            "pause",
            "legal",
            "--topic",
            "medical",
            "--recipient",
            &agent,
            "--reason",
            "source restriction",
        ],
    );
    let message=f.input("managed-report.json",json!({"schema_version":1,"type":"security","topic":"quiet","body":"authorized-managed-report-marker","priority":0,"idempotency_key":"managed-report","reporting":{"category":"security","source_paths":[source_path]}}));
    let payload_hash = f.ok(
        "alice",
        &["policy", "report-fingerprint", "--from-file", &message],
    )["payload_hash"]
        .clone();
    let now = pctx::domain::now();
    let path=f.input("managed-exception.json",json!({"schema_version":1,"restriction_refs":[{"id":packet["restriction_id"],"revision":packet["revision"],"precedence":"exception"}],"role":"legal","recipient":agent,"topic":"quiet","source_scope":[".pctx/decisions/**"],"payload_hash":payload_hash,"category":"security","not_before":now-1,"expires_at":now+600,"reason":"bounded metadata report","owner_evidence":"fixture:owner"}));
    f.ok(
        "alice",
        &["policy", "exception-record", "--from-file", &path],
    );
    let queued = f.ok(
        "alice",
        &[
            "message",
            "send",
            "--from-file",
            &message,
            "--to-session",
            &sid,
        ],
    );
    let before = f.ok("alice", &["inbox", "read", "--session", &sid]);
    assert_eq!(before["messages"].as_array().unwrap().len(), 1);
    write_source("[medical]");
    let after = f.ok("alice", &["inbox", "read", "--session", &sid]);
    assert!(after["messages"].as_array().unwrap().is_empty());
    let text = after.to_string();
    assert!(
        !text.contains("authorized-managed-report-marker")
            && !text.contains("medical")
            && !text.contains("report-source.md")
    );
    let (n, error) = f.run(
        "alice",
        &[
            "message",
            "ack",
            queued["message_id"].as_str().unwrap(),
            "--session",
            &sid,
        ],
    );
    assert_eq!(n, 5, "{error}");
}

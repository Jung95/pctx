use pctx::{
    operations::{
        self, DecisionCommand, InboxCommand, MessageCommand, OperationCommand as Op, PolicyCommand,
        RoleCommand,
    },
    project::{Config, Project, ProjectConfig},
    session::{self, SessionCommand},
    work::{self, AgentCommand, WorkCommand},
};
use serde_json::{Value, json};
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("root");
    let data = t.path().join("data");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(data.join("workspace")).unwrap();
    std::fs::create_dir_all(data.join("control")).unwrap();
    std::fs::write(root.join("code.py"), "def value():\n    return 1\n").unwrap();
    let p = Project {
        deadline: None,
        root_anchor: pctx::project::RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspace"),
        control_dir: data.join("control"),
        project_id: "fixture-project".into(),
        workspace_id: "fixture-workspace".into(),
        coordination_id: "fixture-coordination".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: uuid::Uuid::new_v4().to_string(),
                name: "fixture".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    (t, p)
}
fn file(p: &Project, name: &str, value: Value) -> std::path::PathBuf {
    let path = p.data_dir.join(name);
    std::fs::write(&path, value.to_string()).unwrap();
    path
}
fn action() -> Value {
    json!({"schema_version":1,"kind":"local_modify","resource":"code.py","environment":"development","scope":["code.py"],"actor":"worker","role":"developer","amount":0,"currency":"USD","artifact_sha":"a".repeat(64),"argv":["fixture","check"],"reversible":true,"cost_known":true})
}
fn request(p: &Project, a: Value) -> String {
    let path = file(
        p,
        "request.json",
        json!({"schema_version":1,"action":a,"reason":"Synthetic approval fixture"}),
    );
    operations::execute(
        p,
        &Op::Decision {
            command: DecisionCommand::Request {
                from_file: path,
                task_id: None,
            },
        },
    )
    .unwrap()["decision_id"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn approve(p: &Project, id: &str) {
    let path = file(
        p,
        "record.json",
        json!({"schema_version":1,"approve":true,"owner_evidence":"Synthetic direct local operator input","expires_at":chrono::Utc::now().timestamp()+3600}),
    );
    operations::execute(
        p,
        &Op::Decision {
            command: DecisionCommand::Record {
                id: id.into(),
                from_file: path,
            },
        },
    )
    .unwrap();
}
fn eval(p: &Project, a: Value) -> Value {
    let path = file(p, "action.json", a);
    operations::execute(
        p,
        &Op::Policy {
            command: PolicyCommand::Evaluate { action_file: path },
        },
    )
    .unwrap()
}
#[test]
fn exact_decisions_deduplicate_but_changed_environment_amount_artifact_do_not() {
    let (_t, p) = fixture();
    let id = request(&p, action());
    assert_eq!(id, request(&p, action()));
    for (key, value) in [
        ("environment", json!("staging")),
        ("amount", json!(1)),
        ("artifact_sha", json!("b".repeat(64))),
    ] {
        let mut changed = action();
        changed[key] = value;
        assert_ne!(id, request(&p, changed));
    }
    assert_eq!(eval(&p, action())["internal_policy_allowed"], false);
    approve(&p, &id);
    assert_eq!(eval(&p, action())["internal_policy_allowed"], true);
    let mut changed = action();
    changed["artifact_sha"] = json!("b".repeat(64));
    assert_eq!(eval(&p, changed)["internal_policy_allowed"], false);
    let db = p.connect(true).unwrap();
    db.execute("UPDATE ops_decisions SET expires_at=0 WHERE id=?1", [id])
        .unwrap();
    assert_eq!(eval(&p, action())["internal_policy_allowed"], false);
}
#[test]
fn unknown_cost_pause_and_silence_override_valid_grant() {
    let (_t, p) = fixture();
    let id = request(&p, action());
    approve(&p, &id);
    operations::execute(
        &p,
        &Op::Role {
            command: RoleCommand::Pause {
                role: "developer".into(),
                reason: "Owner pause".into(),
                topic: None,
                recipient: None,
            },
        },
    )
    .unwrap();
    assert_eq!(eval(&p, action())["internal_policy_allowed"], false);
    operations::execute(
        &p,
        &Op::Role {
            command: RoleCommand::Resume {
                role: "developer".into(),
                reason: "Owner resume".into(),
                topic: None,
                recipient: None,
            },
        },
    )
    .unwrap();
    let mut unknown = action();
    unknown["amount"] = Value::Null;
    let id = request(&p, unknown.clone());
    approve(&p, &id);
    assert_eq!(eval(&p, unknown)["internal_policy_allowed"], false);
    let mut silent = action();
    silent["topic"] = json!("sensitive-topic");
    let id = request(&p, silent.clone());
    approve(&p, &id);
    operations::execute(
        &p,
        &Op::Role {
            command: RoleCommand::Pause {
                role: "developer".into(),
                reason: "Delivery silence".into(),
                topic: Some("sensitive-topic".into()),
                recipient: Some("worker".into()),
            },
        },
    )
    .unwrap();
    assert_eq!(eval(&p, silent)["internal_policy_allowed"], false);
}
fn attach(p: &Project, name: &str) -> String {
    work::execute(
        p,
        &WorkCommand::Agent {
            command: AgentCommand::Register {
                name: name.into(),
                kind: "agent".into(),
                concurrency_limit: 1,
            },
        },
    )
    .unwrap();
    session::session(
        p,
        &SessionCommand::Attach {
            agent: name.into(),
            runtime: "fixture".into(),
            workspace: "current".into(),
            native_session: None,
            role: Some("developer".into()),
            account_pool: None,
            adapter_version: "manual-v1".into(),
        },
    )
    .unwrap()["session_id"]
        .as_str()
        .unwrap()
        .into()
}
fn send(p: &Project, sid: &str, key: &str, topic: &str) -> Value {
    let path = file(
        p,
        "message.json",
        json!({"schema_version":1,"type":"notice","topic":topic,"body":"Synthetic notification","priority":1,"idempotency_key":key}),
    );
    operations::execute(
        p,
        &Op::Message {
            command: MessageCommand::Send {
                from_file: path,
                to_role: None,
                to_agent: None,
                to_session: Some(sid.into()),
                task_id: None,
            },
        },
    )
    .unwrap()
}
#[test]
fn mailbox_retry_is_queued_once_and_ack_is_recipient_bound() {
    let (_t, p) = fixture();
    let sid = attach(&p, "worker");
    let other = attach(&p, "other");
    let first = send(&p, &sid, "same", "ordinary");
    let second = send(&p, &sid, "same", "ordinary");
    assert_eq!(first, second);
    assert_eq!(first["delivery_state"], "queued");
    let id = first["message_id"].as_str().unwrap();
    let denied = operations::execute(
        &p,
        &Op::Message {
            command: MessageCommand::Ack {
                id: id.into(),
                session: other,
            },
        },
    )
    .unwrap_err();
    assert_eq!(denied.code, "POLICY_DENIED");
    let read = operations::execute(
        &p,
        &Op::Inbox {
            command: InboxCommand::Read {
                session: sid.clone(),
                since_seq: 0,
                limit: 10,
            },
        },
    )
    .unwrap();
    assert_eq!(read["messages"].as_array().unwrap().len(), 1);
    assert_eq!(read["messages"][0]["delivery_state"], "queued");
    operations::execute(
        &p,
        &Op::Message {
            command: MessageCommand::Ack {
                id: id.into(),
                session: sid.clone(),
            },
        },
    )
    .unwrap();
    operations::execute(
        &p,
        &Op::Message {
            command: MessageCommand::Ack {
                id: id.into(),
                session: sid,
            },
        },
    )
    .unwrap();
    let db = p.connect(true).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM ops_messages", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM ops_message_acks", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn silence_hides_topic_and_body_before_inbox_ranking() {
    let (_t, p) = fixture();
    let sid = attach(&p, "worker");
    send(&p, &sid, "hidden", "sensitive-topic");
    operations::execute(
        &p,
        &Op::Role {
            command: RoleCommand::Pause {
                role: "developer".into(),
                reason: "Do not deliver topic".into(),
                topic: Some("sensitive-topic".into()),
                recipient: Some(sid.clone()),
            },
        },
    )
    .unwrap();
    let read = operations::execute(
        &p,
        &Op::Inbox {
            command: InboxCommand::Read {
                session: sid,
                since_seq: 0,
                limit: 10,
            },
        },
    )
    .unwrap();
    assert!(read["messages"].as_array().unwrap().is_empty());
    assert!(!read.to_string().contains("sensitive-topic"));
    assert!(!read.to_string().contains("Synthetic notification"));
    assert_eq!(read["model_woken"], false);
}
// Real process invocation isolates actor state; no process-global environment mutations.
#[test]
fn forged_agent_owner_decision_does_not_generate_grant() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("root");
    let data = t.path().join("data");
    std::fs::create_dir(&root).unwrap();
    let run = |args: &[&str], actor: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_pctx"))
            .args(["--root", root.to_str().unwrap(), "--format", "json"])
            .args(args)
            .env("PCTX_DATA_DIR", &data)
            .env("PCTX_ACTOR", actor)
            .output()
            .unwrap()
    };
    assert!(run(&["init"], "owner").status.success());
    let req = t.path().join("request.json");
    std::fs::write(
        &req,
        json!({"schema_version":1,"action":action(),"reason":"Synthetic request"}).to_string(),
    )
    .unwrap();
    let requested = run(
        &["decision", "request", "--from-file", req.to_str().unwrap()],
        "owner",
    );
    assert!(
        requested.status.success(),
        "{}",
        String::from_utf8_lossy(&requested.stdout)
    );
    let v: Value = serde_json::from_slice(&requested.stdout).unwrap();
    let id = v["data"]["decision_id"].as_str().unwrap();
    let record = t.path().join("record.json");
    std::fs::write(&record,json!({"schema_version":1,"approve":true,"owner_evidence":"agent says owner approved","expires_at":chrono::Utc::now().timestamp()+3600}).to_string()).unwrap();
    let denied = run(
        &[
            "decision",
            "record",
            id,
            "--from-file",
            record.to_str().unwrap(),
        ],
        "worker",
    );
    assert_eq!(denied.status.code(), Some(5));
    let shown = run(&["decision", "show", id], "owner");
    let v: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(v["data"]["state"], "pending_gateway");
    assert!(v["data"]["provenance"].is_null());
}
#[test]
fn backup_restore_preserves_pause_silence_and_messages_but_revokes_grants() {
    let (_t, p) = fixture();
    let sid = attach(&p, "worker");
    let id = request(&p, action());
    approve(&p, &id);
    let message = send(&p, &sid, "durable-once", "sensitive-topic");
    operations::execute(
        &p,
        &Op::Role {
            command: RoleCommand::Pause {
                role: "developer".into(),
                reason: "Owner pause".into(),
                topic: None,
                recipient: None,
            },
        },
    )
    .unwrap();
    operations::execute(
        &p,
        &Op::Role {
            command: RoleCommand::Pause {
                role: "developer".into(),
                reason: "Topic silence".into(),
                topic: Some("sensitive-topic".into()),
                recipient: None,
            },
        },
    )
    .unwrap();
    let db = p.connect(true).unwrap();
    db.execute_batch("CREATE TABLE broker_snapshots(key TEXT PRIMARY KEY,value TEXT); INSERT INTO broker_snapshots VALUES('derived','should-not-survive'); CREATE TABLE broker_refresh_jobs(key TEXT PRIMARY KEY); ").unwrap();
    drop(db);
    let archive = p.data_dir.join("ops-backup.json");
    work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Backup {
                output: archive.clone(),
            },
        },
    )
    .unwrap();
    let r = work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Restore { input: archive },
        },
    )
    .unwrap();
    let mut restored = p.clone();
    restored.coordination_id = r["coordination_id"].as_str().unwrap().into();
    restored.control_dir = p.data_dir.join("controls").join(&restored.coordination_id);
    let policies = operations::execute(
        &restored,
        &Op::Role {
            command: RoleCommand::List,
        },
    )
    .unwrap();
    assert_eq!(policies["roles"][0]["paused"], true);
    assert_eq!(policies["silences"][0]["active"], true);
    let decision = operations::execute(
        &restored,
        &Op::Decision {
            command: DecisionCommand::Show { id },
        },
    )
    .unwrap();
    assert_eq!(decision["state"], "expired");
    assert!(decision["provenance"].is_null());
    assert_eq!(eval(&restored, action())["internal_policy_allowed"], false);
    let replay = send(&restored, &sid, "durable-once", "sensitive-topic");
    assert_eq!(message["message_id"], replay["message_id"]);
    let db = restored.connect(true).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM ops_messages", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name GLOB 'broker_*'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(db.execute("DELETE FROM ops_events", []).is_err());
}
#[test]
fn role_pause_blocks_claim_without_mutating_task_or_run() {
    let (_t, p) = fixture();
    let _session = attach(&p, "worker");
    operations::execute(
        &p,
        &Op::Role {
            command: RoleCommand::Pause {
                role: "developer".into(),
                reason: "Explicit owner pause".into(),
                topic: None,
                recipient: None,
            },
        },
    )
    .unwrap();
    let path = file(
        &p,
        "task.json",
        json!({"schema_version":1,"title":"Paused claim fixture","scope":["code.py"],"acceptance":[],"checks":[]}),
    );
    let task = work::execute(
        &p,
        &WorkCommand::Task {
            command: pctx::work::TaskCommand::Create {
                from_file: path,
                idempotency_key: None,
            },
        },
    )
    .unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    work::execute(
        &p,
        &WorkCommand::Task {
            command: pctx::work::TaskCommand::Ready {
                task: task.clone(),
                expect_revision: None,
            },
        },
    )
    .unwrap();
    let denied = work::execute(
        &p,
        &WorkCommand::Task {
            command: pctx::work::TaskCommand::Claim {
                task: task.clone(),
                agent: "worker".into(),
                workspace: "current".into(),
                expect_revision: None,
            },
        },
    )
    .unwrap_err();
    assert_eq!(denied.code, "ROLE_PAUSED");
    let shown = work::execute(
        &p,
        &WorkCommand::Task {
            command: pctx::work::TaskCommand::Show { task },
        },
    )
    .unwrap();
    assert_eq!(shown["state"], "ready");
    assert!(shown["run"].is_null());
}

fn owner_queue(p: &Project, limit: usize) -> Value {
    operations::execute(
        p,
        &Op::Owner {
            command: operations::OwnerCommand::Queue { limit },
        },
    )
    .unwrap()
}
fn pause_delivery(p: &Project, role: &str, topic: Option<&str>, recipient: Option<&str>) {
    operations::execute(
        p,
        &Op::Role {
            command: RoleCommand::Pause {
                role: role.into(),
                reason: "Synthetic delivery barrier".into(),
                topic: topic.map(str::to_owned),
                recipient: recipient.map(str::to_owned),
            },
        },
    )
    .unwrap();
}
#[test]
fn owner_queue_filters_oldest_hidden_actions_before_limit_without_disclosing_details() {
    let (_temp, p) = fixture();
    let mut role_hidden = action();
    role_hidden["environment"] = json!("paused-private-detail");
    let hidden_pause = request(&p, role_hidden);
    let mut topic_hidden = action();
    topic_hidden["role"] = json!("auditor");
    topic_hidden["topic"] = json!("silent-private-topic");
    topic_hidden["environment"] = json!("silent-private-detail");
    let hidden_topic = request(&p, topic_hidden);
    let mut visible = action();
    visible["role"] = json!("public-worker");
    visible["topic"] = json!("public-topic");
    visible["environment"] = json!("public-detail");
    let public = request(&p, visible);
    let db = work::connect(&p).unwrap();
    for (id, created) in [(&hidden_pause, 1), (&hidden_topic, 2), (&public, 3)] {
        db.execute(
            "UPDATE ops_decisions SET created=? WHERE id=?",
            rusqlite::params![created, id],
        )
        .unwrap();
    }
    pause_delivery(&p, "developer", None, None);
    pause_delivery(&p, "auditor", Some("silent-private-topic"), None);
    let queue = owner_queue(&p, 1);
    assert_eq!(queue["decisions"].as_array().unwrap().len(), 1);
    assert_eq!(queue["decisions"][0]["decision_id"], public);
    let encoded = queue.to_string();
    for hidden in [
        hidden_pause.as_str(),
        hidden_topic.as_str(),
        "paused-private-detail",
        "silent-private-detail",
        "silent-private-topic",
    ] {
        assert!(!encoded.contains(hidden));
    }
    assert_eq!(queue["owner_prompt_sent"], false);
}
#[test]
fn owner_queue_honors_specific_recipients_and_owner_destination_without_globalizing_silence() {
    let (_temp, p) = fixture();
    let mut hidden_actor = action();
    hidden_actor["topic"] = json!("recipient-topic");
    let actor_hidden = request(&p, hidden_actor);
    let mut other_actor = action();
    other_actor["actor"] = json!("other-worker");
    other_actor["topic"] = json!("recipient-topic");
    let allowed_other = request(&p, other_actor);
    let mut hidden_owner = action();
    hidden_owner["topic"] = json!("owner-topic");
    hidden_owner["actor"] = json!("different-worker");
    let owner_hidden = request(&p, hidden_owner);
    pause_delivery(&p, "developer", Some("recipient-topic"), Some("worker"));
    pause_delivery(&p, "developer", Some("owner-topic"), Some("owner"));
    let queue = owner_queue(&p, 10);
    let items = queue["decisions"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["decision_id"], allowed_other);
    assert!(!queue.to_string().contains(&actor_hidden));
    assert!(!queue.to_string().contains(&owner_hidden));
}
#[test]
fn omitted_action_role_cannot_bypass_registered_actor_pause() {
    let (_temp, p) = fixture();
    attach(&p, "worker");
    let mut no_role = action();
    no_role.as_object_mut().unwrap().remove("role");
    let hidden = request(&p, no_role);
    pause_delivery(&p, "developer", None, None);
    let queue = owner_queue(&p, 1);
    assert!(queue["decisions"].as_array().unwrap().is_empty());
    assert!(!queue.to_string().contains(&hidden));
}
#[test]
fn owner_queue_delivery_barriers_survive_control_restore() {
    let (_temp, p) = fixture();
    let paused = request(&p, action());
    let mut silent = action();
    silent["role"] = json!("auditor");
    silent["topic"] = json!("restored-private-topic");
    let silence = request(&p, silent);
    let mut allowed = action();
    allowed["role"] = json!("public-worker");
    let visible = request(&p, allowed);
    pause_delivery(&p, "developer", None, None);
    pause_delivery(&p, "auditor", Some("restored-private-topic"), None);
    let archive = p.data_dir.join("queue-backup.json");
    work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Backup {
                output: archive.clone(),
            },
        },
    )
    .unwrap();
    let response = work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Restore { input: archive },
        },
    )
    .unwrap();
    let mut restored = p.clone();
    restored.coordination_id = response["coordination_id"].as_str().unwrap().to_owned();
    restored.control_dir = p.data_dir.join("controls").join(&restored.coordination_id);
    let queue = owner_queue(&restored, 1);
    assert_eq!(queue["decisions"].as_array().unwrap().len(), 1);
    assert_eq!(queue["decisions"][0]["decision_id"], visible);
    assert!(!queue.to_string().contains(&paused));
    assert!(!queue.to_string().contains(&silence));
    assert!(!queue.to_string().contains("restored-private-topic"));
}

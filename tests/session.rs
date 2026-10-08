use pctx::{
    project::{Config, Project, ProjectConfig},
    session::{ContextCommand, SessionCommand, context, session},
    work::{self, AgentCommand, TaskCommand, WorkCommand},
};
use serde_json::{Value, json};
fn fixture() -> (tempfile::TempDir, Project, String, String) {
    let t = tempfile::tempdir().unwrap();
    let p = Project {
        root: t.path().join("project"),
        data_dir: t.path().join("data"),
        workspace_dir: t.path().join("data/ws"),
        control_dir: t.path().join("data/control"),
        project_id: "project".into(),
        workspace_id: "ws".into(),
        coordination_id: "coord".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "project".into(),
                name: "test".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    std::fs::create_dir_all(&p.root).unwrap();
    std::fs::create_dir_all(&p.workspace_dir).unwrap();
    std::fs::create_dir_all(&p.control_dir).unwrap();
    std::fs::create_dir_all(p.root.join(".pctx/rules")).unwrap();
    std::fs::write(
        p.root.join(".pctx/rules/security.md"),
        "Never deploy without explicit approval.\n",
    )
    .unwrap();
    std::fs::write(p.root.join("auth.py"), "def login(): pass\n").unwrap();
    let agent = work::execute(
        &p,
        &WorkCommand::Agent {
            command: AgentCommand::Register {
                name: "dev-a".into(),
                kind: "agent".into(),
                concurrency_limit: 1,
            },
        },
    )
    .unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let definition = json!({"schema_version":1,"title":"Fix authentication","scope":["auth.py"],"acceptance":[{"id":"AC1","description":"Authentication works","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]});
    let path = p.root.join("task.json");
    std::fs::write(&path, definition.to_string()).unwrap();
    let task = work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Create {
                from_file: path,
                idempotency_key: None,
            },
        },
    )
    .unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    (t, p, agent, task)
}
fn attach(p: &Project, agent: &str) -> String {
    session(
        p,
        &SessionCommand::Attach {
            agent: agent.into(),
            runtime: "manual".into(),
            workspace: "current".into(),
            native_session: Some("opaque-native-id".into()),
            role: Some("implementer".into()),
            account_pool: None,
            adapter_version: "fixture-v1".into(),
        },
    )
    .unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn get(
    p: &Project,
    task: &str,
    s: &str,
    mode: &str,
    since: Option<&str>,
) -> pctx::domain::Result<Value> {
    context(
        p,
        &ContextCommand::Get {
            task_id: task.into(),
            session: s.into(),
            mode: mode.into(),
            since: since.map(str::to_owned),
            scope: vec!["auth.py".into()],
            budget_bytes: 12000,
        },
    )
}
fn ack(p: &Project, s: &str, v: &Value) -> pctx::domain::Result<Value> {
    context(
        p,
        &ContextCommand::Ack {
            context: v["context_id"].as_str().unwrap().into(),
            session: s.into(),
            epoch: v["context_epoch"].as_i64().unwrap(),
            provenance: "explicit-agent".into(),
        },
    )
}
#[test]
fn explicit_ack_unchanged_delta_and_no_source_ledger() {
    let (_t, p, a, task) = fixture();
    let s = attach(&p, &a);
    let full = get(&p, &task, &s, "full", None).unwrap();
    let id = full["context_id"].as_str().unwrap();
    assert_eq!(
        get(&p, &task, &s, "delta", Some(id)).unwrap_err().code,
        "BASELINE_MISMATCH"
    );
    ack(&p, &s, &full).unwrap();
    let delta = get(&p, &task, &s, "delta", Some(id)).unwrap();
    assert_eq!(delta["context_id"], full["context_id"]);
    for key in ["added", "changed", "removed"] {
        assert!(delta[key].as_array().unwrap().is_empty());
    }
    assert!(delta["unchanged_count"].as_u64().unwrap() > 0);
    let db = p.connect(true).unwrap();
    let stored: String = db
        .query_row(
            "SELECT selection FROM pctx_context_emissions WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!stored.contains("Never deploy"));
    assert!(!stored.contains("def login"));
}
#[test]
fn independent_sessions_boundaries_and_policy_reject_old_baselines() {
    let (_t, mut p, a, task) = fixture();
    let s = attach(&p, &a);
    let full = get(&p, &task, &s, "full", None).unwrap();
    ack(&p, &s, &full).unwrap();
    let id = full["context_id"].as_str().unwrap();
    let other = attach(&p, &a);
    assert_eq!(
        get(&p, &task, &other, "delta", Some(id)).unwrap_err().code,
        "BASELINE_MISMATCH"
    );
    assert_eq!(
        ack(&p, &other, &full).unwrap_err().code,
        "BASELINE_MISMATCH"
    );
    p.config.policy.exclude.push("auth.py".into());
    assert_eq!(
        get(&p, &task, &s, "delta", Some(id)).unwrap_err().code,
        "BASELINE_MISMATCH"
    );
    p.config.policy.exclude.clear();
    session(
        &p,
        &SessionCommand::Boundary {
            session: s.clone(),
            reason: "compact".into(),
        },
    )
    .unwrap();
    assert_eq!(
        get(&p, &task, &s, "delta", Some(id)).unwrap_err().code,
        "BASELINE_MISMATCH"
    );
}
#[test]
fn rule_changes_and_deleted_paths_are_explicit() {
    let (_t, p, a, task) = fixture();
    let s = attach(&p, &a);
    let full = get(&p, &task, &s, "full", None).unwrap();
    ack(&p, &s, &full).unwrap();
    std::fs::write(
        p.root.join(".pctx/rules/security.md"),
        "New mandatory rule.\n",
    )
    .unwrap();
    std::fs::remove_file(p.root.join("auth.py")).unwrap();
    let delta = get(&p, &task, &s, "delta", full["context_id"].as_str()).unwrap();
    assert!(
        delta["changed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["text"] == "New mandatory rule.\n")
    );
    assert!(
        delta["removed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["previous"]["path"] == "auth.py" && v["tombstone"] == true)
    );
}
#[test]
fn suspend_reconcile_preserves_task_pause_and_never_revives_run() {
    let (_t, p, a, task) = fixture();
    work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Assign {
                task: task.clone(),
                agent: a.clone(),
                expect_revision: None,
            },
        },
    )
    .unwrap();
    work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Ready {
                task: task.clone(),
                expect_revision: None,
            },
        },
    )
    .unwrap();
    work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Start {
                task: task.clone(),
                workspace: "current".into(),
                agent: Some(a.clone()),
                expect_revision: None,
            },
        },
    )
    .unwrap();
    work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Pause {
                task: task.clone(),
                reason: "owner hold".into(),
            },
        },
    )
    .unwrap();
    let s = attach(&p, &a);
    let cap = session(
        &p,
        &SessionCommand::Suspend {
            session: s.clone(),
            reason: "quota".into(),
        },
    )
    .unwrap();
    assert!(cap["capsule_id"].as_str().is_some());
    session(
        &p,
        &SessionCommand::Boundary {
            session: s.clone(),
            reason: "model change".into(),
        },
    )
    .unwrap();
    let plan = session(&p, &SessionCommand::Reconcile { session: s }).unwrap();
    assert_eq!(plan["status"], "suspended");
    assert_eq!(plan["resume_allowed"], false);
    assert_eq!(plan["lease_restored"], false);
    let view = work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Show { task },
        },
    )
    .unwrap();
    assert_eq!(view["state"], "paused");
    assert!(view["run"]["run_id"].as_str().is_some());
}
#[test]
fn minimum_budget_failure_creates_no_emission() {
    let (_t, p, a, task) = fixture();
    let s = attach(&p, &a);
    let c = ContextCommand::Get {
        task_id: task,
        session: s,
        mode: "full".into(),
        since: None,
        scope: vec![],
        budget_bytes: 512,
    };
    assert_eq!(context(&p, &c).unwrap_err().code, "BUDGET_TOO_SMALL");
    let db = p.connect(true).unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM pctx_context_emissions", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn identity_secrets_are_rejected_before_persistence() {
    let (_t, p, a, _task) = fixture();
    let secret = format!("sk-{}", "syntheticcredentialvalue");
    let command = SessionCommand::Attach {
        agent: a,
        runtime: "manual".into(),
        workspace: "current".into(),
        native_session: Some(secret),
        role: None,
        account_pool: None,
        adapter_version: "v1".into(),
    };
    assert_eq!(session(&p, &command).unwrap_err().code, "INVALID_ARGUMENT");
    let db = p.connect(true).unwrap();
    let n: i64 = db
        .query_row("SELECT count(*) FROM pctx_sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

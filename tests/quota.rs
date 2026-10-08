use pctx::{
    project::{Config, Project, ProjectConfig},
    quota::{Observation, QuotaCommand, UsageBatch, execute},
    session::{self, SessionCommand},
    work::{self, AgentCommand, TaskCommand, WorkCommand},
};
use serde_json::{Value, json};
fn fixture() -> (tempfile::TempDir, Project, String, String) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("project")).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: pctx::project::RootAnchor::capture(&temp.path().join("project")).unwrap(),
        root: temp.path().join("project"),
        data_dir: temp.path().join("data"),
        workspace_dir: temp.path().join("data/ws"),
        control_dir: temp.path().join("data/control"),
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
    std::fs::create_dir_all(&p.control_dir).unwrap();
    std::fs::create_dir_all(&p.workspace_dir).unwrap();
    let path = p.root.join("task.json");
    std::fs::write(&path,json!({"schema_version":1,"title":"Quota fixture","scope":["src/**"],"acceptance":[{"id":"AC1","description":"Finish","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
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
    let a = work::execute(
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
    let session = session::session(
        &p,
        &SessionCommand::Attach {
            agent: a,
            runtime: "manual".into(),
            workspace: "current".into(),
            native_session: None,
            role: Some("implementer".into()),
            account_pool: Some("main".into()),
            adapter_version: "test-v1".into(),
        },
    )
    .unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    (temp, p, task, session)
}
fn time(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0).unwrap().to_rfc3339()
}
fn observation(task: &str, session: &str, id: &str, value: f64) -> Observation {
    let n = pctx::domain::now();
    Observation {
        observation_id: id.into(),
        pool_id: "main".into(),
        provider: "fixture-provider".into(),
        model: "fixture-model".into(),
        metric: "input_tokens".into(),
        unit: "tokens".into(),
        source: "statusline".into(),
        collector: "fixture-v1".into(),
        source_revision: "native-v1".into(),
        observed_at: time(n - 60),
        window_id: "five-hour".into(),
        window_start: time(n - 3600),
        window_end: time(n + 3600),
        reset_at: Some(time(n + 3600)),
        timezone: Some("Europe/Berlin".into()),
        status: "actual".into(),
        amount: Some(value),
        kind: "cumulative".into(),
        request_id: None,
        session_id: Some(session.into()),
        context_epoch: Some(1),
        counter_epoch: Some("counter-1".into()),
        task_id: Some(task.into()),
        role: Some("implementer".into()),
        pricing_table_version: None,
        counter_origin_zero: true,
        workload: "execution".into(),
    }
}
fn quota(id: &str, value: Option<f64>, status: &str) -> Observation {
    let mut o = observation("unused", "unused", id, value.unwrap_or(0.0));
    o.metric = "subscription_quota".into();
    o.unit = "percentage".into();
    o.source = "provider_quota".into();
    o.kind = "quota".into();
    o.session_id = None;
    o.context_epoch = None;
    o.counter_epoch = None;
    o.task_id = None;
    o.role = None;
    o.counter_origin_zero = false;
    o.amount = value;
    o.status = status.into();
    o
}
fn ingest(p: &Project, key: &str, observations: Vec<Observation>) -> pctx::domain::Result<Value> {
    let path = p.root.join(format!("{key}.json"));
    std::fs::write(
        &path,
        serde_json::to_vec(&UsageBatch {
            schema_version: 1,
            observations,
        })
        .unwrap(),
    )
    .unwrap();
    execute(
        p,
        &QuotaCommand::Ingest {
            from_file: path,
            idempotency_key: key.into(),
        },
    )
}
fn report(p: &Project, pool: &str) -> Value {
    execute(
        p,
        &QuotaCommand::Report {
            pool: Some(pool.into()),
            task_id: None,
            session: None,
            group_by: "pool".into(),
            window: "7d".into(),
            include_coordination: true,
            max_age_seconds: 900,
        },
    )
    .unwrap()
}
#[test]
fn cumulative_counters_are_deltas_and_collector_sources_do_not_double_count() {
    let (_t, p, task, s) = fixture();
    let first = observation(&task, &s, "obs-first", 100.0);
    ingest(&p, "batch-first", vec![first.clone()]).unwrap();
    let mut second = observation(&task, &s, "obs-second", 150.0);
    second.observed_at = time(pctx::domain::now() - 30);
    let mut otel = second.clone();
    otel.source = "otel".into();
    otel.observation_id = "obs-otel".into();
    ingest(&p, "batch-second", vec![second.clone(), otel]).unwrap();
    let r = report(&p, "main");
    assert_eq!(r["groups"][0]["actual_amount"], 150.0);
    assert_eq!(r["ignored_duplicate_source_observations"], 1);
    assert_eq!(r["subscription_token_conversion"], false);
    let same = ingest(&p, "batch-first", vec![first]).unwrap();
    assert_eq!(same["inserted"], 1);
    let db = p.connect(true).unwrap();
    let n: i64 = db
        .query_row("SELECT count(*) FROM quota_observations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 3);
    second.amount = Some(151.0);
    assert_eq!(
        ingest(&p, "batch-second", vec![second]).unwrap_err().code,
        "IDEMPOTENCY_CONFLICT"
    );
}
#[test]
fn first_unknown_baseline_and_missing_provider_do_not_become_zero_or_normal() {
    let (_t, p, task, s) = fixture();
    let empty = report(&p, "main");
    assert_eq!(empty["collector_status"], "unknown");
    assert_eq!(empty["actual_usage_available"], false);
    assert_eq!(empty["quota"][0]["state"], "unknown");
    let mut o = observation(&task, &s, "baseline", 100.0);
    o.counter_origin_zero = false;
    ingest(&p, "baseline-batch", vec![o]).unwrap();
    let r = report(&p, "main");
    assert!(r["groups"][0]["actual_amount"].is_null());
    assert_eq!(r["groups"][0]["status"], "unknown_or_partial");
    ingest(
        &p,
        "missing-collector",
        vec![quota("quota-missing", None, "unknown")],
    )
    .unwrap();
    assert_eq!(report(&p, "main")["quota"][0]["state"], "unknown");
}
#[test]
fn epochs_resets_and_stale_counters_are_not_inherited() {
    let (_t, p, task, s) = fixture();
    ingest(
        &p,
        "initial",
        vec![observation(&task, &s, "initial-obs", 100.0)],
    )
    .unwrap();
    let mut old = observation(&task, &s, "regression", 90.0);
    old.observed_at = time(pctx::domain::now() - 20);
    assert_eq!(
        ingest(&p, "regression", vec![old]).unwrap_err().code,
        "USAGE_OBSERVATION_STALE"
    );
    session::session(
        &p,
        &SessionCommand::Boundary {
            session: s.clone(),
            reason: "compact".into(),
        },
    )
    .unwrap();
    assert_eq!(
        ingest(
            &p,
            "old-epoch",
            vec![observation(&task, &s, "old-epoch-obs", 150.0)]
        )
        .unwrap_err()
        .code,
        "BASELINE_MISMATCH"
    );
    let mut new = observation(&task, &s, "new-epoch-obs", 30.0);
    new.context_epoch = Some(2);
    new.counter_epoch = Some("counter-2".into());
    new.observed_at = time(pctx::domain::now() - 10);
    ingest(&p, "new-epoch", vec![new]).unwrap();
    assert_eq!(report(&p, "main")["groups"][0]["actual_amount"], 130.0);
    let db = p.connect(true).unwrap();
    let n: i64 = db
        .query_row("SELECT count(*) FROM quota_observations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
}
#[test]
fn quota_thresholds_reset_reobservation_and_persistent_pause_silence() {
    let (_t, p, task, _s) = fixture();
    let n = pctx::domain::now();
    let mut expired = quota("expired", Some(95.0), "actual");
    expired.observed_at = time(n - 300);
    expired.window_end = time(n - 100);
    expired.reset_at = Some(time(n - 100));
    ingest(&p, "expired", vec![expired]).unwrap();
    assert_eq!(report(&p, "main")["quota"][0]["state"], "unknown");
    let db = p.connect(true).unwrap();
    db.execute_batch("CREATE TABLE ops_roles(role TEXT PRIMARY KEY,paused INTEGER NOT NULL,reason TEXT NOT NULL,actor TEXT NOT NULL,updated INTEGER NOT NULL);CREATE TABLE ops_silences(role TEXT NOT NULL,recipient TEXT NOT NULL,topic TEXT NOT NULL,active INTEGER NOT NULL,reason TEXT NOT NULL,actor TEXT NOT NULL,updated INTEGER NOT NULL,PRIMARY KEY(role,recipient,topic));INSERT INTO ops_roles VALUES('implementer',1,'owner hold','owner',1);INSERT INTO ops_silences VALUES('implementer','*','deployment',1,'owner hold','owner',1);").unwrap();
    ingest(
        &p,
        "fresh-quota",
        vec![quota("fresh", Some(10.0), "actual")],
    )
    .unwrap();
    let planned = execute(
        &p,
        &QuotaCommand::Plan {
            pool: "main".into(),
            task_id: Some(task),
            max_age_seconds: 900,
        },
    )
    .unwrap();
    assert_eq!(planned["quota"]["state"], "normal");
    assert_eq!(planned["new_work_allowed"], false);
    assert_eq!(planned["paid_calls"], 0);
    let (paused, active): (i64, i64) = db
        .query_row(
            "SELECT (SELECT paused FROM ops_roles),(SELECT active FROM ops_silences)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((paused, active), (1, 1));
    let mut drain = quota("drain", Some(91.0), "actual");
    drain.observed_at = time(n - 20);
    ingest(&p, "drain", vec![drain]).unwrap();
    assert_eq!(report(&p, "main")["quota"][0]["state"], "drain");
}
#[test]
fn reservations_share_pool_same_unit_and_are_soft_idempotent() {
    let (_t, p, task, _s) = fixture();
    ingest(&p, "quota", vec![quota("available", Some(20.0), "actual")]).unwrap();
    let command = QuotaCommand::Reserve {
        task_id: task.clone(),
        pool: "main".into(),
        unit: "tokens".into(),
        amount: 60.0,
        limit: 100.0,
        policy: "coding-small".into(),
        idempotency_key: "reserve-once".into(),
    };
    let first = execute(&p, &command).unwrap();
    let again = execute(&p, &command).unwrap();
    assert_eq!(first["reservation_id"], again["reservation_id"]);
    assert_eq!(first["provider_guaranteed"], false);
    let exhausted = QuotaCommand::Reserve {
        task_id: task,
        pool: "main".into(),
        unit: "tokens".into(),
        amount: 50.0,
        limit: 100.0,
        policy: "coding-small".into(),
        idempotency_key: "reserve-other".into(),
    };
    assert_eq!(
        execute(&p, &exhausted).unwrap_err().code,
        "BUDGET_EXHAUSTED"
    );
    execute(
        &p,
        &QuotaCommand::Release {
            reservation: first["reservation_id"].as_str().unwrap().into(),
        },
    )
    .unwrap();
    execute(&p, &exhausted).unwrap();
    let db = p.connect(true).unwrap();
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM quota_reservations WHERE status='active'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
#[test]
fn pools_are_isolated_and_manual_estimates_do_not_change_actual_usage() {
    let (_t, p, task, s) = fixture();
    let mut a = observation(&task, &s, "actual", 20.0);
    a.kind = "request".into();
    a.request_id = Some("provider-req-1".into());
    a.counter_epoch = None;
    a.source = "provider_request".into();
    let mut manual = a.clone();
    manual.observation_id = "manual".into();
    manual.source = "manual".into();
    manual.status = "estimate".into();
    manual.amount = Some(100.0);
    manual.request_id = Some("manual-req".into());
    ingest(&p, "usage", vec![a, manual]).unwrap();
    let mut other = quota("other-quota", Some(92.0), "actual");
    other.pool_id = "other".into();
    ingest(&p, "other", vec![other]).unwrap();
    assert_eq!(report(&p, "main")["groups"][0]["actual_amount"], 20.0);
    assert_eq!(
        report(&p, "main")["groups"][0]["manual_reported_amount"],
        100.0
    );
    assert_eq!(report(&p, "other")["quota"][0]["state"], "drain");
    assert_eq!(report(&p, "main")["quota"][0]["state"], "unknown");
}

#[test]
fn restored_quota_requires_fresh_observation_and_event_ledgers_stay_immutable() {
    let (_temp, p, _task, _session) = fixture();
    ingest(
        &p,
        "before-backup",
        vec![quota("before", Some(20.0), "actual")],
    )
    .unwrap();
    assert_eq!(report(&p, "main")["quota"][0]["state"], "normal");
    let archive = p.data_dir.join("backup.json");
    work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Backup {
                output: archive.clone(),
            },
        },
    )
    .unwrap();
    let result = work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Restore { input: archive },
        },
    )
    .unwrap();
    let mut restored = p.clone();
    restored.coordination_id = result["coordination_id"].as_str().unwrap().into();
    restored.control_dir = p.data_dir.join("controls").join(&restored.coordination_id);
    let status = report(&restored, "main");
    assert_eq!(status["quota"][0]["state"], "unknown");
    assert_eq!(status["quota"][0]["paid_work_allowed"], false);
    let db = restored.connect(true).unwrap();
    let original: String = db
        .query_row(
            "SELECT payload FROM quota_observations WHERE observation_id='before'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&original).unwrap()["amount"],
        20.0
    );
    for table in ["quota_events", "pctx_session_events"] {
        assert!(
            db.execute(&format!("UPDATE {table} SET kind='forged'"), [])
                .is_err()
        );
        assert!(db.execute(&format!("DELETE FROM {table}"), []).is_err());
    }
    let mut fresh = quota("after", Some(25.0), "actual");
    fresh.observed_at = time(pctx::domain::now());
    ingest(&restored, "after-restore", vec![fresh]).unwrap();
    assert_eq!(report(&restored, "main")["quota"][0]["state"], "normal");
}

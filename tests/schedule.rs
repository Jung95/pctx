use pctx::{
    operations::{self, OperationCommand, RoleCommand},
    project::{Config, Project, ProjectConfig},
    schedule::{self, Cadence, ScheduleCommand, ScheduleDefinition},
};
use serde_json::Value;
fn fixture() -> (tempfile::TempDir, Project) {
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
    std::fs::create_dir_all(&p.control_dir).unwrap();
    std::fs::create_dir_all(&p.workspace_dir).unwrap();
    (t, p)
}
fn definition(namespace: &str, at: &str, start: &str) -> ScheduleDefinition {
    ScheduleDefinition {
        schema_version: 1,
        namespace: namespace.into(),
        id: "owner-digest".into(),
        timezone: "Europe/Berlin".into(),
        cadence: Cadence::Daily { at: at.into() },
        valid_from: start.into(),
        job: "read_query".into(),
        bridge: "manual".into(),
        role: "assistant".into(),
        recipient: "owner".into(),
        topic: "digest".into(),
        session_id: None,
        context_epoch: None,
        action: None,
        decision_id: None,
        enabled: true,
        misfire: "coalesce_latest".into(),
    }
}
fn add(p: &Project, d: &ScheduleDefinition, key: &str) -> Value {
    let path = p.root.join(format!("{key}.json"));
    std::fs::write(&path, serde_json::to_vec(d).unwrap()).unwrap();
    schedule::execute(
        p,
        &ScheduleCommand::Add {
            from_file: path,
            idempotency_key: key.into(),
        },
    )
    .unwrap()
}
fn reconcile(p: &Project, namespace: Option<&str>, at: &str) -> Value {
    schedule::execute(
        p,
        &ScheduleCommand::Reconcile {
            namespace: namespace.map(str::to_owned),
            at: Some(at.into()),
        },
    )
    .unwrap()
}
fn timestamp(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp()
}
#[test]
fn berlin_fall_back_occurs_once_per_local_date_and_restart_reuses_record() {
    let (_t, p) = fixture();
    add(
        &p,
        &definition("owner-a", "02:30", "2024-10-26T00:00:00Z"),
        "fall",
    );
    let first = reconcile(&p, Some("owner-a"), "2024-10-27T00:35:00Z");
    let second = reconcile(&p, Some("owner-a"), "2024-10-27T01:35:00Z");
    assert_eq!(first["created_occurrences"], 1);
    assert_eq!(second["created_occurrences"], 0);
    assert_eq!(
        first["schedules"][0]["occurrence"]["occurrence"],
        "local-date:2024-10-27"
    );
    assert_eq!(
        first["schedules"][0]["occurrence"]["due_at"],
        timestamp("2024-10-27T00:30:00Z")
    );
    assert_eq!(
        second["schedules"][0]["next_due"],
        timestamp("2024-10-28T01:30:00Z")
    );
    let db = p.connect(true).unwrap();
    let n: i64 = db
        .query_row("SELECT count(*) FROM schedule_occurrences", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 1);
    assert!(second["schedules"][0]["last_success"].is_null());
    assert_eq!(second["execution_started"], false);
}
#[test]
fn berlin_spring_gap_moves_to_next_valid_wall_time() {
    let (_t, p) = fixture();
    add(
        &p,
        &definition("owner-a", "02:30", "2024-03-30T00:00:00Z"),
        "spring",
    );
    let r = reconcile(&p, Some("owner-a"), "2024-03-31T01:05:00Z");
    assert_eq!(
        r["schedules"][0]["occurrence"]["due_at"],
        timestamp("2024-03-31T01:00:00Z")
    );
    assert_eq!(
        r["schedules"][0]["occurrence"]["occurrence"],
        "local-date:2024-03-31"
    );
    assert_eq!(
        r["schedules"][0]["next_due"],
        timestamp("2024-04-01T00:30:00Z")
    );
}
#[test]
fn elapsed_intervals_do_not_change_with_dst_and_missed_ticks_coalesce() {
    let (_t, p) = fixture();
    let mut d = definition("interval-owner", "02:30", "2024-03-30T22:00:00Z");
    d.cadence = Cadence::Interval {
        seconds: 7200,
        anchor: "2024-03-30T22:00:00Z".into(),
    };
    add(&p, &d, "interval");
    let a = reconcile(&p, Some("interval-owner"), "2024-03-31T00:15:00Z");
    let b = reconcile(&p, Some("interval-owner"), "2024-03-31T02:15:00Z");
    let aa = a["schedules"][0]["occurrence"]["due_at"].as_i64().unwrap();
    let bb = b["schedules"][0]["occurrence"]["due_at"].as_i64().unwrap();
    assert_eq!(bb - aa, 7200);
    let missed = reconcile(&p, Some("interval-owner"), "2024-04-15T10:15:00Z");
    assert_eq!(missed["created_occurrences"], 1);
    assert_eq!(
        missed["schedules"][0]["occurrence"]["actual_success"],
        false
    );
    assert_eq!(
        missed["schedules"][0]["occurrence"]["coalesced_older_occurrences"],
        true
    );
    let db = p.connect(true).unwrap();
    let n: i64 = db
        .query_row("SELECT count(*) FROM schedule_occurrences", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 3);
    let success: Option<i64> = db
        .query_row("SELECT last_success FROM schedule_definitions", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(success.is_none());
}
#[test]
fn namespaces_and_cas_pause_updates_are_independent() {
    let (_t, p) = fixture();
    let d = definition("owner-a", "09:00", "2024-01-01T00:00:00Z");
    add(&p, &d, "owner-a");
    add(
        &p,
        &definition("owner-b", "09:00", "2024-01-01T00:00:00Z"),
        "owner-b",
    );
    let paused = schedule::execute(
        &p,
        &ScheduleCommand::Pause {
            namespace: "owner-a".into(),
            id: d.id.clone(),
            reason: "owner hold".into(),
            expect_revision: 1,
        },
    )
    .unwrap();
    assert_eq!(paused["definition_revision"], 2);
    let stale = schedule::execute(
        &p,
        &ScheduleCommand::Resume {
            namespace: "owner-a".into(),
            id: d.id.clone(),
            reason: "explicit resume".into(),
            expect_revision: 1,
        },
    );
    assert_eq!(stale.unwrap_err().code, "REVISION_CONFLICT");
    let path = p.root.join("updated.json");
    std::fs::write(&path, serde_json::to_vec(&d).unwrap()).unwrap();
    let updated = schedule::execute(
        &p,
        &ScheduleCommand::Update {
            namespace: "owner-a".into(),
            id: d.id,
            from_file: path,
            expect_revision: 2,
        },
    )
    .unwrap();
    assert_eq!(updated["enabled"], false);
    assert_eq!(updated["pause_source"], "explicit_owner_pause");
    let r = reconcile(&p, None, "2024-02-01T12:00:00Z");
    assert_eq!(r["schedules"][0]["enabled"], false);
    assert_eq!(r["schedules"][1]["enabled"], true);
    let db = p.connect(true).unwrap();
    let n: i64 = db
        .query_row("SELECT count(*) FROM schedule_definitions", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 2);
}
#[test]
fn role_pause_topic_silence_do_not_reactivate() {
    let (_t, p) = fixture();
    let d = definition("owner-a", "09:00", "2024-01-01T00:00:00Z");
    add(&p, &d, "paused");
    operations::execute(
        &p,
        &OperationCommand::Role {
            command: RoleCommand::Pause {
                role: "assistant".into(),
                reason: "owner hold".into(),
                topic: None,
                recipient: None,
            },
        },
    )
    .unwrap();
    operations::execute(
        &p,
        &OperationCommand::Role {
            command: RoleCommand::Pause {
                role: "assistant".into(),
                reason: "topic hold".into(),
                topic: Some("digest".into()),
                recipient: Some("owner".into()),
            },
        },
    )
    .unwrap();
    let r = reconcile(&p, Some("owner-a"), "2024-02-01T12:00:00Z");
    assert_eq!(r["schedules"][0]["occurrence"]["state"], "blocked");
    assert!(
        r["schedules"][0]["role_topic_barriers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "role_paused")
    );
    let again = reconcile(&p, Some("owner-a"), "2024-02-01T12:01:00Z");
    assert_eq!(again["created_occurrences"], 0);
    let db = p.connect(true).unwrap();
    let flags: (i64, i64) = db
        .query_row(
            "SELECT (SELECT paused FROM ops_roles),(SELECT active FROM ops_silences)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(flags, (1, 1));
    assert_eq!(again["model_calls"], 0);
    assert_eq!(again["external_messages"], 0);
}
#[test]
fn reconcile_singleflight_and_managed_bridge_never_claim_install_or_success() {
    let (_t, p) = fixture();
    let mut d = definition("owner-a", "09:00", "2024-01-01T00:00:00Z");
    d.bridge = "managed".into();
    add(&p, &d, "managed");
    let mut joins = Vec::new();
    for _ in 0..4 {
        let p = p.clone();
        joins.push(std::thread::spawn(move || {
            reconcile(&p, Some("owner-a"), "2024-02-01T12:00:00Z")
        }));
    }
    let results: Vec<_> = joins.into_iter().map(|j| j.join().unwrap()).collect();
    let total: u64 = results
        .iter()
        .map(|r| r["created_occurrences"].as_u64().unwrap())
        .sum();
    assert_eq!(total, 1);
    for r in results {
        assert_eq!(r["bridge_registration_performed"], false);
        assert_eq!(r["schedules"][0]["bridge_actual_state"], "unknown");
        assert_eq!(r["execution_started"], false);
    }
    let db = p.connect(true).unwrap();
    let n: i64 = db
        .query_row("SELECT count(*) FROM schedule_occurrences", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 1);
}
#[test]
fn non_read_job_requires_matching_operation_decision() {
    let (_t, p) = fixture();
    let mut d = definition("owner-a", "09:00", "2024-01-01T00:00:00Z");
    d.job = "backup".into();
    let path = p.root.join("backup.json");
    std::fs::write(&path, serde_json::to_vec(&d).unwrap()).unwrap();
    assert_eq!(
        schedule::execute(
            &p,
            &ScheduleCommand::Add {
                from_file: path,
                idempotency_key: "backup".into()
            }
        )
        .unwrap_err()
        .code,
        "OWNER_DECISION_REQUIRED"
    );
}
#[test]
fn session_boundary_invalidates_binding_without_reinstall_or_epoch_inheritance() {
    use pctx::{
        session::{self, SessionCommand},
        work::{self, AgentCommand, WorkCommand},
    };
    let (_t, p) = fixture();
    let a = work::execute(
        &p,
        &WorkCommand::Agent {
            command: AgentCommand::Register {
                name: "assistant-a".into(),
                kind: "agent".into(),
                concurrency_limit: 1,
            },
        },
    )
    .unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let sid = session::session(
        &p,
        &SessionCommand::Attach {
            agent: a,
            runtime: "manual".into(),
            workspace: "current".into(),
            native_session: None,
            role: Some("assistant".into()),
            account_pool: None,
            adapter_version: "test-v1".into(),
        },
    )
    .unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut d = definition("owner-a", "09:00", "2024-01-01T00:00:00Z");
    d.session_id = Some(sid.clone());
    d.context_epoch = Some(1);
    add(&p, &d, "session-bound");
    let first = reconcile(&p, Some("owner-a"), "2024-02-01T12:00:00Z");
    assert_eq!(first["schedules"][0]["occurrence"]["state"], "planned");
    session::session(
        &p,
        &SessionCommand::Boundary {
            session: sid.clone(),
            reason: "restore invalidates context".into(),
        },
    )
    .unwrap();
    let db = p.connect(true).unwrap();
    db.execute(
        "UPDATE pctx_sessions SET status='suspended' WHERE id=?1",
        [&sid],
    )
    .unwrap();
    let second = reconcile(&p, Some("owner-a"), "2024-02-01T12:01:00Z");
    assert_eq!(second["created_occurrences"], 0);
    assert_eq!(second["schedules"][0]["occurrence"]["state"], "blocked");
    assert!(
        second["schedules"][0]["role_topic_barriers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "session_binding_stale_or_suspended")
    );
    assert_eq!(second["bridge_registration_performed"], false);
    let epoch: i64 = db
        .query_row("SELECT epoch FROM pctx_sessions WHERE id=?1", [sid], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(epoch, 2);
}

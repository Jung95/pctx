use pctx::{
    operations::{self, OperationCommand, RoleCommand},
    project::{Config, Project, ProjectConfig},
    schedule::{self, Cadence, ScheduleCommand, ScheduleDefinition},
};
use serde_json::Value;
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(t.path().join("project")).unwrap();
    let p = Project {
        root_anchor: pctx::project::RootAnchor::capture(&t.path().join("project")).unwrap(),
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

fn tick(p: &Project, at: &str, retry_failed: bool) -> Value {
    schedule::execute(
        p,
        &ScheduleCommand::Tick {
            namespace: None,
            at: Some(at.into()),
            retry_failed,
        },
    )
    .unwrap()
}
fn fixture_install(p: &Project, d: &ScheduleDefinition) -> Value {
    let plan = schedule::execute(
        p,
        &ScheduleCommand::Plan {
            namespace: d.namespace.clone(),
            id: d.id.clone(),
            provider: "fixture".into(),
            staging_root: ".pctx/test-bridge".into(),
        },
    )
    .unwrap();
    let path = p.root.join("reviewed-plan.json");
    std::fs::write(&path, plan.to_string()).unwrap();
    schedule::execute(
        p,
        &ScheduleCommand::Install {
            from_file: path,
            expect_hash: plan["plan_hash"].as_str().unwrap().into(),
            apply_native: false,
        },
    )
    .unwrap();
    plan
}
#[test]
fn tick_executes_once_and_only_changed_results_queue_a_local_notice() {
    let (_t, p) = fixture();
    let d = definition("local", "09:00", "2024-01-01T00:00:00Z");
    add(&p, &d, "tick");
    let task = p.root.join("task.json");
    std::fs::write(&task,serde_json::json!({"schema_version":1,"title":"A real changed task","scope":["task.json"],"acceptance":[{"id":"done","description":"Reviewed","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
    pctx::work::execute(
        &p,
        &pctx::work::WorkCommand::Task {
            command: pctx::work::TaskCommand::Create {
                from_file: task,
                idempotency_key: None,
            },
        },
    )
    .unwrap();
    let first = tick(&p, "2024-02-01T12:00:00Z", false);
    assert_eq!(first["runs"][0]["state"], "succeeded");
    assert_eq!(first["runs"][0]["actual_success"], true);
    assert!(!first["runs"][0]["delivery"].is_null());
    assert_eq!(first["external_messages"], 0);
    let replay = tick(&p, "2024-02-01T12:01:00Z", false);
    assert_eq!(replay["runs"][0]["state"], "already_recorded");
    let next = tick(&p, "2024-02-02T12:00:00Z", false);
    assert_eq!(next["runs"][0]["state"], "succeeded");
    assert_eq!(next["runs"][0]["changed"], false);
    assert!(next["runs"][0]["delivery"].is_null());
    let db = p.connect(true).unwrap();
    let counts: (i64, i64) = db
        .query_row(
            "SELECT (SELECT count(*) FROM schedule_runs),(SELECT count(*) FROM ops_messages)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(counts, (2, 1));
    assert_eq!(
        reconcile(&p, None, "2024-02-02T12:02:00Z")["schedules"][0]["occurrence"]["state"],
        "succeeded"
    );
    let persisted: String = db
        .query_row(
            "SELECT state FROM schedule_occurrences ORDER BY due_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(persisted, "succeeded");
}
#[test]
fn managed_bridge_requires_current_reviewed_plan_and_preserves_user_changes() {
    let (_t, p) = fixture();
    let mut d = definition("managed", "09:00", "2024-01-01T00:00:00Z");
    d.bridge = "managed".into();
    add(&p, &d, "managed");
    assert_eq!(
        tick(&p, "2024-02-01T12:00:00Z", false)["runs"][0]["error"],
        "CAPABILITY_UNVERIFIED"
    );
    let plan = fixture_install(&p, &d);
    let inspect = schedule::execute(
        &p,
        &ScheduleCommand::Inspect {
            namespace: d.namespace.clone(),
            id: d.id.clone(),
            observe_native: false,
        },
    )
    .unwrap();
    assert_eq!(inspect["owned_files_match"], true);
    assert_eq!(inspect["stored_state"], "staged");
    assert_eq!(
        tick(&p, "2024-02-01T12:00:00Z", false)["runs"][0]["actual_success"],
        true
    );
    let filename = plan["files"].as_object().unwrap().keys().next().unwrap();
    let target = p.root.join(".pctx/test-bridge").join(filename);
    std::fs::write(&target, "user changed this file").unwrap();
    assert_eq!(
        tick(&p, "2024-02-02T12:00:00Z", false)["runs"][0]["error"],
        "CONFIG_CHANGED"
    );
    let error = schedule::execute(
        &p,
        &ScheduleCommand::Uninstall {
            namespace: d.namespace,
            id: d.id,
            expect_hash: plan["plan_hash"].as_str().unwrap().into(),
            apply_native: false,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "REVISION_CONFLICT");
    assert_eq!(
        std::fs::read_to_string(target).unwrap(),
        "user changed this file"
    );
}
#[test]
fn staged_bridge_uninstall_removes_only_owned_files_and_missing_manifest_can_be_repaired() {
    let (_t, p) = fixture();
    let d = definition("fixture", "09:00", "2024-01-01T00:00:00Z");
    add(&p, &d, "fixture");
    let plan = fixture_install(&p, &d);
    let root = p.root.join(".pctx/test-bridge");
    std::fs::write(root.join("unrelated.json"), "user-owned").unwrap();
    let own = plan["files"].as_object().unwrap().keys().next().unwrap();
    std::fs::remove_file(root.join(own)).unwrap();
    fixture_install(&p, &d);
    let v = schedule::execute(
        &p,
        &ScheduleCommand::Uninstall {
            namespace: d.namespace,
            id: d.id,
            expect_hash: plan["plan_hash"].as_str().unwrap().into(),
            apply_native: false,
        },
    )
    .unwrap();
    assert_eq!(v["removed"], true);
    assert!(!root.join(own).exists());
    assert_eq!(
        std::fs::read_to_string(root.join("unrelated.json")).unwrap(),
        "user-owned"
    );
}
#[test]
fn paused_role_blocks_actual_tick_and_loop_does_not_acquire_keep_awake() {
    let (_t, p) = fixture();
    let d = definition("paused", "09:00", "2024-01-01T00:00:00Z");
    add(&p, &d, "paused");
    operations::execute(
        &p,
        &OperationCommand::Role {
            command: RoleCommand::Pause {
                role: "assistant".into(),
                reason: "quiet".into(),
                topic: None,
                recipient: None,
            },
        },
    )
    .unwrap();
    assert_eq!(
        tick(&p, "2024-02-01T12:00:00Z", false)["runs"][0]["state"],
        "blocked"
    );
    let v = schedule::execute(
        &p,
        &ScheduleCommand::RunLoop {
            namespace: None,
            interval_seconds: 1,
            max_ticks: 1,
            ttl_seconds: 1,
            keep_awake: true,
            purpose: Some("isolated paused job".into()),
        },
    )
    .unwrap();
    assert!(v["ticks"].as_array().unwrap().is_empty());
    assert_eq!(v["model_calls"], 0);
    let db = p.connect(true).unwrap();
    let runs: i64 = db
        .query_row("SELECT count(*) FROM schedule_runs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(runs, 0);
}
#[cfg(unix)]
#[test]
fn project_private_bridge_staging_rejects_user_symlinks_without_changing_target() {
    use std::os::unix::fs::symlink;
    let (_t, p) = fixture();
    let d = definition("link", "09:00", "2024-01-01T00:00:00Z");
    add(&p, &d, "link");
    let external = p.data_dir.join("outside");
    std::fs::create_dir_all(&external).unwrap();
    std::fs::create_dir_all(p.root.join(".pctx")).unwrap();
    symlink(&external, p.root.join(".pctx/test-bridge")).unwrap();
    let err = schedule::execute(
        &p,
        &ScheduleCommand::Plan {
            namespace: d.namespace,
            id: d.id,
            provider: "fixture".into(),
            staging_root: ".pctx/test-bridge".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err.code, "POLICY_DENIED");
    assert_eq!(std::fs::read_dir(external).unwrap().count(), 0);
}
#[test]
fn running_owner_is_never_released_by_ttl_and_restored_binding_is_not_inherited() {
    let (_t, p) = fixture();
    let mut d = definition("unknown", "09:00", "2024-01-01T00:00:00Z");
    d.bridge = "managed".into();
    add(&p, &d, "unknown");
    fixture_install(&p, &d);
    let db = p.connect(true).unwrap();
    db.execute("INSERT INTO schedule_runs(namespace,schedule,revision,occurrence,attempt,state,result,started) VALUES('unknown','owner-digest',1,'local-date:2024-02-01',1,'running',?1,0)",[serde_json::json!({"owner_pid":std::process::id(),"workspace":p.workspace_id,"execution_kind":"in_process"}).to_string()]).unwrap();
    assert_eq!(
        tick(&p, "2024-02-02T12:00:00Z", true)["runs"][0]["state"],
        "execution_owner_unknown_or_busy"
    );
    let err = schedule::execute(
        &p,
        &ScheduleCommand::Recover {
            namespace: d.namespace.clone(),
            id: d.id.clone(),
            revision: 1,
            occurrence: "local-date:2024-02-01".into(),
            attempt: 1,
            reason: "must not release a live process".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err.code, "RESOURCE_OWNER_UNKNOWN");
    db.execute("DELETE FROM schedule_bindings", []).unwrap();
    db.execute("UPDATE schedule_installations SET state='unknown_restored',metadata=?1",[serde_json::json!({"provider":"fixture","plan_hash":"former-local-plan","requires_reapproval":true}).to_string()]).unwrap();
    let inspected = schedule::execute(
        &p,
        &ScheduleCommand::Inspect {
            namespace: d.namespace.clone(),
            id: d.id.clone(),
            observe_native: false,
        },
    )
    .unwrap();
    assert_eq!(inspected["stored_state"], "unknown_restored");
    assert_eq!(inspected["current_binding"], false);
    assert_eq!(
        tick(&p, "2024-02-03T12:00:00Z", true)["runs"][0]["error"],
        "CAPABILITY_UNVERIFIED"
    );
}
#[test]
fn actual_cli_concurrent_ticks_claim_one_occurrence_and_fixture_bridge_invokes_tick() {
    use std::{
        fs,
        process::Command,
        sync::{Arc, Barrier},
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let data = temp.path().join("data");
    fs::create_dir(&root).unwrap();
    let cli = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_pctx"))
            .args(["--root", root.to_str().unwrap(), "--format", "json"])
            .args(args)
            .env("PCTX_DATA_DIR", &data)
            .env("PCTX_ACTOR", "owner")
            .env_remove("PCTX_RUN_CAPABILITY")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "args={args:?} stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["data"].clone()
    };
    cli(&["init"]);
    let mut d = definition("cli", "09:00", "2024-01-01T00:00:00Z");
    d.bridge = "managed".into();
    let file = root.join("schedule.json");
    fs::write(&file, serde_json::to_vec(&d).unwrap()).unwrap();
    cli(&[
        "schedule",
        "add",
        "--from-file",
        file.to_str().unwrap(),
        "--idempotency-key",
        "cli",
    ]);
    let plan = cli(&[
        "schedule",
        "plan",
        "--namespace",
        "cli",
        "owner-digest",
        "--provider",
        "fixture",
        "--staging-root",
        ".pctx/test-bridge",
    ]);
    let review = root.join("reviewed.json");
    fs::write(&review, plan.to_string()).unwrap();
    cli(&[
        "schedule",
        "install",
        "--from-file",
        review.to_str().unwrap(),
        "--expect-hash",
        plan["plan_hash"].as_str().unwrap(),
    ]);
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let b = barrier.clone();
        let r = root.clone();
        let dat = data.clone();
        handles.push(std::thread::spawn(move || {
            b.wait();
            let out = Command::new(env!("CARGO_BIN_EXE_pctx"))
                .args([
                    "--root",
                    r.to_str().unwrap(),
                    "--format",
                    "json",
                    "schedule",
                    "tick",
                    "--namespace",
                    "cli",
                    "--at",
                    "2024-02-01T12:00:00Z",
                ])
                .env("PCTX_DATA_DIR", dat)
                .env("PCTX_ACTOR", "owner")
                .env_remove("PCTX_RUN_CAPABILITY")
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{} {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            serde_json::from_slice::<Value>(&out.stdout).unwrap()["data"].clone()
        }));
    }
    let receipts: Vec<Value> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(
        receipts
            .iter()
            .filter(|v| v["runs"][0]["actual_success"] == true)
            .count(),
        1
    );
    let manifest = plan["files"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .as_str()
        .unwrap();
    let manifest: Value = serde_json::from_str(manifest).unwrap();
    let out = Command::new(manifest["argv"][0].as_str().unwrap())
        .args(
            manifest["argv"]
                .as_array()
                .unwrap()
                .iter()
                .skip(1)
                .map(|v| v.as_str().unwrap()),
        )
        .current_dir(&root)
        .env("PCTX_DATA_DIR", &data)
        .env("PCTX_ACTOR", "owner")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "bridge stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["data"]["model_calls"],
        0
    );
}
#[test]
fn empty_readonly_tick_is_successful_without_notification_or_model_wake() {
    let (_t, p) = fixture();
    let d = definition("quiet", "09:00", "2024-01-01T00:00:00Z");
    add(&p, &d, "quiet");
    let result = tick(&p, "2024-02-01T12:00:00Z", false);
    assert_eq!(result["runs"][0]["actual_success"], true);
    assert!(result["runs"][0]["delivery"].is_null());
    assert_eq!(result["model_calls"], 0);
    assert_eq!(result["external_messages"], 0);
}
#[test]
fn failed_attempt_is_recorded_and_retry_is_explicit_instead_of_synthetic_success() {
    let (_t, p) = fixture();
    let d = definition("failure", "09:00", "2024-01-01T00:00:00Z");
    add(&p, &d, "failure");
    // An actual board read cannot parse this persisted task definition; corruption is not success.
    let task = p.root.join("task.json");
    std::fs::write(&task,serde_json::json!({"schema_version":1,"title":"Failure fixture","scope":["task.json"],"acceptance":[{"id":"done","description":"Reviewed","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
    let id = pctx::work::execute(
        &p,
        &pctx::work::WorkCommand::Task {
            command: pctx::work::TaskCommand::Create {
                from_file: task,
                idempotency_key: None,
            },
        },
    )
    .unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let db = p.connect(true).unwrap();
    let original: String = db
        .query_row("SELECT definition FROM tasks WHERE id=?1", [&id], |r| {
            r.get(0)
        })
        .unwrap();
    db.execute(
        "UPDATE tasks SET definition='broken json' WHERE id=?1",
        [&id],
    )
    .unwrap();
    let failed = tick(&p, "2024-02-01T12:00:00Z", false);
    assert_eq!(failed["runs"][0]["state"], "failed");
    assert_eq!(failed["runs"][0]["actual_success"], false);
    db.execute(
        "UPDATE tasks SET definition=?1 WHERE id=?2",
        rusqlite::params![original, id],
    )
    .unwrap();
    assert_eq!(
        tick(&p, "2024-02-01T12:01:00Z", false)["runs"][0]["state"],
        "already_recorded"
    );
    let retry = tick(&p, "2024-02-01T12:02:00Z", true);
    assert_eq!(retry["runs"][0]["attempt"], 2);
    assert_eq!(retry["runs"][0]["state"], "succeeded");
    let states: Vec<String> = {
        let mut s = db
            .prepare("SELECT state FROM schedule_runs ORDER BY attempt")
            .unwrap();
        s.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    assert_eq!(states, vec!["failed", "succeeded"]);
}

use pctx::{
    project::{Config, Project, ProjectConfig},
    work::{self, AgentCommand, CheckCommand, CriterionCommand, TaskCommand, WorkCommand},
};
use serde_json::{Value, json};
fn setup() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("code.rs"), "fn value() -> u32 { 1 }\n").unwrap();
    let data = dir.path().join("data");
    std::fs::create_dir_all(data.join("workspace")).unwrap();
    std::fs::create_dir_all(data.join("control")).unwrap();
    let p = Project {
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspace"),
        control_dir: data.join("control"),
        project_id: "test-project".into(),
        workspace_id: "test-workspace".into(),
        coordination_id: "test-control".into(),
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
    (dir, p)
}
fn task_file(p: &Project) -> std::path::PathBuf {
    let path = p.data_dir.join("task.json");
    std::fs::write(&path,json!({"schema_version":1,"title":"Change value","scope":["code.rs"],"acceptance":[{"id":"behavior","description":"Validated","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test","output_paths":["reports/**"]}]}).to_string()).unwrap();
    path
}
fn task(p: &Project, c: TaskCommand) -> Value {
    work::execute(p, &WorkCommand::Task { command: c }).unwrap()
}
fn register(p: &Project) -> String {
    work::execute(
        p,
        &WorkCommand::Agent {
            command: AgentCommand::Register {
                name: "worker".into(),
                kind: "agent".into(),
                concurrency_limit: 1,
            },
        },
    )
    .unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .into()
}
fn start(p: &Project) -> (String, i64) {
    let created = task(
        p,
        TaskCommand::Create {
            from_file: task_file(p),
            idempotency_key: None,
        },
    );
    let name = created["task_id"].as_str().unwrap().to_string();
    let a = register(p);
    task(
        p,
        TaskCommand::Ready {
            task: name.clone(),
            expect_revision: Some(1),
        },
    );
    task(
        p,
        TaskCommand::Assign {
            task: name.clone(),
            agent: a,
            expect_revision: Some(2),
        },
    );
    let run = task(
        p,
        TaskCommand::Start {
            task: name,
            workspace: "current".into(),
            agent: None,
            expect_revision: Some(3),
        },
    );
    (
        run["run_id"].as_str().unwrap().into(),
        run["lease_epoch"].as_i64().unwrap(),
    )
}
#[test]
fn claims_reports_reassignment_and_cursor() {
    let (_d, p) = setup();
    let (run, epoch) = start(&p);
    let before = work::board(&p).unwrap();
    let name = before["tasks"][0]["task_id"].as_str().unwrap().to_owned();
    let second = work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Start {
                task: name.clone(),
                workspace: "current".into(),
                agent: None,
                expect_revision: None,
            },
        },
    )
    .unwrap_err();
    assert_eq!(second.code, "TASK_ALREADY_CLAIMED");
    let command = WorkCommand::Agent {
        command: AgentCommand::Report {
            run: run.clone(),
            lease_epoch: epoch,
            report_seq: 1,
            idempotency_key: "progress-1".into(),
            stage: "implementing".into(),
            summary: "Working".into(),
            estimate_percent: Some(100),
        },
    };
    let first = work::execute(&p, &command).unwrap();
    assert_eq!(first, work::execute(&p, &command).unwrap());
    assert_eq!(work::board(&p).unwrap()["tasks"][0]["state"], "in_progress");
    let old = WorkCommand::Agent {
        command: AgentCommand::Report {
            run: run.clone(),
            lease_epoch: epoch,
            report_seq: 1,
            idempotency_key: "other".into(),
            stage: "planning".into(),
            summary: "Late".into(),
            estimate_percent: None,
        },
    };
    assert_eq!(
        work::execute(&p, &old).unwrap_err().code,
        "OUT_OF_ORDER_REPORT"
    );
    let actor = before["tasks"][0]["agent_id"].as_str().unwrap().to_owned();
    task(
        &p,
        TaskCommand::Reassign {
            task: name,
            agent: actor,
            reason: "Interrupted worker".into(),
            expect_revision: None,
        },
    );
    assert_eq!(
        work::execute(
            &p,
            &WorkCommand::Agent {
                command: AgentCommand::Heartbeat {
                    run,
                    lease_epoch: epoch
                }
            }
        )
        .unwrap_err()
        .code,
        "LEASE_REVOKED"
    );
    assert!(
        !work::activity(&p, first["event"]["event_seq"].as_i64().unwrap()).unwrap()["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
fn check(p: &Project, run: &str, tests: u64) -> String {
    let name = work::board(p).unwrap()["tasks"][0]["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let begin = work::execute(
        p,
        &WorkCommand::Check {
            command: CheckCommand::Begin {
                task: name,
                key: "unit".into(),
                run: run.into(),
            },
        },
    )
    .unwrap();
    let id = begin["check_id"].as_str().unwrap().to_owned();
    let report = p.data_dir.join("report.json");
    std::fs::write(&report,json!({"schema_version":1,"check_key":"unit","producer":"fixture","source":"external_report","exit_code":0,"tests":tests,"passed":tests,"failed":0,"errors":0,"skipped":0,"result":"passed","started_at":1,"finished_at":2}).to_string()).unwrap();
    work::execute(
        p,
        &WorkCommand::Check {
            command: CheckCommand::Record {
                check: id.clone(),
                from_file: report,
            },
        },
    )
    .unwrap();
    id
}
#[test]
fn evidence_gate_and_historical_completion() {
    let (_d, p) = setup();
    let (run, _) = start(&p);
    let id = check(&p, &run, 1);
    let name = work::board(&p).unwrap()["tasks"][0]["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    task(
        &p,
        TaskCommand::Criterion {
            command: CriterionCommand::Accept {
                task: name.clone(),
                criterion: "behavior".into(),
                evidence: id,
                note: "".into(),
            },
        },
    );
    task(
        &p,
        TaskCommand::Submit {
            task: name.clone(),
            run,
            expect_revision: None,
        },
    );
    assert_eq!(
        task(
            &p,
            TaskCommand::Complete {
                task: name.clone(),
                dry_run: true,
                expect_revision: None
            }
        )["passed"],
        false
    );
    task(
        &p,
        TaskCommand::Review {
            task: name.clone(),
            approve: true,
            actor: "owner".into(),
            note: "Verified".into(),
        },
    );
    assert_eq!(
        task(
            &p,
            TaskCommand::Complete {
                task: name.clone(),
                dry_run: true,
                expect_revision: None
            }
        )["passed"],
        true
    );
    task(
        &p,
        TaskCommand::Complete {
            task: name,
            dry_run: false,
            expect_revision: None,
        },
    );
    assert_eq!(work::board(&p).unwrap()["valid_done_count"], 1);
    std::fs::write(p.root.join("code.rs"), "fn value() -> u32 { 2 }\n").unwrap();
    let board = work::board(&p).unwrap();
    assert_eq!(board["done_count"], 1);
    assert_eq!(board["valid_done_count"], 0);
}
#[test]
fn zero_tests_and_revision_conflict() {
    let (_d, p) = setup();
    let (run, _) = start(&p);
    let id = check(&p, &run, 0);
    let shown = work::execute(
        &p,
        &WorkCommand::Check {
            command: CheckCommand::Show { check: id },
        },
    )
    .unwrap();
    assert_eq!(shown["result"], "unverified");
    let name = work::board(&p).unwrap()["tasks"][0]["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        work::execute(
            &p,
            &WorkCommand::Task {
                command: TaskCommand::Complete {
                    task: name,
                    dry_run: false,
                    expect_revision: Some(1)
                }
            }
        )
        .unwrap_err()
        .code,
        "REVISION_CONFLICT"
    );
}
#[test]
fn backup_restore_revokes_live_leases_and_preserves_pause() {
    let (_d, p) = setup();
    let (run, epoch) = start(&p);
    let name = work::board(&p).unwrap()["tasks"][0]["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    task(
        &p,
        TaskCommand::Pause {
            task: name,
            reason: "User request".into(),
        },
    );
    let path = p.data_dir.join("backup.json");
    work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Backup {
                output: path.clone(),
            },
        },
    )
    .unwrap();
    let restored = work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Restore { input: path },
        },
    )
    .unwrap();
    assert_eq!(restored["attached"], false);
    let mut restored_project = p.clone();
    restored_project.coordination_id = restored["coordination_id"].as_str().unwrap().into();
    restored_project.control_dir = p
        .data_dir
        .join("controls")
        .join(&restored_project.coordination_id);
    let board = work::board(&restored_project).unwrap();
    assert_eq!(board["tasks"][0]["state"], "paused");
    assert_eq!(board["tasks"][0]["run"]["status"], "interrupted");
    assert_eq!(
        work::execute(
            &restored_project,
            &WorkCommand::Agent {
                command: AgentCommand::Heartbeat {
                    run,
                    lease_epoch: epoch
                }
            }
        )
        .unwrap_err()
        .code,
        "LEASE_REVOKED"
    );
    assert!(!restored_project.control_dir.join("credentials").exists());
}
#[test]
fn current_source_change_blocks_completion() {
    let (_d, p) = setup();
    let (run, _) = start(&p);
    let id = check(&p, &run, 1);
    let name = work::board(&p).unwrap()["tasks"][0]["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    task(
        &p,
        TaskCommand::Criterion {
            command: CriterionCommand::Accept {
                task: name.clone(),
                criterion: "behavior".into(),
                evidence: id,
                note: "".into(),
            },
        },
    );
    task(
        &p,
        TaskCommand::Submit {
            task: name.clone(),
            run,
            expect_revision: None,
        },
    );
    task(
        &p,
        TaskCommand::Review {
            task: name.clone(),
            approve: true,
            actor: "owner".into(),
            note: "OK".into(),
        },
    );
    std::fs::write(p.root.join("code.rs"), "fn changed() {}\n").unwrap();
    let gate = task(
        &p,
        TaskCommand::Complete {
            task: name.clone(),
            dry_run: true,
            expect_revision: None,
        },
    );
    assert_eq!(gate["passed"], false);
    assert!(
        gate["failures"]
            .as_array()
            .unwrap()
            .contains(&json!("submission_stale"))
    );
    assert_eq!(
        work::execute(
            &p,
            &WorkCommand::Task {
                command: TaskCommand::Complete {
                    task: name,
                    dry_run: false,
                    expect_revision: None
                }
            }
        )
        .unwrap_err()
        .code,
        "COMPLETION_GATE_FAILED"
    );
}
#[test]
fn database_rejects_event_modification_and_receipt_conflict() {
    let (_d, p) = setup();
    let path = task_file(&p);
    let command = WorkCommand::Task {
        command: TaskCommand::Create {
            from_file: path.clone(),
            idempotency_key: Some("create-once".into()),
        },
    };
    let first = work::execute(&p, &command).unwrap();
    assert_eq!(first, work::execute(&p, &command).unwrap());
    let mut d: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    d["title"] = json!("Different task");
    std::fs::write(path, d.to_string()).unwrap();
    assert_eq!(
        work::execute(&p, &command).unwrap_err().code,
        "IDEMPOTENCY_CONFLICT"
    );
    let db = work::connect(&p).unwrap();
    assert!(db.execute("UPDATE events SET type='forged'", []).is_err());
    assert!(db.execute("DELETE FROM events", []).is_err());
}
#[test]
fn zero_minimum_cannot_make_zero_tests_pass() {
    let (_d, p) = setup();
    let path = task_file(&p);
    let mut d: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    d["checks"][0]["minimum_executed_tests"] = json!(0);
    std::fs::write(&path, d.to_string()).unwrap();
    let name = task(
        &p,
        TaskCommand::Create {
            from_file: path,
            idempotency_key: None,
        },
    )["task_id"]
        .as_str()
        .unwrap()
        .to_string();
    let a = register(&p);
    task(
        &p,
        TaskCommand::Ready {
            task: name.clone(),
            expect_revision: None,
        },
    );
    task(
        &p,
        TaskCommand::Assign {
            task: name.clone(),
            agent: a,
            expect_revision: None,
        },
    );
    let run = task(
        &p,
        TaskCommand::Start {
            task: name,
            workspace: "current".into(),
            agent: None,
            expect_revision: None,
        },
    )["run_id"]
        .as_str()
        .unwrap()
        .to_string();
    let id = check(&p, &run, 0);
    let shown = work::execute(
        &p,
        &WorkCommand::Check {
            command: CheckCommand::Show { check: id },
        },
    )
    .unwrap();
    assert_eq!(shown["result"], "unverified");
}
#[test]
fn restore_preserves_session_history_without_ack_or_epoch_reuse() {
    use pctx::session::{self, ContextCommand, SessionCommand};
    let (_d, p) = setup();
    let (run, epoch) = start(&p);
    let name = work::board(&p).unwrap()["tasks"][0]["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let attached = session::session(
        &p,
        &SessionCommand::Attach {
            agent: "worker".into(),
            runtime: "fixture".into(),
            workspace: "current".into(),
            native_session: None,
            role: None,
            account_pool: None,
            adapter_version: "manual-v1".into(),
        },
    )
    .unwrap();
    let sid = attached["session_id"].as_str().unwrap().to_owned();
    let context = session::context(
        &p,
        &ContextCommand::Get {
            task_id: name.clone(),
            session: sid.clone(),
            mode: "full".into(),
            since: None,
            scope: vec!["code.rs".into()],
            budget_bytes: 50000,
        },
    )
    .unwrap();
    let ctx = context["context_id"].as_str().unwrap().to_owned();
    session::context(
        &p,
        &ContextCommand::Ack {
            context: ctx,
            session: sid.clone(),
            epoch: 1,
            provenance: "explicit-agent".into(),
        },
    )
    .unwrap();
    task(
        &p,
        TaskCommand::Pause {
            task: name,
            reason: "Preserve user pause".into(),
        },
    );
    session::session(
        &p,
        &SessionCommand::Suspend {
            session: sid.clone(),
            reason: "Capture continuity".into(),
        },
    )
    .unwrap();
    let archive = p.data_dir.join("session-backup.json");
    work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Backup {
                output: archive.clone(),
            },
        },
    )
    .unwrap();
    let restore = work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Restore { input: archive },
        },
    )
    .unwrap();
    let mut restored = p.clone();
    restored.coordination_id = restore["coordination_id"].as_str().unwrap().into();
    restored.control_dir = p.data_dir.join("controls").join(&restored.coordination_id);
    let db = restored.connect(true).unwrap();
    let (new_epoch, status): (i64, String) = db
        .query_row(
            "SELECT epoch,status FROM pctx_sessions WHERE id=?1",
            [&sid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(new_epoch, 2);
    assert_eq!(status, "suspended");
    let acks: i64 = db
        .query_row("SELECT count(*) FROM pctx_context_acks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(acks, 0);
    let capsules: i64 = db
        .query_row(
            "SELECT count(*) FROM pctx_session_capsules WHERE session=?1",
            [&sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(capsules, 1);
    let emissions: i64 = db
        .query_row(
            "SELECT count(*) FROM pctx_context_emissions WHERE session=?1",
            [&sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(emissions, 1);
    assert_eq!(
        work::board(&restored).unwrap()["tasks"][0]["state"],
        "paused"
    );
    assert_eq!(
        work::execute(
            &restored,
            &WorkCommand::Agent {
                command: AgentCommand::Heartbeat {
                    run,
                    lease_epoch: epoch
                }
            }
        )
        .unwrap_err()
        .code,
        "LEASE_REVOKED"
    );
}

#[test]
fn schedule_v2_backup_strips_local_binding_and_restore_revokes_running_authority() {
    let (_dir, p) = setup();
    work::board(&p).unwrap();
    pctx::schedule::execute(
        &p,
        &pctx::schedule::ScheduleCommand::List { namespace: None },
    )
    .unwrap();
    let db = p.connect(true).unwrap();
    db.execute_batch("INSERT INTO schedule_bindings VALUES('owner','digest',1,'ws','policy','fingerprint','private-environment-marker',1);
INSERT INTO schedule_runs VALUES('owner','digest',1,'occurrence',1,'running',NULL,NULL,'{}',NULL,NULL,1,NULL);
INSERT INTO schedule_runs VALUES('owner','digest',1,'older',1,'succeeded','output','result-hash','{\"count\":1}','message',NULL,1,2);
INSERT INTO schedule_installations VALUES('owner','digest','ws','launchd','plan','manifest','registered','{\"absolute_path\":\"/private/installation-marker\",\"environment\":\"private-environment-marker\"}',1);
INSERT INTO schedule_occurrences VALUES('owner','digest',1,'occurrence',1,'running','{}',1);") .unwrap();
    drop(db);
    let archive = p.data_dir.join("schedule-backup.json");
    work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Backup {
                output: archive.clone(),
            },
        },
    )
    .unwrap();
    let bytes: Value = serde_json::from_slice(&std::fs::read(&archive).unwrap()).unwrap();
    let database: Vec<u8> = serde_json::from_value(bytes["database"].clone()).unwrap();
    let content = String::from_utf8_lossy(&database);
    assert!(!content.contains("private-environment-marker"));
    assert!(!content.contains("installation-marker"));
    // Export scrubbing must not mutate the running local installation.
    let source = p.connect(true).unwrap();
    assert_eq!(
        source
            .query_row("SELECT count(*) FROM schedule_bindings", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(source);
    let restored = work::execute(
        &p,
        &WorkCommand::Control {
            command: work::ControlCommand::Restore { input: archive },
        },
    )
    .unwrap();
    let mut destination = p.clone();
    destination.coordination_id = restored["coordination_id"].as_str().unwrap().into();
    destination.control_dir = p
        .data_dir
        .join("controls")
        .join(&destination.coordination_id);
    let db = destination.connect(true).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM schedule_bindings", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT state FROM schedule_runs WHERE occurrence='occurrence'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "interrupted_unknown"
    );
    assert_eq!(
        db.query_row("SELECT state FROM schedule_occurrences", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "interrupted_unknown"
    );
    assert_eq!(
        db.query_row("SELECT state FROM schedule_installations", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "unknown_restored"
    );
    let completed: (String, String, String) = db
        .query_row(
            "SELECT state,result_hash,delivery_ref FROM schedule_runs WHERE occurrence='older'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        completed,
        ("succeeded".into(), "result-hash".into(), "message".into())
    );
}

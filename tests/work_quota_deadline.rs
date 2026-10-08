//! Direct API callers must share the request deadline, including SQL contention.
use pctx::{
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor},
    quota::{self, QuotaCommand},
    work::{self, AgentCommand, CheckCommand, TaskCommand, WorkCommand},
};
use std::{
    fs,
    time::{Duration, Instant},
};
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(data.join("workspace")).unwrap();
    fs::create_dir_all(data.join("control")).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspace"),
        control_dir: data.join("control"),
        project_id: "fixture".into(),
        workspace_id: "workspace".into(),
        coordination_id: "coordination".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "fixture".into(),
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
fn quota_reads() -> Vec<QuotaCommand> {
    vec![
        QuotaCommand::Report {
            pool: None,
            task_id: None,
            session: None,
            group_by: "pool".into(),
            window: "7d".into(),
            include_coordination: false,
            max_age_seconds: 900,
        },
        QuotaCommand::Plan {
            pool: "fixture".into(),
            task_id: None,
            max_age_seconds: 900,
        },
        QuotaCommand::Reconcile {
            pool: "fixture".into(),
            max_age_seconds: 900,
        },
    ]
}
fn work_reads() -> Vec<WorkCommand> {
    vec![
        WorkCommand::Task {
            command: TaskCommand::List,
        },
        WorkCommand::Task {
            command: TaskCommand::Show {
                task: "T-404".into(),
            },
        },
        WorkCommand::Task {
            command: TaskCommand::Complete {
                task: "T-404".into(),
                dry_run: true,
                expect_revision: None,
            },
        },
        WorkCommand::Agent {
            command: AgentCommand::List,
        },
        WorkCommand::Agent {
            command: AgentCommand::Show {
                agent: "missing".into(),
            },
        },
        WorkCommand::Check {
            command: CheckCommand::List { task: None },
        },
        WorkCommand::Check {
            command: CheckCommand::Show {
                check: "missing".into(),
            },
        },
        WorkCommand::Check {
            command: CheckCommand::Plan {
                task_id: "T-404".into(),
                key: "test".into(),
                run: Some("missing".into()),
            },
        },
    ]
}
#[test]
fn already_expired_reads_do_not_initialize_control_storage_or_validate_missing_targets() {
    let (_t, mut p) = fixture();
    p.deadline = Some(Deadline::from_instant(Instant::now()));
    for command in work_reads() {
        assert_eq!(
            work::execute(&p, &command).err().unwrap().code,
            "TIMEOUT",
            "{command:?}"
        );
    }
    for command in quota_reads() {
        assert_eq!(
            quota::execute(&p, &command).err().unwrap().code,
            "TIMEOUT",
            "{command:?}"
        );
    }
    assert!(!p.control_db().exists());
    assert_eq!(fs::read_dir(&p.control_dir).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&p.workspace_dir).unwrap().count(), 0);
}
#[test]
fn work_read_transaction_contention_uses_remaining_budget_without_control_events() {
    let (_t, mut p) = fixture();
    let lock = work::connect(&p).unwrap();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    for command in [
        WorkCommand::Task {
            command: TaskCommand::List,
        },
        WorkCommand::Agent {
            command: AgentCommand::List,
        },
        WorkCommand::Check {
            command: CheckCommand::List { task: None },
        },
    ] {
        p.deadline = Some(Deadline::from_millis(50).unwrap());
        let started = Instant::now();
        let error = work::execute(&p, &command).err().unwrap();
        assert_eq!(error.code, "TIMEOUT", "{command:?}: {error:?}");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "Busy wait renewed beyond original budget"
        );
    }
    lock.execute_batch("ROLLBACK").unwrap();
    let events: i64 = lock
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(events, 0);
}
#[test]
fn quota_read_exclusive_contention_returns_timeout_instead_of_empty_unknown_success() {
    let (_t, mut p) = fixture();
    quota::execute(&p, &quota_reads()[0]).unwrap();
    let lock = work::connect(&p).unwrap();
    lock.pragma_update(None, "journal_mode", "DELETE").unwrap();
    lock.execute_batch("BEGIN EXCLUSIVE").unwrap();
    for command in quota_reads() {
        p.deadline = Some(Deadline::from_millis(50).unwrap());
        let started = Instant::now();
        let error = quota::execute(&p, &command).err().unwrap();
        assert_eq!(error.code, "TIMEOUT", "{command:?}: {error:?}");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "Busy wait renewed beyond original budget"
        );
    }
    lock.execute_batch("ROLLBACK").unwrap();
    let events: i64 = lock
        .query_row("SELECT COUNT(*) FROM quota_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(events, 0);
    let observations: i64 = lock
        .query_row("SELECT COUNT(*) FROM quota_observations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(observations, 0);
}

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

// A missing progress handler must fail this regression rather than leave the
// billion-row fixture computing indefinitely. This never renews the request.
fn guarded_sql<T>(db: &rusqlite::Connection, call: impl FnOnce() -> T) -> T {
    let interrupt = db.get_interrupt_handle();
    let (cancel, receiver) = std::sync::mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        if receiver.recv_timeout(Duration::from_secs(2))
            == Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        {
            interrupt.interrupt();
            true
        } else {
            false
        }
    });
    let result = call();
    let _ = cancel.send(());
    assert!(
        !watchdog.join().unwrap(),
        "SQL fixture required emergency interruption"
    );
    result
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

#[test]
fn cpu_bound_sql_expires_inside_the_engine_under_the_original_budget() {
    let (_temp, mut p) = fixture();
    let db = rusqlite::Connection::open_in_memory().unwrap();
    let mut statement = db.prepare("WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n<1000000000) SELECT sum(n) FROM numbers").unwrap();
    p.deadline = Some(Deadline::from_millis(50).unwrap());
    let original = p.deadline.unwrap().instant();
    let started = Instant::now();
    let error = guarded_sql(&db, || {
        p.sqlite_call(&db, || statement.query_row([], |row| row.get::<_, i64>(0)))
    })
    .unwrap_err();
    assert_eq!(error.code, "TIMEOUT");
    assert_eq!(error.exit, 7);
    assert!(
        statement.get_status(rusqlite::StatementStatus::VmStep) >= 1000,
        "SQL never entered the controlled long computation"
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "SQL did not interrupt within the original budget tolerance"
    );
    assert_eq!(p.deadline.unwrap().instant(), original);
}

#[test]
fn reused_sql_connection_does_not_inherit_an_expired_prior_request_handler() {
    let (_temp, mut p) = fixture();
    let db = rusqlite::Connection::open_in_memory().unwrap();
    p.deadline = Some(Deadline::from_millis(50).unwrap());
    let error = guarded_sql(&db, || p.sqlite_call(&db, || db.query_row("WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n<1000000000) SELECT sum(n) FROM numbers", [], |row| row.get::<_, i64>(0)))).unwrap_err();
    assert_eq!(error.code, "TIMEOUT");
    // This is a distinct caller scope, not a renewal of the expired query.
    p.deadline = None;
    let total: i64 = p.sqlite_call(&db, || db.query_row("WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n<100000) SELECT sum(n) FROM numbers", [], |row| row.get(0))).unwrap();
    assert_eq!(total, 5_000_050_000);
}

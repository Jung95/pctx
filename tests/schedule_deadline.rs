use pctx::{
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor},
    schedule::{self, ScheduleCommand},
};
use std::{fs, time::Instant};
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let root = base.join("project");
    fs::create_dir_all(&root).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: base.join("data"),
        workspace_dir: base.join("data/workspace"),
        control_dir: base.join("data/control"),
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
#[test]
fn expired_schedule_reads_have_no_control_or_staging_effects() {
    let (_t, mut p) = fixture();
    let original = Deadline::from_instant(Instant::now());
    p.deadline = Some(original);
    for c in [
        ScheduleCommand::List { namespace: None },
        ScheduleCommand::Plan {
            namespace: "fixture".into(),
            id: "missing".into(),
            provider: "fixture".into(),
            staging_root: ".pctx/bridge".into(),
        },
        ScheduleCommand::Inspect {
            namespace: "fixture".into(),
            id: "missing".into(),
            observe_native: true,
        },
    ] {
        let e = schedule::execute(&p, &c).unwrap_err();
        assert_eq!(e.code, "TIMEOUT");
        assert_eq!(e.exit, 7);
    }
    assert_eq!(p.deadline.unwrap().instant(), original.instant());
    assert!(!p.data_dir.exists());
    assert!(!p.root.join(".pctx").exists());
}
#[test]
fn default_empty_schedule_read_does_not_initialize_control_state() {
    let (_t, p) = fixture();
    let v = schedule::execute(&p, &ScheduleCommand::List { namespace: None }).unwrap();
    assert!(v["schedules"].as_array().unwrap().is_empty());
    assert_eq!(v["execution_started"], false);
    assert!(p.deadline.is_none());
    assert!(!p.data_dir.exists());
}
#[test]
fn initialized_control_without_schedule_tables_stays_unchanged_after_read() {
    let (_t, p) = fixture();
    fs::create_dir_all(&p.control_dir).unwrap();
    let db = rusqlite::Connection::open(p.control_db()).unwrap();
    db.execute_batch(
        "CREATE TABLE unrelated(value TEXT);INSERT INTO unrelated VALUES('retained');",
    )
    .unwrap();
    drop(db);
    let before = fs::read(p.control_db()).unwrap();
    let v = schedule::execute(&p, &ScheduleCommand::List { namespace: None }).unwrap();
    assert!(v["schedules"].as_array().unwrap().is_empty());
    assert_eq!(fs::read(p.control_db()).unwrap(), before);
    assert!(!p.control_dir.join("connection-init.lock").exists());
    let db = rusqlite::Connection::open(p.control_db()).unwrap();
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name LIKE 'schedule_%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}
#[cfg(unix)]
#[test]
fn inspect_fifo_owned_manifest_is_refused_without_blocking() {
    const PROBE: &str = "PCTX_SCHEDULE_FIFO_DEADLINE_PROBE";
    if std::env::var_os(PROBE).is_none() {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "inspect_fifo_owned_manifest_is_refused_without_blocking",
                "--nocapture",
            ])
            .env(PROBE, "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let end = Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= end {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Owned FIFO fixture exceeded three-second process bound");
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let (_t, mut p) = fixture();
    fs::create_dir_all(&p.control_dir).unwrap();
    fs::create_dir_all(p.root.join(".pctx/bridge")).unwrap();
    let db = rusqlite::Connection::open(p.control_db()).unwrap();
    db.execute_batch("CREATE TABLE schedule_schema(version INTEGER);INSERT INTO schedule_schema VALUES(2);CREATE TABLE schedule_installations(namespace TEXT,schedule TEXT,state TEXT,metadata TEXT);").unwrap();
    let metadata=serde_json::json!({"plan":{"staging_root":".pctx/bridge","files":{"managed.json":"fixture"},"plan_hash":"fixture"}}).to_string();
    db.execute(
        "INSERT INTO schedule_installations VALUES('fixture','fixture','staged',?1)",
        [metadata],
    )
    .unwrap();
    drop(db);
    let path = p.root.join(".pctx/bridge/managed.json");
    use std::os::unix::ffi::OsStrExt;
    let native = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(native.as_ptr(), 0o600) }, 0);
    let original = Deadline::from_millis(1000).unwrap();
    p.deadline = Some(original);
    let before = fs::read(p.control_db()).unwrap();
    let e = schedule::execute(
        &p,
        &ScheduleCommand::Inspect {
            namespace: "fixture".into(),
            id: "fixture".into(),
            observe_native: false,
        },
    )
    .unwrap_err();
    assert_eq!(e.code, "POLICY_DENIED");
    assert_eq!(e.exit, 5);
    assert_eq!(p.deadline.unwrap().instant(), original.instant());
    assert_eq!(fs::read(p.control_db()).unwrap(), before);
    assert!(!p.control_dir.join("connection-init.lock").exists());
}

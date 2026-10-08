//! Request expiry must never publish a partial replacement index/checkpoint.
use fs2::FileExt;
use pctx::{
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor},
    storage,
};
use std::{
    fs,
    time::{Duration, Instant},
};

fn fixture() -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let workspace = temp.path().join("workspace");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&workspace).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: temp.path().join("data"),
        workspace_dir: workspace,
        control_dir: temp.path().join("control"),
        project_id: "project".into(),
        workspace_id: "workspace".into(),
        coordination_id: "coordination".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "project".into(),
                name: "deadline".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    fs::write(p.root.join("source.py"), "def stable():\n    return 1\n").unwrap();
    (temp, p)
}

fn expire(p: &Project) -> Project {
    let mut timed = p.clone();
    let deadline = Deadline::from_millis(1).unwrap();
    while deadline.check().is_ok() {
        std::hint::spin_loop();
    }
    timed.deadline = Some(deadline);
    timed
}

#[test]
fn expired_storage_requests_preserve_generation_and_checkpoint() {
    let (_temp, p) = fixture();
    let generation = storage::update(&p).unwrap()["generation_id"].clone();
    let checkpoint = storage::checkpoint(&p, Some("stable"), true).unwrap();
    fs::write(p.root.join("source.py"), "def changed():\n    return 2\n").unwrap();
    let expired = expire(&p);
    for result in [
        storage::update(&expired),
        storage::rebuild(&expired),
        storage::checkpoint(&expired, Some("expired"), false),
        storage::snapshot(&expired).map(|_| serde_json::Value::Null),
        storage::changes(&expired, "stable"),
        storage::checkpoint_delete(&expired, "stable"),
    ] {
        let error = result.unwrap_err();
        assert_eq!(error.code, "TIMEOUT");
        assert_eq!(error.exit, 7);
    }
    assert_eq!(
        storage::snapshot(&p).unwrap().0.unwrap(),
        generation.as_str().unwrap()
    );
    assert_eq!(
        storage::checkpoint_get(&p, "stable").unwrap()["id"],
        checkpoint["id"]
    );
    assert_eq!(
        storage::checkpoint_list(&p).unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(!fs::read_dir(&p.workspace_dir).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".rebuild")
    }));
}

#[test]
fn writer_and_snapshot_waits_consume_supplied_budget_without_five_second_reset() {
    let (_temp, p) = fixture();
    let generation = storage::update(&p).unwrap()["generation_id"].clone();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(p.workspace_dir.join("writer.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    for snapshot in [false, true] {
        let mut timed = p.clone();
        timed.deadline = Some(Deadline::from_millis(60).unwrap());
        let started = Instant::now();
        let result = if snapshot {
            storage::snapshot(&timed).map(|_| serde_json::Value::Null)
        } else {
            storage::update(&timed)
        };
        assert_eq!(result.unwrap_err().code, "TIMEOUT");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "Request renewed its lock budget"
        );
    }
    FileExt::unlock(&lock).unwrap();
    assert_eq!(
        storage::snapshot(&p).unwrap().0.unwrap(),
        generation.as_str().unwrap()
    );
}

#[test]
fn sqlite_writer_contention_times_out_without_publishing_or_losing_checkpoint() {
    let (_temp, p) = fixture();
    let generation = storage::update(&p).unwrap()["generation_id"].clone();
    let cp = storage::checkpoint(&p, Some("before"), true).unwrap();
    let blocker = p.connect(false).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    fs::write(
        p.root.join("source.py"),
        "def new_source():\n    return 3\n",
    )
    .unwrap();
    let mut timed = p.clone();
    timed.deadline = Some(Deadline::from_millis(60).unwrap());
    let error = storage::update(&timed).unwrap_err();
    assert_eq!(error.code, "TIMEOUT", "{error:?}");
    blocker.execute_batch("ROLLBACK").unwrap();
    let db = p.connect(false).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM generations WHERE state='building'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        storage::snapshot(&p).unwrap().0.unwrap(),
        generation.as_str().unwrap()
    );
    assert_eq!(
        storage::checkpoint_get(&p, "before").unwrap()["id"],
        cp["id"]
    );
}

#[test]
fn parser_expiry_retains_previous_generation_instead_of_marking_source_skipped() {
    let (_temp, p) = fixture();
    let generation = storage::update(&p).unwrap()["generation_id"].clone();
    // Actual parsing work, not a sleep or a production test hook.
    let source = "def expanded():\n    return 1\n".repeat(30_000);
    fs::write(p.root.join("source.py"), source).unwrap();
    let mut timed = p.clone();
    timed.deadline = Some(Deadline::from_millis(1).unwrap());
    assert_eq!(storage::update(&timed).unwrap_err().code, "TIMEOUT");
    let (_, files) = storage::snapshot(&p).unwrap();
    assert_eq!(files[0].symbols[0].name, "stable");
    assert_eq!(
        storage::snapshot(&p).unwrap().0.unwrap(),
        generation.as_str().unwrap()
    );
}

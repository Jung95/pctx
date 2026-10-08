use pctx::{
    deadline::Deadline,
    operations::delivery_barrier_fingerprint as barrier,
    project::{Config, Project, ProjectConfig, RootAnchor},
};
use rusqlite::Connection;
use std::time::{Duration, Instant};
fn fixture() -> (tempfile::TempDir, Project, Connection) {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().canonicalize().unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root: root.clone(),
        data_dir: root.join("data"),
        workspace_dir: root.join("workspace"),
        control_dir: root.join("control"),
        project_id: "fixture".into(),
        workspace_id: "ws".into(),
        coordination_id: "coord".into(),
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
    (t, p, Connection::open_in_memory().unwrap())
}
fn schema(db: &Connection) {
    db.execute_batch("CREATE TABLE agents(id TEXT PRIMARY KEY,name TEXT UNIQUE NOT NULL,kind TEXT NOT NULL,concurrency_limit INTEGER NOT NULL);
    INSERT INTO agents VALUES('AG-one','worker','local',1),('AG-two','other','local',1);
    CREATE TABLE ops_roles(role TEXT PRIMARY KEY,paused INTEGER NOT NULL,reason TEXT NOT NULL,actor TEXT NOT NULL,updated INTEGER NOT NULL);
    CREATE TABLE ops_silences(role TEXT NOT NULL,recipient TEXT NOT NULL,topic TEXT NOT NULL,active INTEGER NOT NULL,reason TEXT NOT NULL,actor TEXT NOT NULL,updated INTEGER NOT NULL,PRIMARY KEY(role,recipient,topic));
    CREATE TABLE ops_events(seq INTEGER PRIMARY KEY AUTOINCREMENT,entity TEXT NOT NULL,kind TEXT NOT NULL,actor TEXT NOT NULL,payload TEXT NOT NULL,created INTEGER NOT NULL);").unwrap();
}
#[test]
fn uninitialized_controls_are_read_only_and_partial_schema_fails_closed() {
    let (_t, p, db) = fixture();
    let before = db.total_changes();
    let a = barrier(&p, &db, "AG-one", None, None).unwrap();
    assert_eq!(a, barrier(&p, &db, "AG-one", None, None).unwrap());
    assert_eq!(db.total_changes(), before);
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM sqlite_master", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    db.execute_batch(
        "CREATE TABLE ops_roles(role TEXT PRIMARY KEY,paused INTEGER,updated INTEGER)",
    )
    .unwrap();
    assert_eq!(
        barrier(&p, &db, "AG-one", None, None).unwrap_err().code,
        "POLICY_UNAVAILABLE"
    );
}
#[test]
fn canonical_alias_registered_role_and_wildcard_pauses_block_without_grant() {
    let (_t, p, db) = fixture();
    schema(&db);
    for role in ["AG-one", "worker", "developer", "*"] {
        db.execute(
            "INSERT INTO ops_roles VALUES(?1,1,'private reason','owner',1)",
            [role],
        )
        .unwrap();
        let changes = db.total_changes();
        let e = barrier(&p, &db, "AG-one", Some("developer"), None).unwrap_err();
        assert_eq!(e.code, "ROLE_PAUSED");
        assert_eq!(e.exit, 5);
        assert!(!e.message.contains("worker") && !e.message.contains("private reason"));
        assert_eq!(changes, db.total_changes());
        db.execute("DELETE FROM ops_roles", []).unwrap();
    }
    db.execute(
        "INSERT INTO ops_roles VALUES('developer',1,'private','owner',1)",
        [],
    )
    .unwrap();
    // A role-less authenticated recipient gets no inferred registered role.
    assert!(barrier(&p, &db, "AG-one", None, None).is_ok());
}
#[test]
fn topic_silence_normalizes_aliases_and_is_exact_to_role_recipient_topic() {
    let (_t, p, db) = fixture();
    schema(&db);
    let before = barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap();
    for (role, recipient) in [
        ("developer", "worker"),
        ("worker", "AG-one"),
        ("*", "*"),
        ("AG-one", "*"),
    ] {
        db.execute(
            "INSERT INTO ops_silences VALUES(?1,?2,'private-topic',1,'hidden body','owner',1)",
            rusqlite::params![role, recipient],
        )
        .unwrap();
        assert_eq!(
            barrier(&p, &db, "AG-one", Some("developer"), Some("private-topic"))
                .unwrap_err()
                .code,
            "TOPIC_SILENCED"
        );
        let e = barrier(&p, &db, "AG-one", Some("developer"), None).unwrap_err();
        assert_eq!(e.code, "DELIVERY_TOPIC_REQUIRED");
        assert!(!e.message.contains("private-topic") && !e.message.contains("hidden body"));
        assert_eq!(
            before,
            barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap()
        );
        db.execute("DELETE FROM ops_silences", []).unwrap();
    }
    db.execute_batch("INSERT INTO ops_silences VALUES('developer','AG-two','private-topic',1,'private','owner',1);
        INSERT INTO ops_silences VALUES('different-role','AG-one','private-topic',1,'private','owner',1);").unwrap();
    assert!(barrier(&p, &db, "AG-one", Some("developer"), None).is_ok());
    assert!(barrier(&p, &db, "AG-one", None, Some("private-topic")).is_ok());
}
#[test]
fn relevant_control_mutation_rebinds_fingerprint_but_unrelated_controls_do_not() {
    let (_t, p, db) = fixture();
    schema(&db);
    let initial = barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap();
    db.execute(
        "INSERT INTO ops_roles VALUES('developer',0,'resume','owner',1)",
        [],
    )
    .unwrap();
    let resumed = barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap();
    assert_ne!(initial, resumed);
    db.execute("UPDATE ops_roles SET updated=2 WHERE role='developer'", [])
        .unwrap();
    let revised = barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap();
    assert_ne!(resumed, revised);
    db.execute_batch(
        "INSERT INTO ops_silences VALUES('*','AG-two','public',1,'private','owner',1);
        INSERT INTO ops_silences VALUES('*','AG-one','other',1,'private','owner',1);",
    )
    .unwrap();
    assert_eq!(
        revised,
        barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap()
    );
    db.execute("UPDATE ops_roles SET paused=1 WHERE role='developer'", [])
        .unwrap();
    assert_eq!(
        barrier(&p, &db, "AG-one", Some("developer"), Some("public"))
            .unwrap_err()
            .code,
        "ROLE_PAUSED"
    );
}
#[test]
fn canonical_id_wins_over_colliding_alias_and_expired_budget_never_writes() {
    let (_t, mut p, db) = fixture();
    schema(&db);
    db.execute("UPDATE agents SET name='AG-two' WHERE id='AG-one'", [])
        .unwrap();
    db.execute(
        "INSERT INTO ops_silences VALUES('*','AG-two','private',1,'hidden','owner',1)",
        [],
    )
    .unwrap();
    assert!(barrier(&p, &db, "AG-one", None, Some("private")).is_ok());
    p.deadline = Some(Deadline::from_instant(
        Instant::now() - Duration::from_secs(1),
    ));
    let changes = db.total_changes();
    let e = barrier(&p, &db, "AG-one", None, None).unwrap_err();
    assert_eq!(e.code, "TIMEOUT");
    assert_eq!(e.exit, 7);
    assert_eq!(changes, db.total_changes());
    assert!(!p.data_dir.exists());
}

#[test]
fn durable_role_event_revision_prevents_same_second_pause_resume_reuse() {
    let (_t, p, db) = fixture();
    schema(&db);
    db.execute(
        "INSERT INTO ops_roles VALUES('developer',0,'resume','owner',100)",
        [],
    )
    .unwrap();
    let before = barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap();
    db.execute_batch("UPDATE ops_roles SET paused=1; INSERT INTO ops_events(entity,kind,actor,payload,created) VALUES('developer','role_policy_changed','owner','{}',100);
        UPDATE ops_roles SET paused=0; INSERT INTO ops_events(entity,kind,actor,payload,created) VALUES('developer','role_policy_changed','owner','{}',100);").unwrap();
    let after = barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap();
    assert_ne!(before, after);
    db.execute("INSERT INTO ops_events(entity,kind,actor,payload,created) VALUES('unrelated','role_policy_changed','owner','{}',100)",[]).unwrap();
    assert_eq!(
        after,
        barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap()
    );
    db.execute("INSERT INTO ops_events(entity,kind,actor,payload,created) VALUES('developer','message_queued','owner','{}',100)",[]).unwrap();
    assert_eq!(
        after,
        barrier(&p, &db, "AG-one", Some("developer"), Some("public")).unwrap()
    );
}
#[test]
fn relevant_topic_controls_are_bounded_and_corrupt_labels_fail_closed() {
    let (_t, p, db) = fixture();
    schema(&db);
    for n in 0..257 {
        db.execute(
            "INSERT INTO ops_silences VALUES('*','AG-one',?1,0,'private','owner',1)",
            [format!("topic-{n}")],
        )
        .unwrap();
    }
    assert_eq!(
        barrier(&p, &db, "AG-one", None, None).unwrap_err().code,
        "POLICY_UNAVAILABLE"
    );
    assert!(barrier(&p, &db, "AG-one", None, Some("topic-1")).is_ok());
    db.execute("DELETE FROM ops_silences", []).unwrap();
    db.execute(
        "INSERT INTO ops_silences VALUES('*','AG-one',?1,0,'private','owner',1)",
        ["x".repeat(257)],
    )
    .unwrap();
    assert_eq!(
        barrier(&p, &db, "AG-one", None, None).unwrap_err().code,
        "POLICY_UNAVAILABLE"
    );
}

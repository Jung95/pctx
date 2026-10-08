use pctx::{
    deadline::Deadline,
    operations::{self, OperationCommand, RoleCommand},
    project::{Config, PolicyConfig, Project, ProjectConfig, RootAnchor, SourceTopic},
    source_delivery::{SourcePolicy, validate_topics},
};
use std::time::{Duration, Instant};
fn fixture() -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("project");
    std::fs::create_dir(&root).unwrap();
    let data = base.join("data");
    std::fs::create_dir_all(data.join("workspace")).unwrap();
    std::fs::create_dir_all(data.join("control")).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspace"),
        control_dir: data.join("control"),
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
    (temp, p)
}
fn assert_no_authority_side_effects(p: &Project) {
    // Constructor directories are fixture prerequisites. Validation must not
    // initialize a database, actor credential, or connection lock within them.
    for path in [&p.workspace_dir, &p.control_dir] {
        assert_eq!(std::fs::read_dir(path).unwrap().count(), 0);
    }
    assert_eq!(std::fs::read_dir(&p.data_dir).unwrap().count(), 2);
}
fn silence(p: &Project, topic: &str) {
    operations::execute(
        p,
        &OperationCommand::Role {
            command: RoleCommand::Pause {
                role: "implementer".into(),
                reason: "Fixture quiet control".into(),
                topic: Some(topic.into()),
                recipient: Some("owner".into()),
            },
        },
    )
    .unwrap();
}
fn assignment(scope: &str, topics: &[&str]) -> SourceTopic {
    SourceTopic {
        scope: vec![scope.into()],
        topics: topics.iter().map(|s| s.to_string()).collect(),
    }
}
#[test]
fn default_empty_policy_serialization_preserves_existing_hash_input() {
    let policy = serde_json::to_value(PolicyConfig::default()).unwrap();
    assert_eq!(
        policy,
        serde_json::json!({"exclude":[],"network":"deny","persist_source":false})
    );
    assert!(
        serde_json::from_str::<SourceTopic>(r#"{"scope":["src/**"],"topics":[],"grant":true}"#)
            .is_err()
    );
    assert!(serde_json::from_str::<SourceTopic>(r#"{"scope":["src/**"]}"#).is_err());
}
#[test]
fn overlapping_owner_assignments_union_and_public_cannot_declassify_quiet_source() {
    let (_t, mut p) = fixture();
    silence(&p, "quiet");
    p.config.policy.source_topics = vec![
        assignment("src/**", &[]),
        assignment("src/private/**", &["quiet"]),
    ];
    let policy = SourcePolicy::new(&p, None, Some("public"), "implementer").unwrap();
    assert!(policy.check("src/public.rs", None).is_ok());
    let empty = Vec::new();
    let e = policy
        .check("src/private/value.rs", Some(&empty))
        .unwrap_err();
    assert_eq!(e.code, "TOPIC_SILENCED");
    assert_eq!(e.exit, 5);
    assert!(!e.message.contains("value.rs") && !e.message.contains("quiet"));
}
#[test]
fn unknown_source_under_quiet_controls_is_denied_even_inside_public_packet() {
    let (_t, p) = fixture();
    silence(&p, "quiet");
    let policy = SourcePolicy::new(&p, None, Some("public"), "implementer").unwrap();
    let e = policy.check("unclassified.rs", None).unwrap_err();
    assert_eq!(e.code, "SOURCE_TOPIC_REQUIRED");
    assert_eq!(e.exit, 5);
    assert!(!e.message.contains("unclassified.rs") && !e.message.contains("quiet"));
    assert!(policy.check("authored-public.rs", Some(&[])).is_ok());
    assert_eq!(
        policy
            .check("declared.rs", Some(&["quiet".into()]))
            .unwrap_err()
            .code,
        "TOPIC_SILENCED"
    );
    assert!(
        policy
            .check("declared.rs", Some(&["public".into()]))
            .is_ok()
    );
}
#[test]
fn unknown_without_quiet_controls_is_allowed_but_bound_to_path_and_classification() {
    let (_t, p) = fixture();
    let policy = SourcePolicy::new(&p, None, None, "implementer").unwrap();
    let unknown = policy.check("one.rs", None).unwrap();
    assert_eq!(unknown, policy.check("one.rs", None).unwrap());
    assert_ne!(unknown, policy.check("two.rs", None).unwrap());
    assert_ne!(unknown, policy.check("one.rs", Some(&[])).unwrap());
    assert!(unknown.len() == 64 && !unknown.contains("one.rs"));
}
#[test]
fn wildcard_pause_rechecks_explicit_public_sources_after_policy_construction() {
    let (_t, p) = fixture();
    let policy = SourcePolicy::new(&p, None, Some("public"), "implementer").unwrap();
    assert!(policy.check("public.rs", Some(&[])).is_ok());
    operations::execute(
        &p,
        &OperationCommand::Role {
            command: RoleCommand::Pause {
                role: "*".into(),
                reason: "Fixture pause".into(),
                topic: None,
                recipient: None,
            },
        },
    )
    .unwrap();
    assert_eq!(
        policy.check("public.rs", Some(&[])).unwrap_err().code,
        "ROLE_PAUSED"
    );
    assert_eq!(
        policy.check("unknown.rs", None).unwrap_err().code,
        "ROLE_PAUSED"
    );
}
#[test]
fn configuration_scope_and_label_bounds_fail_before_delivery_effects() {
    let (_t, mut p) = fixture();
    for scope in [
        "",
        "/absolute/**",
        "../escape/**",
        "a/../b",
        "C:/drive/**",
        "a\\b",
        "[",
        "a//b",
        "./src",
    ] {
        p.config.policy.source_topics = vec![assignment(scope, &[])];
        let e = SourcePolicy::new(&p, None, Some("public"), "implementer")
            .err()
            .unwrap();
        assert_eq!(e.code, "INVALID_CONFIG", "{scope}");
        assert_no_authority_side_effects(&p);
    }
    for topic in [
        "".to_string(),
        "x".repeat(257),
        "token=sk-proj-abcdefghijklmnopqrstuv".into(),
        "line\nbreak".into(),
    ] {
        assert_eq!(
            validate_topics(&[topic]).unwrap_err().code,
            "INVALID_CONFIG"
        );
    }
    assert!(validate_topics(&[]).is_ok());
    assert!(validate_topics(&vec!["label".into(); 33]).is_err());
    p.config.policy.source_topics = vec![assignment("**", &[]); 257];
    assert_eq!(
        SourcePolicy::new(&p, None, Some("public"), "implementer")
            .err()
            .unwrap()
            .code,
        "INVALID_CONFIG"
    );
    p.config.policy.source_topics = vec![SourceTopic {
        scope: vec!["src/**".into(); 33],
        topics: vec![],
    }];
    assert!(SourcePolicy::new(&p, None, Some("public"), "implementer").is_err());
}
#[test]
fn expired_original_budget_and_noncanonical_paths_never_gain_authority() {
    let (_t, mut p) = fixture();
    p.deadline = Some(Deadline::from_instant(
        Instant::now() - Duration::from_secs(1),
    ));
    assert_eq!(
        SourcePolicy::new(&p, None, None, "implementer")
            .err()
            .unwrap()
            .code,
        "TIMEOUT"
    );
    assert_no_authority_side_effects(&p);
    p.deadline = None;
    let policy = SourcePolicy::new(&p, None, Some("public"), "implementer").unwrap();
    for path in [
        "../secret",
        "/absolute",
        "a/./b",
        "a//b",
        "C:/drive",
        "a\\b",
    ] {
        assert_eq!(
            policy.check(path, Some(&[])).unwrap_err().code,
            "PATH_OUTSIDE_ROOT"
        );
    }
}
fn write_config(p: &Project, config: &Config) {
    std::fs::create_dir_all(p.root.join(".pctx")).unwrap();
    std::fs::write(
        p.root.join(".pctx/config.toml"),
        toml::to_string(config).unwrap(),
    )
    .unwrap();
}
#[test]
fn captured_policy_cannot_authorize_changed_current_project_configuration() {
    let (_t, p) = fixture();
    let mut changed = p.config.clone();
    changed.policy.source_topics = vec![assignment("**", &["quiet"])];
    write_config(&p, &changed);
    let e = SourcePolicy::new(&p, None, Some("public"), "implementer")
        .err()
        .unwrap();
    assert_eq!(e.code, "CONCURRENT_MODIFICATION");
    assert_eq!(e.exit, 4);
    assert_no_authority_side_effects(&p);
}
#[test]
fn retained_policy_detects_configuration_deletion_creation_and_policy_changes() {
    let (_t, p) = fixture();
    write_config(&p, &p.config);
    let policy = SourcePolicy::new(&p, None, Some("public"), "implementer").unwrap();
    assert!(policy.revalidate().is_ok());
    let mut changed = p.config.clone();
    changed.index.max_file_bytes += 1;
    write_config(&p, &changed);
    assert!(
        policy.revalidate().is_ok(),
        "Unrelated index configuration is outside policy binding"
    );
    changed.policy.source_topics = vec![assignment("**", &["quiet"])];
    write_config(&p, &changed);
    assert_eq!(
        policy.revalidate().unwrap_err().code,
        "CONCURRENT_MODIFICATION"
    );
    std::fs::remove_file(p.root.join(".pctx/config.toml")).unwrap();
    assert_eq!(
        policy.revalidate().unwrap_err().code,
        "CONCURRENT_MODIFICATION"
    );
    let manual = SourcePolicy::new(&p, None, Some("public"), "implementer").unwrap();
    write_config(&p, &p.config);
    assert_eq!(
        manual.revalidate().unwrap_err().code,
        "CONCURRENT_MODIFICATION"
    );
}
#[cfg(unix)]
#[test]
fn linked_or_oversized_current_configuration_is_never_a_missing_fixture() {
    let (t, p) = fixture();
    std::fs::create_dir(p.root.join(".pctx")).unwrap();
    let config = p.root.join(".pctx/config.toml");
    std::os::unix::fs::symlink(t.path().join("absent"), &config).unwrap();
    assert_eq!(
        SourcePolicy::new(&p, None, Some("public"), "implementer")
            .err()
            .unwrap()
            .code,
        "POLICY_DENIED"
    );
    std::fs::remove_file(&config).unwrap();
    std::fs::write(&config, vec![b'x'; 1024 * 1024 + 1]).unwrap();
    assert_eq!(
        SourcePolicy::new(&p, None, Some("public"), "implementer")
            .err()
            .unwrap()
            .code,
        "INVALID_CONFIG"
    );
    assert_no_authority_side_effects(&p);
}

#[test]
fn initialized_registry_marker_prevents_missing_configuration_fixture_exception() {
    let (_t, p) = fixture();
    write_config(&p, &p.config);
    let policy = SourcePolicy::new(&p, None, Some("public"), "implementer").unwrap();
    std::fs::write(
        p.data_dir.join("registry.json"),
        b"{\"fixture_marker\":true}",
    )
    .unwrap();
    std::fs::remove_file(p.root.join(".pctx/config.toml")).unwrap();
    assert_eq!(
        policy.revalidate().unwrap_err().code,
        "CONCURRENT_MODIFICATION"
    );
    let e = SourcePolicy::new(&p, None, Some("public"), "implementer")
        .err()
        .unwrap();
    assert_eq!(e.code, "CONCURRENT_MODIFICATION");
    assert_eq!(e.exit, 4);
}

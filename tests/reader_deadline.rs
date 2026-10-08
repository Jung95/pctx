//! Cooperative request budgets; no claim of a hard bound on native filesystem I/O.
use pctx::{
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor},
    reader,
};
use std::time::Duration;
fn fixture() -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let project = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: temp.path().join("data"),
        workspace_dir: temp.path().join("data/workspace"),
        control_dir: temp.path().join("data/control"),
        project_id: "reader-deadline".into(),
        workspace_id: "workspace".into(),
        coordination_id: "coordination".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "reader-deadline".into(),
                name: "fixture".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    (temp, project)
}
fn expire(project: &mut Project) {
    project.deadline = Some(Deadline::from_millis(1).unwrap());
    std::thread::sleep(Duration::from_millis(5));
    assert!(project.check_deadline().is_err());
}
#[test]
fn expired_budget_is_timeout_for_read_inventory_manifest_and_path_authorization() {
    let (_temp, mut p) = fixture();
    std::fs::write(p.root.join("source.txt"), b"unchanged source").unwrap();
    expire(&mut p);
    assert_eq!(reader::read(&p, "source.txt").unwrap_err().code, "TIMEOUT");
    assert_eq!(reader::inventory(&p, false).unwrap_err().code, "TIMEOUT");
    assert_eq!(reader::manifest(&p).unwrap_err().code, "TIMEOUT");
    assert_eq!(
        reader::authorize(&p, "source.txt").unwrap_err().code,
        "TIMEOUT"
    );
    assert_eq!(
        std::fs::read(p.root.join("source.txt")).unwrap(),
        b"unchanged source"
    );
}
#[test]
fn cloned_project_keeps_same_expired_budget_instead_of_starting_a_new_phase() {
    let (_temp, mut p) = fixture();
    std::fs::write(p.root.join("source.txt"), b"one source").unwrap();
    expire(&mut p);
    let clone = p.clone();
    assert_eq!(
        clone.deadline.unwrap().instant(),
        p.deadline.unwrap().instant()
    );
    assert_eq!(
        reader::read(&clone, "source.txt").unwrap_err().code,
        "TIMEOUT"
    );
    assert_eq!(reader::inventory(&clone, true).unwrap_err().code, "TIMEOUT");
}
#[test]
fn bounded_chunks_keep_hash_utf8_and_source_policy_contracts() {
    let (_temp, mut p) = fixture();
    let text = format!("{}🚀\r\n한글", "x".repeat(65535));
    std::fs::write(p.root.join("source.txt"), &text).unwrap();
    p.deadline = Some(Deadline::from_millis(10000).unwrap());
    let file = reader::read(&p, "source.txt").unwrap();
    assert_eq!(file.hash, pctx::domain::hash(text.as_bytes()));
    assert_eq!(file.text, text);
    assert_eq!(reader::manifest(&p).unwrap()["source.txt"], file.hash);
    std::fs::write(p.root.join("bad.txt"), [0xff, 0xfe]).unwrap();
    assert_eq!(
        reader::read(&p, "bad.txt").unwrap_err().code,
        "UNSUPPORTED_ENCODING"
    );
    assert_eq!(std::fs::read(p.root.join("bad.txt")).unwrap(), [0xff, 0xfe]);
    std::fs::write(p.root.join(".env"), b"private input").unwrap();
    assert_eq!(reader::read(&p, ".env").unwrap_err().code, "POLICY_DENIED");
}
#[test]
fn expired_inventory_never_returns_empty_or_converts_timeout_to_skipped_metadata() {
    let (_temp, mut p) = fixture();
    std::fs::create_dir_all(p.root.join(".pctx/rules")).unwrap();
    std::fs::write(p.root.join(".pctx/rules/required.md"), b"required rule").unwrap();
    std::fs::write(p.root.join("live.txt"), b"current").unwrap();
    let before = reader::inventory(&p, false).unwrap();
    assert!(before.paths.iter().any(|path| path == "live.txt"));
    assert!(
        before
            .paths
            .iter()
            .any(|path| path == ".pctx/rules/required.md")
    );
    expire(&mut p);
    let error = reader::inventory(&p, false).unwrap_err();
    assert_eq!(error.code, "TIMEOUT");
    assert_eq!(error.exit, 7);
    p.deadline = None;
    assert_eq!(reader::inventory(&p, false).unwrap().paths, before.paths);
}

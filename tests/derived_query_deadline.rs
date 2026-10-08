//! Derived queries use one cooperative request budget, including nested refresh.
use fs2::FileExt;
use pctx::{
    deadline::Deadline,
    extract::{ExtractRequest, extract},
    graph::{GraphCommand, dependencies, execute},
    project::{Config, Project, ProjectConfig, RootAnchor},
    storage,
};
use std::{
    fs,
    time::{Duration, Instant},
};
fn fixture() -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir_all(&root).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspace"),
        control_dir: data.join("control"),
        project_id: "derived-deadline".into(),
        workspace_id: "workspace".into(),
        coordination_id: "coordination".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "derived-deadline".into(),
                name: "fixture".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    fs::create_dir_all(&p.workspace_dir).unwrap();
    fs::create_dir_all(&p.control_dir).unwrap();
    fs::write(
        p.root.join("a.ts"),
        "import { b } from './b';\nexport function a() { return b; }\n",
    )
    .unwrap();
    fs::write(p.root.join("b.ts"), "export const b = 7;\n").unwrap();
    (temp, p)
}
fn request() -> ExtractRequest {
    ExtractRequest {
        from_output: None,
        location: vec!["a.ts:2".into()],
        path: None,
        line: None,
        symbol_id: vec![],
        unit: "enclosing".into(),
        view: "full_span".into(),
        budget_bytes: 10_000,
    }
}
fn expired() -> Deadline {
    Deadline::from_instant(Instant::now().checked_sub(Duration::from_secs(1)).unwrap())
}
#[test]
fn expired_derived_entries_reject_before_index_or_source_changes() {
    let (_temp, mut p) = fixture();
    let original = fs::read(p.root.join("a.ts")).unwrap();
    p.deadline = Some(expired());
    let commands = [
        GraphCommand::Trace {
            path: "a.ts".into(),
            direction: "outgoing".into(),
            depth: 2,
        },
        GraphCommand::Impact {
            path: "a.ts".into(),
            depth: 2,
        },
        GraphCommand::Refs {
            path: "a.ts".into(),
        },
    ];
    for command in commands {
        let error = execute(&p, &command).unwrap_err();
        assert_eq!(error.code, "TIMEOUT");
        assert_eq!(error.exit, 7);
    }
    assert_eq!(
        dependencies(&p, "a.ts", 2, 1000).unwrap_err().code,
        "TIMEOUT"
    );
    assert_eq!(extract(&p, &request()).unwrap_err().code, "TIMEOUT");
    assert!(!p.index_db().exists());
    assert!(!p.workspace_dir.join("writer.lock").exists());
    assert_eq!(fs::read(p.root.join("a.ts")).unwrap(), original);
}
#[test]
fn graph_default_and_supplied_budget_preserve_edges_and_inferred_impact() {
    let (_temp, p) = fixture();
    storage::update(&p).unwrap();
    let default = dependencies(&p, "a.ts", 2, 1000).unwrap();
    let mut bounded = p.clone();
    bounded.deadline = Some(Deadline::from_millis(10_000).unwrap());
    let deadline = bounded.deadline.unwrap().instant();
    assert_eq!(dependencies(&bounded, "a.ts", 2, 1000).unwrap(), default);
    assert_eq!(default["edges"][0]["target"], "b.ts");
    assert_eq!(default["coverage"]["status"], "complete");
    let impact = execute(
        &bounded,
        &GraphCommand::Impact {
            path: "b.ts".into(),
            depth: 2,
        },
    )
    .unwrap();
    assert_eq!(impact["nodes"][0]["path"], "a.ts");
    assert_eq!(impact["nodes"][0]["evidence_status"], "inferred");
    assert_eq!(impact["test_exclusion_safe"], false);
    assert_eq!(bounded.deadline.unwrap().instant(), deadline);
    assert!(p.deadline.is_none());
}
#[test]
fn extract_default_and_supplied_budget_preserve_source_span() {
    let (_temp, p) = fixture();
    let default = extract(&p, &request()).unwrap();
    let mut bounded = p.clone();
    bounded.deadline = Some(Deadline::from_millis(10_000).unwrap());
    let deadline = bounded.deadline.unwrap().instant();
    assert_eq!(extract(&bounded, &request()).unwrap(), default);
    assert_eq!(default["items"][0]["range"]["start_line"], 2);
    assert_eq!(default["items"][0]["representation"], "full_span");
    assert!(
        default["items"][0]["content"]
            .as_str()
            .unwrap()
            .contains("return b")
    );
    assert_eq!(bounded.deadline.unwrap().instant(), deadline);
    assert!(p.deadline.is_none());
}
#[test]
fn nested_extract_refresh_cannot_restart_budget_while_writer_is_busy() {
    let (_temp, mut p) = fixture();
    storage::update(&p).unwrap();
    let before = storage::snapshot(&p).unwrap();
    let original = fs::read(p.root.join("a.ts")).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(p.workspace_dir.join("writer.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    p.deadline = Some(Deadline::from_millis(30).unwrap());
    let deadline = p.deadline.unwrap().instant();
    let start = Instant::now();
    let error = extract(&p, &request()).unwrap_err();
    assert_eq!(error.code, "TIMEOUT");
    assert_eq!(error.exit, 7);
    // This bounds cooperative lock polling, not an arbitrary native filesystem call.
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(p.deadline.unwrap().instant(), deadline);
    FileExt::unlock(&lock).unwrap();
    p.deadline = None;
    assert_eq!(storage::snapshot(&p).unwrap().0, before.0);
    assert_eq!(fs::read(p.root.join("a.ts")).unwrap(), original);
}

use pctx::{
    deadline::Deadline,
    filters::{self, FilterCommand, FilterRecord},
    project::{Config, Project, ProjectConfig, RootAnchor},
};
use std::{fs, path::PathBuf, time::Instant};
fn fixture() -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("project");
    fs::create_dir_all(root.join(".pctx/filters")).unwrap();
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
    (temp, p)
}
#[test]
fn expired_public_filter_queries_and_preview_have_no_effects() {
    let (_temp, mut p) = fixture();
    let original = Deadline::from_instant(Instant::now());
    p.deadline = Some(original);
    for command in [
        FilterCommand::Validate {
            path: PathBuf::from(".pctx/filters/missing.toml"),
        },
        FilterCommand::Apply {
            filter: "missing".into(),
            input: PathBuf::from("-"),
            child_exit: 17,
        },
        FilterCommand::Explain {
            argv: vec!["missing".into()],
        },
    ] {
        let e = filters::execute(&p, &command).unwrap_err();
        assert_eq!(e.code, "TIMEOUT");
        assert_eq!(e.exit, 7);
    }
    let e = filters::preview_records(&p, "missing", &[], 17, "exited").unwrap_err();
    assert_eq!(e.code, "TIMEOUT");
    assert_eq!(e.exit, 7);
    assert_eq!(p.deadline.unwrap().instant(), original.instant());
    assert!(!p.data_dir.exists());
    assert!(!p.index_db().exists());
    assert!(!p.control_db().exists());
}
#[test]
fn default_public_preview_keeps_unicode_negative_status_and_no_evidence() {
    let (_temp, p) = fixture();
    fs::write(
        p.root.join(".pctx/filters/fixture.toml"),
        r#"schema_version=1
id="fixture"
version="1"
priority=0
rules=[]
[match]
program="not-executed"
argv_prefix=[]
stream="both"
[parse]
kind="lines"
[render]
max_bytes=8192
keep_head_lines=10
keep_tail_lines=10
show_omission_counts=true
"#,
    )
    .unwrap();
    let records = vec![FilterRecord {
        stream: "stderr".into(),
        sequence: 0,
        text: "warning café 경고".into(),
        start_byte: 0,
        end_byte: 23,
    }];
    let value = filters::preview_records(&p, "fixture", &records, 17, "exited").unwrap();
    assert_eq!(value["child_exit_code"], 17);
    assert_eq!(value["records"][0]["text"], records[0].text);
    assert_eq!(value["test_result"], "not_evaluated");
    assert_eq!(value["execution_started"], false);
    assert_eq!(value["command_rerun"], false);
    assert!(p.deadline.is_none());
    assert!(!p.data_dir.exists());
}

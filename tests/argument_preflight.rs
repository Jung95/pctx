use pctx::{
    context::{self, BuildRequest},
    extract::{self, ExtractRequest},
    graph::{self, GraphCommand},
    project::{Config, Project, ProjectConfig, RootAnchor},
    storage,
};
use std::{collections::BTreeMap, fs, path::Path};
fn extract_request() -> ExtractRequest {
    ExtractRequest {
        from_output: None,
        location: vec!["code.rs:1".into()],
        path: None,
        line: None,
        symbol_id: vec![],
        unit: "enclosing".into(),
        view: "full_span".into(),
        budget_bytes: 10000,
    }
}
fn build_request() -> BuildRequest {
    BuildRequest {
        task: Some("inspect code".into()),
        task_file: None,
        task_id: None,
        session: None,
        topic: None,
        seed: vec![],
        budget_bytes: 12000,
        budget_tokens: None,
        tokenizer: None,
        handoff: None,
        role: "implementer".into(),
        detail: "adaptive".into(),
        changed_since: None,
        dependency_depth: 0,
        explain: false,
        require_complete: false,
    }
}
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
    fs::create_dir_all(&p.workspace_dir).unwrap();
    fs::create_dir_all(&p.control_dir).unwrap();
    fs::write(p.root.join("code.rs"), "fn before() {}\n").unwrap();
    (t, p)
}
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(base, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
#[test]
fn extract_static_location_errors_leave_dirty_index_and_sources_untouched() {
    let (t, p) = fixture();
    storage::update(&p).unwrap();
    fs::write(p.root.join("code.rs"), "fn changed_after_index() {}\n").unwrap();
    let before = snapshot(t.path());
    let invalid = [
        ExtractRequest {
            location: vec!["code.rs".into()],
            ..extract_request()
        },
        ExtractRequest {
            location: vec!["code.rs:0".into()],
            ..extract_request()
        },
        ExtractRequest {
            location: vec!["code.rs:not-a-line".into()],
            ..extract_request()
        },
        ExtractRequest {
            location: vec![],
            path: Some("code.rs".into()),
            ..extract_request()
        },
        ExtractRequest {
            location: vec![],
            line: Some(1),
            ..extract_request()
        },
        ExtractRequest {
            location: vec![],
            ..extract_request()
        },
        ExtractRequest {
            location: vec!["../outside.rs:1".into()],
            ..extract_request()
        },
    ];
    for r in invalid {
        let e = extract::extract(&p, &r).unwrap_err();
        assert!(matches!(
            e.code.as_str(),
            "INVALID_ARGUMENT" | "PATH_OUTSIDE_ROOT"
        ));
        assert_eq!(snapshot(t.path()), before);
    }
}
#[test]
fn extract_preflight_keeps_multisource_and_artifact_only_requests_dynamic() {
    let r = ExtractRequest {
        from_output: Some("OUT-fixture".into()),
        location: vec!["src:name.rs:2".into(), "code.rs:1".into()],
        path: Some("other.rs".into()),
        line: Some(3),
        symbol_id: vec!["SYM-fixture".into()],
        ..extract_request()
    };
    extract::validate_request(&r).unwrap();
    extract::validate_request(&ExtractRequest {
        from_output: Some("OUT-fixture".into()),
        location: vec![],
        ..extract_request()
    })
    .unwrap();
    let e = extract::validate_request(&ExtractRequest {
        budget_bytes: 511,
        ..extract_request()
    })
    .unwrap_err();
    assert_eq!(e.code, "BUDGET_TOO_SMALL");
    assert_eq!(e.exit, 8);
    for r in [
        ExtractRequest {
            unit: "unknown".into(),
            ..extract_request()
        },
        ExtractRequest {
            view: "unknown".into(),
            ..extract_request()
        },
    ] {
        assert_eq!(
            extract::validate_request(&r).unwrap_err().code,
            "INVALID_ARGUMENT"
        );
    }
}
#[test]
fn context_pure_contract_preserves_error_codes_and_task_source_exclusivity() {
    context::validate_build_request(&build_request()).unwrap();
    for r in [
        BuildRequest {
            budget_tokens: Some(1),
            ..build_request()
        },
        BuildRequest {
            tokenizer: Some("unknown".into()),
            ..build_request()
        },
    ] {
        let e = context::validate_build_request(&r).unwrap_err();
        assert_eq!(e.code, "CAPABILITY_UNAVAILABLE");
        assert_eq!(e.exit, 6);
    }
    let e = context::validate_build_request(&BuildRequest {
        budget_bytes: 511,
        ..build_request()
    })
    .unwrap_err();
    assert_eq!(e.code, "BUDGET_TOO_SMALL");
    assert_eq!(e.exit, 8);
    for r in [
        BuildRequest {
            dependency_depth: 3,
            ..build_request()
        },
        BuildRequest {
            detail: "unknown".into(),
            ..build_request()
        },
        BuildRequest {
            task_file: Some("-".into()),
            ..build_request()
        },
        BuildRequest {
            task_id: Some("T-fixture".into()),
            ..build_request()
        },
        BuildRequest {
            task: None,
            task_file: Some("-".into()),
            task_id: Some("T-fixture".into()),
            ..build_request()
        },
        BuildRequest {
            task: None,
            ..build_request()
        },
    ] {
        let e = context::validate_build_request(&r).unwrap_err();
        assert_eq!(e.code, "INVALID_ARGUMENT");
        assert_eq!(e.exit, 2);
    }
    context::validate_build_request(&BuildRequest {
        task: None,
        handoff: Some("fixture".into()),
        dependency_depth: 2,
        detail: "reference".into(),
        budget_bytes: 512,
        ..build_request()
    })
    .unwrap();
}
#[test]
fn invalid_context_does_not_wait_for_task_stdin_or_touch_storage() {
    let (t, p) = fixture();
    let before = snapshot(t.path());
    let r = BuildRequest {
        task: None,
        task_file: Some("-".into()),
        dependency_depth: 3,
        ..build_request()
    };
    let e = context::build(&p, &r).unwrap_err();
    assert_eq!(e.code, "INVALID_ARGUMENT");
    assert_eq!(snapshot(t.path()), before);
    assert!(!p.control_db().exists());
    assert!(!p.index_db().exists());
}
#[test]
fn graph_existing_depth_and_direction_limits_precede_path_authority() {
    let (t, p) = fixture();
    let before = snapshot(t.path());
    for c in [
        GraphCommand::Trace {
            path: "missing.ts".into(),
            direction: "unknown".into(),
            depth: 2,
        },
        GraphCommand::Trace {
            path: "missing.ts".into(),
            direction: "outgoing".into(),
            depth: 33,
        },
        GraphCommand::Impact {
            path: "missing.ts".into(),
            depth: 33,
        },
    ] {
        let e = graph::execute(&p, &c).unwrap_err();
        assert_eq!(e.code, "INVALID_ARGUMENT");
        assert_eq!(e.exit, 2);
        assert_eq!(snapshot(t.path()), before);
    }
    for depth in [0, 32] {
        graph::validate_request(&GraphCommand::Trace {
            path: "missing.ts".into(),
            direction: "incoming".into(),
            depth,
        })
        .unwrap();
    }
    graph::validate_request(&GraphCommand::Refs {
        path: "missing.ts".into(),
    })
    .unwrap();
    for (max_nodes, depth) in [(0, 2), (1001, 2), (1, 33)] {
        assert_eq!(
            graph::dependencies(&p, "missing.ts", depth, max_nodes)
                .unwrap_err()
                .code,
            "INVALID_ARGUMENT"
        );
    }
    assert_eq!(snapshot(t.path()), before);
}

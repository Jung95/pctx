use pctx::{
    graph::{GraphCommand, dependencies, execute},
    project::{Config, Project, ProjectConfig},
    storage,
};
use serde_json::json;
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("project");
    std::fs::create_dir_all(&root).unwrap();
    let p = Project {
        root,
        data_dir: t.path().join("data"),
        workspace_dir: t.path().join("data/ws"),
        control_dir: t.path().join("data/control"),
        project_id: "project".into(),
        workspace_id: "ws".into(),
        coordination_id: "coord".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "project".into(),
                name: "test".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    std::fs::create_dir_all(&p.workspace_dir).unwrap();
    std::fs::create_dir_all(&p.control_dir).unwrap();
    (t, p)
}
fn write(p: &Project, path: &str, text: &str) {
    let file = p.root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}
#[test]
fn import_cycles_have_bounded_traversal_and_reverse_impact_is_inferred() {
    let (_t, p) = fixture();
    write(&p, "a.ts", "import { b } from './b'; export const a = 1;");
    write(&p, "b.ts", "import { a } from './a'; export const b = a;");
    write(&p, "tests/auth.test.ts", "import { a } from '../a';");
    storage::update(&p).unwrap();
    let outgoing = dependencies(&p, "a.ts", 10, 1000).unwrap();
    assert_eq!(outgoing["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(outgoing["visited_count"], 2);
    let impact = execute(
        &p,
        &GraphCommand::Impact {
            path: "a.ts".into(),
            depth: 2,
        },
    )
    .unwrap();
    assert!(
        impact["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["path"] == "tests/auth.test.ts")
    );
    for n in impact["nodes"].as_array().unwrap() {
        assert_eq!(n["status"], "potentially_affected");
        assert_eq!(n["evidence_status"], "inferred");
    }
    assert_eq!(impact["test_exclusion_safe"], false);
    let bounded = dependencies(&p, "a.ts", 10, 1).unwrap();
    assert_eq!(bounded["truncated"], true);
    assert_eq!(bounded["visited_count"], 1);
}
#[test]
fn alias_configuration_changes_invalidate_without_index_refresh() {
    let (_t, p) = fixture();
    write(
        &p,
        "tsconfig.json",
        &json!({"compilerOptions":{"baseUrl":".","paths":{"@auth":["src/a.ts"]}}}).to_string(),
    );
    write(&p, "entry.ts", "import { auth } from '@auth';");
    write(&p, "src/a.ts", "export const auth = 1;");
    write(&p, "src/b.ts", "export const auth = 2;");
    storage::update(&p).unwrap();
    let old = dependencies(&p, "entry.ts", 1, 1000).unwrap();
    assert_eq!(old["edges"][0]["target"], "src/a.ts");
    write(
        &p,
        "tsconfig.json",
        &json!({"compilerOptions":{"baseUrl":".","paths":{"@auth":["src/b.ts"]}}}).to_string(),
    );
    let new = dependencies(&p, "entry.ts", 1, 1000).unwrap();
    assert_eq!(new["edges"][0]["target"], "src/b.ts");
    assert_ne!(old["resolver_fingerprint"], new["resolver_fingerprint"]);
    assert!(
        new["edges"][0]["config_hashes"]
            .get("tsconfig.json")
            .is_some()
    );
}
#[test]
fn workspace_exports_and_dynamic_import_are_explicit() {
    let (_t, p) = fixture();
    write(&p,"packages/auth/package.json",&json!({"name":"@workspace/auth","exports":{".":"./src/index.ts","./*":"./src/*.ts","./conditional":{"import":"./src/index.ts","require":"./src/other.ts"}}}).to_string());
    write(&p, "packages/auth/src/index.ts", "export const auth=1;");
    write(&p, "packages/auth/src/login.ts", "export const login=1;");
    write(
        &p,
        "entry.ts",
        "import { auth } from '@workspace/auth';\nexport { login } from '@workspace/auth/login';\nimport '@workspace/auth/conditional';\nconst load = import(name);\n",
    );
    storage::update(&p).unwrap();
    let g = dependencies(&p, "entry.ts", 2, 1000).unwrap();
    let edges = g["edges"].as_array().unwrap();
    assert!(
        edges
            .iter()
            .any(|e| e["target"] == "packages/auth/src/index.ts")
    );
    assert!(
        edges
            .iter()
            .any(|e| e["target"] == "packages/auth/src/login.ts")
    );
    assert!(
        edges
            .iter()
            .any(|e| e["reason"] == "conditional_package_export_unsupported")
    );
    assert!(edges.iter().any(|e| e["reason"] == "dynamic_import"));
    assert_eq!(g["coverage"]["status"], "partial");
}
#[test]
fn workspaces_and_policy_are_never_cross_used() {
    let (_ta, pa) = fixture();
    let (_tb, mut pb) = fixture();
    pb.workspace_id = "other-ws".into();
    write(&pa, "entry.ts", "import './a';");
    write(&pa, "a.ts", "export const a=1;");
    write(&pb, "entry.ts", "import './b';");
    write(&pb, "b.ts", "export const b=1;");
    storage::update(&pa).unwrap();
    storage::update(&pb).unwrap();
    let a = dependencies(&pa, "entry.ts", 1, 1000).unwrap();
    let b = dependencies(&pb, "entry.ts", 1, 1000).unwrap();
    assert_eq!(a["nodes"][0]["path"], "a.ts");
    assert_eq!(b["nodes"][0]["path"], "b.ts");
    assert_ne!(a["resolver_fingerprint"], b["resolver_fingerprint"]);
    pb.config.policy.exclude.push("b.ts".into());
    let excluded = dependencies(&pb, "entry.ts", 1, 1000).unwrap();
    assert!(excluded["nodes"].as_array().unwrap().is_empty());
    assert!(excluded["edges"][0]["target"].is_null());
}

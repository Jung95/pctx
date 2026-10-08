use pctx::{
    domain::hash,
    inventory,
    project::{Config, Project, ProjectConfig},
};
use serde_json::{Value, json};
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    let p = Project {
        root: t.path().join("project"),
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
    std::fs::create_dir_all(&p.root).unwrap();
    (t, p)
}
fn write(p: &Project, path: &str, text: &str) {
    let target = p.root.join(path);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(target, text).unwrap();
}
fn manifest(p: &Project) {
    write(p,"package.json",&json!({"name":"root","workspaces":["packages/*"],"engines":{"node":">=24"},"scripts":{"test":"vitest run","danger":"touch EXECUTED"}}).to_string());
    write(
        p,
        "packages/core/package.json",
        &json!({"name":"@local/core","version":"1.0.0"}).to_string(),
    );
    write(p,"packages/web/package.json",&json!({"name":"@local/web","dependencies":{"@local/core":"workspace:*"},"scripts":{"lint":"eslint ."}}).to_string());
    write(
        p,
        "Cargo.toml",
        "[package]\nname = 'fixture'\nversion = '0.1.0'\nedition = '2024'\n",
    );
    write(
        p,
        "src/a.ts",
        "export function login() { return 'not returned'; }\n",
    );
}
#[test]
fn static_workspace_packages_and_checks_have_current_proof_without_execution() {
    let (_t, p) = fixture();
    manifest(&p);
    write(
        &p,
        ".github/workflows/test.yml",
        "name: Validate\njobs:\n  unit:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm test\n",
    );
    let result = inventory::scan(&p, None, 100, 1_000_000).unwrap();
    assert_eq!(result["scripts_executed"], false);
    assert!(!p.root.join("EXECUTED").exists());
    let packages = result["packages"].as_array().unwrap();
    assert!(packages.iter().any(|p| p["name"] == "@local/core"));
    assert_eq!(
        result["reverse_dependency_candidates"]["@local/core"],
        json!(["packages/web/package.json"])
    );
    assert!(
        result["workspace_declarations"][0]["members"]
            .as_array()
            .unwrap()
            .contains(&json!("packages/core/package.json"))
    );
    let checks = result["check_candidates"].as_array().unwrap();
    assert!(
        checks.iter().any(
            |c| c["parser_candidate"] == "vitest" && c["resource_candidate"] == "heavy_compute"
        )
    );
    assert!(
        checks
            .iter()
            .all(|c| c["executed"] == false && c["gate_checks_omission_authorized"] == false)
    );
    assert_eq!(
        result["sources"]["package.json"],
        hash(std::fs::read(p.root.join("package.json")).unwrap())
    );
    let text = result.to_string();
    assert!(!text.contains("not returned"));
}
#[test]
fn dynamic_invalid_and_unsupported_configs_are_explicit() {
    let (_t, p) = fixture();
    write(&p, "package.json", "{not json}");
    write(
        &p,
        "app.config.ts",
        "require('fs').writeFileSync('EXECUTED','x'); export default discover();",
    );
    write(
        &p,
        "tsconfig.json",
        "{\"extends\":\"./other.json\",\"compilerOptions\":{\"paths\":{\"@/*\":[\"src/*\"]}}}",
    );
    write(
        &p,
        "pnpm-workspace.yaml",
        "packages:\n - 'packages/*'\n - '!packages/excluded'\n",
    );
    let result = inventory::scan(&p, None, 100, 1_000_000).unwrap();
    assert_eq!(result["coverage"]["status"], "partial");
    let reasons = result["coverage"]["reasons"].to_string();
    for reason in [
        "UNSUPPORTED_MANIFEST",
        "dynamic_config_not_executed",
        "config_inheritance_not_resolved",
        "workspace_exclusion_requires_resolution",
    ] {
        assert!(reasons.contains(reason));
    }
    assert!(!p.root.join("EXECUTED").exists());
}
#[test]
fn profile_confirms_observation_not_owner_authority_and_detects_conflict_and_stale() {
    let (_t, p) = fixture();
    manifest(&p);
    write(&p, "AGENTS.md", "# Worker\nDeveloper scope: packages/web\n");
    let evidence = hash(std::fs::read(p.root.join("AGENTS.md")).unwrap());
    let make = |expected: &str| json!({"schema_version":1,"id":"local","expectations":[{"source":"package.json","selector":"/engines/node","expected":expected,"confirmation":{"source":"AGENTS.md","hash":evidence,"authority":"owner-provided"}}],"roles":[{"id":"developer","scope":["packages/web/**"],"confirmation":{"source":"AGENTS.md","hash":evidence,"authority":"owner-provided"}}]});
    write(&p, ".pctx/profile.json", &make(">=24").to_string());
    let confirmed = inventory::profile(&p, ".pctx/profile.json").unwrap();
    assert_eq!(confirmed["status"], "confirmed");
    assert_eq!(confirmed["operation_authorized"], false);
    assert_eq!(confirmed["owner_identity_verified"], false);
    assert_eq!(
        confirmed["expectations"][0]["confirmation"]["owner_authority_verified"],
        false
    );
    write(&p, ".pctx/profile.json", &make(">=22").to_string());
    assert_eq!(
        inventory::profile(&p, ".pctx/profile.json").unwrap()["status"],
        "conflicting"
    );
    write(&p, "AGENTS.md", "# Changed rule\n");
    write(&p, ".pctx/profile.json", &make(">=24").to_string());
    assert_eq!(
        inventory::profile(&p, ".pctx/profile.json").unwrap()["status"],
        "unconfirmed"
    );
}
fn registry(p: &Project) -> Value {
    let doc = "# Runtime\nRequires Node >=22.\n\n# Other\nUnrelated text.\n";
    write(p, "README.md", doc);
    json!({"schema_version":1,"since_event":"event-1","source_refs":[{"document":"README.md","section":"Runtime","source":"package.json","selector":"/engines/node","expected":">=22","rendered":"Requires Node >=22.","template":"Requires Node {value}.","source_hash":"previous-source-hash","document_hash":hash(doc),"decision_id":"DEC-runtime"}],"known_proposal_ids":[]})
}
#[test]
fn audit_proposals_are_deterministic_hash_bound_and_do_not_write() {
    let (_t, p) = fixture();
    manifest(&p);
    let r = registry(&p);
    write(&p, ".pctx/docs.json", &r.to_string());
    let before = std::fs::read(p.root.join("README.md")).unwrap();
    let audit = inventory::audit(&p, ".pctx/docs.json", Some("event-1")).unwrap();
    assert_eq!(audit["proposals"].as_array().unwrap().len(), 1);
    let proposal = &audit["proposals"][0];
    assert!(
        proposal["proposal_id"]
            .as_str()
            .unwrap()
            .starts_with("DOC-")
    );
    assert_eq!(proposal["before"], "Requires Node >=22.");
    assert_eq!(proposal["after"], "Requires Node >=24.");
    assert_eq!(proposal["task_created"], false);
    assert_eq!(proposal["proof"]["document"]["hash"], hash(&before));
    assert_eq!(
        inventory::audit(&p, ".pctx/docs.json", Some("event-1")).unwrap(),
        audit
    );
    assert_eq!(std::fs::read(p.root.join("README.md")).unwrap(), before);
    assert!(!p.control_dir.exists());
    assert!(inventory::audit(&p, ".pctx/docs.json", Some("event-2")).is_err());
    let mut changed: Value =
        serde_json::from_slice(&std::fs::read(p.root.join("package.json")).unwrap()).unwrap();
    changed["engines"]["node"] = json!(">=26");
    write(&p, "package.json", &changed.to_string());
    let next = inventory::audit(&p, ".pctx/docs.json", None).unwrap();
    assert_ne!(next["proposals"][0]["proposal_id"], proposal["proposal_id"]);
}
#[test]
fn stale_documents_and_changed_source_only_need_review() {
    let (_t, p) = fixture();
    manifest(&p);
    let r = registry(&p);
    write(&p, ".pctx/docs.json", &r.to_string());
    write(
        &p,
        "README.md",
        "# Runtime\nThe owner edited this sentence.\n",
    );
    let stale = inventory::audit(&p, ".pctx/docs.json", None).unwrap();
    assert!(stale["proposals"].as_array().unwrap().is_empty());
    assert!(
        stale["review_needed"]
            .to_string()
            .contains("document_baseline_stale")
    );
    let mut current = r;
    current["source_refs"][0]["expected"] = json!(">=24");
    write(&p, ".pctx/docs.json", &current.to_string());
    let matching = inventory::audit(&p, ".pctx/docs.json", None).unwrap();
    assert!(matching["proposals"].as_array().unwrap().is_empty());
    assert!(
        matching["review_needed"]
            .to_string()
            .contains("source_hash_changed_claim_still_matches")
    );
}
#[test]
fn exclusions_and_redaction_apply_to_scan_and_current_audit() {
    let (_t, mut p) = fixture();
    manifest(&p);
    write(&p, "excluded/package.json", "{\"name\":\"hidden-package\"}");
    write(
        &p,
        "secret.config.js",
        "const password = 'supersecretpassword';",
    );
    write(&p,"package.json",&json!({"name":"root","scripts":{"secret":"curl https://user:password@host -H ghp_12345678901234567890"}}).to_string());
    p.config.policy.exclude.push("excluded/**".into());
    let result = inventory::scan(&p, None, 100, 1_000_000)
        .unwrap()
        .to_string();
    assert!(!result.contains("hidden-package"));
    assert!(!result.contains("ghp_12345678901234567890"));
    assert!(!result.contains("user:password@"));
    let r = registry(&p);
    write(&p, ".pctx/docs.json", &r.to_string());
    p.config.policy.exclude.push("package.json".into());
    assert!(inventory::audit(&p, ".pctx/docs.json", None).is_err());
}
#[cfg(unix)]
#[test]
fn symlinks_and_budgets_are_not_traversed() {
    let (t, p) = fixture();
    let outside = t.path().join("outside.json");
    std::fs::write(&outside, "{\"name\":\"outside-package\"}").unwrap();
    std::os::unix::fs::symlink(&outside, p.root.join("package.json")).unwrap();
    let result = inventory::scan(&p, None, 10, 100).unwrap();
    assert!(!result.to_string().contains("outside-package"));
    assert!(inventory::profile(&p, "package.json").is_err());
    std::fs::remove_file(p.root.join("package.json")).unwrap();
    write(&p, "package.json", "{\"name\":\"large\"}");
    let small = inventory::scan(&p, None, 1, 1).unwrap();
    assert_eq!(small["read_bytes"], 0);
    assert!(
        small["coverage"]["reasons"]
            .to_string()
            .contains("byte_bound")
    );
}
#[test]
fn unknown_selectors_and_code_structure_do_not_invent_facts() {
    let (_t, p) = fixture();
    manifest(&p);
    let mut r = registry(&p);
    r["source_refs"][0]["selector"] = json!("runtime-evaluate");
    write(&p, ".pctx/docs.json", &r.to_string());
    let audit = inventory::audit(&p, ".pctx/docs.json", None).unwrap();
    assert!(audit["proposals"].as_array().unwrap().is_empty());
    assert!(
        audit["review_needed"]
            .to_string()
            .contains("CAPABILITY_UNVERIFIED")
    );
    assert!(
        audit["changed_sources"]
            .as_array()
            .unwrap()
            .contains(&json!("package.json"))
    );
    let evidence = hash(std::fs::read(p.root.join("src/a.ts")).unwrap());
    let symbols = json!([{"name":"login","qualified_name":"login","kind":"function"}]);
    write(&p,".pctx/profile.json",&json!({"schema_version":1,"id":"syntax","expectations":[{"source":"src/a.ts","selector":"$symbol/login","expected":symbols,"confirmation":{"source":"src/a.ts","hash":evidence,"authority":"owner-provided"}}]}).to_string());
    let observed = inventory::profile(&p, ".pctx/profile.json").unwrap();
    assert_eq!(observed["expectations"][0]["status"], "confirmed");
    write(
        &p,
        "src/a.ts",
        "// export function login exists only in a comment\nconst text = 'login';\n",
    );
    let removed = inventory::profile(&p, ".pctx/profile.json").unwrap();
    assert_eq!(removed["expectations"][0]["status"], "conflicting");
    assert_eq!(removed["expectations"][0]["observed"], json!([]));
}

use pctx::{
    pack::{PackCommand, execute},
    project::{Config, Project, ProjectConfig},
    work::{self, TaskCommand, WorkCommand},
};
use serde_json::{Value, json};
use std::path::PathBuf;
fn fixture() -> (tempfile::TempDir, Project, String) {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(t.path().join("project")).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: pctx::project::RootAnchor::capture(&t.path().join("project")).unwrap(),
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
    std::fs::create_dir_all(p.root.join("src")).unwrap();
    std::fs::create_dir_all(&p.control_dir).unwrap();
    std::fs::create_dir_all(&p.workspace_dir).unwrap();
    let task_json = p.root.join("task.json");
    std::fs::write(&task_json,json!({"schema_version":1,"title":"Review auth","scope":["src/**"],"acceptance":[{"id":"AC1","description":"Review complete","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
    let task = work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Create {
                from_file: task_json,
                idempotency_key: None,
            },
        },
    )
    .unwrap()["task_id"]
        .as_str()
        .unwrap()
        .into();
    (t, p, task)
}
fn plan(p: &Project, task: &str, content: &str, split_bytes: Option<usize>) -> Value {
    execute(
        p,
        &PackCommand::Plan {
            task_id: task.into(),
            scopes: vec!["src".into()],
            content: content.into(),
            budget_bytes: 64000,
            split_bytes,
        },
    )
    .unwrap()
}
fn create(p: &Project, plan: &Value, path: &str) -> pctx::domain::Result<Value> {
    execute(
        p,
        &PackCommand::Create {
            plan: plan["plan_id"].as_str().unwrap().into(),
            expect_hash: plan["plan_hash"].as_str().unwrap().into(),
            output: PathBuf::from(path),
        },
    )
}
fn verify(p: &Project, path: &str, current: bool) -> pctx::domain::Result<Value> {
    execute(
        p,
        &PackCommand::Verify {
            path: path.into(),
            against: current.then(|| "current".into()),
        },
    )
}
#[test]
fn immutable_plan_has_no_source_metadata_default_and_full_masks_preserves_license() {
    let (_t, p, task) = fixture();
    let secret = format!("sk-{}", "syntheticcredentialvalue");
    let source = format!(
        "// Copyright Example; MIT License\nexport function auth() {{ return '{secret}'; }}\n"
    );
    std::fs::write(p.root.join("src/auth.ts"), &source).unwrap();
    let metadata = plan(&p, &task, "metadata", None);
    let stored = std::fs::read_to_string(
        p.control_dir
            .join("pack-plans")
            .join(format!("{}.json", metadata["plan_id"].as_str().unwrap())),
    )
    .unwrap();
    assert!(!stored.contains("export function"));
    assert!(!stored.contains(&secret));
    create(&p, &metadata, "artifacts/metadata").unwrap();
    let body = std::fs::read_to_string(p.root.join("artifacts/metadata/context.json")).unwrap();
    assert!(!body.contains("export function"));
    let full = plan(&p, &task, "full", None);
    let receipt = create(&p, &full, "artifacts/full").unwrap();
    let body = std::fs::read_to_string(p.root.join("artifacts/full/context.json")).unwrap();
    assert!(body.contains("MIT License"));
    assert!(body.contains("[REDACTED]"));
    assert!(!body.contains(&secret));
    assert!(!body.contains(p.root.to_str().unwrap()));
    let actual: usize = std::fs::read_dir(p.root.join("artifacts/full"))
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().len() as usize)
        .sum();
    assert_eq!(receipt["actual_bytes"].as_u64(), Some(actual as u64));
    assert!(actual <= 64000);
    assert_eq!(
        verify(&p, "artifacts/full", true).unwrap()["freshness"],
        "current"
    );
}
#[test]
fn stale_source_policy_task_and_wrong_expected_hash_do_not_publish() {
    let (_t, mut p, task) = fixture();
    std::fs::write(p.root.join("src/auth.ts"), "export const auth=1;\n").unwrap();
    let a = plan(&p, &task, "full", None);
    std::fs::write(p.root.join("src/auth.ts"), "export const auth=2;\n").unwrap();
    assert_eq!(
        create(&p, &a, "artifacts/stale").unwrap_err().code,
        "PACK_PLAN_STALE"
    );
    assert!(!p.root.join("artifacts/stale").exists());
    let b = plan(&p, &task, "metadata", None);
    p.config.policy.exclude.push("src/auth.ts".into());
    assert_eq!(
        create(&p, &b, "artifacts/policy").unwrap_err().code,
        "PACK_PLAN_STALE"
    );
    p.config.policy.exclude.clear();
    let wrong = execute(
        &p,
        &PackCommand::Create {
            plan: b["plan_id"].as_str().unwrap().into(),
            expect_hash: "wrong".into(),
            output: "artifacts/wrong".into(),
        },
    );
    assert_eq!(wrong.unwrap_err().code, "PACK_PLAN_STALE");
    work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Cancel {
                task: task.clone(),
                reason: "cancelled scope".into(),
            },
        },
    )
    .unwrap();
    assert_eq!(
        create(&p, &b, "artifacts/task").unwrap_err().code,
        "PACK_PLAN_STALE"
    );
}
#[test]
fn integrity_tamper_missing_parts_and_missing_original_are_distinct() {
    let (_t, p, task) = fixture();
    for n in 0..3 {
        std::fs::write(
            p.root.join(format!("src/f{n}.ts")),
            format!("export const f{n}=1;\n"),
        )
        .unwrap();
    }
    let a = plan(&p, &task, "full", Some(1500));
    create(&p, &a, "artifacts/split").unwrap();
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(p.root.join("artifacts/split/manifest.json")).unwrap(),
    )
    .unwrap();
    let part = manifest["parts"][0]["path"].as_str().unwrap();
    let partpath = p.root.join("artifacts/split").join(part);
    let original = std::fs::read(&partpath).unwrap();
    std::fs::write(&partpath, b"{}").unwrap();
    assert_eq!(
        verify(&p, "artifacts/split", false).unwrap_err().code,
        "PACK_INTEGRITY_FAILED"
    );
    std::fs::write(&partpath, &original).unwrap();
    std::fs::remove_file(&partpath).unwrap();
    assert_eq!(
        verify(&p, "artifacts/split", false).unwrap_err().code,
        "PACK_INTEGRITY_FAILED"
    );
    std::fs::write(&partpath, &original).unwrap();
    for n in 0..3 {
        std::fs::remove_file(p.root.join(format!("src/f{n}.ts"))).unwrap();
    }
    let result = verify(&p, "artifacts/split", true).unwrap();
    assert_eq!(result["integrity"], "verified");
    assert_eq!(result["freshness"], "unknown");
    assert_eq!(result["acknowledged"], false);
}
#[test]
fn giant_item_and_total_budget_fail_without_plans_or_publication() {
    let (_t, p, task) = fixture();
    std::fs::write(
        p.root.join("src/large.ts"),
        format!(
            "export function auth() {{\n{}\n}}",
            "let x=1;\n".repeat(1000)
        ),
    )
    .unwrap();
    let result = execute(
        &p,
        &PackCommand::Plan {
            task_id: task.clone(),
            scopes: vec!["src".into()],
            content: "full".into(),
            budget_bytes: 64000,
            split_bytes: Some(1200),
        },
    );
    assert_eq!(result.unwrap_err().code, "BUDGET_TOO_SMALL");
    let small = execute(
        &p,
        &PackCommand::Plan {
            task_id: task,
            scopes: vec!["src".into()],
            content: "full".into(),
            budget_bytes: 2000,
            split_bytes: None,
        },
    );
    assert_eq!(small.unwrap_err().code, "BUDGET_TOO_SMALL");
    assert!(!p.control_dir.join("pack-plans").exists());
}
#[test]
fn signatures_keep_notices_and_single_json_verifies() {
    let (_t, p, task) = fixture();
    std::fs::write(p.root.join("src/auth.ts"),"// MIT License\n/** SECURITY: authenticate first. */\nexport function auth(token: string) {\n return token;\n}\n").unwrap();
    let a = plan(&p, &task, "signatures", None);
    create(&p, &a, "artifacts/signatures.json").unwrap();
    let bytes = std::fs::read(p.root.join("artifacts/signatures.json")).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("MIT License"));
    assert!(text.contains("SECURITY"));
    assert!(text.contains("function auth(token: string)"));
    assert!(!text.contains("return token"));
    let checked = verify(&p, "artifacts/signatures.json", true).unwrap();
    assert_eq!(checked["integrity"], "verified");
    assert_eq!(checked["actual_bytes"].as_u64(), Some(bytes.len() as u64));
}
#[cfg(unix)]
#[test]
fn symlink_output_input_overlap_and_overwrite_are_denied() {
    use std::os::unix::fs::symlink;
    let (_t, p, task) = fixture();
    std::fs::write(p.root.join("src/auth.ts"), "export const auth=1;").unwrap();
    let a = plan(&p, &task, "full", None);
    assert_eq!(
        create(&p, &a, "src/pack").unwrap_err().code,
        "POLICY_DENIED"
    );
    std::fs::create_dir_all(p.root.join("artifacts")).unwrap();
    symlink(p.root.join("src"), p.root.join("artifacts/link")).unwrap();
    assert_eq!(
        create(&p, &a, "artifacts/link/pack").unwrap_err().code,
        "POLICY_DENIED"
    );
    create(&p, &a, "artifacts/pack").unwrap();
    assert_eq!(
        create(&p, &a, "artifacts/pack").unwrap_err().code,
        "REVISION_CONFLICT"
    );
}

fn rewrite_manifest(path: &std::path::Path, change: impl FnOnce(&mut Value)) {
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    change(&mut manifest);
    let mut digest = manifest.clone();
    digest
        .as_object_mut()
        .unwrap()
        .remove("manifest_integrity_hash");
    manifest["manifest_integrity_hash"] =
        json!(pctx::domain::hash(serde_json::to_vec(&digest).unwrap()));
    std::fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
}
#[test]
fn inspection_reapplies_source_artifact_policy_and_masks_untrusted_manifest() {
    let (_t, mut p, task) = fixture();
    std::fs::write(p.root.join("src/auth.ts"), "export const auth=1;\n").unwrap();
    let a = plan(&p, &task, "full", None);
    create(&p, &a, "artifacts/review").unwrap();
    let manifest = p.root.join("artifacts/review/manifest.json");
    let token = "ghp_abcdefghijklmnopqrstuv";
    rewrite_manifest(&manifest, |m| {
        m["task"]["description"] = json!(format!(
            "{token} /Users/private/work C:\\Users\\private\\work"
        ));
        m["task"]["capability"] = json!("plain credential value");
        m["raw_output"] = json!("FORGED_SOURCE_BODY");
        m["files"][0]["excerpts"] = json!([{"text":"FORGED_SOURCE_BODY"}]);
    });
    let inspected = execute(
        &p,
        &PackCommand::Inspect {
            path: "artifacts/review".into(),
        },
    )
    .unwrap();
    let text = inspected.to_string();
    assert!(!text.contains(token));
    assert!(!text.contains("/Users/private"));
    assert!(!text.contains("plain credential value"));
    assert!(!text.contains("FORGED_SOURCE_BODY"));
    assert!(!text.contains("C:\\\\Users"));
    p.config.policy.exclude.push("src/auth.ts".into());
    assert_eq!(
        execute(
            &p,
            &PackCommand::Inspect {
                path: "artifacts/review".into()
            }
        )
        .unwrap_err()
        .code,
        "POLICY_DENIED"
    );
    p.config.policy.exclude.clear();
    p.config.policy.exclude.push("**/context.json".into());
    assert_eq!(
        verify(&p, "artifacts/review", false).unwrap_err().code,
        "POLICY_DENIED"
    );
    p.config.policy.exclude.clear();
    p.config.policy.exclude.push("artifacts/**".into());
    assert_eq!(
        verify(&p, "artifacts/review", false).unwrap_err().code,
        "POLICY_DENIED"
    );
}
#[test]
fn task_or_parser_revision_changes_make_current_source_pack_stale() {
    let (_t, p, task) = fixture();
    std::fs::write(p.root.join("src/auth.ts"), "export const auth=1;\n").unwrap();
    let a = plan(&p, &task, "full", None);
    create(&p, &a, "artifacts/review").unwrap();
    assert_eq!(
        verify(&p, "artifacts/review", true).unwrap()["freshness"],
        "current"
    );
    work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Cancel {
                task: task.clone(),
                reason: "changed requirement".into(),
            },
        },
    )
    .unwrap();
    assert_eq!(
        verify(&p, "artifacts/review", true).unwrap()["freshness"],
        "stale"
    );
    let a = plan(&p, &task, "full", None);
    create(&p, &a, "artifacts/parser").unwrap();
    rewrite_manifest(&p.root.join("artifacts/parser/manifest.json"), |m| {
        m["parser_hash"] = json!("different-parser")
    });
    assert_eq!(
        verify(&p, "artifacts/parser", true).unwrap()["freshness"],
        "stale"
    );
}
#[test]
fn forged_budgets_parts_and_json_complexity_are_rejected_before_load() {
    let (_t, p, task) = fixture();
    std::fs::write(p.root.join("src/auth.ts"), "export const auth=1;\n").unwrap();
    let a = plan(&p, &task, "full", None);
    create(&p, &a, "artifacts/review").unwrap();
    let manifest = p.root.join("artifacts/review/manifest.json");
    rewrite_manifest(&manifest, |m| m["budget"]["limit_bytes"] = json!(u64::MAX));
    assert_eq!(
        verify(&p, "artifacts/review", false).unwrap_err().code,
        "BUDGET_TOO_SMALL"
    );
    rewrite_manifest(&manifest, |m| {
        m["budget"]["limit_bytes"] = json!(64000);
        m["parts"][0]["bytes"] = json!(64 * 1024 * 1024 + 1);
    });
    assert_eq!(
        verify(&p, "artifacts/review", false).unwrap_err().code,
        "BUDGET_TOO_SMALL"
    );
    let body = format!(
        "{{\"manifest\":{{}},\"parts\":{},\"complexity\":[{}]}}",
        "{}",
        "0,".repeat(200001) + "0"
    );
    std::fs::write(p.root.join("artifacts/complex.json"), body).unwrap();
    assert_eq!(
        verify(&p, "artifacts/complex.json", false)
            .unwrap_err()
            .code,
        "BUDGET_TOO_SMALL"
    );
}
#[cfg(unix)]
#[test]
fn racing_artifact_parent_symlink_never_reads_outside_marker() {
    use std::{
        os::unix::fs::symlink,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };
    let (_t, p, task) = fixture();
    std::fs::write(p.root.join("src/auth.ts"), "export const auth=1;\n").unwrap();
    let a = plan(&p, &task, "full", None);
    create(&p, &a, "artifacts/review").unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir(outside.path().join("review")).unwrap();
    let source = p.root.join("artifacts/review");
    for name in ["manifest.json", "context.json"] {
        std::fs::copy(source.join(name), outside.path().join("review").join(name)).unwrap();
    }
    rewrite_manifest(&outside.path().join("review/manifest.json"), |m| {
        m["task"]["description"] = json!("OUTSIDE_SCOPE_MARKER")
    });
    let active = Arc::new(AtomicBool::new(true));
    let flag = active.clone();
    let root = p.root.clone();
    let external = outside.path().to_owned();
    let writer = std::thread::spawn(move || {
        while flag.load(Ordering::Relaxed) {
            if std::fs::rename(root.join("artifacts"), root.join("artifacts-held")).is_ok() {
                let _ = symlink(&external, root.join("artifacts"));
                std::thread::yield_now();
                let _ = std::fs::remove_file(root.join("artifacts"));
                let _ = std::fs::rename(root.join("artifacts-held"), root.join("artifacts"));
            }
        }
    });
    for _ in 0..100 {
        if let Ok(value) = execute(
            &p,
            &PackCommand::Inspect {
                path: "artifacts/review".into(),
            },
        ) {
            assert!(!value.to_string().contains("OUTSIDE_SCOPE_MARKER"));
        }
    }
    active.store(false, Ordering::Relaxed);
    writer.join().unwrap();
}

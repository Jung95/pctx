use pctx::{
    domain::hash,
    output::{self, OutputCommand, RunRequest, TrustCommand},
    project::{Config, Project, ProjectConfig},
};
use serde_json::Value;
use std::fs;

fn fixture() -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(temp.path()).unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&data).unwrap();
    fs::write(root.join("input.txt"), "ordinary source\n").unwrap();
    let p = Project {
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspaces/WS-test"),
        control_dir: data.join("controls/COORD-test"),
        project_id: "test".into(),
        workspace_id: "WS-test".into(),
        coordination_id: "COORD-test".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "test".into(),
                name: "test".into(),
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
    (temp, p)
}
fn request(args: &[&str]) -> RunRequest {
    RunRequest {
        task_id: None,
        session: None,
        retain: "temporary".into(),
        execution_timeout_ms: Some(5000),
        budget_bytes: 8192,
        exit_policy: "child".into(),
        stdin: "closed".into(),
        argv: args.iter().map(|s| s.to_string()).collect(),
    }
}
fn trust(p: &Project, r: &RunRequest) {
    let plan = output::trust(
        p,
        &TrustCommand::Plan {
            argv: r.argv.clone(),
        },
    )
    .unwrap();
    output::trust(
        p,
        &TrustCommand::Add {
            argv: r.argv.clone(),
            expect_hash: plan["fingerprint"].as_str().unwrap().into(),
        },
    )
    .unwrap();
}
fn full(p: &Project, v: &Value) -> Value {
    output::output(
        p,
        &OutputCommand::Show {
            id: v["output_id"].as_str().unwrap().into(),
            view: "full".into(),
            stream: None,
            lines: None,
        },
    )
    .unwrap()
}
#[cfg(unix)]
#[test]
fn exact_trust_child_exit_and_reread_without_execution() {
    let (_t, p) = fixture();
    let r = request(&["/usr/bin/printf", "error: bad input\n"]);
    assert_eq!(
        output::run(&p, &r).unwrap_err().code,
        "OWNER_DECISION_REQUIRED"
    );
    trust(&p, &r);
    let v = output::run(&p, &r).unwrap();
    assert_eq!(v["child_exit_code"], 0);
    assert_eq!(v["parse_status"], "unsupported");
    assert_eq!(v["test_result"], "not_evaluated");
    let a = full(&p, &v);
    assert_eq!(a["command_rerun"], false);
    assert_eq!(a["records"][0]["text"], "error: bad input");
    let out = v["output_id"].as_str().unwrap();
    output::record_delivery(&p, out, 100, "compact").unwrap();
    output::record_delivery(&p, out, 1000, "retrieval").unwrap();
    let s = output::savings(&p).unwrap();
    assert!(s["net_bytes_saved"].as_i64().unwrap() < 0);
    assert_eq!(s["provider_usage"], "unknown");
    let mut changed = request(&["/usr/bin/printf", "different\n"]);
    assert_eq!(
        output::run(&p, &changed).unwrap_err().code,
        "OWNER_DECISION_REQUIRED"
    );
    changed.budget_bytes = 2;
    assert_eq!(
        output::run(&p, &changed).unwrap_err().code,
        "BUDGET_TOO_SMALL"
    );
}
#[cfg(unix)]
#[test]
fn nonzero_and_timeout_are_not_filter_success() {
    let (_t, p) = fixture();
    let r = request(&["/usr/bin/false"]);
    trust(&p, &r);
    let v = output::run(&p, &r).unwrap();
    assert_eq!(v["child_exit_code"], 1);
    assert_eq!(v["task_completion"], "not_evaluated");
    let mut sleep = request(&["/bin/sleep", "5"]);
    sleep.execution_timeout_ms = Some(20);
    trust(&p, &sleep);
    let v = output::run(&p, &sleep).unwrap();
    assert_eq!(v["termination"], "timed_out");
    assert_eq!(v["pctx_error"], "TIMEOUT");
    assert!(v["signal"].is_number());
}
#[cfg(unix)]
#[test]
fn masking_is_before_storage_and_cr_preserves_error() {
    let (_t, p) = fixture();
    let secret = "ghp_abcdefghijklmnop123456789";
    let body = format!(
        "error important\rprogress 90%\n{secret}\n-----BEGIN PRIVATE KEY-----\nbase64privatevalue\n-----END PRIVATE KEY-----\n"
    );
    let r = request(&["/usr/bin/printf", "%s", &body]);
    trust(&p, &r);
    let v = output::run(&p, &r).unwrap();
    let a = full(&p, &v);
    assert_eq!(a["records"][0]["text"], "error important");
    assert!(
        serde_json::to_string(&a).unwrap().contains("[REDACTED]"),
        "{a}"
    );
    let bytes = fs::read(
        p.data_dir
            .join("outputs/WS-test")
            .join(format!("{}.json", v["output_id"].as_str().unwrap())),
    )
    .unwrap();
    let disk = String::from_utf8(bytes).unwrap();
    assert!(!disk.contains(secret));
    assert!(!disk.contains("base64privatevalue"));
    assert_eq!(v["capture_complete"], false);
}
#[cfg(unix)]
#[test]
fn none_expiry_tamper_and_policy_change_cannot_restore_records() {
    let (_t, mut p) = fixture();
    let mut r = request(&["/usr/bin/printf", "data\n"]);
    r.retain = "none".into();
    trust(&p, &r);
    let v = output::run(&p, &r).unwrap();
    assert_eq!(v["raw_available"], false);
    assert_eq!(
        output::output(
            &p,
            &OutputCommand::Render {
                id: v["output_id"].as_str().unwrap().into(),
                filter: "builtin".into()
            }
        )
        .unwrap_err()
        .code,
        "OUTPUT_EXPIRED"
    );
    r.retain = "temporary".into();
    let v = output::run(&p, &r).unwrap();
    let path = p
        .data_dir
        .join("outputs/WS-test")
        .join(format!("{}.json", v["output_id"].as_str().unwrap()));
    let mut stored: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    stored["records"][0]["text"] = serde_json::json!("tampered");
    fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
    assert_eq!(
        output::output(
            &p,
            &OutputCommand::Render {
                id: v["output_id"].as_str().unwrap().into(),
                filter: "builtin".into()
            }
        )
        .unwrap_err()
        .code,
        "OUTPUT_PARTIAL"
    );
    stored["records_hash"] =
        serde_json::json!(hash(serde_json::to_vec(&stored["records"]).unwrap()));
    stored["expires_at"] = serde_json::json!(0);
    fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
    assert_eq!(
        output::output(
            &p,
            &OutputCommand::Render {
                id: v["output_id"].as_str().unwrap().into(),
                filter: "builtin".into()
            }
        )
        .unwrap_err()
        .code,
        "OUTPUT_EXPIRED"
    );
    stored["expires_at"] = serde_json::json!(i64::MAX);
    fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
    p.config.policy.exclude.push("input.txt".into());
    assert_eq!(
        output::output(
            &p,
            &OutputCommand::Render {
                id: v["output_id"].as_str().unwrap().into(),
                filter: "builtin".into()
            }
        )
        .unwrap_err()
        .code,
        "POLICY_DENIED"
    );
}
#[test]
fn opaque_shell_heavy_and_bad_identifiers_rejected() {
    let (_t, p) = fixture();
    assert_eq!(
        output::trust(
            &p,
            &TrustCommand::Plan {
                argv: vec!["/bin/sh".into(), "-c".into(), "echo hello".into()]
            }
        )
        .unwrap_err()
        .code,
        "CAPABILITY_UNAVAILABLE"
    );
    assert_eq!(
        output::output(
            &p,
            &OutputCommand::Render {
                id: "../../input.txt".into(),
                filter: "builtin".into()
            }
        )
        .unwrap_err()
        .code,
        "INVALID_ARGUMENT"
    );
}

#[cfg(unix)]
#[test]
fn typed_full_report_preserves_failed_counts_without_becoming_gate_evidence() {
    let (_t, p) = fixture();
    let text = r#"{"schema_version":1,"check_key":"unit","producer":"fixture","source":"external_report","exit_code":0,"tests":3,"passed":1,"failed":1,"errors":0,"skipped":1,"result":"failed","started_at":1,"finished_at":2}"#;
    let r = request(&["/usr/bin/printf", text]);
    trust(&p, &r);
    let v = output::run(&p, &r).unwrap();
    assert_eq!(v["parse_status"], "complete");
    assert_eq!(v["typed_report"]["summary"]["failed"], 1);
    assert_eq!(v["typed_report"]["gate_evidence"], false);
    assert_eq!(v["test_result"], "not_evaluated");
}

#[cfg(unix)]
#[test]
fn agent_cannot_mint_execution_trust_and_git_helpers_are_unavailable() {
    let t = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(t.path()).unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir_all(&root).unwrap();
    let invoke = |args: &[&str], actor: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_pctx"))
            .env("PCTX_DATA_DIR", &data)
            .env("PCTX_ACTOR", actor)
            .arg("--root")
            .arg(&root)
            .arg("--format")
            .arg("json")
            .args(args)
            .output()
            .unwrap()
    };
    assert!(invoke(&["init"], "owner").status.success());
    let plan = invoke(&["trust", "plan", "--", "/usr/bin/printf", "safe"], "owner");
    assert!(plan.status.success());
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    let fingerprint = plan["data"]["fingerprint"].as_str().unwrap();
    let denied = invoke(
        &[
            "trust",
            "add",
            "--expect-hash",
            fingerprint,
            "--",
            "/usr/bin/printf",
            "safe",
        ],
        "agent",
    );
    assert_eq!(denied.status.code(), Some(5));
    let denied: Value = serde_json::from_slice(&denied.stdout).unwrap();
    assert_eq!(denied["errors"][0]["code"], "POLICY_DENIED");
    assert!(!data.join("trust/executions").exists());
    let git = invoke(&["trust", "plan", "--", "git", "status"], "owner");
    assert_eq!(git.status.code(), Some(6));
    let git: Value = serde_json::from_slice(&git.stdout).unwrap();
    assert_eq!(git["errors"][0]["code"], "CAPABILITY_UNAVAILABLE");
}
#[cfg(unix)]
#[test]
fn diagnostic_extract_binds_original_source_and_rereads_without_execution() {
    let (_t, p) = fixture();
    let r = request(&["/usr/bin/printf", "%s", "input.txt:1:2: error fixture\n"]);
    trust(&p, &r);
    let run = output::run(&p, &r).unwrap();
    let id = run["output_id"].as_str().unwrap();
    let request = pctx::extract::ExtractRequest {
        from_output: Some(id.into()),
        location: vec![],
        path: None,
        line: None,
        symbol_id: vec![],
        unit: "lines".into(),
        view: "full_span".into(),
        budget_bytes: 12000,
    };
    let extracted = pctx::extract::extract(&p, &request).unwrap();
    assert_eq!(extracted["items"][0]["path"], "input.txt");
    assert_eq!(extracted["diagnostic_sources"]["command_rerun"], false);
    fs::write(p.root.join("input.txt"), "changed source\n").unwrap();
    assert_eq!(
        pctx::extract::extract(&p, &request).unwrap_err().code,
        "STALE_RESULT"
    );
    assert_eq!(full(&p, &run)["command_rerun"], false);
}

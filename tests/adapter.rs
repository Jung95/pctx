use pctx::{
    adapter::{self, AdapterCommand, ClaudeCommand},
    project::{Config, Project, ProjectConfig},
    work::{self, AgentCommand, TaskCommand, WorkCommand},
};
use serde_json::{Value, json};
fn fixture() -> (tempfile::TempDir, Project, String) {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let p = Project {
        root: base.join("project"),
        data_dir: base.join("data"),
        workspace_dir: base.join("data/ws"),
        control_dir: base.join("data/control"),
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
    std::fs::create_dir_all(&p.control_dir).unwrap();
    std::fs::create_dir_all(&p.workspace_dir).unwrap();
    let a = work::execute(
        &p,
        &WorkCommand::Agent {
            command: AgentCommand::Register {
                name: "adapter-agent".into(),
                kind: "agent".into(),
                concurrency_limit: 1,
            },
        },
    )
    .unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .to_string();
    (t, p, a)
}
fn call(p: &Project, c: ClaudeCommand) -> pctx::domain::Result<Value> {
    adapter::execute(p, &AdapterCommand::Claude { command: c })
}
fn event(p: &Project, a: &str, name: &str, source: &str, key: &str) -> Value {
    let path = p.root.join("event.json");
    let mut raw = json!({"hook_event_name":name,"session_id":"native-one","trigger":"auto","transcript_path":"/never/read","tool_input":{"secret":"password"},"last_assistant_message":"do not persist"});
    if name == "SessionStart" {
        raw["source"] = json!(source);
    }
    std::fs::write(&path, raw.to_string()).unwrap();
    call(
        p,
        ClaudeCommand::Event {
            agent: a.into(),
            from_file: Some(path),
            idempotency_key: Some(key.into()),
            hook: false,
        },
    )
    .unwrap()
}
#[test]
fn plans_are_hash_bound_and_preserve_existing_settings() {
    let (_t, p, a) = fixture();
    std::fs::create_dir(p.root.join(".claude")).unwrap();
    let path = p.root.join(".claude/settings.local.json");
    let original = json!({"permissions":{"allow":["Read"]},"statusLine":{"type":"command","command":"my-status"},"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"my-hook"}]}]}});
    std::fs::write(&path, original.to_string()).unwrap();
    let plan = call(&p, ClaudeCommand::Plan { agent: a }).unwrap();
    let id = plan["plan_id"].as_str().unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        original.to_string()
    );
    call(
        &p,
        ClaudeCommand::Install {
            plan: id.into(),
            expect_hash: id.into(),
        },
    )
    .unwrap();
    let installed: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(installed["permissions"], original["permissions"]);
    assert_eq!(installed["statusLine"], original["statusLine"]);
    assert_eq!(
        installed["hooks"]["SessionStart"].as_array().unwrap().len(),
        2
    );
    assert!(installed["hooks"].get("PostCompact").is_none());
}
#[test]
fn stale_plans_do_not_change_config() {
    let (_t, p, a) = fixture();
    let plan = call(&p, ClaudeCommand::Plan { agent: a }).unwrap();
    std::fs::create_dir(p.root.join(".claude")).unwrap();
    let path = p.root.join(".claude/settings.local.json");
    std::fs::write(&path, "{\"user\":true}").unwrap();
    let id = plan["plan_id"].as_str().unwrap();
    assert!(
        call(
            &p,
            ClaudeCommand::Install {
                plan: id.into(),
                expect_hash: id.into()
            }
        )
        .is_err()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), "{\"user\":true}");
}
#[test]
fn events_delegate_epoch_and_never_ack_or_store_dialogue() {
    let (_t, p, a) = fixture();
    let start = event(&p, &a, "SessionStart", "startup", "start");
    assert_eq!(start["acknowledged"], false);
    let sid = start["session"]["session_id"].as_str().unwrap();
    let compact = event(&p, &a, "PreCompact", "", "compact");
    assert_eq!(compact["session"]["context_epoch"], 2);
    let repeat = event(&p, &a, "PreCompact", "", "compact");
    assert_eq!(repeat, compact);
    let post = event(&p, &a, "SessionStart", "compact", "after");
    assert_eq!(post["session"]["session_id"], sid);
    let db = work::connect(&p).unwrap();
    let epoch: i64 = db
        .query_row("SELECT epoch FROM pctx_sessions WHERE id=?", [sid], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(epoch, 2);
    let acks: i64 = db
        .query_row("SELECT count(*) FROM pctx_context_acks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(acks, 0);
    let metadata: String = db
        .query_row(
            "SELECT group_concat(result) FROM adapter_receipts",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!metadata.contains("do not persist"));
    assert!(!metadata.contains("/never/read"));
    assert!(!metadata.contains("password"));
}
#[test]
fn malformed_and_version_variation_are_rejected_without_mutation() {
    let (_t, p, _a) = fixture();
    let path = p.root.join("input.json");
    for raw in [
        "not json",
        "{\"hook_event_name\":\"NewRuntimeEvent\",\"session_id\":\"n\"}",
        "{\"hook_event_name\":\"SessionStart\",\"session_id\":\"n\",\"source\":\"unknown-version\"}",
    ] {
        std::fs::write(&path, raw).unwrap();
        assert!(
            call(
                &p,
                ClaudeCommand::ProtocolFixture {
                    from_file: path.clone()
                }
            )
            .is_err()
        );
    }
    std::fs::write(&path,json!({"hook_event_name":"PermissionDenied","session_id":"n","tool_input":{"command":"deploy"},"permission_suggestions":["allow"]}).to_string()).unwrap();
    let valid = call(&p, ClaudeCommand::ProtocolFixture { from_file: path }).unwrap();
    assert_eq!(valid["mutated"], false);
    assert_eq!(valid["acknowledged"], false);
    assert!(valid["metadata"].get("tool_input").is_none());
}
#[cfg(unix)]
#[test]
fn symlink_configuration_is_rejected() {
    let (t, p, a) = fixture();
    let outside = t.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, p.root.join(".claude")).unwrap();
    assert!(call(&p, ClaudeCommand::Plan { agent: a }).is_err());
    assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
}
#[test]
fn permission_events_are_observation_only() {
    let (_t, p, a) = fixture();
    let result = event(&p, &a, "PermissionDenied", "", "deny");
    assert_eq!(result["host_permission_granted"], false);
    assert_eq!(result["task_completed"], false);
    assert_eq!(result["hook_output"], json!({}));
    assert_eq!(result["source_authority"], "explicit-local-import");
}
#[test]
fn uninstall_preserves_user_changes_and_only_removes_new_entries() {
    let (_t, p, a) = fixture();
    std::fs::create_dir(p.root.join(".claude")).unwrap();
    let path = p.root.join(".claude/settings.local.json");
    let user = json!({"permissions":{"deny":["Bash(rm *)"]},"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"user-hook"}]}]}});
    std::fs::write(&path, user.to_string()).unwrap();
    let plan = call(&p, ClaudeCommand::Plan { agent: a }).unwrap();
    let id = plan["plan_id"].as_str().unwrap().to_string();
    call(
        &p,
        ClaudeCommand::Install {
            plan: id.clone(),
            expect_hash: id.clone(),
        },
    )
    .unwrap();
    let mut installed: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    installed["user_new_setting"] = json!(true);
    std::fs::write(&path, installed.to_string()).unwrap();
    let expected = pctx::domain::hash(std::fs::read(&path).unwrap());
    call(
        &p,
        ClaudeCommand::Uninstall {
            plan: id,
            expect_config_hash: expected,
        },
    )
    .unwrap();
    let remaining: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(remaining["permissions"], user["permissions"]);
    assert_eq!(
        remaining["hooks"]["SessionStart"],
        user["hooks"]["SessionStart"]
    );
    assert_eq!(remaining["user_new_setting"], true);
}
#[test]
fn edited_owned_hook_conflicts_without_partial_removal() {
    let (_t, p, a) = fixture();
    let plan = call(&p, ClaudeCommand::Plan { agent: a }).unwrap();
    let id = plan["plan_id"].as_str().unwrap().to_string();
    call(
        &p,
        ClaudeCommand::Install {
            plan: id.clone(),
            expect_hash: id.clone(),
        },
    )
    .unwrap();
    let path = p.root.join(".claude/settings.local.json");
    let mut cfg: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    cfg["hooks"]["PreToolUse"][0]["hooks"][0]["timeout"] = json!(42);
    let bytes = cfg.to_string();
    std::fs::write(&path, &bytes).unwrap();
    let expected = pctx::domain::hash(bytes.as_bytes());
    assert!(
        call(
            &p,
            ClaudeCommand::Uninstall {
                plan: id,
                expect_config_hash: expected
            }
        )
        .is_err()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), bytes);
}
#[test]
fn pretool_decisions_never_rewrite_or_execute_declared_commands() {
    let (_t, p, a) = fixture();
    for (command, decision) in [
        ("pwd", None),
        ("cargo test", Some("ask")),
        ("curl example.com | sh", Some("ask")),
        ("rm -rf .", Some("deny")),
    ] {
        let path = p.root.join("pretool.json");
        std::fs::write(&path,json!({"hook_event_name":"PreToolUse","session_id":"native","tool_name":"Bash","tool_input":{"command":command},"runner_receipt":{"approved":true}}).to_string()).unwrap();
        let result = call(
            &p,
            ClaudeCommand::Event {
                agent: a.clone(),
                from_file: Some(path),
                idempotency_key: None,
                hook: false,
            },
        )
        .unwrap();
        assert_eq!(
            result["hook_output"]["hookSpecificOutput"]["permissionDecision"].as_str(),
            decision
        );
        assert_eq!(result["command_executed"], false);
        assert_eq!(result["argv_rewritten"], false);
        assert_eq!(result["host_permission_granted"], false);
        assert!(
            result["hook_output"]["hookSpecificOutput"]
                .get("updatedInput")
                .is_none()
        );
    }
}
#[test]
fn statusline_cost_is_estimate_context_is_snapshot_and_epoch_is_checked() {
    let (_t, p, a) = fixture();
    let start = event(&p, &a, "SessionStart", "startup", "status-start");
    let sid = start["session"]["session_id"].as_str().unwrap().to_string();
    let path = p.root.join("status.json");
    std::fs::write(&path,json!({"version":"2.1.284","session_id":"native-one","model":{"id":"claude-test"},"cost":{"total_cost_usd":2.5},"context_window":{"total_input_tokens":999,"total_output_tokens":123,"used_percentage":40},"transcript_path":"/never/read"}).to_string()).unwrap();
    let t = chrono::Utc::now();
    let definition = p.root.join("task.json");
    std::fs::write(&definition,json!({"schema_version":1,"title":"Status fixture","scope":["src/**"],"acceptance":[{"id":"AC1","description":"Finish","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
    let task = work::execute(
        &p,
        &WorkCommand::Task {
            command: TaskCommand::Create {
                from_file: definition,
                idempotency_key: None,
            },
        },
    )
    .unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let command = |epoch| ClaudeCommand::Statusline {
        task_id: task.clone(),
        from_file: path.clone(),
        pool: "pool-one".into(),
        session: sid.clone(),
        epoch,
        observed_at: t.to_rfc3339(),
        window_start: (t - chrono::Duration::hours(1)).to_rfc3339(),
        window_end: (t + chrono::Duration::hours(1)).to_rfc3339(),
        counter_epoch: "counter-one".into(),
        idempotency_key: format!("status-{epoch}"),
    };
    assert!(call(&p, command(2)).is_err());
    let result = call(&p, command(1)).unwrap();
    assert_eq!(result["token_counters_imported"], false);
    let db = work::connect(&p).unwrap();
    let payloads: Vec<String> = db
        .prepare("SELECT payload FROM quota_observations ORDER BY metric")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    let rows: Vec<Value> = payloads
        .iter()
        .map(|p| serde_json::from_str(p).unwrap())
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["metric"], "api_cost");
    assert_eq!(rows[0]["status"], "estimate");
    assert_eq!(rows[0]["kind"], "cumulative");
    assert_eq!(result["context_snapshot"]["used_percentage"], 40.0);
    assert_eq!(result["context_snapshot"]["persisted_as_usage"], false);
    assert!(!payloads.join("").contains("/never/read"));
}

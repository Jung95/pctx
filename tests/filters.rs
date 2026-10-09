use pctx::{
    domain::hash,
    filters::{self, FilterCommand, FilterRecord},
    project::{Config, Project, ProjectConfig},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn fixture() -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(temp.path()).unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir_all(root.join(".pctx/filters")).unwrap();
    fs::create_dir_all(&data).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: pctx::project::RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspace"),
        control_dir: data.join("control"),
        project_id: "project".into(),
        workspace_id: "workspace".into(),
        coordination_id: "coordination".into(),
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
    (temp, p)
}
fn definition(id: &str) -> String {
    format!(
        r#"schema_version = 1
id = "{id}"
version = "1.0.0"
priority = 100
[match]
program = "/usr/bin/printf"
argv_prefix = ["status"]
stream = "both"
[parse]
kind = "lines"
[render]
max_bytes = 8192
keep_head_lines = 12
keep_tail_lines = 20
show_omission_counts = true
[[rules]]
op = "protect"
pattern = '(?i)error|failed|warning'
[[rules]]
op = "drop"
pattern = '.*'
[[rules]]
op = "deduplicate_exact"
show_count = true
"#
    )
}
fn write_filter(p: &Project, id: &str, text: &str) {
    fs::write(p.root.join(format!(".pctx/filters/{id}.toml")), text).unwrap();
}
fn apply(p: &Project, id: &str, body: &str, exit: i32) -> Value {
    fs::write(p.root.join("preview.log"), body).unwrap();
    filters::execute(
        p,
        &FilterCommand::Apply {
            filter: id.into(),
            input: PathBuf::from("preview.log"),
            child_exit: exit,
        },
    )
    .unwrap()
}
#[test]
fn protect_beats_drop_dedup_keeps_occurrences_ranges_and_no_pass_claim() {
    let (_t, p) = fixture();
    write_filter(&p, "sample", &definition("sample"));
    let a = apply(&p, "sample", "error E42\nprogress\nerror E42\n", 1);
    assert_eq!(a["child_exit_code"], 1);
    assert_eq!(a["test_result"], "not_evaluated");
    assert_eq!(a["execution_started"], false);
    assert_eq!(a["records"].as_array().unwrap().len(), 1);
    assert_eq!(a["records"][0]["count"], 2);
    assert_eq!(a["records"][0]["ranges"].as_array().unwrap().len(), 2);
    assert_eq!(a["omitted_occurrences"], 1);
    let b = apply(&p, "sample", "error E42\nprogress\nerror E42\n", 1);
    assert_eq!(a, b);
}
#[test]
fn strict_keys_operations_and_regex_no_code_execution() {
    let (_t, p) = fixture();
    for (i, text) in [
        format!("{}\nunknown = true\n", definition("bad")),
        definition("bad").replace("op = \"drop\"", "op = \"execute\""),
        definition("bad").replace("pattern = '.*'", "pattern = '(a)\\1'"),
    ]
    .iter()
    .enumerate()
    {
        write_filter(&p, "bad", text);
        assert_eq!(
            filters::execute(
                &p,
                &FilterCommand::Validate {
                    path: PathBuf::from(".pctx/filters/bad.toml")
                }
            )
            .unwrap_err()
            .code,
            "FILTER_INVALID",
            "variant {i}"
        );
    }
    write_filter(&p, "sample", &definition("sample"));
    let result = apply(&p, "sample", "error: $(touch owned)\n", 1);
    assert_eq!(result["execution_started"], false);
    assert!(!p.root.join("owned").exists());
    assert_eq!(
        filters::execute(
            &p,
            &FilterCommand::Activate {
                id: "sample".into(),
                expect_hash: hash(definition("sample"))
            }
        )
        .unwrap_err()
        .code,
        "POLICY_DENIED"
    );
}
#[test]
fn secrets_cr_and_unicode_are_safe_and_protected_survives_excerpt() {
    let (_t, p) = fixture();
    write_filter(&p, "sample", &definition("sample"));
    let result = apply(
        &p,
        "sample",
        "error E42\rprogress 90%\nwarning ghp_abcdefghijklmnopqrstuv\n",
        1,
    );
    let text = result.to_string();
    assert!(text.contains("E42"));
    assert!(text.contains("[REDACTED]"));
    assert!(!text.contains("ghp_abcdefghijklmnopqrstuv"));
    let body =
        "-----BEGIN PRIVATE KEY-----\nprivatevalue\n-----END PRIVATE KEY-----\nerror preserved\n";
    let result = apply(&p, "sample", body, 1);
    assert!(!result.to_string().contains("privatevalue"));
    assert!(result.to_string().contains("preserved"));
}
fn records(text: &str, stream: &str) -> Vec<FilterRecord> {
    let mut offset = 0;
    let mut result = Vec::new();
    for line in text.split_inclusive(['\n', '\r']) {
        let end = offset + line.len();
        result.push(FilterRecord {
            stream: stream.into(),
            sequence: result.len() as u64,
            text: pctx::reader::redact(line.trim_end_matches(['\n', '\r'])).0,
            start_byte: offset,
            end_byte: end,
        });
        offset = end;
    }
    result
}
fn passing_suite(p: &Project, id: &str, dir: &str) {
    fs::create_dir_all(p.root.join(dir)).unwrap();
    let large = format!("{}\n", "normal ".repeat(10000));
    let cases = [
        ("success", "summary: completed\n", 0, "exited", ""),
        ("nonzero", "error E42\n", 1, "exited", "E42"),
        ("warning-only", "warning W12\n", 0, "exited", "W12"),
        ("empty", "", 0, "exited", ""),
        ("timeout", "timeout signal\n", 137, "timed_out", "timeout"),
        ("malformed", "error malformed [\n", 1, "exited", "malformed"),
        ("large", large.as_str(), 0, "exited", ""),
        ("unicode", "warning 🦀\n", 0, "exited", "🦀"),
        ("cr-progress", "error E22\rprogress\n", 1, "exited", "E22"),
        ("streams", "summary out\n", 1, "exited", "error stderr"),
        (
            "split-secret",
            "warning ghp_abcdefghijklmnopqrstuv\n",
            0,
            "exited",
            "[REDACTED]",
        ),
    ];
    let mut manifest = Vec::new();
    for (i, (name, text, exit, termination, keep)) in cases.into_iter().enumerate() {
        let file = format!("case{i}.log");
        fs::write(p.root.join(dir).join(&file), text).unwrap();
        let mut input = records(text, "stdout");
        let stderr = if name == "streams" {
            let file = "stderr.log";
            fs::write(p.root.join(dir).join(file), "error stderr\n").unwrap();
            input.extend(records("error stderr\n", "stderr"));
            Some(file)
        } else {
            None
        };
        let view = filters::preview_records(p, id, &input, exit, termination).unwrap();
        let count = view["protected_occurrences"].as_u64().unwrap();
        manifest.push(json!({"name":name,"input":file,"stderr":stderr,"child_exit":exit,"termination":termination,"must_keep":if keep.is_empty(){vec![]}else{vec![keep]},"minimum_protected":count,"expected_view_hash":hash(serde_json::to_vec(&view).unwrap())}));
    }
    fs::write(
        p.root.join(dir).join("manifest.json"),
        json!({"schema_version":1,"executable":"/usr/bin/printf","cases":manifest}).to_string(),
    )
    .unwrap();
    let report = filters::execute(
        p,
        &FilterCommand::Test {
            path: PathBuf::from(format!(".pctx/filters/{id}.toml")),
            fixtures: PathBuf::from(dir),
        },
    )
    .unwrap();
    assert_eq!(report["all_passed"], true, "{report}");
}
fn activate(p: &Project, id: &str) {
    let valid = filters::execute(
        p,
        &FilterCommand::Validate {
            path: PathBuf::from(format!(".pctx/filters/{id}.toml")),
        },
    )
    .unwrap();
    filters::execute(
        p,
        &FilterCommand::Activate {
            id: id.into(),
            expect_hash: valid["filter_hash"].as_str().unwrap().into(),
        },
    )
    .unwrap();
}
#[test]
fn full_fixture_activation_hash_invalidation_and_exact_match_ambiguity() {
    let (_t, p) = fixture();
    write_filter(&p, "sample", &definition("sample"));
    passing_suite(&p, "sample", "fixtures/sample");
    activate(&p, "sample");
    let view = filters::apply_records(&p, "sample", &records("error E42\n", "stdout"), 1).unwrap();
    assert_eq!(view["child_exit_code"], 1);
    let explain = filters::execute(
        &p,
        &FilterCommand::Explain {
            argv: vec!["/usr/bin/printf".into(), "status".into()],
        },
    )
    .unwrap();
    assert_eq!(explain["selected"]["id"], "sample");
    let explain = filters::execute(
        &p,
        &FilterCommand::Explain {
            argv: vec!["/usr/bin/printf".into(), "status-other".into()],
        },
    )
    .unwrap();
    assert!(explain["selected"].is_null());
    write_filter(&p, "second", &definition("second"));
    passing_suite(&p, "second", "fixtures/second");
    activate(&p, "second");
    assert_eq!(
        filters::execute(
            &p,
            &FilterCommand::Explain {
                argv: vec!["/usr/bin/printf".into(), "status".into()]
            }
        )
        .unwrap_err()
        .code,
        "FILTER_AMBIGUOUS"
    );
    write_filter(
        &p,
        "sample",
        &definition("sample").replace("priority = 100", "priority = 101"),
    );
    assert_eq!(
        filters::apply_records(&p, "sample", &records("error E42\n", "stdout"), 1)
            .unwrap_err()
            .code,
        "CONFIG_CHANGED"
    );
}
#[test]
fn changed_fixture_cannot_activate_old_report_and_bounds_are_explicit() {
    let (_t, p) = fixture();
    write_filter(&p, "sample", &definition("sample"));
    passing_suite(&p, "sample", "fixtures/sample");
    fs::write(p.root.join("fixtures/sample/case0.log"), "changed").unwrap();
    assert_eq!(
        filters::execute(
            &p,
            &FilterCommand::Activate {
                id: "sample".into(),
                expect_hash: hash(definition("sample"))
            }
        )
        .unwrap_err()
        .code,
        "CONFIG_CHANGED"
    );
    let inputs = vec![FilterRecord {
        stream: "stdout".into(),
        sequence: 0,
        text: "x".repeat(1024 * 1024 + 1),
        start_byte: 0,
        end_byte: 1024 * 1024 + 1,
    }];
    assert_eq!(
        filters::preview_records(&p, "sample", &inputs, 0, "exited")
            .unwrap_err()
            .code,
        "FILTER_LIMIT_EXCEEDED"
    );
    assert_eq!(
        filters::execute(
            &p,
            &FilterCommand::Validate {
                path: Path::new("../outside.toml").to_owned()
            }
        )
        .unwrap_err()
        .code,
        "POLICY_DENIED"
    );
}

#[test]
fn structured_fields_grouping_and_excerpt_keep_protected_metadata() {
    let (_t, p) = fixture();
    let text = definition("json")
        .replace("kind = \"lines\"", "kind = \"json\"")
        .replace("pattern = '.*'", "pattern = '^progress'");
    let text = format!(
        "{text}\n[[rules]]\nop = \"select_fields\"\nfields = [\"message\"]\n[[rules]]\nop = \"group_by\"\nfields = [\"message\"]\n[[rules]]\nop = \"deduplicate_exact\"\nshow_count = true\n"
    );
    write_filter(&p, "json", &text);
    let body = "{\"severity\":\"info\",\"message\":\"hello\",\"ignored\":1}\n{\"severity\":\"info\",\"message\":\"hello\",\"ignored\":1}\n{\"severity\":\"error\",\"message\":\"E42\",\"diagnostic\":\"D1\"}\n";
    let value = apply(&p, "json", body, 1);
    assert_eq!(value["records"].as_array().unwrap().len(), 2);
    assert!(
        value["records"][0]["group"]
            .as_str()
            .unwrap()
            .contains("hello")
    );
    assert_eq!(value["records"][0]["count"], 2);
    assert!(value["records"][0]["fields"].get("ignored").is_none());
    assert_eq!(value["records"][1]["fields"]["diagnostic"], "D1");
    let excerpt =
        format!("{text}\n[[rules]]\nop = \"excerpt\"\nkeep_head_lines = 0\nkeep_tail_lines = 0\n");
    write_filter(&p, "json", &excerpt);
    let value = apply(&p, "json", body, 1);
    assert_eq!(value["records"].as_array().unwrap().len(), 1);
    assert_eq!(value["records"][0]["fields"]["diagnostic"], "D1");
}
#[cfg(unix)]
#[test]
fn saved_output_custom_render_preserves_observed_failure_without_rerun() {
    use pctx::output::{self, OutputCommand, RunRequest, TrustCommand};
    let (_t, p) = fixture();
    write_filter(&p, "sample", &definition("sample"));
    passing_suite(&p, "sample", "fixtures/sample");
    activate(&p, "sample");
    let r = RunRequest {
        task_id: None,
        session: None,
        retain: "temporary".into(),
        execution_timeout_ms: Some(5000),
        budget_bytes: 8192,
        exit_policy: "child".into(),
        stdin: "closed".into(),
        argv: vec!["/usr/bin/printf".into(), "error before illegal %".into()],
    };
    let plan = output::trust(
        &p,
        &TrustCommand::Plan {
            argv: r.argv.clone(),
        },
    )
    .unwrap();
    output::trust(
        &p,
        &TrustCommand::Add {
            argv: r.argv.clone(),
            expect_hash: plan["fingerprint"].as_str().unwrap().into(),
        },
    )
    .unwrap();
    let run = output::run(&p, &r).unwrap();
    assert_ne!(run["child_exit_code"], 0);
    let rendered = output::output(
        &p,
        &OutputCommand::Render {
            id: run["output_id"].as_str().unwrap().into(),
            filter: "sample".into(),
        },
    )
    .unwrap();
    assert_eq!(rendered["child_exit_code"], run["child_exit_code"]);
    assert_eq!(rendered["termination"], run["termination"]);
    assert_eq!(rendered["command_rerun"], false);
    assert_eq!(rendered["test_result"], "not_evaluated");
    assert!(rendered.to_string().contains("error before illegal"));
}

#[test]
fn stored_filter_authority_corruption_keeps_hash_priority_and_cannot_publish_binding() {
    let (_t, p) = fixture();
    write_filter(&p, "sample", &definition("sample"));
    passing_suite(&p, "sample", "fixtures/sample");
    activate(&p, "sample");
    let binding_path = p.data_dir.join("filter-bindings/sample.json");
    let report_path = p.data_dir.join("filter-fixture-reports/sample.json");
    let original_binding = fs::read(&binding_path).unwrap();
    let original_report = fs::read(&report_path).unwrap();
    let preview = || filters::apply_records(&p, "sample", &records("error E42\n", "stdout"), 1);
    let assert_corrupt = |error: pctx::domain::Error| {
        assert_eq!((error.code.as_str(), error.exit), ("DB_CORRUPT", 7));
        assert!(!error.message.contains("PCTX_STORED_SENTINEL"));
    };
    for raw in ["{PCTX_STORED_SENTINEL", "{}"] {
        fs::write(&binding_path, raw).unwrap();
        assert_corrupt(preview().unwrap_err());
        assert_eq!(fs::read(&binding_path).unwrap(), raw.as_bytes());
        assert_eq!(fs::read(&report_path).unwrap(), original_report);
        fs::write(&binding_path, &original_binding).unwrap();
        fs::write(&report_path, raw).unwrap();
        let error = preview().unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("CONFIG_CHANGED", 9));
        // Match the saved integrity binding, reaching the actual typed decoder.
        let mut binding: Value = serde_json::from_slice(&original_binding).unwrap();
        binding["fixture_report_hash"] = json!(hash(raw));
        fs::write(&binding_path, binding.to_string()).unwrap();
        assert_corrupt(preview().unwrap_err());
        let before = fs::read(&binding_path).unwrap();
        assert_corrupt(
            filters::execute(
                &p,
                &FilterCommand::Activate {
                    id: "sample".into(),
                    expect_hash: hash(definition("sample")),
                },
            )
            .unwrap_err(),
        );
        assert_eq!(fs::read(&binding_path).unwrap(), before);
        assert_eq!(fs::read(&report_path).unwrap(), raw.as_bytes());
        fs::write(&binding_path, &original_binding).unwrap();
        fs::write(&report_path, &original_report).unwrap();
    }
    assert!(preview().is_ok());
}

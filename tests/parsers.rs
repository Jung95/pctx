use pctx::parsers::parse;
use serde_json::{Value, json};
fn records(s: &str) -> Vec<Value> {
    vec![json!({"stream":"stdout","sequence":7,"text":s})]
}
fn parse_json(id: &str, v: Value, exit: i32) -> Value {
    parse(id, &records(&v.to_string()), Some(exit), "exited", true)
}
fn native() -> Value {
    json!({"schema_version":1,"check_key":"unit","producer":"fixture","source":"external_report","exit_code":0,"tests":2,"passed":2,"failed":0,"errors":0,"skipped":0,"result":"passed","started_at":10,"finished_at":11})
}
fn suite_report() -> Value {
    json!({"numTotalTestSuites":1,"numPassedTestSuites":1,"numFailedTestSuites":0,"numPendingTestSuites":0,"numTotalTests":1,"numPassedTests":1,"numFailedTests":0,"numPendingTests":0,"numTodoTests":0,"startTime":1697737019307u64,"success":true,"testResults":[{"name":"src/a.test.ts","status":"passed","startTime":1697737019307u64,"endTime":1697737019317u64,"message":"","assertionResults":[{"fullName":"suite checks value","status":"passed","failureMessages":[],"ancestorTitles":["suite"],"title":"checks value","location":{"line":4,"column":2}}]}]})
}
#[test]
fn native_complete_claim_is_never_gate_evidence() {
    let v = parse_json("pctx-check-report-v1", native(), 0);
    assert_eq!(v["parse_status"], "complete");
    assert_eq!(v["reported_result"], "passed");
    assert_eq!(v["gate_evidence"], false);
    assert_eq!(v["summary"]["tests"], 2);
}
#[test]
fn native_zero_all_skipped_nonzero_and_exit_mismatch_cannot_pass() {
    for change in ["zero", "skipped", "failed", "exit"] {
        let mut n = native();
        match change {
            "zero" => {
                n["tests"] = json!(0);
                n["passed"] = json!(0)
            }
            "skipped" => {
                n["passed"] = json!(0);
                n["skipped"] = json!(2)
            }
            "failed" => {
                n["passed"] = json!(1);
                n["failed"] = json!(1)
            }
            _ => n["exit_code"] = json!(1),
        }
        let v = parse_json("pctx-check-report-v1", n, 0);
        assert_ne!(v["reported_result"], "passed");
        assert_eq!(v["gate_evidence"], false);
    }
}
#[test]
fn native_capture_timeout_or_missing_final_never_pass() {
    for (capture, termination, exit) in [
        (false, "exited", Some(0)),
        (true, "timed_out", None),
        (true, "signal", None),
    ] {
        let v = parse(
            "pctx-check-report-v1",
            &records(&native().to_string()),
            exit,
            termination,
            capture,
        );
        assert_ne!(v["reported_result"], "passed");
        assert_eq!(v["parse_status"], "partial");
    }
    let mut n = native();
    n.as_object_mut().unwrap().remove("finished_at");
    assert_eq!(
        parse_json("pctx-check-report-v1", n, 0)["parse_status"],
        "partial"
    );
}
#[test]
fn eslint_keeps_error_warning_fatal_ranges_and_distinct_files() {
    let file = |path: &str| json!({"filePath":path,"errorCount":1,"warningCount":1,"fatalErrorCount":1,"messages":[{"ruleId":null,"severity":2,"fatal":true,"message":"Unexpected token","line":2,"column":3,"endLine":2,"endColumn":4},{"ruleId":"no-console","severity":1,"message":"Unexpected console statement","line":8,"column":1}]});
    let v = parse_json(
        "eslint-json-v1",
        json!([file("src/a.ts"), file("src/b.ts")]),
        1,
    );
    assert_eq!(v["parse_status"], "complete");
    assert_eq!(v["summary"]["errors"], 2);
    assert_eq!(v["summary"]["warnings"], 2);
    assert_eq!(v["diagnostics"].as_array().unwrap().len(), 4);
    assert_ne!(v["diagnostics"][0]["path"], v["diagnostics"][2]["path"]);
    assert_eq!(v["diagnostics"][0]["end_column"], 4);
}
#[test]
fn eslint_count_drift_or_metadata_wrapper_is_partial() {
    let n = json!([{"filePath":"a.js","errorCount":1,"warningCount":0,"fatalErrorCount":0,"messages":[]}]);
    let v = parse_json("eslint-json-v1", n, 0);
    assert_eq!(v["parse_status"], "partial");
    assert_ne!(v["reported_result"], "passed");
    assert_eq!(
        parse_json("eslint-json-v1", json!({"results":[]}), 0)["parse_status"],
        "partial"
    );
}
#[test]
fn test_report_counts_survive_but_retry_history_is_not_invented() {
    for id in ["jest-json-v1", "vitest-json-v1"] {
        let v = parse_json(id, suite_report(), 0);
        assert_eq!(v["summary"]["passed"], 1);
        assert_eq!(v["reported_result"], "unverified");
        assert_eq!(v["gate_evidence"], false);
        assert!(
            v["reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("retry_history_not_certified"))
        );
    }
}
#[test]
fn retries_prior_failures_and_suite_crash_are_preserved() {
    let mut n = suite_report();
    let a = &mut n["testResults"][0]["assertionResults"][0];
    a["invocations"] = json!(2);
    a["failureMessages"] = json!(["Earlier retry failed at src/a.test.ts:4:2"]);
    let v = parse_json("jest-json-v1", n, 0);
    assert_eq!(v["summary"]["flaky_or_retried"], 1);
    assert_eq!(v["diagnostics"][0]["flaky"], true);
    assert_ne!(v["reported_result"], "passed");
    let mut n = suite_report();
    n["success"] = json!(false);
    n["numPassedTestSuites"] = json!(0);
    n["numFailedTestSuites"] = json!(1);
    n["testResults"][0]["status"] = json!("failed");
    n["testResults"][0]["message"] = json!("Worker terminated unexpectedly");
    n["testResults"][0]["testExecError"] = json!({"message":"Worker terminated unexpectedly"});
    assert_eq!(
        parse_json("vitest-json-v1", n, 1)["summary"]["suite_errors"],
        1
    );
}
#[test]
fn missing_final_and_unknown_assertions_are_partial() {
    let mut n = suite_report();
    n.as_object_mut().unwrap().remove("success");
    assert_eq!(parse_json("jest-json-v1", n, 0)["parse_status"], "partial");
    let mut n = suite_report();
    n["testResults"][0]["assertionResults"][0]["status"] = json!("future_unknown");
    assert_eq!(
        parse_json("vitest-json-v1", n, 0)["parse_status"],
        "partial"
    );
}
#[test]
fn typescript_diagnostics_keep_codes_locations_and_record_provenance() {
    let v = parse(
        "typescript-text-v1",
        &records(
            "src/a.ts(4,2): error TS2322: Type 'string' is not assignable to type 'number'.\nsrc/b.ts(4,2): error TS2322: Type 'string' is not assignable to type 'number'.\nerror TS18003: No inputs were found.",
        ),
        Some(2),
        "exited",
        true,
    );
    assert_eq!(v["summary"]["errors"], 3);
    assert_eq!(v["diagnostics"].as_array().unwrap().len(), 3);
    assert_eq!(v["diagnostics"][0]["code"], "TS2322");
    assert_eq!(v["diagnostics"][0]["record_ref"]["sequence"], 7);
    assert_eq!(v["diagnostics"][2]["path"], Value::Null);
}
#[test]
fn git_nul_status_preserves_rename_whitespace_and_conflicts() {
    let v = parse(
        "git-status-porcelain-v1-z",
        &records("R  new name\n.ts\0old name.ts\0UU conflict.ts\0?? untracked file.ts\0"),
        Some(0),
        "exited",
        true,
    );
    assert_eq!(v["parse_status"], "complete");
    assert_eq!(v["items"][0]["path"], "new name\n.ts");
    assert_eq!(v["items"][0]["previous_path"], "old name.ts");
    assert_eq!(v["summary"]["conflicts"], 1);
    assert_eq!(v["summary"]["untracked"], 1);
    assert_eq!(
        parse(
            "git-status-porcelain-v1-z",
            &records(" M no separator"),
            Some(0),
            "exited",
            true
        )["parse_status"],
        "partial"
    );
}
#[test]
fn git_log_selected_fields_and_diff_ranges_remain_review_only() {
    let oid = "a".repeat(40);
    let v = parse(
        "git-log-nul-v1",
        &records(&format!("{oid}\0\0subject with spaces\0")),
        Some(0),
        "exited",
        true,
    );
    assert_eq!(v["summary"]["commits"], 1);
    assert_eq!(v["items"][0]["subject"], "subject with spaces");
    let diff = "diff --git a/file name.ts b/file name.ts\n--- a/file name.ts\n+++ b/file name.ts\n@@ -1,2 +1,3 @@ fn\n old\n-old\n+new\n+more\n";
    let v = parse(
        "git-diff-unified-v1",
        &records(diff),
        Some(0),
        "exited",
        true,
    );
    assert_eq!(v["summary"]["patch_usable"], false);
    assert_eq!(v["diagnostics"][0]["old_count"], 2);
    assert_eq!(v["diagnostics"][0]["new_count"], 3);
    assert_eq!(v["diagnostics"][0]["new_path"], "b/file name.ts");
    assert_eq!(v["gate_evidence"], false);
}
#[test]
fn malformed_complex_oversize_and_unknown_inputs_fail_safely() {
    for s in [
        "{broken".into(),
        "[".repeat(65) + &"]".repeat(65),
        "x".repeat(1024 * 1024 + 1),
    ] {
        let v = parse("jest-json-v1", &records(&s), Some(0), "exited", true);
        assert_eq!(v["parse_status"], "partial");
        assert_ne!(v["reported_result"], "passed");
    }
    assert_eq!(
        parse(
            "future-parser",
            &records("all passed"),
            Some(0),
            "exited",
            true
        )["parse_status"],
        "unsupported"
    );
    let v = parse(
        "git-status-porcelain-v1-z",
        &records("é x\0"),
        Some(0),
        "exited",
        true,
    );
    assert_eq!(v["parse_status"], "partial");
}

#[test]
fn duplicate_json_keys_never_override_failure_into_pass() {
    let raw = native().to_string();
    let injected = raw.replacen("\"failed\":0", "\"failed\":2,\"failed\":0", 1);
    assert_ne!(injected, raw);
    let v = parse(
        "pctx-check-report-v1",
        &records(&injected),
        Some(0),
        "exited",
        true,
    );
    assert_eq!(v["parse_status"], "partial");
    assert_ne!(v["reported_result"], "passed");
}
#[test]
fn diagnostic_limit_keeps_total_counts_and_masks_messages() {
    let messages:Vec<_>=(0..200).map(|i|json!({"severity":2,"message":format!("error {i} token=abcdefghijklmno"),"line":i+1,"column":1})).collect();
    let v = parse_json(
        "eslint-json-v1",
        json!([{"filePath":"src/a.ts","errorCount":200,"warningCount":0,"fatalErrorCount":0,"messages":messages}]),
        1,
    );
    assert_eq!(v["summary"]["errors"], 200);
    assert_eq!(v["diagnostics"].as_array().unwrap().len(), 128);
    assert_eq!(v["groups_omitted"], 72);
    assert!(!v.to_string().contains("abcdefghijklmno"));
    assert_eq!(v["reported_result"], "failed");
}
#[test]
fn test_zero_all_skipped_and_observed_nonzero_never_pass() {
    for mode in ["zero", "skipped", "nonzero"] {
        let mut n = suite_report();
        if mode == "zero" {
            n["numTotalTests"] = json!(0);
            n["numPassedTests"] = json!(0);
            n["testResults"][0]["assertionResults"] = json!([]);
        }
        if mode == "skipped" {
            n["numPassedTests"] = json!(0);
            n["numPendingTests"] = json!(1);
            n["testResults"][0]["assertionResults"][0]["status"] = json!("pending");
        }
        let v = parse_json("vitest-json-v1", n, if mode == "nonzero" { 2 } else { 0 });
        assert_ne!(v["reported_result"], "passed");
        assert!(
            v["reasons"]
                .as_array()
                .unwrap()
                .contains(&json!("invalid_pass_claim"))
        );
    }
}

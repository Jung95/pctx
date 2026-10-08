//! Bounded presentation parsers. No execution, storage, policy authority or gate evidence.
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const INPUT_LIMIT: usize = 1024 * 1024;
const RECORD_LIMIT: usize = 8192;
const ITEM_LIMIT: usize = 128;
const TEXT_LIMIT: usize = 2048;

fn safe(text: &str) -> String {
    if text.len() > 8192 {
        return "[text omitted: parser bound]".into();
    }
    let (masked, _) = crate::reader::redact(text);
    let end = masked
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|i| *i <= TEXT_LIMIT)
        .last()
        .unwrap_or(0);
    if masked.len() <= TEXT_LIMIT {
        masked
    } else {
        format!("{}[excerpt omitted]", &masked[..end])
    }
}
fn number(v: &Value, key: &str) -> Result<u64, &'static str> {
    v.get(key)
        .and_then(Value::as_u64)
        .filter(|n| {
            *n <= if key.ends_with("Time") {
                10_000_000_000_000_000
            } else {
                1_000_000_000
            }
        })
        .ok_or("invalid_count")
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    v.get(key).and_then(Value::as_str).ok_or("missing_string")
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, &'static str> {
    v.get(key).and_then(Value::as_array).ok_or("missing_array")
}
fn preflight(s: &str) -> bool {
    let (mut depth, mut nodes, mut quoted, mut escape, mut string_start) =
        (0usize, 0usize, false, false, 0usize);
    let mut containers: Vec<Option<std::collections::BTreeSet<String>>> = vec![];
    for (i, b) in s.bytes().enumerate() {
        if quoted {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                quoted = false;
                if s[i + 1..].trim_start().starts_with(':') {
                    let Some(Some(keys)) = containers.last_mut() else {
                        return false;
                    };
                    let Ok(key) = serde_json::from_str::<String>(&s[string_start..=i]) else {
                        return false;
                    };
                    if !keys.insert(key) {
                        return false;
                    }
                }
            }
        } else {
            match b {
                b'"' => {
                    quoted = true;
                    string_start = i;
                    nodes += 1;
                }
                b'[' | b'{' => {
                    depth += 1;
                    nodes += 1;
                    containers.push(if b == b'{' {
                        Some(std::collections::BTreeSet::new())
                    } else {
                        None
                    });
                }
                b']' | b'}' => {
                    let Some(d) = depth.checked_sub(1) else {
                        return false;
                    };
                    depth = d;
                    containers.pop();
                }
                b',' | b':' => nodes += 1,
                _ => (),
            }
        }
        if depth > 64 || nodes > 8192 {
            return false;
        }
    }
    !quoted && depth == 0
}
struct Parsed {
    summary: Value,
    items: Vec<Value>,
    diagnostics: Vec<Value>,
    reasons: Vec<&'static str>,
    omitted: usize,
    result: &'static str,
    start: Instant,
}
impl Parsed {
    fn new() -> Self {
        Self {
            summary: json!({}),
            items: vec![],
            diagnostics: vec![],
            reasons: vec![],
            omitted: 0,
            result: "not_evaluated",
            start: Instant::now(),
        }
    }
    fn reason(&mut self, why: &'static str) {
        if !self.reasons.contains(&why) {
            self.reasons.push(why);
        }
    }
    fn push(&mut self, diagnostic: bool, item: Value) {
        let list = if diagnostic {
            &mut self.diagnostics
        } else {
            &mut self.items
        };
        if list.len() < ITEM_LIMIT {
            list.push(item);
        } else {
            self.omitted += 1;
        }
    }
    fn deadline(&self) -> Result<(), &'static str> {
        if self.start.elapsed() >= Duration::from_secs(1) {
            Err("parser_timeout")
        } else {
            Ok(())
        }
    }
}
fn report(p: &mut Parsed, v: &Value, exit: Option<i32>) -> Result<(), &'static str> {
    let r: crate::work::CheckReport =
        serde_json::from_value(v.clone()).map_err(|_| "invalid_native_report")?;
    if r.schema_version != 1
        || r.check_key.is_empty()
        || r.producer.is_empty()
        || r.finished_at < r.started_at
        || r.passed
            .checked_add(r.failed)
            .and_then(|n| n.checked_add(r.skipped))
            != Some(r.tests)
        || !["passed", "failed", "cancelled", "timed_out", "unverified"]
            .contains(&r.result.as_str())
    {
        return Err("invalid_native_report");
    }
    p.summary = json!({"tests":r.tests,"passed":r.passed,"failed":r.failed,"errors":r.errors,"skipped":r.skipped,"reported_result":safe(&r.result),"check_key":safe(&r.check_key),"producer":safe(&r.producer),"started_at":r.started_at,"finished_at":r.finished_at});
    if exit != Some(r.exit_code) {
        p.reason("exit_report_mismatch");
    }
    if r.result == "passed"
        && (r.exit_code != 0 || r.tests == 0 || r.passed == 0 || r.failed > 0 || r.errors > 0)
    {
        p.reason("invalid_pass_claim");
    }
    p.result = if r.failed > 0 || r.errors > 0 || r.exit_code != 0 {
        "failed"
    } else if p.reasons.is_empty() && r.result == "passed" {
        "passed"
    } else {
        "unverified"
    };
    Ok(())
}
fn eslint(p: &mut Parsed, v: &Value, exit: Option<i32>) -> Result<(), &'static str> {
    let files = v.as_array().ok_or("eslint_requires_json_array")?;
    let (mut errors, mut warnings, mut fatals) = (0u64, 0u64, 0u64);
    for f in files {
        p.deadline()?;
        let path = text(f, "filePath")?;
        let (e, w, fatal) = (
            number(f, "errorCount")?,
            number(f, "warningCount")?,
            number(f, "fatalErrorCount")?,
        );
        let (mut seen_e, mut seen_w, mut seen_fatal) = (0, 0, 0);
        for m in array(f, "messages")? {
            let severity = number(m, "severity")?;
            if ![1, 2].contains(&severity) {
                return Err("unknown_eslint_severity");
            }
            seen_e += u64::from(severity == 2);
            seen_w += u64::from(severity == 1);
            let is_fatal = m.get("fatal").and_then(Value::as_bool).unwrap_or(false);
            seen_fatal += u64::from(is_fatal);
            p.push(true,json!({"path":safe(path),"code":m.get("ruleId").and_then(Value::as_str).map(safe),"severity":if severity==2{"error"}else{"warning"},"message":safe(text(m,"message")?),"fatal":is_fatal,"line":m.get("line").and_then(Value::as_u64),"column":m.get("column").and_then(Value::as_u64),"end_line":m.get("endLine").and_then(Value::as_u64),"end_column":m.get("endColumn").and_then(Value::as_u64),"source_index":p.diagnostics.len()+p.omitted}));
        }
        if (seen_e, seen_w, seen_fatal) != (e, w, fatal) {
            p.reason("eslint_count_mismatch");
        }
        errors += e;
        warnings += w;
        fatals += fatal;
        p.push(false,json!({"path":safe(path),"errors":e,"warnings":w,"fatal_errors":fatal,"suppressed_messages":f.get("suppressedMessages").and_then(Value::as_array).map(Vec::len)}));
    }
    p.summary =
        json!({"files":files.len(),"errors":errors,"warnings":warnings,"fatal_errors":fatals});
    p.result = if errors > 0 || exit.is_some_and(|e| e != 0) {
        "failed"
    } else {
        "not_evaluated"
    };
    if errors > 0 && exit == Some(0) {
        p.reason("exit_report_mismatch");
    }
    Ok(())
}
fn tests(p: &mut Parsed, v: &Value, exit: Option<i32>) -> Result<(), &'static str> {
    let total = number(v, "numTotalTests")?;
    let passed = number(v, "numPassedTests")?;
    let failed = number(v, "numFailedTests")?;
    let skipped = number(v, "numPendingTests")?;
    let todo = number(v, "numTodoTests")?;
    let suites = number(v, "numTotalTestSuites")?;
    let suite_failed = number(v, "numFailedTestSuites")?;
    let suite_passed = number(v, "numPassedTestSuites")?;
    let suite_pending = number(v, "numPendingTestSuites")?;
    let success = v
        .get("success")
        .and_then(Value::as_bool)
        .ok_or("missing_final_result")?;
    number(v, "startTime")?;
    let files = array(v, "testResults")?;
    let (mut count_pass, mut count_fail, mut count_skip, mut count_todo, mut crashes, mut retries) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    for f in files {
        p.deadline()?;
        let path = text(f, "name")?;
        let status = text(f, "status")?;
        if !["passed", "failed", "skipped", "pending", "todo"].contains(&status) {
            return Err("unknown_suite_status");
        }
        number(f, "startTime")?;
        let end = number(f, "endTime")?;
        if end < number(f, "startTime")? {
            p.reason("invalid_suite_time");
        }
        let assertions = array(f, "assertionResults")?;
        let message = text(f, "message")?;
        let crash = f.get("testExecError").is_some_and(|e| !e.is_null())
            || (status == "failed" && assertions.is_empty());
        crashes += u64::from(crash);
        if !message.is_empty() || crash {
            p.push(true,json!({"path":safe(path),"severity":"error","kind":if crash{"suite_error"}else{"suite_message"},"message":safe(message)}));
        }
        for a in assertions {
            let state = text(a, "status")?;
            match state {
                "passed" => count_pass += 1,
                "failed" => count_fail += 1,
                "pending" | "skipped" | "disabled" => count_skip += 1,
                "todo" => count_todo += 1,
                _ => return Err("unknown_assertion_status"),
            }
            let title = text(a, "fullName")?;
            let failures = array(a, "failureMessages")?;
            if failures.iter().any(|f| !f.is_string()) {
                return Err("invalid_failure_messages");
            }
            let invocation = a.get("invocations").and_then(Value::as_u64).unwrap_or(1);
            let retry = a.get("retryCount").and_then(Value::as_u64).unwrap_or(0);
            let flaky = invocation > 1 || retry > 0 || a.get("flaky") == Some(&Value::Bool(true));
            retries += u64::from(flaky);
            if state != "passed" || !failures.is_empty() || flaky {
                p.push(true,json!({"path":safe(path),"kind":"assertion","name":safe(title),"status":state,"failure_messages":failures.iter().take(8).filter_map(Value::as_str).map(safe).collect::<Vec<_>>(),"failure_messages_omitted":failures.len().saturating_sub(8),"flaky":flaky,"invocations":invocation,"retry_count":retry,"location":a.get("location").and_then(|l|Some(json!({"line":l.get("line")?.as_u64()?,"column":l.get("column")?.as_u64()?})))}));
            }
            if state == "passed" && !failures.is_empty() {
                p.reason("passing_assertion_retains_failure");
            }
        }
        p.push(false,json!({"path":safe(path),"status":status,"assertions":assertions.len(),"start_time":f["startTime"],"end_time":f["endTime"]}));
    }
    if passed + failed + skipped + todo != total
        || (count_pass, count_fail, count_skip, count_todo) != (passed, failed, skipped, todo)
        || suite_passed + suite_failed + suite_pending != suites
        || files.len() as u64 > suites
    {
        p.reason("test_count_mismatch");
    }
    if success
        && (failed > 0
            || suite_failed > 0
            || crashes > 0
            || total == 0
            || passed == 0
            || exit != Some(0)
            || retries > 0)
    {
        p.reason("invalid_pass_claim");
    }
    if !success && exit == Some(0) {
        p.reason("exit_report_mismatch");
    }
    // The documented JSON variants do not prove that previous retry failures
    // were all retained. Counts are observations; no synthetic clean pass.
    p.reason("retry_history_not_certified");
    p.summary = json!({"tests":total,"passed":passed,"failed":failed,"skipped":skipped,"todo":todo,"suites":suites,"failed_suites":suite_failed,"suite_errors":crashes,"flaky_or_retried":retries,"final_success":success});
    p.result = if failed > 0 || suite_failed > 0 || crashes > 0 || exit.is_some_and(|e| e != 0) {
        "failed"
    } else {
        "unverified"
    };
    Ok(())
}
fn typescript(p: &mut Parsed, records: &[Value]) -> Result<(), &'static str> {
    let re = regex::Regex::new(
        r"^(?:(.+)\(([0-9]+),([0-9]+)\): )?(error|warning|message) TS([0-9]+): (.*)$",
    )
    .map_err(|_| "parser_unavailable")?;
    let (mut errors, mut warnings) = (0, 0);
    for r in records {
        p.deadline()?;
        for line in text(r, "text")?.lines() {
            if let Some(c) = re.captures(line) {
                errors += usize::from(&c[4] == "error");
                warnings += usize::from(&c[4] == "warning");
                p.push(true,json!({"path":c.get(1).map(|m|safe(m.as_str())),"line":c.get(2).and_then(|m|m.as_str().parse::<u64>().ok()),"column":c.get(3).and_then(|m|m.as_str().parse::<u64>().ok()),"severity":&c[4],"code":format!("TS{}",&c[5]),"message":safe(&c[6]),"record_ref":{"stream":r["stream"],"sequence":r["sequence"]}}));
            } else if !line.trim().is_empty() {
                p.reason("unparsed_typescript_line");
            }
        }
    }
    p.summary = json!({"errors":errors,"warnings":warnings});
    p.result = if errors > 0 {
        "failed"
    } else {
        "not_evaluated"
    };
    Ok(())
}
fn git_status(p: &mut Parsed, s: &str) -> Result<(), &'static str> {
    if !s.is_empty() && !s.ends_with('\0') {
        return Err("missing_nul_terminator");
    }
    let mut tokens = s.split_terminator('\0');
    let (mut changed, mut untracked, mut conflicts) = (0, 0, 0);
    while let Some(t) = tokens.next() {
        p.deadline()?;
        if t.len() < 4 || t.as_bytes()[2] != b' ' {
            return Err("invalid_status_record");
        }
        if !t.as_bytes()[..2].iter().all(|b| b" MADRCU?!T".contains(b)) {
            return Err("unknown_status_code");
        }
        let xy = &t[..2];
        let path = &t[3..];
        let old = if xy.contains('R') || xy.contains('C') {
            Some(tokens.next().ok_or("missing_rename_source")?)
        } else {
            None
        };
        changed += 1;
        untracked += usize::from(xy == "??");
        conflicts += usize::from(xy.contains('U') || xy == "AA" || xy == "DD");
        p.push(false,json!({"path":safe(path),"previous_path":old.map(safe),"index_status":&xy[..1],"worktree_status":&xy[1..],"conflict":xy.contains('U')||xy=="AA"||xy=="DD","untracked":xy=="??"}));
    }
    p.summary = json!({"changed_entries":changed,"untracked":untracked,"conflicts":conflicts,"head":"not_in_this_format"});
    Ok(())
}
fn git_log(p: &mut Parsed, s: &str) -> Result<(), &'static str> {
    if !s.is_empty() && !s.ends_with('\0') {
        return Err("missing_nul_terminator");
    }
    let tokens = s.split_terminator('\0').collect::<Vec<_>>();
    if !tokens.len().is_multiple_of(3) {
        return Err("invalid_log_fields");
    }
    for f in tokens.as_chunks::<3>().0 {
        p.deadline()?;
        let oid = f[0].trim_start_matches('\n');
        if ![40, 64].contains(&oid.len())
            || !oid.bytes().all(|c| c.is_ascii_hexdigit())
            || f[1]
                .split(' ')
                .filter(|s| !s.is_empty())
                .any(|s| ![40, 64].contains(&s.len()) || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err("invalid_commit_id");
        }
        p.push(false,json!({"commit":oid,"parents":f[1].split(' ').filter(|s|!s.is_empty()).collect::<Vec<_>>(),"subject":safe(f[2])}));
    }
    p.summary = json!({"commits":tokens.len()/3,"selected_fields":["commit","parents","subject"],"ref_names":"not_requested"});
    Ok(())
}
fn git_diff(p: &mut Parsed, s: &str) -> Result<(), &'static str> {
    let re = regex::Regex::new(r"^@@ -([0-9]+)(?:,([0-9]+))? \+([0-9]+)(?:,([0-9]+))? @@")
        .map_err(|_| "parser_unavailable")?;
    let mut file = json!({});
    let (mut files, mut hunks) = (0, 0);
    for line in s.lines() {
        p.deadline()?;
        if line.starts_with("diff --git ") {
            if file.as_object().is_some_and(|o| !o.is_empty()) {
                p.push(false, file);
            }
            file = json!({"kind":"diff","staging":"caller_binding_required","binary":false});
            files += 1;
            // Binary-only sections have no ---/+++; retain their original header
            // rather than guessing a whitespace-delimited filename.
            file["header"] = json!(safe(line));
        } else if let Some(path) = line.strip_prefix("--- ") {
            file["old_path"] = json!(safe(path.split('\t').next().unwrap_or(path)));
        } else if let Some(path) = line.strip_prefix("+++ ") {
            file["new_path"] = json!(safe(path.split('\t').next().unwrap_or(path)));
        } else if let Some(path) = line.strip_prefix("rename from ") {
            file["rename_from"] = json!(safe(path));
        } else if let Some(path) = line.strip_prefix("rename to ") {
            file["rename_to"] = json!(safe(path));
        } else if line.starts_with("deleted file mode ") {
            file["deleted"] = json!(true);
        } else if line.starts_with("new file mode ") {
            file["added"] = json!(true);
        } else if line.starts_with("Binary files ") || line == "GIT binary patch" {
            file["binary"] = json!(true);
        } else if let Some(c) = re.captures(line) {
            hunks += 1;
            p.push(true,json!({"kind":"hunk","old_path":file["old_path"],"new_path":file["new_path"],"original_header":safe(line),"old_start":c[1].parse::<u64>().map_err(|_|"invalid_hunk_range")?,"old_count":c.get(2).map_or(Ok(1),|m|m.as_str().parse::<u64>()).map_err(|_|"invalid_hunk_range")?,"new_start":c[3].parse::<u64>().map_err(|_|"invalid_hunk_range")?,"new_count":c.get(4).map_or(Ok(1),|m|m.as_str().parse::<u64>()).map_err(|_|"invalid_hunk_range")?}));
        } else if line.starts_with("@@") {
            p.reason("unsupported_combined_or_malformed_hunk");
        }
    }
    if file.as_object().is_some_and(|o| !o.is_empty()) {
        p.push(false, file);
    }
    if !s.is_empty() && files == 0 {
        return Err("not_unified_diff");
    }
    p.summary = json!({"files":files,"hunks":hunks,"patch_usable":false,"base_head":"caller_binding_required"});
    p.reason("hunk_bodies_not_validated");
    Ok(())
}

/// The caller selects an identity from registered executable/reporter metadata.
/// `records` contain already normalized/masked text, stream and sequence. No
/// permission or report authenticity is inferred from identity or child text.
pub fn parse(
    identity: &str,
    records: &[Value],
    child_exit: Option<i32>,
    termination: &str,
    capture_complete: bool,
) -> Value {
    let mut p = Parsed::new();
    let mut bytes = 0usize;
    let mut valid = true;
    for r in records.iter().take(RECORD_LIMIT + 1) {
        let Some(t) = r.get("text").and_then(Value::as_str) else {
            valid = false;
            break;
        };
        bytes = bytes.saturating_add(t.len() + 1);
        if bytes > INPUT_LIMIT {
            valid = false;
            break;
        }
        if ![Some("stdout"), Some("stderr")].contains(&r.get("stream").and_then(Value::as_str))
            || r.get("sequence").and_then(Value::as_u64).is_none()
        {
            valid = false;
            break;
        }
    }
    if records.len() > RECORD_LIMIT || !valid {
        p.reason("parser_input_budget_or_record_shape");
    } else {
        let nul = identity.starts_with("git-status-") || identity == "git-log-nul-v1";
        let stdout = records
            .iter()
            .filter(|r| r["stream"] == "stdout")
            .map(|r| r["text"].as_str().unwrap())
            .collect::<Vec<_>>()
            .join(if nul { "" } else { "\n" });
        let parsed = match identity {
            "typescript-text-v1" => typescript(&mut p, records),
            "git-status-porcelain-v1-z" => git_status(&mut p, &stdout),
            "git-log-nul-v1" => git_log(&mut p, &stdout),
            "git-diff-unified-v1" => git_diff(&mut p, &stdout),
            "pctx-check-report-v1" | "eslint-json-v1" | "jest-json-v1" | "vitest-json-v1" => {
                if !preflight(&stdout) {
                    Err("json_complexity_or_incomplete")
                } else {
                    match serde_json::from_str::<Value>(&stdout) {
                        Ok(v) => match identity {
                            "pctx-check-report-v1" => report(&mut p, &v, child_exit),
                            "eslint-json-v1" => eslint(&mut p, &v, child_exit),
                            _ => tests(&mut p, &v, child_exit),
                        },
                        Err(_) => Err("malformed_json_report"),
                    }
                }
            }
            _ => Err("unsupported_parser_identity"),
        };
        if let Err(reason) = parsed {
            p.reason(reason);
            p.result = "unverified";
        }
    }
    if !capture_complete {
        p.reason("capture_incomplete");
    }
    if termination != "exited" {
        p.reason("execution_not_normally_exited");
    }
    if child_exit.is_none() {
        p.reason("child_exit_unknown");
    }
    if !p.reasons.is_empty() && p.result == "passed" {
        p.result = "unverified";
    }
    let excerpts = if !p.reasons.is_empty() {
        records.iter().take(3).chain(records.iter().skip(records.len().saturating_sub(3)).filter(|_|records.len()>3)).filter_map(|r|r.get("text").and_then(Value::as_str).map(|t|json!({"stream":r.get("stream").and_then(Value::as_str),"sequence":r.get("sequence").and_then(Value::as_u64),"text":safe(t)}))).collect::<Vec<_>>()
    } else {
        vec![]
    };
    json!({"schema_version":1,"parser":safe(identity),"parse_status":if p.reasons.contains(&"unsupported_parser_identity"){"unsupported"}else if p.reasons.is_empty(){"complete"}else{"partial"},"reasons":p.reasons,"summary":p.summary,"items":p.items,"diagnostics":p.diagnostics,"groups_omitted":p.omitted,"excerpt_records":excerpts,"record_refs":records.iter().take(ITEM_LIMIT).map(|r|json!({"stream":r.get("stream").and_then(Value::as_str).filter(|s|["stdout","stderr"].contains(s)),"sequence":r.get("sequence").and_then(Value::as_u64)})).collect::<Vec<_>>(),"record_refs_omitted":records.len().saturating_sub(ITEM_LIMIT),"input_records":records.len(),"input_bytes_examined":bytes,"parser_input_limit_bytes":INPUT_LIMIT,"gate_evidence":false,"reported_result":p.result,"child_exit_code":child_exit,"termination":safe(termination),"capture_complete":capture_complete,"command_rerun":false,"report_origin":"child_output_claim"})
}

use serde_json::Value;
use std::{
    fs,
    process::{Command, Output},
};
struct Fixture {
    _temp: tempfile::TempDir,
    root: std::path::PathBuf,
    data: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let data = temp.path().join("data");
        fs::create_dir(&root).unwrap();
        Self {
            _temp: temp,
            root,
            data,
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .args(["--root", self.root.to_str().unwrap(), "--format", "json"])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> Value {
        let o = self.run(args);
        assert!(
            o.status.success(),
            "{:?}: {} {}",
            args,
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).unwrap()
    }
}
#[test]
fn non_git_exploration_changes_and_idempotent_init() {
    let f = Fixture::new();
    fs::write(
        f.root.join("auth.ts"),
        "export function login(name: string) {\n return name;\n}\n",
    )
    .unwrap();
    let init = f.ok(&["init"]);
    let again = f.ok(&["init"]);
    assert_eq!(init["project_id"], again["project_id"]);
    assert_eq!(init["workspace_id"], again["workspace_id"]);
    f.ok(&["index", "update"]);
    let found = f.ok(&["find", "login", "--kind", "symbol"]);
    assert_eq!(found["data"]["items"][0]["path"], "auth.ts");
    let outline = f.ok(&["outline", "auth.ts"]);
    let _ = outline;
    let cp = f.ok(&["checkpoint", "create", "--name", "before"]);
    assert!(cp["data"]["id"].is_string());
    fs::write(f.root.join("auth.ts"), "export function logout() {}\n").unwrap();
    let stale = f.run(&["outline", "auth.ts"]);
    assert_eq!(stale.status.code(), Some(4));
    let changes = f.ok(&["changes", "--since", "before"]);
    assert!(
        changes["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["path"] == "auth.ts" && x["change"] == "modified")
    );
    f.ok(&["index", "update"]);
    let old = f.ok(&["find", "login", "--kind", "symbol"]);
    assert!(old["data"]["items"].as_array().unwrap().is_empty());
}
#[test]
fn security_applies_to_direct_reads_and_cached_metadata() {
    let f = Fixture::new();
    fs::write(f.root.join(".env"), "SECRET=do-not-expose").unwrap();
    fs::write(f.root.join("safe.py"), "def safe():\n    return 1\n").unwrap();
    f.ok(&["init"]);
    f.ok(&["index", "update"]);
    for path in ["../outside", ".env"] {
        let o = f.run(&["read", path]);
        assert_eq!(o.status.code(), Some(5));
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["status"], "error");
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc/passwd", f.root.join("linked")).unwrap();
        assert_eq!(f.run(&["read", "linked"]).status.code(), Some(5));
    }
    let config = f.root.join(".pctx/config.toml");
    let s = fs::read_to_string(&config)
        .unwrap()
        .replace("exclude = []", "exclude = [\"safe.py\"]");
    fs::write(config, s).unwrap();
    let found = f.ok(&["find", "safe", "--kind", "symbol", "--freshness", "off"]);
    assert!(found["data"]["items"].as_array().unwrap().is_empty());
    assert_eq!(f.run(&["read", "safe.py"]).status.code(), Some(5));
}
#[test]
fn context_budget_is_exact_json_and_required_rules_are_not_cut() {
    let f = Fixture::new();
    fs::write(f.root.join("auth.ts"), "function auth() { return true; }\n").unwrap();
    f.ok(&["init"]);
    let a = f.run(&[
        "build",
        "--task",
        "fix auth",
        "--seed",
        "auth.ts",
        "--budget-bytes",
        "2000",
    ]);
    assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stdout));
    assert!(a.stdout.len() <= 2000);
    let av: Value = serde_json::from_slice(&a.stdout).unwrap();
    let b = f.ok(&[
        "build",
        "--task",
        "fix auth",
        "--seed",
        "auth.ts",
        "--budget-bytes",
        "2000",
    ]);
    assert_eq!(av["data"], b["data"]);
    fs::create_dir_all(f.root.join(".pctx/rules")).unwrap();
    fs::write(
        f.root.join(".pctx/rules/required.md"),
        format!(
            "---\nschema_version: 1\nid: required\nscope: ['**']\nrequired: true\n---\n{}",
            "Mandatory: ".repeat(500)
        ),
    )
    .unwrap();
    let o = f.run(&["build", "--task", "fix auth", "--budget-bytes", "2000"]);
    assert_eq!(o.status.code(), Some(8));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["errors"][0]["code"], "BUDGET_TOO_SMALL");
}
#[test]
fn invalid_json_cli_inputs_still_return_one_json_document() {
    let f = Fixture::new();
    let o = f.run(&["find", "--does-not-exist"]);
    assert_eq!(o.status.code(), Some(2));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["errors"][0]["code"], "INVALID_ARGUMENT");
}

#[test]
fn ndjson_snapshot_follow_and_resume_have_no_event_gap() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::time::Duration;
    let f = Fixture::new();
    f.ok(&["init"]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_pctx"))
        .args([
            "--root",
            f.root.to_str().unwrap(),
            "--format",
            "ndjson",
            "board",
            "--watch",
        ])
        .env("PCTX_DATA_DIR", &f.data)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let snapshot = rx.recv_timeout(Duration::from_secs(5));
    if snapshot.is_err() {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    let snapshot: Value = serde_json::from_str(&snapshot.unwrap()).unwrap();
    assert_eq!(snapshot["type"], "board_snapshot");
    let cursor = snapshot["data"]["as_of_seq"].as_i64().unwrap();
    f.ok(&[
        "agent",
        "register",
        "--name",
        "stream-worker",
        "--kind",
        "agent",
    ]);
    let event = rx.recv_timeout(Duration::from_secs(5));
    child.kill().unwrap();
    child.wait().unwrap();
    reader.join().unwrap();
    let event: Value = serde_json::from_str(&event.unwrap()).unwrap();
    assert!(event["event_seq"].as_i64().unwrap() > cursor);
    assert_eq!(event["event_namespace"], "work");
    let resumed = Command::new(env!("CARGO_BIN_EXE_pctx"))
        .args([
            "--root",
            f.root.to_str().unwrap(),
            "--format",
            "ndjson",
            "activity",
            "--since-seq",
            &cursor.to_string(),
        ])
        .env("PCTX_DATA_DIR", &f.data)
        .output()
        .unwrap();
    assert!(resumed.status.success());
    let lines: Vec<Value> = String::from_utf8(resumed.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines[0]["event_seq"], event["event_seq"]);
    assert_eq!(f.run(&["board", "--watch"]).status.code(), Some(2));
    assert_eq!(
        f.run(&["activity", "--since-seq", "-1"]).status.code(),
        Some(2)
    );
}

#[test]
fn inventory_cli_reports_static_candidates_without_execution_or_authority() {
    let f = Fixture::new();
    f.ok(&["init"]);
    fs::write(
        f.root.join("package.json"),
        r#"{"name":"fixture","scripts":{"test":"touch must-not-run"},"engines":{"node":">=22"}}"#,
    )
    .unwrap();
    let scan = f.ok(&[
        "inventory",
        "scan",
        "--max-files",
        "10",
        "--max-bytes",
        "4096",
    ]);
    assert!(
        scan["data"]["check_candidates"]
            .as_array()
            .is_some_and(|v| !v.is_empty()),
        "{scan}"
    );
    assert!(!f.root.join("must-not-run").exists());
    assert_eq!(
        f.run(&["resource", "status", "--host", "remote"])
            .status
            .code(),
        Some(2)
    );
    let unavailable = f.run(&["job", "cancel", "missing-job"]);
    let result: Value = serde_json::from_slice(&unavailable.stdout).unwrap();
    assert!(!unavailable.status.success());
    assert_eq!(result["command"], "job");
}

#[test]
fn markdown_is_a_safe_document_and_unsupported_formats_precede_writes() {
    let f = Fixture::new();
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .args(["--root", f.root.to_str().unwrap(), "--format", "markdown"])
            .args(args)
            .env("PCTX_DATA_DIR", &f.data)
            .output()
            .unwrap()
    };
    let denied = invoke(&["init"]);
    assert_eq!(denied.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&denied.stdout).unwrap()["errors"][0]["code"],
        "INVALID_ARGUMENT"
    );
    assert!(!f.root.join(".pctx").exists());
    f.ok(&["init"]);
    fs::write(
        f.root.join("source.py"),
        "def hello():\n    return '```'\n# ghp_abcdefghijklmnop123456789\n",
    )
    .unwrap();
    f.ok(&["index", "update"]);
    let result = invoke(&["read", "source.py", "--lines", "1:3"]);
    assert_eq!(result.status.code(), Some(0));
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.starts_with("# PCTX read"));
    assert!(text.contains("def hello()"));
    assert!(text.contains("Complete envelope metadata"));
    assert!(!text.contains("ghp_abcdefghijklmnop123456789"));
    assert!(text.contains("[REDACTED]"));
    assert!(text.len() <= 65536);
    fs::create_dir_all(f.root.join(".pctx/rules")).unwrap();
    fs::write(
        f.root.join(".pctx/rules/required.md"),
        format!(
            "---\nschema_version: 1\nid: required\nscope: ['**']\nrequired: true\n---\n{}",
            "Mandatory: ".repeat(500)
        ),
    )
    .unwrap();
    let oversized = invoke(&["build", "--task", "inspect hello", "--budget-bytes", "2000"]);
    assert_eq!(oversized.status.code(), Some(8));
    assert!(oversized.stdout.len() <= 2000);
    let error: Value = serde_json::from_slice(&oversized.stdout).unwrap();
    assert_eq!(error["errors"][0]["code"], "BUDGET_TOO_SMALL");
}

#[test]
fn invalid_project_and_merged_user_policy_never_become_successful_empty_results() {
    let f = Fixture::new();
    fs::write(f.root.join("auth.py"), "def auth():\n    return True\n").unwrap();
    f.ok(&["init"]);
    f.ok(&["index", "update"]);
    let config = f.root.join(".pctx/config.toml");
    let original = fs::read_to_string(&config).unwrap();
    fs::write(
        &config,
        original.replace("exclude = []", "exclude = [\"[\"]"),
    )
    .unwrap();
    for args in [
        vec!["find", "auth", "--kind", "symbol", "--freshness", "matched"],
        vec!["find", "auth", "--kind", "symbol", "--freshness", "off"],
        vec!["find", "auth", "--kind", "symbol", "--freshness", "strict"],
        vec!["outline", "auth.py"],
        vec!["query", "--language", "python"],
    ] {
        let output = f.run(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["errors"][0]["code"], "INVALID_CONFIG");
        assert_eq!(value["status"], "error");
    }
    fs::write(&config, &original).unwrap();
    let user = f._temp.path().join("invalid-user.toml");
    fs::write(&user, "[policy]\nexclude = ['[']\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pctx"))
        .args([
            "--root",
            f.root.to_str().unwrap(),
            "find",
            "auth",
            "--kind",
            "symbol",
        ])
        .env("PCTX_DATA_DIR", &f.data)
        .env("PCTX_USER_CONFIG", &user)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["errors"][0]["code"],
        "INVALID_CONFIG"
    );
    fs::write(
        &config,
        original.replace("exclude = []", "exclude = [\"auth.py\"]"),
    )
    .unwrap();
    assert!(
        f.ok(&["find", "auth", "--kind", "symbol"])["data"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn strict_lookup_retains_available_evidence_and_reports_incomplete_refresh() {
    let f = Fixture::new();
    fs::write(f.root.join("auth.py"), "def auth():\n    return True\n").unwrap();
    fs::write(f.root.join("broken.py"), "def broken():\n    pass\n").unwrap();
    f.ok(&["init"]);
    f.ok(&["index", "update"]);
    fs::write(f.root.join("broken.py"), [0xff, 0xfe]).unwrap();
    for args in [
        vec!["find", "auth", "--kind", "symbol", "--freshness", "strict"],
        vec!["query", "--language", "python", "--freshness", "strict"],
        vec!["outline", "auth.py", "--freshness", "strict"],
    ] {
        let output = f.run(&args);
        assert_eq!(
            output.status.code(),
            Some(3),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["status"], "partial");
        assert_eq!(value["data"]["coverage"]["status"], "partial");
        assert!(value["data"]["omitted_count"].is_null());
        assert_eq!(value["data"]["refresh"]["skipped_count"], 1);
        assert!(
            value["data"]["refresh"]["reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "UNSUPPORTED_ENCODING")
        );
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("auth.py")
        );
    }
}

fn invalid_capacity(out: &Output) -> usize {
    assert_eq!(
        out.status.code(),
        Some(2),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let documents = serde_json::Deserializer::from_slice(&out.stdout)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(out.stdout.last(), Some(&b'\n'));
    let value = &documents[0];
    assert_eq!(value["status"], "error");
    assert_eq!(value["errors"][0]["code"], "INVALID_ARGUMENT");
    let minimum = value["data"]["minimum_budget_bytes"].as_u64().unwrap() as usize;
    assert!(
        (500..1024).contains(&minimum),
        "This capacity guard must not impose the separate execution metadata minimum"
    );
    assert!(out.stdout.len() <= minimum);
    minimum
}
#[test]
fn minimum_error_budget_boundary_is_argument_validation_before_project_access() {
    let f = Fixture::new();
    let first = f.run(&["build", "--task", "inspect", "--budget-bytes", "1"]);
    let minimum = invalid_capacity(&first);
    assert!(!f.root.join(".pctx").exists());
    assert!(!f.data.exists());
    let below = (minimum - 1).to_string();
    assert_eq!(
        invalid_capacity(&f.run(&["build", "--task", "inspect", "--budget-bytes", &below])),
        minimum
    );
    let at = minimum.to_string();
    let accepted = f.run(&["build", "--task", "inspect", "--budget-bytes", &at]);
    let value: Value = serde_json::from_slice(&accepted.stdout).unwrap();
    // A valid capacity now reaches ordinary project validation, without initializing it.
    assert_eq!(accepted.status.code(), Some(6));
    assert_eq!(value["errors"][0]["code"], "NOT_INITIALIZED");
    assert!(accepted.stdout.len() <= minimum);
    assert!(!f.root.join(".pctx").exists());
    assert!(!f.data.exists());
}
#[test]
fn every_stdout_budget_route_rejects_tiny_capacity_without_child_or_local_files() {
    let f = Fixture::new();
    let marker = f.root.join("unexpected-child.txt");
    let script = f.root.join("marker.sh");
    fs::write(&script, format!("printf child > '{}'\n", marker.display())).unwrap();
    let cases = vec![
        vec![
            "context",
            "get",
            "--task-id",
            "missing",
            "--session",
            "missing",
            "--budget-bytes",
            "1",
        ],
        vec![
            "run",
            "--budget-bytes",
            "1",
            "--",
            "/bin/sh",
            script.to_str().unwrap(),
        ],
        vec![
            "runner",
            "check-run",
            "--task-id",
            "missing",
            "--key",
            "unit",
            "--run",
            "missing",
            "--budget-bytes",
            "1",
        ],
        vec![
            "check",
            "run",
            "unit",
            "--task-id",
            "missing",
            "--run",
            "missing",
            "--budget-bytes",
            "1",
        ],
        vec![
            "check",
            "run",
            "--key",
            "unit",
            "--task-id",
            "missing",
            "--run",
            "missing",
            "--budget-bytes",
            "1",
        ],
        vec![
            "runner",
            "helper-request",
            "--task-id",
            "missing",
            "--key",
            "unit",
            "--run",
            "missing",
            "--budget-bytes",
            "1",
        ],
        vec![
            "extract",
            "--path",
            "marker.sh",
            "--line",
            "1",
            "--budget-bytes",
            "1",
        ],
    ];
    for args in cases {
        invalid_capacity(&f.run(&args));
        assert!(
            !f.root.join(".pctx").exists(),
            "{args:?} initialized project metadata"
        );
        assert!(!f.data.exists(), "{args:?} created private execution state");
        assert!(!marker.exists(), "{args:?} launched the child");
    }
    let output = f.root.join("rejected-output.json");
    let rejected = f.run(&[
        "--output",
        output.to_str().unwrap(),
        "build",
        "--task",
        "inspect",
        "--budget-bytes",
        "1",
    ]);
    invalid_capacity(&rejected);
    assert!(!output.exists());
}
fn logical_database_state(path: &std::path::Path) -> Vec<(String, Vec<Vec<String>>)> {
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let tables: Vec<String> = {
        let mut s = db
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        s.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    tables
        .into_iter()
        .map(|table| {
            let name = table.replace('"', "\"\"");
            let mut s = db
                .prepare(&format!("SELECT * FROM \"{name}\" ORDER BY rowid"))
                .unwrap();
            let columns = s.column_count();
            let rows = s
                .query_map([], |row| {
                    (0..columns)
                        .map(|col| row.get_ref(col).map(|v| format!("{v:?}")))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (table, rows)
        })
        .collect()
}
#[test]
fn tiny_budgets_preserve_index_lease_checks_and_context_receipt_ledgers() {
    let f = Fixture::new();
    fs::write(f.root.join("source.py"), "def safe():\n    return 1\n").unwrap();
    f.ok(&["init"]);
    f.ok(&["index", "update"]);
    let agent = f.ok(&["agent", "register", "--name", "budget-worker"])["data"]["agent_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let definition = f.root.join("task.json");
    fs::write(&definition,serde_json::json!({"schema_version":1,"title":"Budget admission fixture","scope":["source.py"],"acceptance":[{"id":"behavior","description":"Observed test evidence","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
    let task = f.ok(&[
        "task",
        "create",
        "--from-file",
        definition.to_str().unwrap(),
    ])["data"]["task_id"]
        .as_str()
        .unwrap()
        .to_owned();
    f.ok(&["task", "ready", &task]);
    f.ok(&["task", "assign", &task, "--agent", &agent]);
    let run = f.ok(&["task", "start", &task])["data"]["run_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let session = f.ok(&[
        "session",
        "attach",
        "--agent",
        &agent,
        "--runtime",
        "manual",
    ])["data"]["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let context = f.ok(&[
        "context",
        "get",
        "--task-id",
        &task,
        "--session",
        &session,
        "--budget-bytes",
        "6000",
    ])["data"]["context_id"]
        .as_str()
        .unwrap()
        .to_owned();
    f.ok(&[
        "context",
        "ack",
        &context,
        "--session",
        &session,
        "--epoch",
        "1",
    ]);
    let status = f.ok(&["status"]);
    let index =
        std::path::PathBuf::from(status["data"]["local_storage"]["index"].as_str().unwrap());
    let control =
        std::path::PathBuf::from(status["data"]["local_storage"]["control"].as_str().unwrap());
    let index_before = logical_database_state(&index);
    let control_before = logical_database_state(&control);
    // Changed current source would force strict build publication if it were admitted.
    fs::write(f.root.join("source.py"), "def changed():\n    return 2\n").unwrap();
    for args in [
        vec![
            "build",
            "--task",
            "inspect",
            "--seed",
            "source.py",
            "--budget-bytes",
            "1",
        ],
        vec![
            "context",
            "get",
            "--task-id",
            &task,
            "--session",
            &session,
            "--budget-bytes",
            "1",
        ],
        vec![
            "check",
            "run",
            "unit",
            "--task-id",
            &task,
            "--run",
            &run,
            "--budget-bytes",
            "1",
        ],
        vec![
            "runner",
            "check-run",
            "--task-id",
            &task,
            "--key",
            "unit",
            "--run",
            &run,
            "--budget-bytes",
            "1",
        ],
        vec![
            "run",
            "--task-id",
            &task,
            "--session",
            &session,
            "--budget-bytes",
            "1",
            "--",
            "/bin/echo",
            "not-executed",
        ],
    ] {
        invalid_capacity(&f.run(&args));
        assert_eq!(
            logical_database_state(&index),
            index_before,
            "Index changed for {args:?}"
        );
        assert_eq!(
            logical_database_state(&control),
            control_before,
            "Lease/check/context ledger changed for {args:?}"
        );
    }
}
#[test]
fn valid_minimum_capacity_with_oversize_mandatory_rule_keeps_budget_exit_eight() {
    let f = Fixture::new();
    f.ok(&["init"]);
    fs::create_dir_all(f.root.join(".pctx/rules")).unwrap();
    fs::write(
        f.root.join(".pctx/rules/mandatory.md"),
        format!(
            "---\nschema_version: 1\nid: mandatory\nscope: ['**']\nrequired: true\n---\n{}",
            "Required rule. ".repeat(1000)
        ),
    )
    .unwrap();
    let minimum = invalid_capacity(&f.run(&["build", "--task", "inspect", "--budget-bytes", "1"]));
    let result = f.run(&[
        "build",
        "--task",
        "inspect",
        "--budget-bytes",
        &minimum.to_string(),
    ]);
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result.status.code(), Some(8));
    assert_eq!(value["errors"][0]["code"], "BUDGET_TOO_SMALL");
    assert!(result.stdout.len() <= minimum);
    assert!(value["data"].get("items").is_none());
}
#[test]
fn invalid_capacity_is_json_even_for_markdown_and_never_writes_output() {
    let f = Fixture::new();
    for format in ["compact", "markdown"] {
        let target = f.root.join(format!("invalid-{format}.md"));
        let out = Command::new(env!("CARGO_BIN_EXE_pctx"))
            .args([
                "--root",
                f.root.to_str().unwrap(),
                "--format",
                format,
                "--output",
                target.to_str().unwrap(),
                "build",
                "--task",
                "inspect",
                "--budget-bytes",
                "1",
            ])
            .env("PCTX_DATA_DIR", &f.data)
            .output()
            .unwrap();
        invalid_capacity(&out);
        assert!(!target.exists());
        assert!(!f.data.exists());
        assert!(!f.root.join(".pctx").exists());
    }
}

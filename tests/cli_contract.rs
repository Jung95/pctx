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

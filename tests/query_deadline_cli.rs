use fs2::FileExt;
use serde_json::Value;
use std::{
    fs,
    process::Command,
    time::{Duration, Instant},
};

fn command(root: &std::path::Path, data: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pctx"));
    command
        .args(["--format", "json", "--root"])
        .arg(root)
        .env("PCTX_DATA_DIR", data);
    command
}

#[test]
fn invalid_or_inapplicable_timeout_has_no_project_effects() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("missing");
    let data = temp.path().join("data");
    for args in [
        vec!["--timeout-ms", "0", "find", "needle"],
        vec!["find", "needle", "--timeout-ms", "0"],
        vec!["init", "--timeout-ms", "100"],
        vec![
            "run",
            "--timeout-ms",
            "100",
            "--",
            "/bin/sh",
            "-c",
            "exit 0",
        ],
    ] {
        let output = command(&root, &data).args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["errors"][0]["code"], "INVALID_ARGUMENT");
        assert!(!root.exists());
        assert!(!data.exists());
    }
}

#[test]
fn strict_find_waits_only_remaining_budget_and_preserves_active_generation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let data = temp.path().join("data");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("source.py"), "def needle(): pass\n").unwrap();
    for args in [vec!["init"], vec!["index", "update"]] {
        let output = command(&root, &data).args(args).output().unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    let workspace = fs::read_dir(data.join("workspaces"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let db = rusqlite::Connection::open(workspace.join("index.sqlite3")).unwrap();
    let generation = || {
        db.query_row("SELECT active_generation_id FROM workspace_meta", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap()
    };
    let before = generation();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(workspace.join("writer.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    fs::write(root.join("source.py"), "def replacement(): pass\n").unwrap();
    let start = Instant::now();
    let output = command(&root, &data)
        .args([
            "find",
            "needle",
            "--freshness",
            "strict",
            "--timeout-ms",
            "120",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["errors"][0]["code"], "TIMEOUT");
    assert!(start.elapsed() < Duration::from_secs(3));
    assert_eq!(generation(), before);
    assert_eq!(
        fs::read_to_string(root.join("source.py")).unwrap(),
        "def replacement(): pass\n"
    );
}

fn initialized() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("source.py"), "def stable(): return 1\n").unwrap();
    for args in [["init"].as_slice(), ["index", "update"].as_slice()] {
        let output = command(&root, &data).args(args).output().unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    (temp, root, data)
}
#[test]
fn finite_saved_routes_accept_timeout_and_reject_zero_before_project_effects() {
    let (_temp, root, data) = initialized();
    fs::create_dir_all(root.join(".pctx/handoffs")).unwrap();
    fs::write(
        root.join(".pctx/handoffs/saved.md"),
        "---\n{}\n---\nSaved 한글\n",
    )
    .unwrap();
    for args in [
        vec!["handoff", "show", "saved"],
        vec!["output", "show", "OUT-missing"],
        vec!["output", "find", "OUT-missing", "--literal", "needle"],
        vec!["output", "render", "OUT-missing"],
        vec!["savings", "report"],
        vec!["savings", "opportunities"],
    ] {
        let output = command(&root, &data)
            .args(&args)
            .args(["--timeout-ms", "5000"])
            .output()
            .unwrap();
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_ne!(
            response["errors"][0]["code"], "INVALID_ARGUMENT",
            "{args:?}: {output:?}"
        );
        let missing = root.join("absent");
        let untouched = data.join("untouched");
        let output = command(&missing, &untouched)
            .args(&args)
            .args(["--timeout-ms", "0"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        assert!(!missing.exists());
        assert!(!untouched.exists());
    }
    let output = command(&root, &data)
        .args(["handoff", "show", "saved", "--timeout-ms", "5000"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8(output.stdout).unwrap().contains("한글"));
}
#[test]
fn doctor_contention_reports_timeout_with_original_budget_and_unchanged_database() {
    let (_temp, root, data) = initialized();
    let workspace = fs::read_dir(data.join("workspaces"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let db = rusqlite::Connection::open(workspace.join("index.sqlite3")).unwrap();
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
        .unwrap();
    let before: String = db
        .query_row("SELECT active_generation_id FROM workspace_meta", [], |r| {
            r.get(0)
        })
        .unwrap();
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let started = Instant::now();
    let output = command(&root, &data)
        .args(["doctor", "--timeout-ms", "150"])
        .output()
        .unwrap();
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    assert_eq!(response["errors"][0]["code"], "TIMEOUT", "{output:?}");
    assert!(started.elapsed() < Duration::from_secs(3));
    db.execute_batch("ROLLBACK").unwrap();
    let after: String = db
        .query_row("SELECT active_generation_id FROM workspace_meta", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(before, after);
}
#[test]
fn finite_ndjson_contention_preserves_error_code_without_creating_events() {
    let (_temp, root, data) = initialized();
    let output = command(&root, &data).arg("board").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let control = fs::read_dir(data.join("controls"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let db = rusqlite::Connection::open(control.join("control.sqlite3")).unwrap();
    let before: i64 = db
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(control.join("connection-init.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    // Select NDJSON once, rather than duplicate a clap option in the common helper.
    let output = Command::new(env!("CARGO_BIN_EXE_pctx"))
        .args(["--root"])
        .arg(&root)
        .args(["--format", "ndjson", "activity", "--timeout-ms", "150"])
        .env("PCTX_DATA_DIR", &data)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["data"]["code"], "TIMEOUT");
    assert!(output.stdout.is_empty());
    FileExt::unlock(&lock).unwrap();
    let after: i64 = db
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, after);
}
#[cfg(unix)]
#[test]
fn saved_cli_reread_never_reruns_and_failed_metering_preserves_delivery_truth() {
    use std::os::unix::fs::PermissionsExt;
    let (_temp, root, data) = initialized();
    let emitter = root.join("saved-fixture-emitter");
    fs::write(
        &emitter,
        "#!/bin/sh\nprintf x >> invocation-count\nprintf '%s\\n' 'warning saved 한글 🦀'\n",
    )
    .unwrap();
    fs::set_permissions(&emitter, fs::Permissions::from_mode(0o700)).unwrap();
    let plan = command(&root, &data)
        .args(["trust", "plan", "--"])
        .arg(&emitter)
        .env("PCTX_ACTOR", "owner")
        .output()
        .unwrap();
    assert!(plan.status.success(), "{plan:?}");
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    let trust = command(&root, &data)
        .args([
            "trust",
            "add",
            "--expect-hash",
            plan["data"]["fingerprint"].as_str().unwrap(),
            "--",
        ])
        .arg(&emitter)
        .env("PCTX_ACTOR", "owner")
        .output()
        .unwrap();
    assert!(trust.status.success(), "{trust:?}");
    let run = command(&root, &data)
        .args(["run", "--"])
        .arg(&emitter)
        .env("PCTX_ACTOR", "owner")
        .output()
        .unwrap();
    assert!(run.status.success(), "{run:?}");
    let run: Value = serde_json::from_slice(&run.stdout).unwrap();
    let id = run["data"]["output_id"].as_str().unwrap();
    fs::remove_file(&emitter).unwrap();
    let workspace = fs::read_dir(data.join("workspaces"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name();
    let artifact = data
        .join("outputs")
        .join(workspace)
        .join(format!("{id}.json"));
    let before = fs::read(&artifact).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(data.join("output-metrics.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    for args in [
        vec!["output", "show", id, "--view", "full"],
        vec!["output", "find", id, "--literal", "한글"],
        vec!["output", "render", id, "--filter", "builtin"],
    ] {
        let output = command(&root, &data)
            .args(args)
            .args(["--timeout-ms", "5000"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(response.to_string().contains("한글"));
        assert_eq!(response["data"]["command_rerun"], false);
        let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(diagnostic["code"], "OUTPUT_MEASUREMENT_UNRECORDED");
        assert_eq!(diagnostic["delivery_written"], true);
        assert_eq!(diagnostic["measurement_recorded"], "unknown");
        assert!(!String::from_utf8_lossy(&output.stderr).contains(root.to_str().unwrap()));
        assert_eq!(fs::read(&artifact).unwrap(), before);
        assert_eq!(fs::read(root.join("invocation-count")).unwrap(), b"x");
    }
    // Failed output delivery never reaches accounting, even when the metrics lock is available.
    FileExt::unlock(&lock).unwrap();
    let destination = root.join("already-exists.json");
    fs::write(&destination, b"unchanged destination").unwrap();
    let output = command(&root, &data)
        .args(["output", "show", id, "--view", "full", "--output"])
        .arg(&destination)
        .args(["--timeout-ms", "5000"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("OUTPUT_MEASUREMENT_UNRECORDED"));
    assert_eq!(fs::read(&destination).unwrap(), b"unchanged destination");
    assert_eq!(fs::read(&artifact).unwrap(), before);
}

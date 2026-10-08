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

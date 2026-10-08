//! CLI regressions for independent core review findings CR01–10.
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        let data = temp.path().join("data");
        fs::create_dir(&root).unwrap();
        let f = Self {
            _temp: temp,
            root,
            data,
        };
        f.ok(&["init"]);
        f
    }
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.args(["--root", self.root.to_str().unwrap(), "--format", "json"])
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_ACTOR", "owner");
        c
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }
    fn ok(&self, args: &[&str]) -> Value {
        let o = self.run(args);
        assert!(
            o.status.success(),
            "{args:?}: {} {}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        json(&o)
    }
}
fn json(o: &Output) -> Value {
    serde_json::from_slice(&o.stdout).unwrap_or_else(|e| {
        panic!(
            "Invalid JSON {e}: {} {}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
    })
}
#[cfg(unix)]
#[test]
fn cr01_handoff_symlink_cannot_write_outside_project() {
    let f = Fixture::new();
    let outside = f._temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, f.root.join(".pctx/handoffs")).unwrap();
    let input = f._temp.path().join("handoff.md");
    fs::write(&input, "Goal: fixture\nNext action: inspect fixture\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    let before = fs::metadata(&outside).unwrap().permissions().mode();
    let o = f.run(&[
        "handoff",
        "create",
        "--from-file",
        input.to_str().unwrap(),
        "--name",
        "fixture",
    ]);
    assert_eq!(o.status.code(), Some(5));
    assert_eq!(json(&o)["errors"][0]["code"], "POLICY_DENIED");
    assert!(!outside.join("fixture.md").exists());
    assert_eq!(fs::metadata(&outside).unwrap().permissions().mode(), before);
}
#[test]
fn cr02_embedded_private_key_fragments_are_masked() {
    let f = Fixture::new();
    let synthetic = "SYNTHETIC_PRIVATE_BLOCK_0123456789";
    let mut body = String::from("-----BEGIN PRIVATE KEY-----\n");
    for _ in 0..90 {
        body.push_str(synthetic);
        body.push('\n');
    }
    body.push_str("-----END PRIVATE KEY-----\n");
    fs::write(f.root.join("allowed.txt"), body).unwrap();
    f.ok(&["index", "update"]);
    for args in [
        vec!["read", "allowed.txt"],
        vec!["read", "allowed.txt", "--lines", "2:3"],
        vec!["find", synthetic, "--kind", "text", "--snippet-lines", "1"],
    ] {
        let o = f.run(&args);
        let _ = json(&o);
        assert!(
            !String::from_utf8_lossy(&o.stdout).contains(synthetic),
            "{args:?} leaked synthetic block: {}",
            String::from_utf8_lossy(&o.stdout)
        );
    }
}
#[test]
fn cr04_require_complete_does_not_hide_skipped_input() {
    let f = Fixture::new();
    fs::write(f.root.join("safe.py"), "def fixture():\n    return 1\n").unwrap();
    fs::write(f.root.join("oversized.txt"), vec![b'x'; 1048577]).unwrap();
    let o = f.run(&[
        "build",
        "--task",
        "fixture",
        "--seed",
        "safe.py",
        "--require-complete",
        "--budget-bytes",
        "6000",
    ]);
    assert_eq!(o.status.code(), Some(3));
    let v = json(&o);
    assert_eq!(v["status"], "error");
    assert_eq!(v["errors"][0]["code"], "PARTIAL_RESULT");
}
#[test]
fn cr05_public_envelope_reports_truncation_and_parser_partial() {
    let f = Fixture::new();
    fs::write(f.root.join("long.txt"), "ordinary line\n".repeat(100)).unwrap();
    fs::write(f.root.join("broken.py"), "def broken(:\n    pass\n").unwrap();
    f.ok(&["index", "update"]);
    let o = f.run(&["read", "long.txt"]);
    let read = json(&o);
    assert_eq!(read["truncation"]["truncated"], true);
    assert_ne!(read["coverage"]["status"], "complete");
    let outline = json(&f.run(&["outline", "broken.py"]));
    assert_eq!(outline["data"]["files"][0]["parse_status"], "partial");
    assert_ne!(outline["coverage"]["status"], "complete");
}
#[test]
fn cr06_equals_format_errors_are_single_json_documents() {
    let f = Fixture::new();
    let o = Command::new(env!("CARGO_BIN_EXE_pctx"))
        .args([
            "--root",
            f.root.to_str().unwrap(),
            "--format=json",
            "not-a-command",
        ])
        .env("PCTX_DATA_DIR", &f.data)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(json(&o)["errors"][0]["code"], "INVALID_ARGUMENT");
}
#[test]
fn cr08_checkpoint_metadata_obeys_new_security_exclusions() {
    let f = Fixture::new();
    fs::write(
        f.root.join("sensitive-location.txt"),
        "ordinary fixture text",
    )
    .unwrap();
    f.ok(&["checkpoint", "create", "--name", "before"]);
    let path = f.root.join(".pctx/config.toml");
    let s = fs::read_to_string(&path)
        .unwrap()
        .replace("exclude = []", "exclude = [\"sensitive-location.txt\"]");
    assert!(s.contains("sensitive-location.txt"));
    fs::write(path, s).unwrap();
    let o = f.run(&["checkpoint", "list"]);
    let _ = json(&o);
    assert!(!String::from_utf8_lossy(&o.stdout).contains("sensitive-location.txt"));
    let changes = f.run(&["changes", "--since", "before"]);
    let _ = json(&changes);
    assert!(!String::from_utf8_lossy(&changes.stdout).contains("sensitive-location.txt"));
}
#[test]
fn cr09_freshness_is_validated_and_off_is_visible() {
    let f = Fixture::new();
    fs::write(f.root.join("safe.py"), "def fixture():\n    return 1\n").unwrap();
    f.ok(&["index", "update"]);
    let off = f.ok(&["outline", "safe.py", "--freshness", "off"]);
    assert!(["off", "unchecked"].contains(&off["validation"]["mode"].as_str().unwrap_or("")));
    let bad = f.run(&["outline", "safe.py", "--freshness", "invalid"]);
    assert_eq!(bad.status.code(), Some(2));
    assert_eq!(json(&bad)["errors"][0]["code"], "INVALID_ARGUMENT");
}
#[test]
fn cr10_explicit_seed_limit_is_reported() {
    let f = Fixture::new();
    let mut args = vec![
        "build".to_string(),
        "--task".into(),
        "fixture".into(),
        "--budget-bytes".into(),
        "200000".into(),
    ];
    for n in 0..201 {
        let path = format!("file-{n:03}.py");
        fs::write(
            f.root.join(&path),
            format!("def fixture_{n}():\n    return {n}\n"),
        )
        .unwrap();
        args.push("--seed".into());
        args.push(path);
    }
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let o = f.run(&refs);
    let v = json(&o);
    assert_eq!(v["data"]["selection_complete"], false);
    let omissions = v["data"]["omitted_items"].as_array().unwrap();
    assert!(omissions.iter().any(|i| i["reason"] == "candidate_limit"));
}
#[test]
fn rebuild_preserves_checkpoints_and_quarantines_corrupt_index() {
    let f = Fixture::new();
    fs::write(f.root.join("safe.py"), "def fixture():\n    return 1\n").unwrap();
    f.ok(&["index", "update"]);
    f.ok(&["checkpoint", "create", "--name", "baseline"]);
    let status = f.ok(&["status"]);
    let path = PathBuf::from(status["data"]["local_storage"]["index"].as_str().unwrap());
    fs::write(f.root.join("safe.py"), "def fixture():\n    return 2\n").unwrap();
    let rebuilt = f.ok(&["index", "rebuild"]);
    assert_eq!(rebuilt["data"]["checkpoints_preserved"], 1);
    let quarantined = PathBuf::from(rebuilt["data"]["previous_database"].as_str().unwrap());
    assert!(quarantined.exists());
    assert_eq!(
        f.ok(&["changes", "--since", "baseline"])["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["path"] == "safe.py")
            .unwrap()["change"],
        "modified"
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
        .unwrap();
    drop(db);
    let corrupt = b"corrupt index fixture";
    fs::write(&path, corrupt).unwrap();
    let recovered = f.ok(&["index", "rebuild"]);
    assert!(recovered["data"]["recovery_warning"].is_string());
    assert_eq!(
        fs::read(recovered["data"]["previous_database"].as_str().unwrap()).unwrap(),
        corrupt
    );
    assert!(f.ok(&["outline", "safe.py"])["data"].is_object());
}
#[test]
fn rebuild_refuses_newer_schema_without_replacing_original() {
    let f = Fixture::new();
    fs::write(f.root.join("safe.py"), "x = 1\n").unwrap();
    f.ok(&["index", "update"]);
    let status = f.ok(&["status"]);
    let path = PathBuf::from(status["data"]["local_storage"]["index"].as_str().unwrap());
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch(
        "PRAGMA user_version=999; PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;",
    )
    .unwrap();
    drop(db);
    let old = fs::read(&path).unwrap();
    let o = f.run(&["index", "rebuild"]);
    assert_eq!(o.status.code(), Some(7));
    assert_eq!(json(&o)["errors"][0]["code"], "MIGRATION_REQUIRED");
    assert_eq!(fs::read(path).unwrap(), old);
}
#[test]
fn rebuild_refuses_foreign_workspace_without_replacing_index() {
    let f = Fixture::new();
    fs::write(f.root.join("safe.py"), "x = 1\n").unwrap();
    f.ok(&["index", "update"]);
    let status = f.ok(&["status"]);
    let path = PathBuf::from(status["data"]["local_storage"]["index"].as_str().unwrap());
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("UPDATE workspace_meta SET workspace_id='WS-foreign'", [])
        .unwrap();
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
        .unwrap();
    drop(db);
    let old = fs::read(&path).unwrap();
    let o = f.run(&["index", "rebuild"]);
    assert_eq!(o.status.code(), Some(9));
    assert_eq!(json(&o)["errors"][0]["code"], "WORKSPACE_MISMATCH");
    assert_eq!(fs::read(path).unwrap(), old);
}
#[test]
fn checkpoint_scope_unique_rename_and_checksum_failure_are_explicit() {
    let f = Fixture::new();
    fs::create_dir(f.root.join("src")).unwrap();
    fs::write(f.root.join("src/a.py"), "value = 7\n").unwrap();
    fs::write(f.root.join("outside.py"), "unrelated = 1\n").unwrap();
    f.ok(&[
        "checkpoint",
        "create",
        "--name",
        "scoped",
        "--scope",
        "src/**",
    ]);
    fs::rename(f.root.join("src/a.py"), f.root.join("src/b.py")).unwrap();
    fs::write(f.root.join("outside.py"), "unrelated = 2\n").unwrap();
    let changed = f.ok(&["changes", "--since", "scoped"]);
    assert_eq!(changed["data"]["rename_hints"][0]["from"], "src/a.py");
    assert!(
        changed["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x["path"].as_str().unwrap().starts_with("src/"))
    );
    let status = f.ok(&["status"]);
    let db = rusqlite::Connection::open(status["data"]["local_storage"]["index"].as_str().unwrap())
        .unwrap();
    db.execute(
        "UPDATE checkpoints SET manifest='{}' WHERE name='scoped'",
        [],
    )
    .unwrap();
    drop(db);
    let o = f.run(&["checkpoint", "list"]);
    assert_eq!(o.status.code(), Some(7));
    assert_eq!(json(&o)["errors"][0]["code"], "INVALID_ARCHIVE");
}

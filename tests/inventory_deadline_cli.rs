//! Actual finite inventory frontend and pre-effect contracts.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn command(root: &Path, data: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
    c.args(["--format", "json", "--root"])
        .arg(root)
        .env("PCTX_DATA_DIR", data)
        .env("PCTX_USER_CONFIG", root.join("absent-user-config"));
    c
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, current: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        if !current.exists() {
            return;
        }
        out.insert(current.strip_prefix(root).unwrap().into(), None);
        for e in fs::read_dir(current).unwrap() {
            let e = e.unwrap();
            let p = e.path();
            assert!(!e.file_type().unwrap().is_symlink());
            if p.is_dir() {
                visit(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().into(),
                    Some(fs::read(p).unwrap()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}
#[test]
fn inventory_limits_and_zero_budget_refuse_before_project_discovery() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("missing");
    let data = t.path().join("data");
    for args in [
        vec!["--timeout-ms", "0", "inventory", "scan"],
        vec![
            "--timeout-ms",
            "0",
            "inventory",
            "profile",
            "--path",
            "profile.json",
        ],
        vec![
            "--timeout-ms",
            "0",
            "inventory",
            "audit",
            "--registry",
            "registry.json",
        ],
        vec!["inventory", "scan", "--max-files", "0"],
        vec!["inventory", "scan", "--max-files", "4097"],
        vec!["inventory", "scan", "--max-bytes", "0"],
        vec!["inventory", "scan", "--max-bytes", "16777217"],
    ] {
        let out = command(&root, &data).args(&args).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?} {out:?}");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["errors"][0]["code"], "INVALID_ARGUMENT");
        assert!(v["data"].is_null());
        assert!(!root.exists());
        assert!(!data.exists());
    }
}
#[test]
fn finite_inventory_cli_preserves_files_and_reports_explicit_claims() {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir(&root).unwrap();
    assert!(
        command(&root, &data)
            .arg("init")
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::write(root.join("package.json"),json!({"name":"fixture","version":"2.0.0","engines":{"node":">=24"},"scripts":{"danger":"touch EXECUTED"}}).to_string()).unwrap();
    fs::write(root.join("profile.json"),json!({"schema_version":1,"id":"fixture","expectations":[{"source":"package.json","selector":"/version","expected":"1.0.0"},{"source":"missing.json","selector":"/version","expected":"1.0.0"}]}).to_string()).unwrap();
    fs::write(
        root.join("registry.json"),
        json!({"schema_version":1,"since_event":"event-1","source_refs":[]}).to_string(),
    )
    .unwrap();
    let before = (snapshot(&root), snapshot(&data));
    for args in [
        vec!["inventory", "scan"],
        vec!["inventory", "profile", "--path", "profile.json"],
        vec![
            "inventory",
            "audit",
            "--registry",
            "registry.json",
            "--since",
            "event-1",
        ],
    ] {
        let out = command(&root, &data)
            .args(["--timeout-ms", "5000"])
            .args(&args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{args:?} {out:?}");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["schema_version"], "1.0");
        assert_eq!(v["status"], "ok");
        if args[1] == "audit" {
            assert_eq!(v["data"]["documents_modified"], false);
            assert_eq!(v["data"]["tasks_created"], false);
        } else {
            assert_eq!(v["data"]["operation_authorized"], false);
        }
        if args[1] == "scan" {
            assert_eq!(v["data"]["scripts_executed"], false);
        }
        if args[1] == "profile" {
            assert_eq!(v["data"]["expectations"][0]["status"], "conflicting");
            assert_eq!(v["data"]["expectations"][1]["status"], "unconfirmed");
        }
        assert_eq!((snapshot(&root), snapshot(&data)), before);
        assert!(!root.join("EXECUTED").exists());
    }
    let out = command(&root, &data)
        .args([
            "--timeout-ms",
            "5000",
            "inventory",
            "audit",
            "--registry",
            "registry.json",
            "--since",
            "wrong",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(9));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["errors"][0]["code"], "BASELINE_MISMATCH");
    assert!(v["data"].is_null());
    assert_eq!((snapshot(&root), snapshot(&data)), before);
}

use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn cli(root: &Path, home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pctx"));
    command
        .args(["--format", "json", "--root"])
        .arg(root)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("PCTX_ACTOR", "owner")
        .env_remove("PCTX_RUN_ID")
        .env_remove("PCTX_RUN_CAPABILITY")
        .env("PCTX_DATA_DIR", home.join("data"))
        .env("PCTX_CONFIG_DIR", home.join("config"));
    command
}

#[test]
fn adapter_query_zero_budget_is_rejected_before_discovery_or_input() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("missing-project");
    let home = temp.path().join("missing-home");
    for args in [
        vec!["doctor"],
        vec!["verify"],
        vec!["protocol-fixture", "--from-file", "missing.json"],
    ] {
        let output = cli(&root, &home)
            .args(["--timeout-ms", "0", "adapter", "claude"])
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["errors"][0]["code"], "INVALID_ARGUMENT");
        assert!(!root.exists());
        assert!(!home.exists());
    }
}

#[test]
fn adapter_finite_reads_keep_envelope_and_do_not_install_configuration() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("project");
    let home = base.join("home");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&home).unwrap();
    let initialized = cli(&root, &home).arg("init").output().unwrap();
    assert!(initialized.status.success(), "{initialized:?}");
    let input = base.join("protocol.json");
    fs::write(
        &input,
        r#"{"hook_event_name":"SessionStart","session_id":"fixture","source":"startup"}"#,
    )
    .unwrap();
    for args in [
        vec!["verify".into()],
        vec![
            "protocol-fixture".into(),
            "--from-file".into(),
            input.into_os_string(),
        ],
    ] {
        let output = cli(&root, &home)
            .args(["adapter", "claude", "--timeout-ms", "1000"])
            .args(&args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{args:?}: {output:?}");
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["schema_version"], "1.0");
        assert_eq!(response["status"], "ok");
        assert_eq!(response["data"]["live_verified"], false);
        assert!(!root.join(".claude").exists());
        assert!(
            !fs::read_dir(home.join("data/controls"))
                .unwrap()
                .any(|entry| entry.unwrap().path().join("adapter.lock").exists())
        );
    }
}

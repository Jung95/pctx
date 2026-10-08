use pctx::{
    deadline::Deadline,
    inventory::{self, InventoryCommand},
    project::{Config, Project, ProjectConfig, RootAnchor},
};
use std::{fs, time::Instant};
fn fixture() -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("project");
    fs::create_dir_all(&root).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: base.join("data"),
        workspace_dir: base.join("data/workspace"),
        control_dir: base.join("data/control"),
        project_id: "fixture".into(),
        workspace_id: "workspace".into(),
        coordination_id: "coordination".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "fixture".into(),
                name: "fixture".into(),
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
#[test]
fn expired_direct_and_dispatch_queries_cannot_be_successful_partial() {
    let (_t, mut p) = fixture();
    let original = Deadline::from_instant(Instant::now());
    p.deadline = Some(original);
    for result in [
        inventory::scan(&p, None, 100, 100000),
        inventory::profile(&p, "missing.json"),
        inventory::audit(&p, "missing.json", None),
        inventory::execute(
            &p,
            &InventoryCommand::Scan {
                profile: None,
                max_files: 100,
                max_bytes: 100000,
            },
        ),
        inventory::execute(
            &p,
            &InventoryCommand::Profile {
                path: "missing.json".into(),
            },
        ),
        inventory::execute(
            &p,
            &InventoryCommand::Audit {
                registry: "missing.json".into(),
                since: None,
            },
        ),
    ] {
        let e = result.unwrap_err();
        assert_eq!(e.code, "TIMEOUT");
        assert_eq!(e.exit, 7);
    }
    assert_eq!(p.deadline.unwrap().instant(), original.instant());
    assert!(!p.data_dir.exists());
}
#[test]
fn semantic_scan_limits_precede_source_reads_and_default_empty_scan_has_no_effects() {
    let (_t, p) = fixture();
    assert!(inventory::validate_limits(1, 1).is_ok());
    assert!(inventory::validate_limits(4096, 16777216).is_ok());
    for (files, bytes) in [(0, 1), (4097, 1), (1, 0), (1, 16777217)] {
        assert_eq!(
            inventory::scan(&p, None, files, bytes).unwrap_err().code,
            "INVALID_ARGUMENT"
        );
    }
    let v = inventory::scan(&p, None, 100, 100000).unwrap();
    assert!(v["items"].as_array().unwrap().is_empty());
    assert_eq!(v["scripts_executed"], false);
    assert!(p.deadline.is_none());
    assert!(!p.data_dir.exists());
}
#[test]
fn missing_and_unsupported_items_remain_truthful_unconfirmed_claims() {
    let (_t, p) = fixture();
    fs::write(
        p.root.join("dynamic.js"),
        "throw new Error('never executed')",
    )
    .unwrap();
    fs::write(p.root.join("profile.json"),serde_json::json!({"schema_version":1,"id":"fixture","expectations":[{"source":"missing.json","selector":"/value","expected":1},{"source":"dynamic.js","selector":"/value","expected":1}],"roles":[]}).to_string()).unwrap();
    let v = inventory::profile(&p, "profile.json").unwrap();
    assert_eq!(v["status"], "unconfirmed");
    assert_eq!(v["expectations"][0]["status"], "unconfirmed");
    assert_eq!(v["expectations"][1]["reason"], "CAPABILITY_UNVERIFIED");
    assert_eq!(v["operation_authorized"], false);
    assert!(!p.data_dir.exists());
}
#[cfg(unix)]
#[test]
fn fifo_profile_source_is_refused_without_waiting_or_effects() {
    const PROBE: &str = "PCTX_INVENTORY_FIFO_DEADLINE_PROBE";
    if std::env::var_os(PROBE).is_none() {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "fifo_profile_source_is_refused_without_waiting_or_effects",
                "--nocapture",
            ])
            .env(PROBE, "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let end = Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= end {
                let _ = child.kill();
                let _ = child.wait();
                panic!("FIFO inventory fixture exceeded owned three-second bound");
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let o = child.wait_with_output().unwrap();
        assert!(
            o.status.success(),
            "{} {}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        return;
    }
    let (_t, mut p) = fixture();
    let path = p.root.join("profile.json");
    use std::os::unix::ffi::OsStrExt;
    let native = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(native.as_ptr(), 0o600) }, 0);
    p.deadline = Some(Deadline::from_millis(1000).unwrap());
    let e = inventory::profile(&p, "profile.json").unwrap_err();
    assert_eq!(e.code, "INVALID_ARGUMENT");
    assert_eq!(e.exit, 2);
    assert!(!p.data_dir.exists());
}

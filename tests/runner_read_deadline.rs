//! Finite public read entries preserve deadlines and never execute child work.
use pctx::{
    deadline::Deadline,
    output::{self, TrustCommand},
    project::{Config, Project, ProjectConfig, RootAnchor},
    runner::{self, RunnerCommand},
};
use std::fs;
use std::time::Instant;
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
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
    (t, p)
}
#[test]
fn expired_reads_fail_before_directory_database_or_trust_effects() {
    let (_t, mut p) = fixture();
    let original = Deadline::from_instant(Instant::now());
    p.deadline = Some(original);
    for command in [
        RunnerCommand::CheckPlan {
            task_id: "missing".into(),
            key: "missing".into(),
            run: None,
        },
        RunnerCommand::ResourceStatus,
        RunnerCommand::HelperStatus {
            helper: "HELP-missing".into(),
        },
    ] {
        let e = runner::execute(&p, &command).unwrap_err();
        assert_eq!(e.code, "TIMEOUT");
        assert_eq!(e.exit, 7);
    }
    let e = output::trust(
        &p,
        &TrustCommand::Plan {
            argv: vec!["missing".into()],
        },
    )
    .unwrap_err();
    assert_eq!(e.code, "TIMEOUT");
    assert_eq!(e.exit, 7);
    assert_eq!(p.deadline.unwrap().instant(), original.instant());
    assert!(!p.data_dir.exists());
    assert!(!p.control_db().exists());
    assert!(!p.index_db().exists());
}
#[test]
fn default_helper_read_returns_real_receipt_without_writes() {
    let (_t, p) = fixture();
    let dir = p.workspace_dir.join("helpers");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("HELP-fixture.json");
    let raw=serde_json::to_vec(&serde_json::json!({"schema_version":1,"helper_id":"HELP-fixture","task_id":"T-fixture","run_id":"RUN-fixture","mode":"cloud","scope":[],"workspace":p.workspace_id,"execution_workspace":null,"state":"queued_intent","job_id":null,"output_id":null,"started":false,"created_at":0})).unwrap();
    fs::write(&path, &raw).unwrap();
    let value = runner::execute(
        &p,
        &RunnerCommand::HelperStatus {
            helper: "HELP-fixture".into(),
        },
    )
    .unwrap();
    assert_eq!(value["helper"]["started"], false);
    assert_eq!(value["helper"]["state"], "queued_intent");
    assert_eq!(fs::read(path).unwrap(), raw);
    assert!(p.deadline.is_none());
    assert!(!p.control_db().exists());
    assert!(!p.index_db().exists());
    assert!(!p.data_dir.join("host-resources").exists());
}
#[cfg(unix)]
#[test]
fn finite_helper_fifo_is_refused_before_reading_or_creating_other_state() {
    const PROBE: &str = "PCTX_HELPER_FIFO_DEADLINE_PROBE";
    if std::env::var_os(PROBE).is_none() {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "finite_helper_fifo_is_refused_before_reading_or_creating_other_state",
                "--nocapture",
            ])
            .env(PROBE, "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let bound = Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= bound {
                let _ = child.kill();
                let _ = child.wait();
                panic!("FIFO metadata probe exceeded owned three-second bound");
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    let (_t, mut p) = fixture();
    let dir = p.workspace_dir.join("helpers");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("HELP-fixture.json");
    use std::os::unix::ffi::OsStrExt;
    let native = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(native.as_ptr(), 0o600) }, 0);
    let original = Deadline::from_millis(1000).unwrap();
    p.deadline = Some(original);
    let started = Instant::now();
    let e = runner::execute(
        &p,
        &RunnerCommand::HelperStatus {
            helper: "HELP-fixture".into(),
        },
    )
    .unwrap_err();
    assert_eq!(e.code, "INVALID_ARGUMENT");
    assert_eq!(e.exit, 2);
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    assert_eq!(p.deadline.unwrap().instant(), original.instant());
    assert!(!p.control_db().exists());
    assert!(!p.index_db().exists());
    assert!(!p.data_dir.join("host-resources").exists());
}

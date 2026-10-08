//! Saved-artifact fixtures represent stored observations, never a child execution.
use pctx::{
    deadline::Deadline,
    domain::{hash, now},
    output::{self, OutputCommand},
    project::{Config, Project, ProjectConfig, RootAnchor},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};
#[derive(Serialize)]
struct Record {
    stream: &'static str,
    sequence: u64,
    text: &'static str,
    redacted: bool,
}
fn fixture() -> (tempfile::TempDir, Project, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("project");
    fs::create_dir(&root).unwrap();
    let data = base.join("data");
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspaces/WS-test"),
        control_dir: data.join("controls/COORD-test"),
        project_id: "test".into(),
        workspace_id: "WS-test".into(),
        coordination_id: "COORD-test".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "test".into(),
                name: "fixture".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    fs::create_dir_all(&p.workspace_dir).unwrap();
    fs::create_dir_all(&p.control_dir).unwrap();
    let records = [Record {
        stream: "stdout",
        sequence: 0,
        text: "warning 한글 🦀",
        redacted: false,
    }];
    let captured_bytes = records
        .iter()
        .map(|record| record.text.len() + 1)
        .sum::<usize>();
    let artifact = json!({"schema_version":1,"output_id":"OUT-fixture","execution_id":"fixture-observation",
        "workspace_id":p.workspace_id,"policy_hash":p.policy_hash(),"input_fingerprint":"fixture-no-executable",
        "created_at":now(),"expires_at":now()+3600,"records_hash":hash(serde_json::to_vec(&records).unwrap()),
        "records":records,"captured_bytes":captured_bytes,"normalized_bytes":captured_bytes,"redacted_bytes":captured_bytes,"omitted_bytes":0,
        "capture_complete":true,"retained":true,"spawned":false,"termination":"exited","child_exit_code":null,
        "signal":null,"pctx_error":null,"input_stage":"ordinary","emitted_bytes":0,"retrieval_bytes":0,"delivery_attempts":0});
    let path = data.join("outputs/WS-test/OUT-fixture.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    (temp, p, path)
}
fn commands() -> [OutputCommand; 4] {
    [
        OutputCommand::Show {
            id: "OUT-fixture".into(),
            view: "full".into(),
            stream: None,
            lines: None,
        },
        OutputCommand::Show {
            id: "OUT-fixture".into(),
            view: "compact".into(),
            stream: None,
            lines: None,
        },
        OutputCommand::Find {
            id: "OUT-fixture".into(),
            literal: "한글".into(),
            limit: 1,
        },
        OutputCommand::Render {
            id: "OUT-fixture".into(),
            filter: "builtin".into(),
        },
    ]
}
#[test]
fn expired_saved_queries_and_accounting_preserve_exact_artifact() {
    let (_temp, mut p, path) = fixture();
    let original = fs::read(&path).unwrap();
    p.deadline = Some(Deadline::from_instant(
        Instant::now() - Duration::from_secs(1),
    ));
    for command in commands() {
        let error = output::output(&p, &command).unwrap_err();
        assert_eq!(error.code, "TIMEOUT");
        assert_eq!(error.exit, 7);
    }
    assert_eq!(output::savings(&p).unwrap_err().code, "TIMEOUT");
    assert_eq!(
        output::record_delivery(&p, "OUT-fixture", 123, "retrieval")
            .unwrap_err()
            .code,
        "TIMEOUT"
    );
    assert!(!p.data_dir.join("output-metrics.lock").exists());
    assert_eq!(fs::read(&path).unwrap(), original);
}
#[test]
fn saved_views_keep_unicode_and_original_budget_without_executing_or_mutating() {
    let (_temp, p, path) = fixture();
    let original = fs::read(&path).unwrap();
    let mut bounded = p.clone();
    bounded.deadline = Some(Deadline::from_millis(10_000).unwrap());
    let deadline = bounded.deadline.unwrap().instant();
    for command in commands() {
        let expected = output::output(&p, &command).unwrap();
        let actual = output::output(&bounded, &command).unwrap();
        assert_eq!(actual, expected);
        assert!(actual.to_string().contains("한글"));
    }
    assert_eq!(bounded.deadline.unwrap().instant(), deadline);
    assert!(p.deadline.is_none());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(
        output::savings(&bounded).unwrap()["metrics_available"],
        false
    );
}
#[test]
fn retention_expiry_is_distinct_from_request_expiry_and_never_rewrites_artifact() {
    let (_temp, mut p, path) = fixture();
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["expires_at"] = json!(now() - 1);
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let original = fs::read(&path).unwrap();
    p.deadline = Some(Deadline::from_millis(10_000).unwrap());
    assert_eq!(
        output::output(&p, &commands()[0]).unwrap_err().code,
        "OUTPUT_EXPIRED"
    );
    p.deadline = Some(Deadline::from_instant(
        Instant::now() - Duration::from_secs(1),
    ));
    assert_eq!(
        output::output(&p, &commands()[0]).unwrap_err().code,
        "TIMEOUT"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
}
#[test]
fn failed_accounting_under_contention_never_increments_observed_bytes() {
    use fs2::FileExt;
    let (_temp, mut p, path) = fixture();
    let original = fs::read(&path).unwrap();
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(p.data_dir.join("output-metrics.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    p.deadline = Some(Deadline::from_millis(10_000).unwrap());
    let error = output::record_delivery(&p, "OUT-fixture", 123, "retrieval").unwrap_err();
    assert_eq!(error.code, "RESOURCE_BUSY");
    assert_eq!(fs::read(&path).unwrap(), original);
    FileExt::unlock(&lock).unwrap();
    assert_eq!(output::savings(&p).unwrap()["emitted_retrieval_bytes"], 0);
}
#[cfg(unix)]
#[test]
fn fifo_artifact_is_rejected_without_waiting_for_writer() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let (_temp, p, path) = fixture();
    fs::remove_file(&path).unwrap();
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let started = Instant::now();
    assert_eq!(
        output::output(&p, &commands()[0]).unwrap_err().code,
        "POLICY_DENIED"
    );
    assert!(started.elapsed() < Duration::from_secs(2));
}

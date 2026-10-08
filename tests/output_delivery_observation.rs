//! Actual CLI delivery observations of synthetic stored artifacts: no child command
//! was executed. The Unix pipe case proves cooperative post-write expiry, not a
//! hard cancellation guarantee for a blocked stdout write.
use pctx::{
    domain::{hash, now},
    project::{Config, Project, RootAnchor},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};

// Regular-file capture and bounded process polling avoid pipe-reader threads.
struct Reap(Child);
impl Drop for Reap {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn bounded_output(command: &mut Command) -> Output {
    let mut stdout = tempfile::tempfile().unwrap();
    let mut stderr = tempfile::tempfile().unwrap();
    let mut child = Reap(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout.try_clone().unwrap()))
            .stderr(Stdio::from(stderr.try_clone().unwrap()))
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "fixture CLI exceeded finite wait"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    use std::io::{Read, Seek, SeekFrom};
    fn collected(file: &mut fs::File) -> Vec<u8> {
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        file.take(1024 * 1024 + 1).read_to_end(&mut bytes).unwrap();
        assert!(bytes.len() <= 1024 * 1024, "fixture capture exceeded bound");
        bytes
    }
    Output {
        status,
        stdout: collected(&mut stdout),
        stderr: collected(&mut stderr),
    }
}
#[derive(Serialize)]
struct Record {
    stream: &'static str,
    sequence: u64,
    text: String,
    redacted: bool,
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
    artifact: PathBuf,
}
impl Fixture {
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pctx"));
        command
            .args(["--format", "json", "--root"])
            .arg(&self.root)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_ACTOR", "owner");
        command
    }
    fn stored(&self) -> Value {
        serde_json::from_slice(&fs::read(&self.artifact).unwrap()).unwrap()
    }
}
fn fixture(large: bool) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir(&root).unwrap();
    let mut f = Fixture {
        _temp: temp,
        root,
        data,
        artifact: PathBuf::new(),
    };
    let initialized = bounded_output(f.command().arg("init"));
    assert!(initialized.status.success(), "{initialized:?}");
    let config: Config =
        toml::from_str(&fs::read_to_string(f.root.join(".pctx/config.toml")).unwrap()).unwrap();
    let registry: Value =
        serde_json::from_slice(&fs::read(f.data.join("registry.json")).unwrap()).unwrap();
    let binding = &registry["roots"][f.root.to_str().unwrap()];
    let workspace_id = binding["workspace_id"].as_str().unwrap().to_owned();
    let coordination_id = binding["coordination_id"].as_str().unwrap().to_owned();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&f.root).unwrap(),
        root: f.root.clone(),
        data_dir: f.data.clone(),
        workspace_dir: f.data.join("workspaces").join(&workspace_id),
        control_dir: f.data.join("controls").join(&coordination_id),
        project_id: config.project.id.clone(),
        workspace_id,
        coordination_id,
        config,
    };
    let records: Vec<_> = (0..if large { 4 } else { 1 })
        .map(|sequence| Record {
            stream: "stdout",
            sequence,
            text: if large {
                format!("observed fixture {sequence} 한글 {}", "x".repeat(64 * 1024))
            } else {
                "Observed Unicode 한글 🦀\nembedded newline".into()
            },
            redacted: false,
        })
        .collect();
    let bytes: usize = records.iter().map(|r| r.text.len() + 1).sum();
    let artifact = json!({"schema_version":1,"output_id":"OUT-delivery-fixture","execution_id":"synthetic-stored-observation",
        "workspace_id":p.workspace_id,"policy_hash":p.policy_hash(),"input_fingerprint":"fixture-no-executable",
        "created_at":now(),"expires_at":now()+3600,"records_hash":hash(serde_json::to_vec(&records).unwrap()),
        "records":records,"captured_bytes":bytes,"normalized_bytes":bytes,"redacted_bytes":bytes,"omitted_bytes":0,
        "capture_complete":true,"retained":true,"spawned":false,"termination":"exited","child_exit_code":null,
        "signal":null,"pctx_error":null,"input_stage":"ordinary","emitted_bytes":0,"retrieval_bytes":0,"delivery_attempts":0});
    f.artifact = f
        .data
        .join("outputs")
        .join(&p.workspace_id)
        .join("OUT-delivery-fixture.json");
    fs::create_dir_all(f.artifact.parent().unwrap()).unwrap();
    fs::write(&f.artifact, serde_json::to_vec(&artifact).unwrap()).unwrap();
    f
}
#[test]
fn successful_full_delivery_counts_exact_stdout_including_final_newline() {
    let f = fixture(false);
    let before = f.stored();
    let output = bounded_output(f.command().args([
        "output",
        "show",
        "OUT-delivery-fixture",
        "--view",
        "full",
        "--timeout-ms",
        "1000",
    ]));
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{:?}", output.stderr);
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["status"], "ok");
    assert_eq!(response["data"]["command_rerun"], false);
    let after = f.stored();
    assert_eq!(
        after["retrieval_bytes"].as_u64().unwrap() - before["retrieval_bytes"].as_u64().unwrap(),
        output.stdout.len() as u64
    );
    assert_eq!(after["delivery_attempts"], 1);
    assert_eq!(after["records"], before["records"]);
    assert_eq!(after["records_hash"], before["records_hash"]);
    assert_eq!(after["spawned"], false);
    assert_eq!(after["child_exit_code"], Value::Null);
    let savings = bounded_output(
        f.command()
            .args(["savings", "report", "--timeout-ms", "1000"]),
    );
    assert!(savings.status.success(), "{savings:?}");
    let savings: Value = serde_json::from_slice(&savings.stdout).unwrap();
    for key in [
        "provider_usage",
        "subscription_quota",
        "api_cost",
        "tokenizer_tokens",
    ] {
        assert_eq!(savings["data"][key], "unknown");
    }
    assert_eq!(savings["data"]["delivery_receipt"], "not_observed");
}

#[cfg(unix)]
#[test]
fn observed_live_pipe_write_keeps_delivery_success_after_original_budget_expires() {
    use std::{io::Read, os::fd::AsRawFd};
    let f = fixture(true);
    let original = fs::read(&f.artifact).unwrap();
    let stderr_path = f.data.join("isolated-delivery-stderr.json");
    let stderr = fs::File::create(&stderr_path).unwrap();
    let launched = Instant::now();
    let mut child = Reap(
        f.command()
            .args([
                "output",
                "show",
                "OUT-delivery-fixture",
                "--view",
                "full",
                "--timeout-ms",
                "1000",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr))
            .spawn()
            .unwrap(),
    );
    let mut stdout = child.0.stdout.take().unwrap();
    let fd = stdout.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    assert!(flags >= 0);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) },
        0
    );
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    // Observe a real write while the original positive deadline is still live.
    loop {
        match stdout.read(&mut chunk) {
            Ok(n) if n > 0 => {
                bytes.extend_from_slice(&chunk[..n]);
                break;
            }
            Ok(_) => panic!(
                "CLI closed stdout before live-write observation: {:?}",
                fs::read_to_string(&stderr_path)
            ),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("stdout observation failed: {e}"),
        }
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "CLI exited before write observation"
        );
        assert!(
            launched.elapsed() < Duration::from_millis(800),
            "preparation did not finish inside original budget"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(launched.elapsed() < Duration::from_millis(1000));
    assert!(child.0.try_wait().unwrap().is_none());
    // No pipe reads occur during this bounded stall. Linux/macOS pipe capacities
    // are below this 256KiB response; live observation, not a blind launch sleep,
    // establishes that preparation succeeded and output delivery has started.
    let resume_at = launched + Duration::from_millis(1300);
    while Instant::now() < resume_at {
        std::thread::sleep(
            resume_at
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(10)),
        );
    }
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "response unexpectedly fit in the pipe; blocked-write proof absent"
    );
    let drain_deadline = Instant::now() + Duration::from_secs(4);
    loop {
        match stdout.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(2))
            }
            Err(e) => panic!("drain failed: {e}"),
        }
        assert!(bytes.len() < 1024 * 1024, "fixture response exceeded bound");
        assert!(
            Instant::now() < drain_deadline,
            "CLI output did not close after draining"
        );
    }
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < drain_deadline,
            "CLI did not finish bookkeeping within drain bound"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(status.success(), "delivered outcome was changed: {status}");
    assert_eq!(bytes.last(), Some(&b'\n'));
    let response: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(response["status"], "ok");
    assert_eq!(response["data"]["records"].as_array().unwrap().len(), 4);
    assert_eq!(response["data"]["command_rerun"], false);
    let diagnostic_text = fs::read_to_string(&stderr_path).unwrap();
    let diagnostic: Value = serde_json::from_str(diagnostic_text.trim()).unwrap();
    assert_eq!(diagnostic["code"], "OUTPUT_MEASUREMENT_UNRECORDED");
    assert_eq!(diagnostic["reason"], "TIMEOUT");
    assert_eq!(diagnostic["delivery_written"], true);
    assert_eq!(diagnostic["measurement_recorded"], "unknown");
    assert!(!diagnostic_text.contains(f.root.to_str().unwrap()));
    assert!(!diagnostic_text.contains("observed fixture"));
    assert_eq!(fs::read(&f.artifact).unwrap(), original);
}

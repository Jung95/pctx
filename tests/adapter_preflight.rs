//! PCTX01-G03-R10: existing adapter identity admission.
use pctx::{
    adapter::{self, AdapterCommand, ClaudeCommand},
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};

type Snapshot = BTreeMap<PathBuf, Option<Vec<u8>>>;
fn snapshot(root: &Path) -> Snapshot {
    fn walk(base: &Path, dir: &Path, out: &mut Snapshot) {
        if !dir.exists() {
            return;
        }
        out.insert(dir.strip_prefix(base).unwrap().into(), None);
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            assert!(!entry.file_type().unwrap().is_symlink());
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(base).unwrap().into(),
                    Some(fs::read(path).unwrap()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    root: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        Self {
            root: base.join("project"),
            data: base.join("data"),
            base,
            _temp: temp,
        }
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_pctx"));
        command
            .current_dir(&self.base)
            .args(["--format", "json", "--root"])
            .arg(&self.root)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_USER_CONFIG", self.base.join("absent-config"));
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }
    fn state(&self) -> (Snapshot, Snapshot) {
        (snapshot(&self.root), snapshot(&self.data))
    }
    fn init(&self) {
        fs::create_dir(&self.root).unwrap();
        let output = self.run(&["init"]);
        assert!(output.status.success(), "{output:?}");
    }
    fn project(&self) -> Project {
        fs::create_dir(&self.root).unwrap();
        Project {
            deadline: None,
            root_anchor: RootAnchor::capture(&self.root).unwrap(),
            root: self.root.clone(),
            data_dir: self.data.clone(),
            workspace_dir: self.data.join("workspace"),
            control_dir: self.data.join("control"),
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
        }
    }
}
fn commands(path: &Path) -> Vec<ClaudeCommand> {
    let mut out = vec![
        ClaudeCommand::Plan {
            agent: String::new(),
        },
        ClaudeCommand::Plan {
            agent: "fixture\nagent".into(),
        },
        ClaudeCommand::Event {
            agent: "a".repeat(257),
            from_file: Some(path.into()),
            idempotency_key: None,
            hook: false,
        },
        ClaudeCommand::Uninstall {
            plan: "\t".into(),
            expect_config_hash: "fixture".into(),
        },
    ];
    for field in 0..5 {
        let mut values = [
            "fixture".to_string(),
            "fixture".into(),
            "fixture".into(),
            "fixture".into(),
            "fixture".into(),
        ];
        values[field] = String::new();
        let [task_id, pool, session, counter_epoch, idempotency_key] = values;
        out.push(ClaudeCommand::Statusline {
            task_id,
            from_file: path.into(),
            pool,
            session,
            epoch: 1,
            observed_at: "2026-10-08T00:00:00Z".into(),
            window_start: "2026-10-08T00:00:00Z".into(),
            window_end: "2026-10-09T00:00:00Z".into(),
            counter_epoch,
            idempotency_key,
        });
    }
    out
}
fn args(command: &ClaudeCommand) -> Vec<String> {
    let mut out = vec!["adapter".into(), "claude".into()];
    match command {
        ClaudeCommand::Plan { agent } => {
            out.extend(["plan".into(), "--agent".into(), agent.clone()])
        }
        ClaudeCommand::Event {
            agent, from_file, ..
        } => out.extend([
            "event".into(),
            "--agent".into(),
            agent.clone(),
            "--from-file".into(),
            from_file.as_ref().unwrap().to_str().unwrap().into(),
        ]),
        ClaudeCommand::Uninstall {
            plan,
            expect_config_hash,
        } => out.extend([
            "uninstall".into(),
            "--plan".into(),
            plan.clone(),
            "--expect-config-hash".into(),
            expect_config_hash.clone(),
        ]),
        ClaudeCommand::Statusline {
            task_id,
            from_file,
            pool,
            session,
            epoch,
            observed_at,
            window_start,
            window_end,
            counter_epoch,
            idempotency_key,
        } => out.extend([
            "statusline".into(),
            "--task-id".into(),
            task_id.clone(),
            "--from-file".into(),
            from_file.to_str().unwrap().into(),
            "--pool".into(),
            pool.clone(),
            "--session".into(),
            session.clone(),
            "--epoch".into(),
            epoch.to_string(),
            "--observed-at".into(),
            observed_at.clone(),
            "--window-start".into(),
            window_start.clone(),
            "--window-end".into(),
            window_end.clone(),
            "--counter-epoch".into(),
            counter_epoch.clone(),
            "--idempotency-key".into(),
            idempotency_key.clone(),
        ]),
        _ => unreachable!(),
    }
    out
}
#[test]
fn adapter_labels_precede_project_and_response_effects() {
    for initialized in [false, true] {
        let f = Fixture::new();
        if initialized {
            f.init();
        }
        let response = f.base.join("response.json");
        if initialized {
            fs::write(&response, "existing response").unwrap();
        }
        let before = f.state();
        for format in ["json", "compact"] {
            for command in commands(&f.base.join("absent-event.json")) {
                let o = Command::new(env!("CARGO_BIN_EXE_pctx"))
                    .current_dir(&f.base)
                    .args(["--format", format, "--root"])
                    .arg(&f.root)
                    .arg("--output")
                    .arg(&response)
                    .env("PCTX_DATA_DIR", &f.data)
                    .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                    .env("PCTX_ACTOR", "agent:fixture")
                    .args(args(&command))
                    .output()
                    .unwrap();
                assert_eq!(o.status.code(), Some(2), "{o:?}");
                let v: Value = serde_json::from_slice(&o.stdout).unwrap();
                assert_eq!(v["errors"][0]["code"], "INVALID_ARGUMENT");
                assert!(v["project_id"].is_null());
                assert!(o.stderr.is_empty());
                assert_eq!(f.state(), before);
                if initialized {
                    assert_eq!(fs::read(&response).unwrap(), b"existing response");
                } else {
                    assert!(!response.exists());
                }
            }
        }
    }
}
#[test]
fn direct_adapter_labels_keep_original_expiry_and_no_effects() {
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    for command in commands(&f.base.join("absent-event.json")) {
        let error = adapter::execute(&p, &AdapterCommand::Claude { command }).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for command in commands(&f.base.join("absent-event.json")) {
        assert_eq!(
            adapter::execute(&p, &AdapterCommand::Claude { command })
                .unwrap_err()
                .code,
            "TIMEOUT"
        );
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}

#[test]
fn adapter_label_boundaries_preserve_existing_grammar() {
    for agent in ["a".repeat(256), "é".repeat(128), " ".into()] {
        adapter::validate_adapter_request(&AdapterCommand::Claude {
            command: ClaudeCommand::Plan { agent },
        })
        .unwrap();
    }
    for agent in [
        "a".repeat(257),
        "é".repeat(129),
        String::new(),
        "a\tb".into(),
        format!("sk-{}", "SYNTHETICFIXTURE".repeat(2)),
    ] {
        let error = adapter::validate_adapter_request(&AdapterCommand::Claude {
            command: ClaudeCommand::Plan { agent },
        })
        .unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
    }
    let f = Fixture::new();
    let error = adapter::validate_adapter_request(&AdapterCommand::Claude {
        command: ClaudeCommand::Event {
            agent: "fixture".into(),
            from_file: Some(f.base.join("absent")),
            idempotency_key: None,
            hook: true,
        },
    })
    .unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
    assert!(!f.root.exists());
    assert!(!f.data.exists());
}

#[test]
fn native_event_keys_remain_input_dependent_and_replay_is_stable() {
    let f = Fixture::new();
    f.init();
    let path = f.base.join("event.json");
    fs::write(
        &path,
        serde_json::json!({"hook_event_name":"PreToolUse",
        "session_id":"native-fixture", "tool_name":"Bash",
        "tool_input":{"command":"pwd"}})
        .to_string(),
    )
    .unwrap();
    let before = f.state();
    let event = |key: &str| {
        f.command()
            .args([
                "adapter",
                "claude",
                "event",
                "--agent",
                "fixture",
                "--from-file",
            ])
            .arg(&path)
            .args(["--idempotency-key", key])
            .output()
            .unwrap()
    };
    for key in [String::new(), "a\tb".into(), "a".repeat(257)] {
        let output = event(&key);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["data"]["hook_output"], serde_json::json!({}));
        assert_eq!(value["data"]["command_executed"], false);
        assert_eq!(value["data"]["host_permission_granted"], false);
        assert!(output.stderr.is_empty());
        assert_eq!(f.state(), before);
    }
    fs::write(
        &path,
        serde_json::json!({"hook_event_name":"PermissionDenied",
        "session_id":"native-fixture"})
        .to_string(),
    )
    .unwrap();
    let output = event("");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], "INVALID_ARGUMENT");
    assert_eq!(f.state(), before);
    let first = event("fixture-key");
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let imported = f.state();
    let second = event("fixture-key");
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    let second: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(first["data"], second["data"]);
    assert_eq!(f.state(), imported);
}

#[test]
fn native_accepted_plan_labels_preserve_owner_authority() {
    let f = Fixture::new();
    f.init();
    for agent in ["a".repeat(256), " ".into()] {
        let before = f.state();
        let output = f
            .command()
            .env("PCTX_ACTOR", "agent:fixture")
            .args(["adapter", "claude", "plan", "--agent", &agent])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(9), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["errors"][0]["code"], "POLICY_DENIED");
        assert_eq!(f.state(), before);
        let output = f.run(&["adapter", "claude", "plan", "--agent", &agent]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["data"]["requires_explicit_install"], true);
        assert!(!f.root.join(".claude").exists());
    }
}

#[test]
fn isolated_nonowner_adapter_admission_precedes_authority() {
    const MARKER: &str = "PCTX_ADAPTER_PREFLIGHT_CHILD";
    if std::env::var_os(MARKER).is_none() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "isolated_nonowner_adapter_admission_precedes_authority",
                "--nocapture",
            ])
            .env(MARKER, "1")
            .env("PCTX_ACTOR", "agent:fixture")
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "isolated child {status}");
                return;
            }
            if Instant::now() >= end {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("isolated child exceeded parent bound");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    for command in commands(&f.base.join("absent-event.json")) {
        let error = adapter::execute(&p, &AdapterCommand::Claude { command }).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    let error = adapter::execute(
        &p,
        &AdapterCommand::Claude {
            command: ClaudeCommand::Plan {
                agent: String::new(),
            },
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "TIMEOUT");
    assert_eq!(p.deadline.unwrap().instant(), original);
    assert_eq!(f.state(), before);
}

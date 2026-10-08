//! PCTX01-G03-R11: existing schedule argument admission.
use pctx::{
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor},
    schedule::{self, ScheduleCommand},
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
            .arg("--root")
            .arg(&self.root)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_USER_CONFIG", self.base.join("absent-config"));
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command()
            .args(["--format", "json"])
            .args(args)
            .output()
            .unwrap()
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
fn commands(path: &Path) -> Vec<ScheduleCommand> {
    let mut out = vec![
        ScheduleCommand::Add {
            from_file: path.into(),
            idempotency_key: String::new(),
        },
        ScheduleCommand::Update {
            namespace: String::new(),
            id: "fixture".into(),
            from_file: path.into(),
            expect_revision: 1,
        },
        ScheduleCommand::Update {
            namespace: "fixture".into(),
            id: "a\tb".into(),
            from_file: path.into(),
            expect_revision: 1,
        },
        ScheduleCommand::Remove {
            namespace: "fixture".into(),
            id: "a".repeat(257),
            expect_revision: 1,
        },
        ScheduleCommand::List {
            namespace: Some("a".repeat(257)),
        },
        ScheduleCommand::Reconcile {
            namespace: Some(String::new()),
            at: None,
        },
        ScheduleCommand::Reconcile {
            namespace: None,
            at: Some("invalid".into()),
        },
        ScheduleCommand::Tick {
            namespace: Some(String::new()),
            at: None,
            retry_failed: false,
        },
        ScheduleCommand::Tick {
            namespace: None,
            at: Some("2026-10-08T00:00:00".into()),
            retry_failed: false,
        },
    ];
    for (namespace, id, reason) in [
        ("".into(), "fixture".into(), "fixture".into()),
        ("fixture".into(), "".into(), "fixture".into()),
        ("fixture".into(), "fixture".into(), " ".into()),
        ("fixture".into(), "fixture".into(), "a".repeat(2049)),
    ] {
        out.push(ScheduleCommand::Recover {
            namespace,
            id,
            revision: 1,
            occurrence: "fixture".into(),
            attempt: 1,
            reason,
        });
    }
    for (interval_seconds, max_ticks, ttl_seconds, keep_awake, purpose) in [
        (0, 1, 1, false, None),
        (3601, 1, 1, false, None),
        (1, 0, 1, false, None),
        (1, 10001, 1, false, None),
        (1, 1, 0, false, None),
        (1, 1, 86401, false, None),
        (1, 1, 1, true, None),
        (1, 1, 1, true, Some(" ".into())),
        (1, 1, 1, true, Some("a".repeat(257))),
        (
            1,
            1,
            1,
            true,
            Some(format!("sk-{}", "SYNTHETICFIXTURE".repeat(2))),
        ),
    ] {
        out.push(ScheduleCommand::RunLoop {
            namespace: None,
            interval_seconds,
            max_ticks,
            ttl_seconds,
            keep_awake,
            purpose,
        });
    }
    out
}
fn args(c: &ScheduleCommand) -> Vec<String> {
    let mut out = vec!["schedule".into()];
    match c {
        ScheduleCommand::Add {
            from_file,
            idempotency_key,
        } => out.extend([
            "add".into(),
            "--from-file".into(),
            from_file.to_str().unwrap().into(),
            "--idempotency-key".into(),
            idempotency_key.clone(),
        ]),
        ScheduleCommand::Update {
            namespace,
            id,
            from_file,
            expect_revision,
        } => out.extend([
            "update".into(),
            "--namespace".into(),
            namespace.clone(),
            id.clone(),
            "--from-file".into(),
            from_file.to_str().unwrap().into(),
            "--expect-revision".into(),
            expect_revision.to_string(),
        ]),
        ScheduleCommand::Remove {
            namespace,
            id,
            expect_revision,
        } => out.extend([
            "remove".into(),
            "--namespace".into(),
            namespace.clone(),
            id.clone(),
            "--expect-revision".into(),
            expect_revision.to_string(),
        ]),
        ScheduleCommand::Recover {
            namespace,
            id,
            revision,
            occurrence,
            attempt,
            reason,
        } => out.extend([
            "recover".into(),
            "--namespace".into(),
            namespace.clone(),
            id.clone(),
            "--revision".into(),
            revision.to_string(),
            "--occurrence".into(),
            occurrence.clone(),
            "--attempt".into(),
            attempt.to_string(),
            "--reason".into(),
            reason.clone(),
        ]),
        ScheduleCommand::List { namespace } => {
            out.push("list".into());
            if let Some(n) = namespace {
                out.extend(["--namespace".into(), n.clone()]);
            }
        }
        ScheduleCommand::Reconcile { namespace, at }
        | ScheduleCommand::Tick { namespace, at, .. } => {
            out.push(
                if matches!(c, ScheduleCommand::Tick { .. }) {
                    "tick"
                } else {
                    "reconcile"
                }
                .into(),
            );
            if let Some(n) = namespace {
                out.extend(["--namespace".into(), n.clone()]);
            }
            if let Some(at) = at {
                out.extend(["--at".into(), at.clone()]);
            }
        }
        ScheduleCommand::RunLoop {
            namespace,
            interval_seconds,
            max_ticks,
            ttl_seconds,
            keep_awake,
            purpose,
        } => {
            out.extend([
                "run-loop".into(),
                "--interval-seconds".into(),
                interval_seconds.to_string(),
                "--max-ticks".into(),
                max_ticks.to_string(),
                "--ttl-seconds".into(),
                ttl_seconds.to_string(),
            ]);
            if let Some(n) = namespace {
                out.extend(["--namespace".into(), n.clone()]);
            }
            if *keep_awake {
                out.push("--keep-awake".into());
            }
            if let Some(p) = purpose {
                out.extend(["--purpose".into(), p.clone()]);
            }
        }
        _ => unreachable!(),
    }
    out
}
#[test]
fn schedule_arguments_precede_project_and_response_effects() {
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
            for command in commands(&f.base.join("absent-definition.json")) {
                let output = f
                    .command()
                    .args(["--format", format])
                    .arg("--output")
                    .arg(&response)
                    .env("PCTX_ACTOR", "agent:fixture")
                    .args(args(&command))
                    .output()
                    .unwrap();
                assert_eq!(output.status.code(), Some(2), "{output:?}");
                let value: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(value["errors"][0]["code"], "INVALID_ARGUMENT");
                assert!(value["project_id"].is_null());
                assert!(output.stderr.is_empty());
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
fn direct_schedule_arguments_keep_original_expiry_and_no_effects() {
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    for command in commands(&f.base.join("absent-definition.json")) {
        let error = schedule::execute(&p, &command).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for command in commands(&f.base.join("absent-definition.json")) {
        assert_eq!(schedule::execute(&p, &command).unwrap_err().code, "TIMEOUT");
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}

#[test]
fn schedule_boundaries_preserve_existing_exceptions() {
    for value in ["a".repeat(256), "é".repeat(128), " ".into()] {
        for command in [
            ScheduleCommand::Add {
                from_file: "absent".into(),
                idempotency_key: value.clone(),
            },
            ScheduleCommand::Update {
                namespace: value.clone(),
                id: value.clone(),
                from_file: "absent".into(),
                expect_revision: 1,
            },
            ScheduleCommand::Remove {
                namespace: value.clone(),
                id: value.clone(),
                expect_revision: 1,
            },
            ScheduleCommand::Recover {
                namespace: value.clone(),
                id: value.clone(),
                revision: 1,
                occurrence: String::new(),
                attempt: 0,
                reason: format!("a{}", "\n".repeat(2047)),
            },
            ScheduleCommand::List {
                namespace: Some(value.clone()),
            },
        ] {
            schedule::validate_schedule_request(&command).unwrap();
        }
    }
    for at in [
        "2000-01-01T00:00:00Z",
        "2000-01-01T01:00:00+01:00",
        "2099-01-01T00:00:00.123Z",
    ] {
        for command in [
            ScheduleCommand::Reconcile {
                namespace: None,
                at: Some(at.into()),
            },
            ScheduleCommand::Tick {
                namespace: None,
                at: Some(at.into()),
                retry_failed: false,
            },
        ] {
            schedule::validate_schedule_request(&command).unwrap();
        }
    }
    for (interval_seconds, max_ticks, ttl_seconds) in [(1, 1, 1), (3600, 10000, 86400)] {
        for (keep_awake, purpose) in [
            (false, None),
            (false, Some("a".repeat(257))),
            (false, Some(" ".into())),
            (true, Some("a\nb".into())),
            (true, Some("a".repeat(256))),
        ] {
            schedule::validate_schedule_request(&ScheduleCommand::RunLoop {
                namespace: Some(String::new()),
                interval_seconds,
                max_ticks,
                ttl_seconds,
                keep_awake,
                purpose,
            })
            .unwrap();
        }
    }
    // These lookup keys have no existing standalone grammar. Recovery occurrence/
    // attempt/revision remain state-bound rather than new synthetic constraints.
    for command in [
        ScheduleCommand::Inspect {
            namespace: String::new(),
            id: String::new(),
            observe_native: false,
        },
        ScheduleCommand::Uninstall {
            namespace: String::new(),
            id: String::new(),
            expect_hash: String::new(),
            apply_native: false,
        },
    ] {
        schedule::validate_schedule_request(&command).unwrap();
    }
}

#[test]
fn native_schedule_boundaries_replay_and_future_tick_preserve_contracts() {
    let f = Fixture::new();
    f.init();
    let before = f.state();
    for namespace in ["a".repeat(256), " ".into()] {
        let output = f.run(&["schedule", "list", "--namespace", &namespace]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["data"]["schedules"], serde_json::json!([]));
        assert_eq!(f.state(), before);
    }
    let future = ScheduleCommand::Tick {
        namespace: None,
        at: Some("2099-01-01T00:00:00Z".into()),
        retry_failed: false,
    };
    schedule::validate_schedule_request(&future).unwrap();
    let output = f.run(&args(&future).iter().map(String::as_str).collect::<Vec<_>>());
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], "INVALID_ARGUMENT");
    assert_eq!(f.state(), before);
    // No eligible schedule: the unmatched namespace is never passed to tick,
    // and no keep-awake assertion or model/job invocation is acquired.
    let output = f.run(&[
        "schedule",
        "run-loop",
        "--namespace",
        "",
        "--interval-seconds",
        "1",
        "--max-ticks",
        "1",
        "--ttl-seconds",
        "1",
        "--purpose",
        "a\nb",
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["ticks"], serde_json::json!([]));
    assert_eq!(value["data"]["model_calls"], 0);
    assert_eq!(value["data"]["keep_awake_requested"], false);
    let path = f.base.join("definition.json");
    fs::write(
        &path,
        serde_json::json!({"schema_version":1,"namespace":"fixture","id":"fixture",
        "timezone":"Europe/Berlin","cadence":{"kind":"daily","at":"09:00"},
        "valid_from":"2024-01-01T00:00:00Z","job":"read_query","bridge":"manual",
        "role":"assistant","recipient":"owner","topic":"fixture"})
        .to_string(),
    )
    .unwrap();
    let key = "a".repeat(256);
    let add = || {
        f.run(&[
            "schedule",
            "add",
            "--from-file",
            path.to_str().unwrap(),
            "--idempotency-key",
            &key,
        ])
    };
    let first = add();
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let imported = f.state();
    let second = add();
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    let second: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(first["data"], second["data"]);
    assert_eq!(f.state(), imported);
}

#[test]
fn isolated_nonowner_schedule_admission_keeps_authority_and_expiry() {
    const MARKER: &str = "PCTX_SCHEDULE_PREFLIGHT_CHILD";
    if std::env::var_os(MARKER).is_none() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "isolated_nonowner_schedule_admission_keeps_authority_and_expiry",
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
                assert!(status.success(), "{status}");
                return;
            }
            if Instant::now() >= end {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("parent bound exceeded");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    for command in commands(&f.base.join("absent-definition.json")) {
        let error = schedule::execute(&p, &command).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let valid = [
        ScheduleCommand::Add {
            from_file: f.base.join("absent"),
            idempotency_key: "a".repeat(256),
        },
        ScheduleCommand::Update {
            namespace: "fixture".into(),
            id: "fixture".into(),
            from_file: f.base.join("absent"),
            expect_revision: 1,
        },
        ScheduleCommand::Remove {
            namespace: "fixture".into(),
            id: "fixture".into(),
            expect_revision: 1,
        },
        ScheduleCommand::Recover {
            namespace: "fixture".into(),
            id: "fixture".into(),
            revision: 1,
            occurrence: "fixture".into(),
            attempt: 1,
            reason: "a".repeat(2048),
        },
        ScheduleCommand::Tick {
            namespace: None,
            at: None,
            retry_failed: false,
        },
        ScheduleCommand::RunLoop {
            namespace: None,
            interval_seconds: 1,
            max_ticks: 1,
            ttl_seconds: 1,
            keep_awake: false,
            purpose: None,
        },
    ];
    for command in &valid {
        let error = schedule::execute(&p, command).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("POLICY_DENIED", 5));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for command in commands(&f.base.join("absent-definition.json"))
        .iter()
        .chain(valid.iter())
    {
        assert_eq!(schedule::execute(&p, command).unwrap_err().code, "TIMEOUT");
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}

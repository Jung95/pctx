//! PCTX01 argument/input admission; existing control features are not extended.
use pctx::{
    deadline::Deadline,
    operations::{self, OperationCommand, RoleCommand},
    project::{Config, Project, ProjectConfig, RootAnchor},
    quota::{self, QuotaCommand},
    schedule::{self, Cadence, ScheduleCommand, ScheduleDefinition},
    session::{self, SessionCommand},
    work::{self, TaskCommand, WorkCommand},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};
type FileSnapshot = BTreeMap<PathBuf, Option<Vec<u8>>>;
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
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.current_dir(&self.base)
            .args(["--format", "json", "--root"])
            .arg(&self.root)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_USER_CONFIG", self.base.join("absent-config"));
        c
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }
    fn init(&self) {
        fs::create_dir(&self.root).unwrap();
        fs::write(self.root.join("code.py"), "def code():\n    return 1\n").unwrap();
        let o = self.run(&["init"]);
        assert!(o.status.success(), "{o:?}");
    }
    fn state(&self) -> (FileSnapshot, FileSnapshot) {
        (snapshot(&self.root), snapshot(&self.data))
    }
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(base: &Path, dir: &Path, m: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        if !dir.exists() {
            return;
        }
        m.insert(dir.strip_prefix(base).unwrap().into(), None);
        for e in fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            assert!(!e.file_type().unwrap().is_symlink());
            let p = e.path();
            if p.is_dir() {
                walk(base, &p, m)
            } else {
                m.insert(
                    p.strip_prefix(base).unwrap().into(),
                    Some(fs::read(p).unwrap()),
                );
            }
        }
    }
    let mut m = BTreeMap::new();
    walk(root, root, &mut m);
    m
}
fn refusal(o: &Output, code: &str) -> Value {
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(o.stderr.is_empty());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["errors"][0]["code"], code);
    assert_eq!(v["status"], "error");
    v
}

fn no_effects(cases: &[Vec<&str>]) {
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
            for args in cases {
                let out = Command::new(env!("CARGO_BIN_EXE_pctx"))
                    .current_dir(&f.base)
                    .args(["--format", format, "--root"])
                    .arg(&f.root)
                    .arg("--output")
                    .arg(&response)
                    .env("PCTX_DATA_DIR", &f.data)
                    .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                    .args(args)
                    .output()
                    .unwrap();
                let value = refusal(&out, "INVALID_ARGUMENT");
                assert!(value["project_id"].is_null());
                assert_eq!(f.state(), before, "{args:?}");
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
fn quota_pure_arguments_precede_project_and_response_effects() {
    no_effects(&[
        vec!["quota", "report", "--window", ""],
        vec!["quota", "report", "--window", "0h"],
        vec!["quota", "report", "--window", "1m"],
        vec![
            "quota",
            "report",
            "--pool",
            "pool",
            "--max-age-seconds",
            "0",
        ],
        vec!["quota", "report", "--max-age-seconds", "0"],
        vec!["quota", "plan", "--pool", ""],
        vec![
            "quota",
            "plan",
            "--pool",
            "pool",
            "--max-age-seconds",
            "86401",
        ],
        vec!["quota", "reconcile", "--pool", ""],
        vec![
            "quota",
            "reserve",
            "--task-id",
            "missing",
            "--pool",
            "pool",
            "--unit",
            "bogus",
            "--amount",
            "1",
            "--limit",
            "2",
            "--idempotency-key",
            "key",
        ],
        vec![
            "quota",
            "reserve",
            "--task-id",
            "missing",
            "--pool",
            "pool",
            "--unit",
            "tokens",
            "--amount",
            "NaN",
            "--limit",
            "2",
            "--idempotency-key",
            "key",
        ],
        vec![
            "quota",
            "reserve",
            "--task-id",
            "missing",
            "--pool",
            "pool",
            "--unit",
            "tokens",
            "--amount",
            "3",
            "--limit",
            "2",
            "--idempotency-key",
            "key",
        ],
    ]);
}
#[test]
fn work_pure_arguments_precede_project_and_response_effects() {
    no_effects(&[
        vec![
            "task", "reassign", "missing", "--agent", "missing", "--reason", "",
        ],
        vec!["task", "block", "missing", "--reason", " "],
        vec!["task", "pause", "missing", "--reason", ""],
        vec!["task", "cancel", "missing", "--reason", ""],
        vec!["task", "reopen", "missing", "--reason", ""],
        vec!["agent", "register", "--name", "fixture", "--kind", "other"],
        vec!["agent", "register", "--name", ""],
        vec![
            "agent",
            "register",
            "--name",
            "fixture",
            "--concurrency-limit",
            "0",
        ],
    ]);
}
#[test]
fn schedule_pure_arguments_precede_project_and_response_effects() {
    let long = "r".repeat(2049);
    no_effects(&[
        vec!["schedule", "plan", "--namespace", "", "missing"],
        vec![
            "schedule",
            "plan",
            "--namespace",
            "fixture",
            "missing",
            "--staging-root",
            "../escape",
        ],
        vec![
            "schedule",
            "plan",
            "--namespace",
            "fixture",
            "missing",
            "--staging-root",
            "/absolute",
        ],
        vec![
            "schedule",
            "pause",
            "--namespace",
            "fixture",
            "missing",
            "--reason",
            "",
            "--expect-revision",
            "1",
        ],
        vec![
            "schedule",
            "resume",
            "--namespace",
            "fixture",
            "missing",
            "--reason",
            &long,
            "--expect-revision",
            "1",
        ],
    ]);
}
#[test]
fn session_pure_arguments_precede_project_and_response_effects() {
    no_effects(&[
        vec!["session", "attach", "--agent", "", "--runtime", "manual"],
        vec!["session", "attach", "--agent", "missing", "--runtime", ""],
        vec![
            "session",
            "attach",
            "--agent",
            "missing",
            "--runtime",
            "manual",
            "--native-session",
            "",
        ],
        vec![
            "session",
            "attach",
            "--agent",
            "missing",
            "--runtime",
            "manual",
            "--role",
            "",
        ],
        vec![
            "session",
            "attach",
            "--agent",
            "missing",
            "--runtime",
            "manual",
            "--account-pool",
            "",
        ],
        vec![
            "session",
            "attach",
            "--agent",
            "missing",
            "--runtime",
            "manual",
            "--adapter-version",
            "",
        ],
    ]);
}
#[test]
fn role_pure_arguments_precede_project_and_response_effects() {
    no_effects(&[
        vec!["role", "pause", "", "--reason", "why"],
        vec!["role", "pause", "fixture", "--reason", " "],
        vec!["role", "resume", "fixture", "--reason", ""],
    ]);
}

enum Request {
    Quota(QuotaCommand),
    Work(WorkCommand),
    Schedule(ScheduleCommand),
    Session(SessionCommand),
    Role(OperationCommand),
    Context(pctx::session::ContextCommand),
}
impl Request {
    fn execute(&self, p: &Project) -> pctx::domain::Result<Value> {
        match self {
            Self::Quota(c) => quota::execute(p, c),
            Self::Work(c) => work::execute(p, c),
            Self::Schedule(c) => schedule::execute(p, c),
            Self::Session(c) => session::session(p, c),
            Self::Role(c) => operations::execute(p, c),
            Self::Context(c) => session::context(p, c),
        }
    }
}
#[test]
fn direct_producers_refuse_before_database_and_preserve_original_expiry() {
    let f = Fixture::new();
    fs::create_dir(&f.root).unwrap();
    let mut p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&f.root).unwrap(),
        root: f.root.clone(),
        data_dir: f.data.clone(),
        workspace_dir: f.data.join("workspace"),
        control_dir: f.data.join("control"),
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
    fs::create_dir_all(&p.control_dir).unwrap();
    fs::create_dir_all(&p.workspace_dir).unwrap();
    let before = f.state();
    let requests = [
        Request::Quota(QuotaCommand::Report {
            pool: None,
            task_id: None,
            session: None,
            group_by: "pool".into(),
            window: "7d".into(),
            include_coordination: false,
            max_age_seconds: 0,
        }),
        Request::Quota(QuotaCommand::Reserve {
            task_id: "missing".into(),
            pool: "pool".into(),
            unit: "tokens".into(),
            amount: f64::NAN,
            limit: 2.0,
            policy: "fixture".into(),
            idempotency_key: "key".into(),
        }),
        Request::Work(WorkCommand::Task {
            command: TaskCommand::Block {
                task: "missing".into(),
                reason: "".into(),
            },
        }),
        Request::Schedule(ScheduleCommand::Pause {
            namespace: "fixture".into(),
            id: "missing".into(),
            reason: "".into(),
            expect_revision: 1,
        }),
        Request::Session(SessionCommand::Attach {
            agent: "missing".into(),
            runtime: "".into(),
            workspace: "current".into(),
            native_session: None,
            role: None,
            account_pool: None,
            adapter_version: "fixture".into(),
        }),
        Request::Role(OperationCommand::Role {
            command: RoleCommand::Pause {
                role: "fixture".into(),
                reason: "".into(),
                topic: None,
                recipient: None,
            },
        }),
    ];
    for request in &requests {
        let error = request.execute(&p).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for request in &requests {
        assert_eq!(request.execute(&p).unwrap_err().code, "TIMEOUT");
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}
#[test]
fn valid_control_arguments_reach_existing_authorization_and_state_flows() {
    let f = Fixture::new();
    f.init();
    let out = f.run(&[
        "agent",
        "register",
        "--name",
        "fixture",
        "--kind",
        "agent",
        "--concurrency-limit",
        "1",
    ]);
    assert!(out.status.success(), "{out:?}");
    let out = f.run(&[
        "session",
        "attach",
        "--agent",
        "fixture",
        "--runtime",
        "manual",
    ]);
    assert!(out.status.success(), "{out:?}");
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(value["data"]["session_id"].is_string());
    assert!(
        f.run(&[
            "quota",
            "report",
            "--window",
            "1h",
            "--max-age-seconds",
            "1"
        ])
        .status
        .success()
    );
    assert!(
        f.run(&[
            "quota",
            "plan",
            "--pool",
            "fixture",
            "--max-age-seconds",
            "86400"
        ])
        .status
        .success()
    );
    for command in ["pause", "resume"] {
        let out = f.run(&[
            "role",
            command,
            "fixture",
            "--reason",
            "explicit fixture action",
        ]);
        assert!(out.status.success(), "{out:?}");
    }
    let out = f.run(&["schedule", "plan", "--namespace", "fixture", "missing"]);
    assert_eq!(out.status.code(), Some(6), "{out:?}");
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], "SCHEDULE_NOT_FOUND");
    let out = f.run(&["task", "block", "missing", "--reason", "valid reason"]);
    assert_eq!(out.status.code(), Some(6), "{out:?}");
}

#[test]
fn malformed_reviewed_schedule_provider_refuses_without_panic_or_installation() {
    let f = Fixture::new();
    f.init();
    let definition = ScheduleDefinition {
        schema_version: 1,
        namespace: "fixture".into(),
        id: "owner-digest".into(),
        timezone: "Europe/Berlin".into(),
        cadence: Cadence::Daily { at: "12:00".into() },
        valid_from: "2025-01-01T00:00:00Z".into(),
        job: "read_query".into(),
        bridge: "manual".into(),
        role: "assistant".into(),
        recipient: "owner".into(),
        topic: "digest".into(),
        session_id: None,
        context_epoch: None,
        action: None,
        decision_id: None,
        enabled: true,
        misfire: "coalesce_latest".into(),
    };
    let input = f.base.join("definition.json");
    fs::write(&input, serde_json::to_vec(&definition).unwrap()).unwrap();
    let out = f
        .command()
        .args(["schedule", "add", "--from-file"])
        .arg(&input)
        .args(["--idempotency-key", "fixture-add"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let out = f.run(&["schedule", "plan", "--namespace", "fixture", "owner-digest"]);
    assert!(out.status.success(), "{out:?}");
    let mut value: Value = serde_json::from_slice(&out.stdout).unwrap();
    value["data"]["provider"] = serde_json::json!("bogus");
    let input = f.base.join("reviewed.json");
    fs::write(&input, serde_json::to_vec(&value["data"]).unwrap()).unwrap();
    let before = f.state();
    let out = f
        .command()
        .args(["schedule", "install", "--from-file"])
        .arg(&input)
        .args([
            "--expect-hash",
            value["data"]["plan_hash"].as_str().unwrap(),
        ])
        .output()
        .unwrap();
    refusal(&out, "INVALID_ARGUMENT");
    assert_eq!(f.state(), before);
}
#[test]
fn valid_grammar_does_not_bypass_non_owner_authority() {
    let f = Fixture::new();
    f.init();
    for args in [
        vec!["agent", "register", "--name", "fixture"],
        vec![
            "session",
            "attach",
            "--agent",
            "fixture",
            "--runtime",
            "manual",
        ],
        vec!["role", "resume", "fixture", "--reason", "prepare baseline"],
    ] {
        let out = f.run(&args);
        assert!(out.status.success(), "{out:?}");
    }
    let before = f.state();
    for args in [
        vec!["role", "pause", "fixture", "--reason", "explicit fixture"],
        vec!["agent", "register", "--name", "fixture"],
        vec![
            "session",
            "attach",
            "--agent",
            "missing",
            "--runtime",
            "manual",
        ],
        vec![
            "schedule",
            "pause",
            "--namespace",
            "fixture",
            "missing",
            "--reason",
            "explicit fixture",
            "--expect-revision",
            "1",
        ],
        vec![
            "quota",
            "reserve",
            "--task-id",
            "missing",
            "--pool",
            "fixture",
            "--unit",
            "tokens",
            "--amount",
            "1",
            "--limit",
            "2",
            "--idempotency-key",
            "key",
        ],
    ] {
        let out = f
            .command()
            .env("PCTX_ACTOR", "agent:unregistered-fixture")
            .args(args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(5), "{out:?}");
        let value: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(value["errors"][0]["code"], "POLICY_DENIED");
        assert_eq!(f.state(), before);
    }
}

#[test]
fn runner_pure_arguments_precede_project_and_response_effects() {
    let long = "HELP-".to_owned() + &"x".repeat(60);
    let scopes = vec!["a"; 65];
    let mut many = vec![
        "runner",
        "helper-request",
        "--task-id",
        "missing",
        "--key",
        "fixture",
        "--run",
        "missing",
    ];
    for scope in &scopes {
        many.extend(["--scope", scope]);
    }
    no_effects(&[
        vec!["job", "cancel", "../escape"],
        vec!["runner", "job-cancel", "JOB-../escape"],
        vec!["runner", "helper-status", ""],
        vec!["runner", "helper-status", &long],
        vec!["runner", "helper-cancel", "HELP-../escape"],
        vec![
            "runner",
            "helper-release",
            "wrong",
            "--evidence",
            "OUT-fixture",
        ],
        vec![
            "runner",
            "helper-request",
            "--task-id",
            "missing",
            "--key",
            "fixture",
            "--run",
            "missing",
            "--mode",
            "bogus",
        ],
        vec![
            "runner",
            "helper-request",
            "--task-id",
            "missing",
            "--key",
            "fixture",
            "--run",
            "missing",
            "--mode",
            "cloud",
        ],
        vec![
            "runner",
            "helper-request",
            "--task-id",
            "missing",
            "--key",
            "fixture",
            "--run",
            "missing",
            "--mode",
            "native",
        ],
        vec![
            "runner",
            "helper-request",
            "--task-id",
            "missing",
            "--key",
            "fixture",
            "--run",
            "missing",
            "--scope",
            "sk-proj-FAKE012345678901234567890123456789012345678901234567890",
        ],
        many,
    ]);
}

#[test]
fn runner_direct_pure_refusal_preserves_storage_and_original_expiry() {
    use pctx::runner::{self, RunnerCommand};
    let f = Fixture::new();
    f.init();
    let mut p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&f.root).unwrap(),
        root: f.root.clone(),
        data_dir: f.data.clone(),
        workspace_dir: f.data.join("workspace"),
        control_dir: f.data.join("control"),
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
    let before = f.state();
    let cases = [
        RunnerCommand::JobCancel {
            job: "../escape".into(),
        },
        RunnerCommand::HelperStatus {
            helper: "wrong".into(),
        },
        RunnerCommand::HelperCancel {
            helper: "wrong".into(),
        },
        RunnerCommand::HelperRelease {
            helper: "wrong".into(),
            evidence: "OUT-fixture".into(),
        },
        RunnerCommand::HelperRequest {
            task_id: "missing".into(),
            key: "fixture".into(),
            run: "missing".into(),
            mode: "bogus".into(),
            scope: vec![],
            budget_bytes: 8192,
        },
        RunnerCommand::HelperRequest {
            task_id: "missing".into(),
            key: "fixture".into(),
            run: "missing".into(),
            mode: "cloud".into(),
            scope: vec![],
            budget_bytes: 8192,
        },
    ];
    for c in &cases {
        let e = runner::execute(&p, c).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for c in &cases {
        assert_eq!(runner::execute(&p, c).unwrap_err().code, "TIMEOUT");
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}

#[test]
fn runner_valid_boundaries_preserve_existing_non_owner_policy() {
    use pctx::runner::{RunnerCommand, validate_runner_request};
    for (prefix, helper) in [("JOB-", false), ("HELP-", true)] {
        for len in [64, 65] {
            let id = format!("{prefix}{}", "x".repeat(len - prefix.len()));
            let c = if helper {
                RunnerCommand::HelperStatus { helper: id }
            } else {
                RunnerCommand::JobCancel { job: id }
            };
            assert_eq!(validate_runner_request(&c).is_ok(), len == 64);
        }
    }
    for mode in ["local", "cloud", "native"] {
        let c = RunnerCommand::HelperRequest {
            task_id: "missing".into(),
            key: "unit".into(),
            run: "missing".into(),
            mode: mode.into(),
            scope: vec!["code.py".into(); 64],
            budget_bytes: 8192,
        };
        validate_runner_request(&c).unwrap();
    }
    let f = Fixture::new();
    f.init();
    let before = f.state();
    for args in [
        vec!["job", "cancel", "JOB-fixture"],
        vec!["runner", "helper-cancel", "HELP-fixture"],
        vec![
            "runner",
            "helper-release",
            "HELP-fixture",
            "--evidence",
            "OUT-fixture",
        ],
        vec![
            "runner",
            "helper-request",
            "--task-id",
            "missing",
            "--key",
            "unit",
            "--run",
            "missing",
            "--mode",
            "cloud",
            "--scope",
            "code.py",
        ],
    ] {
        let o = f
            .command()
            .env("PCTX_ACTOR", "agent:unregistered-fixture")
            .args(args)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(5), "{o:?}");
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["errors"][0]["code"], "POLICY_DENIED");
        assert_eq!(f.state(), before);
    }
}

#[test]
fn operation_remaining_pure_arguments_precede_project_and_response_effects() {
    let long = "x".repeat(4097);
    let long_label = "r".repeat(257);
    no_effects(&[
        vec!["owner", "queue", "--limit", "0"],
        vec!["owner", "queue", "--limit", "1001"],
        vec!["inbox", "read", "--session", "missing", "--limit", "0"],
        vec!["inbox", "read", "--session", "missing", "--limit", "1001"],
        vec!["inbox", "read", "--session", "missing", "--since-seq=-1"],
        vec!["message", "send", "--from-file", "absent.json"],
        vec![
            "message",
            "send",
            "--from-file",
            "absent.json",
            "--to-role",
            "review",
            "--to-agent",
            "missing",
        ],
        vec![
            "message",
            "send",
            "--from-file",
            "absent.json",
            "--to-role",
            "",
        ],
        vec!["message", "resolve", "missing", "--evidence", ""],
        vec!["role", "pause", " review", "--reason", "fixture"],
        vec!["role", "pause", &long_label, "--reason", "fixture"],
        vec![
            "role",
            "pause",
            "review",
            "--reason",
            "fixture",
            "--topic",
            &long_label,
        ],
        vec!["role", "resume", "review", "--reason", &long],
        vec![
            "role",
            "pause",
            "review",
            "--reason",
            "fixture",
            "--recipient",
            "owner",
        ],
        vec![
            "role", "pause", "review", "--reason", "fixture", "--topic", "",
        ],
    ]);
}

#[test]
fn operation_direct_refusals_preserve_state_and_original_expiry() {
    use pctx::operations::{InboxCommand, MessageCommand, OwnerCommand};
    let f = Fixture::new();
    fs::create_dir(&f.root).unwrap();
    let mut p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&f.root).unwrap(),
        root: f.root.clone(),
        data_dir: f.data.clone(),
        workspace_dir: f.data.join("workspace"),
        control_dir: f.data.join("control"),
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
    let before = f.state();
    let commands = [
        OperationCommand::Owner {
            command: OwnerCommand::Queue { limit: 0 },
        },
        OperationCommand::Inbox {
            command: InboxCommand::Read {
                session: "missing".into(),
                since_seq: -1,
                limit: 1,
            },
        },
        OperationCommand::Message {
            command: MessageCommand::Send {
                from_file: f.base.join("absent.json"),
                to_role: None,
                to_agent: None,
                to_session: None,
                task_id: None,
            },
        },
        OperationCommand::Message {
            command: MessageCommand::Resolve {
                id: "missing".into(),
                evidence: "".into(),
            },
        },
        OperationCommand::Role {
            command: RoleCommand::Pause {
                role: " review".into(),
                reason: "fixture".into(),
                topic: None,
                recipient: None,
            },
        },
        OperationCommand::Role {
            command: RoleCommand::Resume {
                role: "review".into(),
                reason: "x".repeat(4097),
                topic: None,
                recipient: None,
            },
        },
    ];
    for c in &commands {
        let e = operations::execute(&p, c).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for c in &commands {
        assert_eq!(operations::execute(&p, c).unwrap_err().code, "TIMEOUT");
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}

#[test]
fn operation_boundaries_aliases_and_valid_non_owner_policy_are_preserved() {
    let f = Fixture::new();
    f.init();
    for limit in ["1", "1000"] {
        let o = f.run(&["owner", "queue", "--limit", limit]);
        assert!(o.status.success(), "{o:?}");
    }
    let registered = f.run(&["agent", "register", "--name", "inbox-fixture"]);
    assert!(registered.status.success());
    let attached = f.run(&[
        "session",
        "attach",
        "--agent",
        "inbox-fixture",
        "--runtime",
        "manual",
    ]);
    assert!(attached.status.success(), "{attached:?}");
    let v: Value = serde_json::from_slice(&attached.stdout).unwrap();
    let sid = v["data"]["session_id"].as_str().unwrap();
    for limit in ["1", "1000"] {
        let o = f.run(&["inbox", "read", "--session", sid, "--limit", limit]);
        assert!(o.status.success(), "{o:?}");
    }
    let label = "r".repeat(256);
    let reason_bound = "x".repeat(4096);
    pctx::policy_controls::validate_restriction_arguments(
        &label,
        Some(&label),
        Some(&label),
        &reason_bound,
    )
    .unwrap();
    assert_eq!(
        pctx::policy_controls::validate_restriction_arguments(
            &"r".repeat(257),
            None,
            None,
            "fixture"
        )
        .unwrap_err()
        .exit,
        2
    );
    let role = "r".repeat(256);
    let reason = "x".repeat(4096);
    let o = f.run(&["role", "pause", &role, "--reason", &reason]);
    assert!(o.status.success(), "{o:?}");
    let o = f.run(&["role", "resume", &role, "--reason", &reason]);
    assert!(o.status.success(), "{o:?}");
    for alias in [" spaced-alias", "\tcontrol-alias"] {
        let o = f.run(&["agent", "register", "--name", alias]);
        assert!(o.status.success(), "{o:?}");
        let o = f.run(&[
            "role",
            "pause",
            "review",
            "--reason",
            "fixture",
            "--topic",
            "topic",
            "--recipient",
            alias,
        ]);
        assert!(o.status.success(), "{o:?}");
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["data"]["active"], true);
        let o = f.run(&[
            "role",
            "resume",
            "review",
            "--reason",
            "fixture",
            "--topic",
            "topic",
            "--recipient",
            alias,
        ]);
        assert!(o.status.success(), "{o:?}");
    }
    let before = f.state();
    for args in [
        vec!["owner", "queue", "--limit", "1"],
        vec!["role", "pause", "review", "--reason", "fixture"],
        vec!["message", "resolve", "missing", "--evidence", "evidence"],
        vec![
            "message",
            "send",
            "--from-file",
            "absent.json",
            "--to-role",
            "review",
        ],
    ] {
        let o = f
            .command()
            .env("PCTX_ACTOR", "agent:unregistered-fixture")
            .args(args)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(5), "{o:?}");
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["errors"][0]["code"], "POLICY_DENIED");
        assert_eq!(f.state(), before);
    }
}

#[test]
fn report_session_ingest_arguments_precede_project_and_response_effects() {
    let long_key = "k".repeat(257);
    let summary = "x".repeat(4097);
    let reason = "r".repeat(1025);
    let report = vec![
        "agent",
        "report",
        "--run",
        "missing",
        "--lease-epoch",
        "1",
        "--report-seq",
        "1",
        "--idempotency-key",
        "key",
        "--stage",
        "bogus",
        "--summary",
        "fixture",
    ];
    let mut large = report.clone();
    large[11] = "planning";
    large[13] = &summary;
    let mut estimate = report.clone();
    estimate[11] = "testing";
    estimate.extend(["--estimate-percent", "101"]);
    // All scalar options remain parseable; refusal belongs to shared semantic admission.
    no_effects(&[
        vec![
            "quota",
            "ingest",
            "--from-file",
            "absent.json",
            "--idempotency-key",
            &long_key,
        ],
        report.clone(),
        large,
        estimate,
        vec![
            "session",
            "boundary",
            "--session",
            "missing",
            "--reason",
            "",
        ],
        vec![
            "session",
            "suspend",
            "--session",
            "missing",
            "--reason",
            " ",
        ],
        vec![
            "session",
            "boundary",
            "--session",
            "missing",
            "--reason",
            &reason,
        ],
        vec![
            "session",
            "suspend",
            "--session",
            "missing",
            "--reason",
            "sk-proj-FAKE01234567890123456789012345678901234567890",
        ],
        vec![
            "quota",
            "ingest",
            "--from-file",
            "absent.json",
            "--idempotency-key",
            "",
        ],
    ]);
}

#[test]
fn report_session_ingest_direct_refusals_preserve_storage_and_original_expiry() {
    use pctx::session::ContextCommand;
    use pctx::work::{AgentCommand, CheckCommand};
    let f = Fixture::new();
    fs::create_dir(&f.root).unwrap();
    let mut p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&f.root).unwrap(),
        root: f.root.clone(),
        data_dir: f.data.clone(),
        workspace_dir: f.data.join("workspace"),
        control_dir: f.data.join("control"),
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
    let before = f.state();
    let keyless = WorkCommand::Check {
        command: CheckCommand::Run {
            key: None,
            registered_key: None,
            task_id: "missing".into(),
            run: "missing".into(),
            budget_bytes: 8192,
        },
    };
    assert_eq!(work::validate_work_request(&keyless).unwrap_err().exit, 2);
    let ack = ContextCommand::Ack {
        context: "missing".into(),
        session: "missing".into(),
        epoch: 1,
        provenance: "bogus".into(),
    };
    assert_eq!(session::validate_context_request(&ack).unwrap_err().exit, 2);
    let requests = [
        Request::Work(WorkCommand::Agent {
            command: AgentCommand::Report {
                run: "missing".into(),
                lease_epoch: 1,
                report_seq: 1,
                idempotency_key: "fixture".into(),
                stage: "bogus".into(),
                summary: "fixture".into(),
                estimate_percent: None,
            },
        }),
        Request::Work(keyless),
        Request::Session(SessionCommand::Boundary {
            session: "missing".into(),
            reason: "".into(),
        }),
        Request::Session(SessionCommand::Suspend {
            session: "missing".into(),
            reason: "r".repeat(1025),
        }),
        Request::Context(ack),
        Request::Quota(QuotaCommand::Ingest {
            from_file: f.base.join("absent.json"),
            idempotency_key: "".into(),
        }),
    ];
    for r in &requests {
        let e = r.execute(&p).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for r in &requests {
        assert_eq!(r.execute(&p).unwrap_err().code, "TIMEOUT");
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}

#[test]
fn report_and_session_accepted_boundaries_keep_replay_and_state_contracts() {
    use pctx::session::ContextCommand;
    use pctx::work::AgentCommand;
    let f = Fixture::new();
    f.init();
    let input = f.base.join("task.json");
    fs::write(&input,serde_json::json!({"schema_version":1,"title":"Report fixture","scope":["code.py"],"acceptance":[{"id":"behavior","description":"Observed","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
    let o = f
        .command()
        .args(["task", "create", "--from-file"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    let task = v["data"]["task_id"].as_str().unwrap();
    assert!(
        f.run(&["agent", "register", "--name", "report-fixture"])
            .status
            .success()
    );
    assert!(f.run(&["task", "ready", task]).status.success());
    assert!(
        f.run(&["task", "assign", task, "--agent", "report-fixture"])
            .status
            .success()
    );
    let o = f.run(&["task", "start", task]);
    assert!(o.status.success(), "{o:?}");
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    let run = v["data"]["run_id"].as_str().unwrap();
    let epoch = v["data"]["lease_epoch"].as_i64().unwrap().to_string();
    let summary = "x".repeat(4096);
    for (i, stage) in [
        "planning",
        "implementing",
        "testing",
        "waiting",
        "submitting",
    ]
    .iter()
    .enumerate()
    {
        let seq = (i + 1).to_string();
        let key = format!("boundary-{i}");
        let args = [
            "agent",
            "report",
            "--run",
            run,
            "--lease-epoch",
            &epoch,
            "--report-seq",
            &seq,
            "--idempotency-key",
            &key,
            "--stage",
            stage,
            "--summary",
            &summary,
            "--estimate-percent",
            "100",
        ];
        let o = f.run(&args);
        assert!(o.status.success(), "{o:?}");
        let first: Value = serde_json::from_slice(&o.stdout).unwrap();
        let before_replay = f.state();
        let replay = f.run(&args);
        assert!(replay.status.success());
        let second: Value = serde_json::from_slice(&replay.stdout).unwrap();
        assert_eq!(first["data"], second["data"]);
        assert_eq!(f.state(), before_replay);
    }
    let empty = WorkCommand::Agent {
        command: AgentCommand::Report {
            run: run.into(),
            lease_epoch: 1,
            report_seq: 1,
            idempotency_key: "fixture".into(),
            stage: "planning".into(),
            summary: "".into(),
            estimate_percent: Some(0),
        },
    };
    work::validate_work_request(&empty).unwrap();
    let o = f.run(&[
        "session",
        "attach",
        "--agent",
        "report-fixture",
        "--runtime",
        "manual",
    ]);
    assert!(o.status.success());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    let sid = v["data"]["session_id"].as_str().unwrap();
    let reason = "r".repeat(1024);
    let o = f.run(&["session", "boundary", "--session", sid, "--reason", &reason]);
    assert!(o.status.success(), "{o:?}");
    let o = f.run(&["session", "suspend", "--session", sid, "--reason", &reason]);
    assert!(o.status.success(), "{o:?}");
    for provenance in ["explicit-agent", "transport-receipt"] {
        session::validate_context_request(&ContextCommand::Ack {
            context: "missing".into(),
            session: sid.into(),
            epoch: 1,
            provenance: provenance.into(),
        })
        .unwrap();
    }
    quota::validate_quota_request(&QuotaCommand::Ingest {
        from_file: f.base.join("absent.json"),
        idempotency_key: "k".repeat(256),
    })
    .unwrap();
    let now = chrono::Utc::now();
    let usage = f.base.join("usage.json");
    fs::write(&usage,serde_json::json!({"schema_version":1,"observations":[{"observation_id":"quota-boundary-fixture","pool_id":"fixture","provider":"fixture","model":"none","metric":"subscription_quota","unit":"percentage","source":"manual","collector":"fixture-v1","source_revision":"fixture-v1","observed_at":now.to_rfc3339(),"window_id":"fixture","window_start":(now-chrono::Duration::hours(1)).to_rfc3339(),"window_end":(now+chrono::Duration::hours(1)).to_rfc3339(),"status":"actual","amount":10.0,"kind":"quota","counter_origin_zero":false,"workload":"coordination"}]}).to_string()).unwrap();
    let key = "k".repeat(256);
    let ingest = || {
        f.command()
            .args(["quota", "ingest", "--from-file"])
            .arg(&usage)
            .args(["--idempotency-key", &key])
            .output()
            .unwrap()
    };
    let first = ingest();
    assert!(first.status.success(), "{first:?}");
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let before = f.state();
    let second = ingest();
    assert!(second.status.success());
    let second: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(first["data"], second["data"]);
    assert_eq!(f.state(), before);
    let denied = f
        .command()
        .env("PCTX_ACTOR", "agent:unregistered-fixture")
        .args(["quota", "ingest", "--from-file"])
        .arg(&usage)
        .args(["--idempotency-key", &key])
        .output()
        .unwrap();
    assert_eq!(denied.status.code(), Some(5));
    assert_eq!(f.state(), before);
}

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
}
impl Request {
    fn execute(&self, p: &Project) -> pctx::domain::Result<Value> {
        match self {
            Self::Quota(c) => quota::execute(p, c),
            Self::Work(c) => work::execute(p, c),
            Self::Schedule(c) => schedule::execute(p, c),
            Self::Session(c) => session::session(p, c),
            Self::Role(c) => operations::execute(p, c),
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

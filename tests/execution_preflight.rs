//! PCTX01-G03-R08: exact execution input grammar before effects.
use pctx::{
    deadline::Deadline,
    output::{self, RunRequest, TrustCommand},
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
fn request(argv: Vec<String>) -> RunRequest {
    RunRequest {
        task_id: None,
        session: None,
        retain: "temporary".into(),
        execution_timeout_ms: Some(1000),
        budget_bytes: 8192,
        exit_policy: "child".into(),
        stdin: "closed".into(),
        argv,
    }
}
fn invalid_argv() -> [Vec<String>; 3] {
    [
        vec![],
        vec!["missing-executable".into(); 257],
        vec!["x".repeat(17000); 4],
    ]
}
#[test]
fn execution_argv_limits_precede_project_and_response_effects() {
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
            for argv in invalid_argv().into_iter().skip(1) {
                for route in [
                    vec!["run"],
                    vec!["trust", "plan"],
                    vec!["trust", "add", "--expect-hash", "fixture"],
                ] {
                    let output = Command::new(env!("CARGO_BIN_EXE_pctx"))
                        .current_dir(&f.base)
                        .args(["--format", format, "--root"])
                        .arg(&f.root)
                        .arg("--output")
                        .arg(&response)
                        .env("PCTX_DATA_DIR", &f.data)
                        .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                        .env("PCTX_ACTOR", "agent:fixture")
                        .args(&route)
                        .arg("--")
                        .args(&argv)
                        .output()
                        .unwrap();
                    assert_eq!(output.status.code(), Some(2), "{route:?}: {output:?}");
                    assert!(output.stderr.is_empty());
                    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
                    assert_eq!(value["errors"][0]["code"], "INVALID_ARGUMENT");
                    assert_eq!(value["status"], "error");
                    assert!(value["project_id"].is_null());
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
}
#[test]
fn direct_execution_refusals_preserve_storage_and_original_expiry() {
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    let mut requests: Vec<_> = invalid_argv().into_iter().map(request).collect();
    for field in ["retain", "stdin", "exit_policy"] {
        let mut r = request(vec!["missing-executable".into()]);
        match field {
            "retain" => r.retain = "bogus".into(),
            "stdin" => r.stdin = "bogus".into(),
            _ => r.exit_policy = "bogus".into(),
        };
        requests.push(r);
    }
    for r in &requests {
        let error = output::run(&p, r).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
        let data = output::run_cli(&p, r);
        assert_eq!(data["pctx_error"], "INVALID_ARGUMENT");
        assert_eq!(data["spawned"], false);
        assert_eq!(data["termination"], "not_started");
        assert_eq!(f.state(), before);
    }
    for argv in invalid_argv() {
        for command in [
            TrustCommand::Plan { argv: argv.clone() },
            TrustCommand::Add {
                argv,
                expect_hash: "fixture".into(),
            },
        ] {
            let error = output::trust(&p, &command).unwrap_err();
            assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
            assert_eq!(f.state(), before);
        }
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for r in &requests {
        assert_eq!(output::run(&p, r).unwrap_err().code, "TIMEOUT");
        let data = output::run_cli(&p, r);
        assert_eq!(data["pctx_error"], "TIMEOUT");
        assert_eq!(data["spawned"], false);
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
    for argv in invalid_argv() {
        for command in [
            TrustCommand::Plan { argv: argv.clone() },
            TrustCommand::Add {
                argv,
                expect_hash: "fixture".into(),
            },
        ] {
            assert_eq!(output::trust(&p, &command).unwrap_err().code, "TIMEOUT");
            assert_eq!(p.deadline.unwrap().instant(), original);
            assert_eq!(f.state(), before);
        }
    }
}
#[test]
fn shared_execution_boundaries_preserve_modes_empty_elements_and_budget_priority() {
    for retain in ["temporary", "none"] {
        for exit in ["child", "pctx"] {
            let mut r = request(vec![
                String::new(),
                " ".into(),
                "--literal-option".into(),
                "한글".into(),
            ]);
            r.retain = retain.into();
            r.exit_policy = exit.into();
            output::validate_run_request(&r).unwrap();
        }
    }
    for argv in [vec![String::new(); 256], vec!["한".repeat(21845) + "x"]] {
        assert!(argv.iter().map(String::len).sum::<usize>() <= 65536);
        output::validate_run_request(&request(argv.clone())).unwrap();
        output::validate_trust_request(&TrustCommand::Plan { argv }).unwrap();
    }
    for argv in [
        vec![String::new(); 257],
        vec!["한".repeat(21845) + "xx"],
        vec![],
    ] {
        assert_eq!(
            output::validate_run_request(&request(argv.clone()))
                .unwrap_err()
                .exit,
            2
        );
        assert_eq!(
            output::validate_trust_request(&TrustCommand::Add {
                argv,
                expect_hash: String::new()
            })
            .unwrap_err()
            .exit,
            2
        );
    }
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    let mut r = request(vec![]);
    r.budget_bytes = 2999;
    r.stdin = "bogus".into();
    let error = output::run(&p, &r).unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), ("BUDGET_TOO_SMALL", 8));
    assert_eq!(f.state(), before);
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    assert_eq!(output::run(&p, &r).unwrap_err().code, "TIMEOUT");
    assert_eq!(p.deadline.unwrap().instant(), original);
    assert_eq!(f.state(), before);
}
#[test]
fn isolated_nonowner_trust_admission_keeps_invalid_expired_and_authority_distinct() {
    const MARKER: &str = "PCTX_TEST_NONOWNER_TRUST_ADMISSION";
    if std::env::var_os(MARKER).is_none() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "isolated_nonowner_trust_admission_keeps_invalid_expired_and_authority_distinct",
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
                assert!(status.success());
                break;
            }
            if Instant::now() >= end {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Isolated admission child exceeded parent bound");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        return;
    }
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    for argv in invalid_argv() {
        let error = output::trust(
            &p,
            &TrustCommand::Add {
                argv,
                expect_hash: "fixture".into(),
            },
        )
        .unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let command = TrustCommand::Add {
        argv: vec!["missing-executable".into()],
        expect_hash: "fixture".into(),
    };
    assert_eq!(
        output::trust(&p, &command).unwrap_err().code,
        "POLICY_DENIED"
    );
    assert_eq!(f.state(), before);
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for argv in invalid_argv() {
        assert_eq!(
            output::trust(
                &p,
                &TrustCommand::Add {
                    argv,
                    expect_hash: "fixture".into()
                }
            )
            .unwrap_err()
            .code,
            "TIMEOUT"
        );
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}
#[cfg(unix)]
#[test]
fn native_trust_plan_accepts_exact_boundaries_without_execution_or_trust_writes() {
    let f = Fixture::new();
    f.init();
    let before = f.state();
    let executable = "/usr/bin/printf";
    let mut count = vec!["fixture".to_string(); 256];
    count[0] = executable.into();
    let mut bytes = vec![executable.to_string()];
    let remaining = 65536 - executable.len();
    for _ in 0..4 {
        bytes.push("한".repeat(5000));
    }
    let tail = remaining - 60000;
    bytes.push("한".repeat(tail / 3) + &"x".repeat(tail % 3));
    assert_eq!(bytes.iter().map(String::len).sum::<usize>(), 65536);
    for argv in [count, bytes] {
        let output = f
            .command()
            .args(["trust", "plan", "--"])
            .args(&argv)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(value["data"]["fingerprint"].as_str().is_some());
        assert_eq!(f.state(), before);
        let output = f
            .command()
            .env("PCTX_ACTOR", "agent:fixture")
            .args(["trust", "add", "--expect-hash", "fixture", "--"])
            .args(&argv)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(5), "{output:?}");
        assert_eq!(f.state(), before);
    }
    let output = f.run(&[
        "trust",
        "add",
        "--expect-hash",
        "incorrect",
        "--",
        executable,
        "fixture",
    ]);
    assert_eq!(output.status.code(), Some(9), "{output:?}");
    assert_eq!(f.state(), before);
}

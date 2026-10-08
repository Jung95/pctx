//! PCTX01-G03-C004: existing Install hash admission.
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

fn install(plan: &str, expected: &str) -> AdapterCommand {
    AdapterCommand::Claude {
        command: ClaudeCommand::Install {
            plan: plan.into(),
            expect_hash: expected.into(),
        },
    }
}
fn invalid() -> Vec<(String, String)> {
    let mut cases: Vec<_> = [
        "",
        &"a".repeat(63),
        &"a".repeat(65),
        &"g".repeat(64),
        &"é".repeat(32),
    ]
    .into_iter()
    .map(|s| (s.to_string(), s.to_string()))
    .collect();
    cases.extend([
        ("a".repeat(64), String::new()),
        ("a".repeat(64), "b".repeat(64)),
        ("a".repeat(64), "A".repeat(64)),
    ]);
    cases
}
#[test]
fn native_invalid_install_is_before_scope_authority_and_response() {
    for initialized in [false, true] {
        let f = Fixture::new();
        if initialized {
            f.init();
        }
        for existing in [false, true] {
            let response = f.base.join("response.json");
            if existing {
                fs::write(&response, "original response").unwrap();
            }
            let before = f.state();
            for format in ["json", "compact"] {
                for (plan, expected) in invalid() {
                    let o = Command::new(env!("CARGO_BIN_EXE_pctx"))
                        .current_dir(&f.base)
                        .args(["--format", format, "--root"])
                        .arg(&f.root)
                        .arg("--output")
                        .arg(&response)
                        .env("PCTX_DATA_DIR", &f.data)
                        .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                        .env("PCTX_ACTOR", "agent:fixture")
                        .args([
                            "adapter",
                            "claude",
                            "install",
                            "--plan",
                            &plan,
                            "--expect-hash",
                            &expected,
                        ])
                        .output()
                        .unwrap();
                    assert_eq!(o.status.code(), Some(2), "{o:?}");
                    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
                    assert_eq!(v["errors"][0]["code"], "PLAN_MISMATCH");
                    assert!(v["project_id"].is_null());
                    assert!(v["data"].is_null());
                    assert!(o.stderr.is_empty());
                    assert_eq!(f.state(), before);
                    if existing {
                        assert_eq!(fs::read(&response).unwrap(), b"original response");
                    } else {
                        assert!(!response.exists());
                    }
                }
            }
        }
    }
}
#[test]
fn direct_invalid_install_keeps_state_and_original_expiry() {
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    for (plan, expected) in invalid() {
        let e = adapter::execute(&p, &install(&plan, &expected)).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("PLAN_MISMATCH", 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for (plan, expected) in invalid() {
        let e = adapter::execute(&p, &install(&plan, &expected)).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("TIMEOUT", 7));
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}
#[test]
fn install_hash_admission_preserves_exact_hex_grammar() {
    for plan in ["a".repeat(64), "F".repeat(64), "aF09".repeat(16)] {
        adapter::validate_adapter_request(&install(&plan, &plan)).unwrap();
    }
    for (plan, expected) in invalid() {
        let e = adapter::validate_adapter_request(&install(&plan, &expected)).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("PLAN_MISMATCH", 2));
    }
}
#[test]
fn isolated_nonowner_direct_install_admission_and_authority() {
    const MARKER: &str = "PCTX_INSTALL_PREFLIGHT_CHILD";
    if std::env::var_os(MARKER).is_none() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "isolated_nonowner_direct_install_admission_and_authority",
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
    let p = f.project();
    let before = f.state();
    for (plan, expected) in invalid() {
        let e = adapter::execute(&p, &install(&plan, &expected)).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("PLAN_MISMATCH", 2));
        assert_eq!(f.state(), before);
    }
    let hash = "a".repeat(64);
    let e = adapter::execute(&p, &install(&hash, &hash)).unwrap_err();
    assert_eq!((e.code.as_str(), e.exit), ("POLICY_DENIED", 5));
    assert_eq!(f.state(), before);
}
#[test]
fn native_accepted_install_keeps_authority_and_publishes_only_planned_settings() {
    let f = Fixture::new();
    f.init();
    let hash = "a".repeat(64);
    let before = f.state();
    let denied = f
        .command()
        .env("PCTX_ACTOR", "agent:fixture")
        .args([
            "adapter",
            "claude",
            "install",
            "--plan",
            &hash,
            "--expect-hash",
            &hash,
        ])
        .output()
        .unwrap();
    assert_eq!(denied.status.code(), Some(5), "{denied:?}");
    let denied: Value = serde_json::from_slice(&denied.stdout).unwrap();
    assert_eq!(denied["errors"][0]["code"], "POLICY_DENIED");
    assert_eq!(f.state(), before);
    fs::create_dir(f.root.join(".claude")).unwrap();
    let settings = f.root.join(".claude/settings.local.json");
    let original = serde_json::json!({"permissions":{"allow":["Read"]},"statusLine":{"type":"command","command":"private-fixture-status"}});
    fs::write(&settings, original.to_string()).unwrap();
    let planned = f.run(&["adapter", "claude", "plan", "--agent", "fixture"]);
    assert_eq!(planned.status.code(), Some(0), "{planned:?}");
    let planned: Value = serde_json::from_slice(&planned.stdout).unwrap();
    let hash = planned["data"]["plan_id"].as_str().unwrap();
    assert_eq!(planned["data"]["plan_hash"], hash);
    assert_eq!(
        fs::read(&settings).unwrap(),
        original.to_string().as_bytes()
    );
    let installed = f.run(&[
        "adapter",
        "claude",
        "install",
        "--plan",
        hash,
        "--expect-hash",
        hash,
    ]);
    assert_eq!(installed.status.code(), Some(0), "{installed:?}");
    let installed: Value = serde_json::from_slice(&installed.stdout).unwrap();
    assert!(installed["errors"].as_array().unwrap().is_empty());
    let actual: Value = serde_json::from_slice(&fs::read(settings).unwrap()).unwrap();
    assert_eq!(actual["permissions"], original["permissions"]);
    assert_eq!(actual["statusLine"], original["statusLine"]);
    assert!(actual["hooks"]["SessionStart"].is_array());
}

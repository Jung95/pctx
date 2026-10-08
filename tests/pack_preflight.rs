//! PCTX01 argument/input admission; existing pack features are not extended.
use pctx::{
    deadline::Deadline,
    pack::{self, PackCommand},
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
fn refusal(o: &Output, code: &str, exit: i32) -> Value {
    assert_eq!(o.status.code(), Some(exit), "{o:?}");
    assert!(o.stderr.is_empty());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["errors"][0]["code"], code);
    assert_eq!(v["status"], "error");
    v
}
#[test]
fn invalid_pack_plan_arguments_have_no_project_or_response_effects() {
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
            for (invalid, exit) in [
                (vec!["--scope", ""], 2),
                (vec!["--scope", "."], 2),
                (vec!["--scope", "../escape"], 2),
                (vec!["--scope", "/absolute"], 2),
                (vec!["--scope", "a\\b"], 2),
                (vec!["--scope", "code.py", "--budget-bytes", "1023"], 8),
                (vec!["--scope", "code.py", "--budget-bytes", "67108865"], 8),
                (vec!["--scope", "code.py", "--split-bytes", "511"], 8),
                (vec!["--scope", "code.py", "--split-bytes", "64001"], 8),
            ] {
                let out = Command::new(env!("CARGO_BIN_EXE_pctx"))
                    .current_dir(&f.base)
                    .args(["--format", format, "--root"])
                    .arg(&f.root)
                    .arg("--output")
                    .arg(&response)
                    .env("PCTX_DATA_DIR", &f.data)
                    .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                    .args(["pack", "plan", "--task-id", "missing"])
                    .args(&invalid)
                    .output()
                    .unwrap();
                let value = refusal(
                    &out,
                    if exit == 2 {
                        "INVALID_ARGUMENT"
                    } else {
                        "BUDGET_TOO_SMALL"
                    },
                    exit,
                );
                assert!(value["project_id"].is_null());
                assert_eq!(f.state(), before, "{format}: {invalid:?}");
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
fn pack_plan_producer_preserves_pure_errors_and_original_expiry() {
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
    for (scopes, content, budget_bytes, split_bytes, exit) in [
        (vec![], "metadata", 64000, None, 2),
        (vec!["../escape".into()], "metadata", 64000, None, 2),
        (vec!["code.py".into()], "unknown", 64000, None, 2),
        (vec!["code.py".into()], "metadata", 1023, None, 8),
        (vec!["code.py".into()], "metadata", 64000, Some(511), 8),
    ] {
        let command = PackCommand::Plan {
            task_id: "missing".into(),
            session: None,
            topic: None,
            scopes,
            content: content.into(),
            budget_bytes,
            split_bytes,
        };
        let error = pack::execute(&p, &command).unwrap_err();
        assert_eq!(
            (error.code.as_str(), error.exit),
            (
                if exit == 2 {
                    "INVALID_ARGUMENT"
                } else {
                    "BUDGET_TOO_SMALL"
                },
                exit
            )
        );
        assert_eq!(f.state(), before);
    }
    for plan in ["", "PACKPLAN-../escape"] {
        let command = PackCommand::Create {
            plan: plan.into(),
            expect_hash: "unknown".into(),
            output: f.base.join("artifact.json"),
        };
        let error = pack::execute(&p, &command).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
        assert!(!f.base.join("artifact.json").exists());
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    let command = PackCommand::Plan {
        task_id: "missing".into(),
        session: None,
        topic: None,
        scopes: vec![],
        content: "metadata".into(),
        budget_bytes: 64000,
        split_bytes: None,
    };
    assert_eq!(pack::execute(&p, &command).unwrap_err().code, "TIMEOUT");
    assert_eq!(p.deadline.unwrap().instant(), original);
    assert_eq!(f.state(), before);
}
#[test]
fn valid_pack_plan_preserves_metadata_and_duplicate_scopes() {
    let f = Fixture::new();
    f.init();
    assert!(f.run(&["index", "update"]).status.success());
    let definition = f.base.join("task.json");
    fs::write(&definition, r#"{"schema_version":1,"title":"Review code","scope":["code.py"],"acceptance":[{"id":"AC1","description":"Reviewed","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}"#).unwrap();
    let out = f
        .command()
        .args(["task", "create", "--from-file"])
        .arg(&definition)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let task: Value = serde_json::from_slice(&out.stdout).unwrap();
    let out = f.run(&[
        "pack",
        "plan",
        "--task-id",
        task["data"]["task_id"].as_str().unwrap(),
        "--scope",
        "code.py",
        "--scope",
        "code.py",
        "--content",
        "metadata",
        "--budget-bytes",
        "64000",
        "--split-bytes",
        "512",
    ]);
    assert_eq!(out.status.code(), Some(8), "{out:?}");
    let refused: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(refused["errors"][0]["code"], "BUDGET_TOO_SMALL");
    // A distinct feasible part bound; minimum syntax does not guarantee an item fits.
    let out = f.run(&[
        "pack",
        "plan",
        "--task-id",
        task["data"]["task_id"].as_str().unwrap(),
        "--scope",
        "code.py",
        "--scope",
        "code.py",
        "--content",
        "metadata",
        "--budget-bytes",
        "64000",
        "--split-bytes",
        "64000",
    ]);
    assert!(out.status.success(), "{out:?}");
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(value["data"]["plan_id"].is_string());
    assert!(!String::from_utf8(out.stdout).unwrap().contains("return 1"));
}

#[test]
fn invalid_pack_create_ids_have_no_project_or_artifact_effects() {
    let long = format!("PACKPLAN-{}", "n".repeat(120));
    for initialized in [false, true] {
        let f = Fixture::new();
        if initialized {
            f.init();
        }
        let artifact = f.base.join("artifact.json");
        if initialized {
            fs::write(&artifact, "existing artifact").unwrap();
        }
        let before = f.state();
        for format in ["json", "compact"] {
            for plan in [
                "",
                "other",
                "PACKPLAN-../escape",
                "PACKPLAN-a\\b",
                long.as_str(),
            ] {
                let out = Command::new(env!("CARGO_BIN_EXE_pctx"))
                    .current_dir(&f.base)
                    .args(["--format", format, "--root"])
                    .arg(&f.root)
                    .env("PCTX_DATA_DIR", &f.data)
                    .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                    .args([
                        "pack",
                        "create",
                        "--plan",
                        plan,
                        "--expect-hash",
                        "unknown",
                        "--output",
                    ])
                    .arg(&artifact)
                    .output()
                    .unwrap();
                refusal(&out, "INVALID_ARGUMENT", 2);
                assert_eq!(f.state(), before);
                if initialized {
                    assert_eq!(fs::read(&artifact).unwrap(), b"existing artifact");
                } else {
                    assert!(!artifact.exists());
                }
            }
        }
    }
}

//! PCTX01 argument/input admission; existing broker features are not extended.
use pctx::{
    broker::{self, RepoCommand},
    project::{Config, Project, ProjectConfig, RootAnchor},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
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
#[test]
fn invalid_repo_arguments_have_no_project_or_response_effects() {
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
            for invalid in [
                vec!["--workspace", "other"],
                vec!["--workspace", ""],
                vec!["--fields", ""],
                vec!["--fields", "unknown"],
                vec!["--fields", "branch,"],
                vec!["--fields", ",head"],
                vec!["--fields", "branch,,head"],
                vec!["--fields", "branch, head"],
            ] {
                let mut cmd = Command::new(env!("CARGO_BIN_EXE_pctx"));
                let out = cmd
                    .current_dir(&f.base)
                    .args(["--format", format, "--root"])
                    .arg(&f.root)
                    .arg("--output")
                    .arg(&response)
                    .env("PCTX_DATA_DIR", &f.data)
                    .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                    .args(["repo", "status"])
                    .args(&invalid)
                    .output()
                    .unwrap();
                let value = refusal(&out, "INVALID_ARGUMENT");
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
fn repo_producer_validation_precedes_database() {
    let f = Fixture::new();
    fs::create_dir(&f.root).unwrap();
    let p = Project {
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
    for (fields, workspace) in [
        ("branch", "other"),
        ("", "current"),
        ("branch,,head", "current"),
    ] {
        let command = RepoCommand::Status {
            fields: fields.into(),
            workspace: workspace.into(),
        };
        let error = broker::repo(&p, &command).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let command = RepoCommand::Status {
        fields: "branch".into(),
        workspace: "current".into(),
    };
    let error = broker::repo_with_timeout(&p, &command, Duration::ZERO).unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
    assert_eq!(f.state(), before);
}
#[test]
fn valid_repo_selection_preserves_non_git_unsupported_truth() {
    let f = Fixture::new();
    f.init();
    let out = f.run(&[
        "repo",
        "status",
        "--fields",
        "counts,branch,branch",
        "--workspace",
        "current",
    ]);
    assert_eq!(out.status.code(), Some(6), "{out:?}");
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["data"]["source_status"], "unsupported");
    assert_eq!(value["data"]["coverage"]["status"], "unsupported");
    assert_eq!(value["coverage"]["status"], "unsupported");
    assert_eq!(value["errors"][0]["code"], "UNSUPPORTED");
    assert_eq!(value["status"], "error");
}

#[test]
fn valid_repo_fields_preserve_supported_git_observation() {
    let f = Fixture::new();
    f.init();
    let out = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&f.root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let out = f.run(&[
        "repo",
        "status",
        "--fields",
        "counts,branch,branch",
        "--workspace",
        "current",
    ]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["coverage"]["status"], "complete");
    assert_eq!(value["data"]["coverage"]["status"], "complete");
    assert!(value["data"]["items"].get("counts").is_some());
    assert!(value["data"]["items"].get("branch").is_some());
    assert!(value["data"]["items"].get("head").is_none());
}

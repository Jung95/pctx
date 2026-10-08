//! PCTX01 argument/input admission; existing checkpoint features are not extended.
use pctx::{
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor},
    storage,
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
#[test]
fn invalid_checkpoint_arguments_have_no_project_or_response_effects() {
    let long = "n".repeat(257);
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
        for invalid in [
            vec!["--name", ""],
            vec!["--name", long.as_str()],
            vec!["--scope", "../escape"],
            vec!["--scope", "/absolute"],
            vec!["--scope", "a\\b"],
            vec!["--scope", "["],
        ] {
            let output = f
                .command()
                .arg("--output")
                .arg(&response)
                .args(["checkpoint", "create"])
                .args(&invalid)
                .output()
                .unwrap();
            let value = refusal(&output, "INVALID_ARGUMENT");
            assert!(value["project_id"].is_null());
            assert_eq!(f.state(), before, "{invalid:?}");
            if initialized {
                assert_eq!(fs::read(&response).unwrap(), b"existing response");
            } else {
                assert!(!response.exists());
            }
        }
    }
}

#[test]
fn checkpoint_producer_validation_precedes_writer_and_manifest() {
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
    fs::create_dir_all(&p.workspace_dir).unwrap();
    let before = f.state();
    for (name, scopes) in [
        (Some(""), vec![]),
        (None, vec!["../escape".into()]),
        (None, vec!["[".into()]),
    ] {
        let error = storage::checkpoint_scoped(&p, name, false, &scopes).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    let error = storage::checkpoint_scoped(&p, Some("valid"), false, &[]).unwrap_err();
    assert_eq!(error.code, "TIMEOUT");
    assert_eq!(p.deadline.unwrap().instant(), original);
    assert_eq!(f.state(), before);
}

#[test]
fn checkpoint_display_names_and_dot_glob_scopes_remain_supported() {
    let f = Fixture::new();
    f.init();
    assert!(f.run(&["index", "update"]).status.success());
    for (name, scopes) in [("release / notes", vec!["."]), ("python", vec!["**/*.py"])] {
        let mut command = f.command();
        command.args(["checkpoint", "create", "--name", name]);
        for scope in scopes {
            command.args(["--scope", scope]);
        }
        let out = command.output().unwrap();
        assert!(out.status.success(), "{out:?}");
        let value: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(value["data"]["name"], name);
        assert!(value["data"]["files"].get("code.py").is_some());
    }
}

//! PCTX01-G03-R09: existing filter grammar before effects.
use pctx::{
    deadline::Deadline,
    filters::{self, FilterCommand},
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
fn commands() -> Vec<(FilterCommand, &'static str)> {
    let mut commands = vec![];
    for id in [String::new(), "a".repeat(65), "../bad".into()] {
        commands.push((
            FilterCommand::Activate {
                id,
                expect_hash: "fixture".into(),
            },
            "FILTER_INVALID",
        ));
    }
    for argv in [vec![], vec!["missing".into(); 257]] {
        commands.push((FilterCommand::Explain { argv }, "INVALID_ARGUMENT"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_vec(b"opaque-\xff".to_vec()));
        commands.extend([
            (
                FilterCommand::Validate { path: path.clone() },
                "INVALID_ARGUMENT",
            ),
            (
                FilterCommand::Test {
                    path: path.clone(),
                    fixtures: PathBuf::from("fixtures"),
                },
                "INVALID_ARGUMENT",
            ),
            (
                FilterCommand::Test {
                    path: PathBuf::from("fixture"),
                    fixtures: path.clone(),
                },
                "INVALID_ARGUMENT",
            ),
            (
                FilterCommand::Apply {
                    filter: "fixture".into(),
                    input: path,
                    child_exit: 0,
                },
                "INVALID_ARGUMENT",
            ),
        ]);
    }
    commands
}
#[test]
fn filter_arguments_precede_project_and_response_effects() {
    use std::ffi::OsString;
    let mut cases: Vec<(Vec<OsString>, &str)> = vec![];
    for id in [String::new(), "a".repeat(65), "../bad".into()] {
        cases.push((
            vec![
                "filter".into(),
                "activate".into(),
                id.into(),
                "--expect-hash".into(),
                "fixture".into(),
            ],
            "FILTER_INVALID",
        ));
    }
    let mut argv: Vec<OsString> = vec!["filter".into(), "explain".into(), "--".into()];
    argv.extend(vec![OsString::from("missing"); 257]);
    cases.push((argv, "INVALID_ARGUMENT"));
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let path = OsString::from_vec(b"opaque-\xff".to_vec());
        cases.extend([
            (
                vec!["filter".into(), "validate".into(), path.clone()],
                "INVALID_ARGUMENT",
            ),
            (
                vec![
                    "filter".into(),
                    "test".into(),
                    path.clone(),
                    "--fixtures".into(),
                    "fixtures".into(),
                ],
                "INVALID_ARGUMENT",
            ),
            (
                vec![
                    "filter".into(),
                    "test".into(),
                    "fixture".into(),
                    "--fixtures".into(),
                    path.clone(),
                ],
                "INVALID_ARGUMENT",
            ),
            (
                vec![
                    "filter".into(),
                    "apply".into(),
                    "--filter".into(),
                    "fixture".into(),
                    "--input".into(),
                    path,
                    "--child-exit".into(),
                    "0".into(),
                ],
                "INVALID_ARGUMENT",
            ),
        ]);
    }
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
            for (args, code) in &cases {
                let output = Command::new(env!("CARGO_BIN_EXE_pctx"))
                    .current_dir(&f.base)
                    .args(["--format", format, "--root"])
                    .arg(&f.root)
                    .arg("--output")
                    .arg(&response)
                    .env("PCTX_DATA_DIR", &f.data)
                    .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                    .env("PCTX_ACTOR", "agent:fixture")
                    .args(args)
                    .output()
                    .unwrap();
                assert_eq!(output.status.code(), Some(2), "{output:?}");
                let value: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(value["errors"][0]["code"], *code);
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
fn direct_filter_refusals_preserve_storage_and_original_expiry() {
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    for (command, code) in commands() {
        let error = filters::execute(&p, &command).unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), (code, 2));
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for (command, _) in commands() {
        assert_eq!(filters::execute(&p, &command).unwrap_err().code, "TIMEOUT");
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}
#[test]
fn shared_filter_grammar_preserves_paths_modes_and_existing_bounds() {
    filters::validate_filter_request(&FilterCommand::Activate {
        id: "a".repeat(64),
        expect_hash: String::new(),
    })
    .unwrap();
    for argv in [vec![String::new(); 256], vec!["한".repeat(22000)]] {
        filters::validate_filter_request(&FilterCommand::Explain { argv }).unwrap();
    }
    for filter in [
        "fixture",
        ".pctx/filters/fixture.toml",
        "/absolute/root/.pctx/filters/fixture.toml",
    ] {
        filters::validate_filter_request(&FilterCommand::Apply {
            filter: filter.into(),
            input: "-".into(),
            child_exit: 17,
        })
        .unwrap();
        filters::validate_filter_request(&FilterCommand::Validate {
            path: filter.into(),
        })
        .unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_vec(
            b"/opaque-\xff/filter.toml".to_vec(),
        ));
        filters::validate_filter_request(&FilterCommand::Validate { path: path.clone() }).unwrap();
        filters::validate_filter_request(&FilterCommand::Test {
            path: path.clone(),
            fixtures: path.clone(),
        })
        .unwrap();
        filters::validate_filter_request(&FilterCommand::Apply {
            filter: "fixture".into(),
            input: path,
            child_exit: 0,
        })
        .unwrap();
    }
}
#[cfg(unix)]
#[test]
fn native_filter_paths_preview_and_state_guards_keep_original_contracts() {
    let f = Fixture::new();
    f.init();
    fs::create_dir_all(f.root.join(".pctx/filters")).unwrap();
    fs::write(
        f.root.join(".pctx/filters/fixture.toml"),
        r#"schema_version = 1
id = "fixture"
version = "1.0.0"
priority = 100
[match]
program = "/usr/bin/printf"
argv_prefix = []
stream = "both"
[parse]
kind = "lines"
[render]
max_bytes = 8192
keep_head_lines = 12
keep_tail_lines = 20
show_omission_counts = true
[[rules]]
op = "protect"
pattern = 'error|warning'
"#,
    )
    .unwrap();
    fs::write(f.root.join("preview.log"), "error: fixture\nordinary\n").unwrap();
    let before = f.state();
    let absolute = f.root.join(".pctx/filters/fixture.toml");
    let mut digest = String::new();
    for path in [
        PathBuf::from("fixture"),
        PathBuf::from(".pctx/filters/fixture.toml"),
        absolute.clone(),
    ] {
        let o = f
            .command()
            .args(["filter", "validate"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(o.status.success(), "{o:?}");
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["data"]["execution_started"], false);
        digest = v["data"]["filter_hash"].as_str().unwrap().into();
        assert_eq!(f.state(), before);
    }
    for filter in [
        "fixture".to_string(),
        ".pctx/filters/fixture.toml".into(),
        absolute.to_str().unwrap().into(),
    ] {
        let o = f.run(&[
            "filter",
            "apply",
            "--filter",
            &filter,
            "--input",
            "preview.log",
            "--child-exit",
            "17",
        ]);
        assert!(o.status.success(), "{o:?}");
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["data"]["child_exit_code"], 17);
        assert_eq!(f.state(), before);
    }
    let o = f.run(&["filter", "activate", "fixture", "--expect-hash", &digest]);
    assert_eq!(o.status.code(), Some(5), "{o:?}");
    assert_eq!(f.state(), before);
    let o = f
        .command()
        .env("PCTX_ACTOR", "agent:fixture")
        .args(["filter", "activate", "fixture", "--expect-hash", &digest])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(5), "{o:?}");
    assert_eq!(f.state(), before);
    let o = f
        .command()
        .args(["filter", "validate"])
        .arg(f.base.join("outside.toml"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(5), "{o:?}");
    assert_eq!(f.state(), before);
    let mut argv = vec!["fixture".to_string(); 256];
    argv[0] = "/usr/bin/printf".into();
    let o = f
        .command()
        .args(["filter", "explain", "--"])
        .args(argv)
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    assert_eq!(f.state(), before);
}

//! PCTX01-G03-C001: existing Activity cursor admission.
use pctx::{
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor},
    watch, work,
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

fn command(f: &Fixture, format: &str, cursor: i64, follow: bool) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
    c.current_dir(&f.base)
        .args(["--format", format, "--root"])
        .arg(&f.root)
        .env("PCTX_DATA_DIR", &f.data)
        .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
        .args(["activity", &format!("--since-seq={cursor}")]);
    if follow {
        c.arg("--follow");
    }
    c
}
#[test]
fn native_negative_activity_cursor_precedes_discovery_without_changing_transport() {
    for initialized in [false, true] {
        let f = Fixture::new();
        if initialized {
            f.init();
        }
        let response = f.base.join("response.json");
        fs::write(&response, "original response").unwrap();
        let before = f.state();
        for cursor in [-1, i64::MIN] {
            for format in ["json", "compact", "ndjson"] {
                for follow in [false, true] {
                    for output in [false, true] {
                        let mut c = command(&f, format, cursor, follow);
                        if output {
                            c.arg("--output").arg(&response);
                        }
                        let o = c.output().unwrap();
                        assert_eq!(o.status.code(), Some(2), "{o:?}");
                        if follow && format == "json" {
                            assert!(o.stderr.is_empty());
                            let v: Value = serde_json::from_slice(&o.stdout).unwrap();
                            assert_eq!(
                                v["errors"][0]["message"],
                                "JSON watch is unavailable; use compact or ndjson"
                            );
                        } else if follow || format == "ndjson" {
                            assert!(o.stdout.is_empty());
                            let v: Value = serde_json::from_slice(&o.stderr).unwrap();
                            assert_eq!(v["type"], "error");
                            assert_eq!(v["data"]["code"], "INVALID_ARGUMENT");
                            let message = if output {
                                "Streaming requires stdout; redirect explicitly"
                            } else {
                                "Sequence cannot be negative"
                            };
                            assert_eq!(v["data"]["message"], message);
                        } else {
                            assert!(o.stderr.is_empty());
                            let v: Value = serde_json::from_slice(&o.stdout).unwrap();
                            assert_eq!(v["status"], "error");
                            assert_eq!(
                                v["coverage"],
                                serde_json::json!({"status":"partial","reasons":["INVALID_ARGUMENT"]})
                            );
                            assert_eq!(v["errors"][0]["code"], "INVALID_ARGUMENT");
                            assert_eq!(v["errors"][0]["message"], "Sequence cannot be negative");
                            assert!(v["project_id"].is_null());
                        }
                        assert_eq!(f.state(), before);
                        assert_eq!(fs::read(&response).unwrap(), b"original response");
                    }
                }
            }
        }
    }
}
#[test]
fn direct_activity_and_watch_refusal_keep_original_expiry_and_state() {
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    for cursor in [-1, i64::MIN] {
        let e = work::activity(&p, cursor).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("INVALID_ARGUMENT", 2));
        for board in [false, true] {
            for follow in [false, true] {
                let e = watch::run(&p, board, cursor, follow, true).unwrap_err();
                assert_eq!((e.code.as_str(), e.exit), ("INVALID_ARGUMENT", 2));
            }
        }
        assert_eq!(f.state(), before);
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for cursor in [-1, i64::MIN, 0, i64::MAX] {
        let e = work::activity(&p, cursor).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("TIMEOUT", 7));
        for board in [false, true] {
            for follow in [false, true] {
                let e = watch::run(&p, board, cursor, follow, true).unwrap_err();
                assert_eq!((e.code.as_str(), e.exit), ("TIMEOUT", 7));
            }
        }
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}
#[test]
fn accepted_activity_boundaries_return_empty_without_cursor_narrowing_or_new_clock() {
    let f = Fixture::new();
    f.init();
    let o = f.run(&["activity"]);
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    let before = f.state();
    for cursor in [0, i64::MAX] {
        for format in ["json", "compact", "ndjson"] {
            let o = command(&f, format, cursor, false).output().unwrap();
            assert_eq!(o.status.code(), Some(0), "{o:?}");
            assert!(o.stderr.is_empty());
            if format == "ndjson" {
                assert!(o.stdout.is_empty());
            } else {
                let v: Value = serde_json::from_slice(&o.stdout).unwrap();
                assert_eq!(v["data"]["events"], serde_json::json!([]));
                assert_eq!(v["data"]["next_cursor"], cursor);
                assert_eq!(v["data"]["has_more"], false);
            }
            assert_eq!(f.state(), before);
        }
    }
    let before = f.state();
    for format in ["compact", "ndjson"] {
        let o = command(&f, format, 0, true)
            .args(["--timeout-ms", "100"])
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(2), "{o:?}");
        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(v["errors"][0]["code"], "INVALID_ARGUMENT");
        assert!(
            v["errors"][0]["message"]
                .as_str()
                .unwrap()
                .contains("timeout")
        );
        assert!(o.stderr.is_empty());
        assert_eq!(f.state(), before);
    }
}

#[test]
fn malformed_activity_cursor_types_refuse_before_discovery_and_output() {
    for initialized in [false, true] {
        let f = Fixture::new();
        if initialized {
            f.init();
        }
        let response = f.base.join("response.json");
        fs::write(&response, "original response").unwrap();
        let before = f.state();
        for cursor in ["9223372036854775808", "-9223372036854775809", "invalid", ""] {
            for format in ["json", "compact"] {
                for output in [false, true] {
                    let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
                    c.current_dir(&f.base)
                        .args(["--format", format, "--no-color", "--root"])
                        .arg(&f.root)
                        .env("PCTX_DATA_DIR", &f.data)
                        .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                        .args(["activity", &format!("--since-seq={cursor}")]);
                    if output {
                        c.arg("--output").arg(&response);
                    }
                    let o = c.output().unwrap();
                    assert_eq!(o.status.code(), Some(2), "{o:?}");
                    if format == "json" {
                        assert!(o.stderr.is_empty());
                        let v: Value = serde_json::from_slice(&o.stdout).unwrap();
                        assert_eq!(v["command"], "arguments");
                        assert_eq!(v["errors"][0]["code"], "INVALID_ARGUMENT");
                        assert_eq!(v["coverage"]["status"], "partial");
                        assert!(v["project_id"].is_null());
                    } else {
                        assert!(o.stdout.is_empty());
                        assert!(!o.stderr.is_empty());
                        assert!(!o.stderr.contains(&27));
                    }
                    assert_eq!(f.state(), before);
                    assert_eq!(fs::read(&response).unwrap(), b"original response");
                }
            }
        }
    }
}

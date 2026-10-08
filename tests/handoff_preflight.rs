//! PCTX01 argument/input admission; existing handoff features are not extended.
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
    process::{Command, Output, Stdio},
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
fn invalid_names_on_every_leaf_refuse_before_discovery_or_response_writes() {
    let f = Fixture::new();
    for name in ["", "../escape", "has space", "a.b", "a/b", "é"] {
        for leaf in ["create", "update", "show"] {
            let mut c = f.command();
            c.args(["--output", "refused-response.json", "handoff", leaf]);
            if leaf == "create" {
                c.args(["--name", name, "--from-file", "missing-input"]);
            } else {
                c.arg(name);
                if leaf == "update" {
                    c.args(["--from-file", "missing-input"]);
                }
            }
            let v = refusal(&c.output().unwrap(), "INVALID_ARGUMENT");
            assert!(v["project_id"].is_null());
            assert!(
                !f.root.exists()
                    && !f.data.exists()
                    && !f.base.join("refused-response.json").exists()
            );
        }
    }
    for name in ["A-1_z", "-", "_"] {
        storage::validate_handoff_name(name).unwrap();
    }
}
#[test]
fn oversize_and_invalid_utf8_inputs_preserve_every_index_checkpoint_and_file() {
    let f = Fixture::new();
    f.init();
    let before = f.state();
    let large = f.base.join("large.md");
    fs::File::create(&large)
        .unwrap()
        .set_len((pctx::input::MAX_TASK_BYTES + 1) as u64)
        .unwrap();
    fs::write(f.base.join("invalid.md"), [255, 254]).unwrap();
    for (path, code) in [
        ("large.md", "FILE_TOO_LARGE"),
        ("invalid.md", "UNSUPPORTED_ENCODING"),
    ] {
        for leaf in ["create", "update"] {
            let mut c = f.command();
            c.args(["handoff", leaf]);
            if leaf == "create" {
                c.args(["--name", "fixture"]);
            } else {
                c.arg("fixture");
            }
            c.args(["--from-file", path]);
            refusal(&c.output().unwrap(), code);
            assert_eq!(f.state(), before);
        }
    }
}
#[test]
fn explicit_regular_file_semantics_and_create_update_show_are_preserved() {
    let f = Fixture::new();
    f.init();
    fs::write(f.base.join("-"), "Goal: initial\nNext: inspect\n").unwrap();
    let created = f.run(&["handoff", "create", "--name", "A-1_z", "--from-file", "-"]);
    assert!(created.status.success(), "{created:?}");
    let saved = fs::read(f.root.join(".pctx/handoffs/A-1_z.md")).unwrap();
    let collision = f.run(&["handoff", "create", "--name", "A-1_z", "--from-file", "-"]);
    assert!(!collision.status.success());
    assert_eq!(
        fs::read(f.root.join(".pctx/handoffs/A-1_z.md")).unwrap(),
        saved
    );
    fs::write(
        f.base.join("replacement.md"),
        "Goal: replacement\nNext: review\n",
    )
    .unwrap();
    let updated = f.run(&[
        "handoff",
        "update",
        "A-1_z",
        "--from-file",
        "replacement.md",
    ]);
    assert!(updated.status.success(), "{updated:?}");
    let shown = f.run(&["handoff", "show", "A-1_z", "--validate"]);
    assert!(shown.status.success(), "{shown:?}");
    let v: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert!(
        v["data"]["content"]
            .as_str()
            .unwrap()
            .contains("replacement")
    );
    assert_eq!(v["data"]["evidence_origin"], "user_provided");
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        std::os::unix::fs::symlink(f.base.join("replacement.md"), f.base.join("linked.md"))
            .unwrap();
        assert!(
            f.run(&["handoff", "update", "A-1_z", "--from-file", "linked.md"])
                .status
                .success()
        );
        let path = f
            .base
            .join(std::ffi::OsString::from_vec(b"input-\xff.md".to_vec()));
        let written = fs::write(&path, "Goal: opaque filename\n");
        #[cfg(target_os = "macos")]
        let before = f.state();
        let o = f
            .command()
            .args(["handoff", "update", "A-1_z", "--from-file"])
            .arg(path)
            .output()
            .unwrap();
        match written {
            Ok(()) => assert!(o.status.success(), "{o:?}"),
            Err(error) => {
                // APFS rejects this filename before a file can exist; retain OS
                // classification through PathBuf rather than replacing bytes.
                #[cfg(not(target_os = "macos"))]
                panic!("Unexpected filename creation failure: {error}");
                #[cfg(target_os = "macos")]
                {
                    assert_eq!(error.raw_os_error(), Some(libc::EILSEQ));
                    assert_eq!(o.status.code(), Some(7), "{o:?}");
                    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
                    assert_eq!(v["errors"][0]["code"], "IO_ERROR");
                    assert_eq!(f.state(), before);
                }
            }
        }
    }
}
#[test]
fn library_invalid_name_and_original_expiry_refuse_before_input_or_artifacts() {
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
    for replace in [false, true] {
        let e =
            storage::handoff_create(&p, "../bad", &f.base.join("missing"), replace).unwrap_err();
        assert_eq!((e.code.as_str(), e.exit), ("INVALID_ARGUMENT", 2));
    }
    assert_eq!(
        storage::handoff_show(&p, "", false).unwrap_err().code,
        "INVALID_ARGUMENT"
    );
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    let e = storage::handoff_create(&p, "valid", &f.base.join("missing"), false).unwrap_err();
    assert_eq!((e.code.as_str(), e.exit), ("TIMEOUT", 7));
    assert_eq!(p.deadline.unwrap().instant(), original);
    assert_eq!(f.state(), before);
}
#[cfg(unix)]
#[test]
fn fifo_input_is_refused_without_blocking_or_creating_checkpoints() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let f = Fixture::new();
    f.init();
    let before = f.state();
    let path = f.base.join("input.fifo");
    let c = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    let mut child = f
        .command()
        .args(["handoff", "create", "--name", "fifo", "--from-file"])
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= end {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("FIFO reader blocked instead of refusing; parent cleaned up");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    refusal(&child.wait_with_output().unwrap(), "INVALID_ARGUMENT");
    assert_eq!(f.state(), before);
}

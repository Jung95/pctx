use fs2::FileExt;
use pctx::{
    adapter::{self, AdapterCommand, ClaudeCommand},
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor},
};
use serde_json::Value;
use std::{
    fs,
    time::{Duration, Instant},
};
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(data.join("control")).unwrap();
    fs::create_dir_all(data.join("workspace")).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data.clone(),
        control_dir: data.join("control"),
        workspace_dir: data.join("workspace"),
        project_id: "fixture".into(),
        workspace_id: "ws".into(),
        coordination_id: "coord".into(),
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
    (t, p)
}
fn call(p: &Project, c: ClaudeCommand) -> pctx::domain::Result<Value> {
    adapter::execute(p, &AdapterCommand::Claude { command: c })
}
#[test]
fn expired_read_leaves_have_no_lock_config_or_storage_effects() {
    let (_t, mut p) = fixture();
    p.deadline = Some(Deadline::from_instant(Instant::now()));
    for command in [
        ClaudeCommand::Doctor,
        ClaudeCommand::Verify,
        ClaudeCommand::ProtocolFixture {
            from_file: p.root.join("missing.json"),
        },
    ] {
        assert_eq!(call(&p, command).err().unwrap().code, "TIMEOUT");
    }
    assert_eq!(fs::read_dir(&p.control_dir).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&p.workspace_dir).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&p.root).unwrap().count(), 0);
}
#[test]
fn existing_install_lock_obeys_explicit_budget_without_installation_effects() {
    let (_t, mut p) = fixture();
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(p.control_dir.join("adapter.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    p.deadline = Some(Deadline::from_millis(1000).unwrap());
    assert_eq!(
        call(&p, ClaudeCommand::Verify).unwrap()["live_verified"],
        false
    );
    assert_eq!(fs::read_dir(&p.control_dir).unwrap().count(), 1);
    assert!(!p.root.join(".claude").exists());
    p.deadline = Some(Deadline::from_millis(50).unwrap());
    let start = Instant::now();
    assert_eq!(
        call(
            &p,
            ClaudeCommand::Install {
                plan: "a".repeat(64),
                expect_hash: "a".repeat(64)
            }
        )
        .err()
        .unwrap()
        .code,
        "TIMEOUT"
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(!p.root.join(".claude").exists());
    assert!(!p.control_db().exists());
    FileExt::unlock(&lock).unwrap();
    p.deadline = Some(Deadline::from_millis(1000).unwrap());
    assert_eq!(
        call(&p, ClaudeCommand::Verify).unwrap()["live_verified"],
        false
    );
}
#[cfg(unix)]
struct Cli {
    t: tempfile::TempDir,
    root: std::path::PathBuf,
    bin: std::path::PathBuf,
}
#[cfg(unix)]
impl Cli {
    fn new() -> Self {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        let root = base.join("project");
        let bin = base.join("bin");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(base.join("home")).unwrap();
        let f = Self { t, root, bin };
        let o = f.run(&["init"]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stdout));
        f
    }
    fn run(&self, args: &[&str]) -> std::process::Output {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_pctx"));
        if args.first() == Some(&"adapter") {
            command.args(["--timeout-ms", "1500"]);
        }
        command
            .args(["--format", "json", "--root"])
            .arg(&self.root)
            .args(args)
            .env("PCTX_DATA_DIR", self.t.path().join("data"))
            .env("PCTX_ACTOR", "owner")
            .env_remove("PCTX_RUN_ID")
            .env_remove("PCTX_RUN_CAPABILITY")
            .env("HOME", self.t.path().join("home"))
            .env("USERPROFILE", self.t.path().join("home"))
            .env("GIT_CONFIG_GLOBAL", self.t.path().join("empty.gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("PATH", &self.bin)
            .env("PROBE_MARKER", self.t.path().join("probe.started"))
            .output()
            .unwrap()
    }
    fn script(&self, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        let path = self.bin.join("claude");
        fs::write(&path, body).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}
#[cfg(unix)]
#[test]
fn delayed_version_probe_times_out_after_actual_start_not_installed_false() {
    let f = Cli::new();
    // Record actual entry through private HOME rather than depending on custom
    // environment forwarded by the version-only query supervisor.
    f.script("#!/bin/sh\nprintf started > \"$HOME/probe.started\"\n/bin/sleep 30\nprintf fixture-version\n");
    let start = Instant::now();
    let output = f.run(&["adapter", "claude", "doctor"]);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(7), "{value}");
    assert_eq!(value["errors"][0]["code"], "TIMEOUT", "{value}");
    assert!(
        f.t.path().join("home/probe.started").exists(),
        "Version fixture never reached its entry marker: {value}; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(start.elapsed() < Duration::from_secs(5));
    assert!(!value.to_string().contains("installed"));
}
#[cfg(unix)]
#[test]
fn version_absence_and_native_failure_are_distinct() {
    let f = Cli::new();
    let absent = f.run(&["adapter", "claude", "doctor"]);
    let value: Value = serde_json::from_slice(&absent.stdout).unwrap();
    assert!(absent.status.success(), "{value}");
    assert_eq!(value["data"]["installed"], false);
    f.script("#!/bin/sh\nprintf failed-secret-marker >&2\nexit 3\n");
    let failed = f.run(&["adapter", "claude", "doctor"]);
    let value: Value = serde_json::from_slice(&failed.stdout).unwrap();
    assert_eq!(failed.status.code(), Some(6), "{value}");
    assert_eq!(value["errors"][0]["code"], "SOURCE_UNAVAILABLE");
    assert!(!value.to_string().contains("failed-secret-marker"));
}

#[cfg(unix)]
#[test]
fn version_success_empty_and_invalid_encoding_are_distinct() {
    let f = Cli::new();
    f.script("#!/bin/sh\nprintf 'Claude fixture-version\\n'\n");
    let output = f.run(&["adapter", "claude", "doctor"]);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{value}");
    assert_eq!(value["data"]["installed"], true);
    assert_eq!(value["data"]["version"], "Claude fixture-version");
    for script in ["#!/bin/sh\nexit 0\n", "#!/bin/sh\nprintf '\\377'\n"] {
        f.script(script);
        let output = f.run(&["adapter", "claude", "doctor"]);
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(6), "{value}");
        assert_eq!(value["errors"][0]["code"], "SOURCE_UNAVAILABLE");
    }
}

#[cfg(unix)]
#[test]
fn version_capture_overflow_is_reported_without_installation_inference() {
    let f = Cli::new();
    f.script("#!/bin/sh\nexec /usr/bin/head -c 200000 /dev/zero\n");
    let output = f.run(&["adapter", "claude", "doctor"]);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(3), "{value}");
    assert_eq!(value["errors"][0]["code"], "PARTIAL_RESULT");
    assert!(value["data"].is_null());
}

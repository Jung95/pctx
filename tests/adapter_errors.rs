//! PCTX01-G05: adapter source error classification.
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
fn refusal(output: &Output, code: &str, exit: i32) {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], code);
    assert_eq!(value["schema_version"], "1.0");
    assert_eq!(value["status"], "error");
    assert!(value["data"].is_null());
    assert!(output.stderr.is_empty());
}

#[test]
fn native_adapter_hook_error_and_io_keep_envelope_and_state() {
    use std::io::Write;
    use std::process::Stdio;
    let f = Fixture::new();
    f.init();
    let before = f.state();
    refusal(
        &f.run(&[
            "adapter",
            "claude",
            "protocol-fixture",
            "--from-file",
            f.base.join("absent.json").to_str().unwrap(),
        ]),
        "IO_ERROR",
        7,
    );
    assert_eq!(f.state(), before);
    let mut child = f
        .command()
        .args(["adapter", "claude", "event", "--agent", "fixture", "--hook"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    input
        .write_all(br#"{"hook_event_name":"Unknown","session_id":"native"}"#)
        .unwrap();
    drop(input);
    let output = child.wait_with_output().unwrap();
    refusal(&output, "CAPABILITY_UNAVAILABLE", 6);
    assert_eq!(f.state(), before);
}
#[test]
fn native_adapter_owner_refusal_is_policy_five_in_both_formats() {
    let f = Fixture::new();
    f.init();
    let before = f.state();
    for format in ["json", "compact"] {
        let output = Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(&f.base)
            .args(["--root"])
            .arg(&f.root)
            .args(["--format", format])
            .env("PCTX_DATA_DIR", &f.data)
            .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
            .env("PCTX_ACTOR", "agent:fixture")
            .args(["adapter", "claude", "plan", "--agent", "fixture"])
            .output()
            .unwrap();
        refusal(&output, "POLICY_DENIED", 5);
        assert_eq!(f.state(), before);
    }
}
#[test]
fn native_adapter_input_cap_and_configuration_shape_are_input_two() {
    let f = Fixture::new();
    f.init();
    let path = f.base.join("event.json");
    fs::write(&path, vec![b' '; 128 * 1024 + 1]).unwrap();
    let before = f.state();
    refusal(
        &f.run(&[
            "adapter",
            "claude",
            "protocol-fixture",
            "--from-file",
            path.to_str().unwrap(),
        ]),
        "BUDGET_EXCEEDED",
        2,
    );
    assert_eq!(f.state(), before);
    fs::create_dir(f.root.join(".claude")).unwrap();
    for contents in [r#"{"hooks":[]}"#, r#"{"hooks":{"SessionStart":{}}}"#] {
        fs::write(f.root.join(".claude/settings.local.json"), contents).unwrap();
        let before = f.state();
        refusal(
            &f.run(&["adapter", "claude", "plan", "--agent", "fixture"]),
            "CONFIG_CONFLICT",
            2,
        );
        assert_eq!(f.state(), before);
    }
}
#[test]
fn native_adapter_unsupported_protocol_is_capability_six() {
    let f = Fixture::new();
    f.init();
    let path = f.base.join("event.json");
    for raw in [
        serde_json::json!({"hook_event_name":"Unknown","session_id":"native"}),
        serde_json::json!({"hook_event_name":"SessionStart","session_id":"native","source":"unsupported"}),
        serde_json::json!({"hook_event_name":"PreCompact","session_id":"native","trigger":"unsupported"}),
    ] {
        fs::write(&path, raw.to_string()).unwrap();
        let before = f.state();
        refusal(
            &f.run(&[
                "adapter",
                "claude",
                "protocol-fixture",
                "--from-file",
                path.to_str().unwrap(),
            ]),
            "CAPABILITY_UNAVAILABLE",
            6,
        );
        assert_eq!(f.state(), before);
    }
}
#[cfg(unix)]
#[test]
fn native_adapter_symlink_refusal_is_access_five() {
    let f = Fixture::new();
    f.init();
    let target = f.base.join("event.json");
    let link = f.base.join("link.json");
    fs::write(&target, b"fixture").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let before = f.state();
    refusal(
        &f.run(&[
            "adapter",
            "claude",
            "protocol-fixture",
            "--from-file",
            link.to_str().unwrap(),
        ]),
        "PATH_DENIED",
        5,
    );
    assert_eq!(f.state(), before);
    assert_eq!(fs::read(&target).unwrap(), b"fixture");
    assert_eq!(fs::read_link(&link).unwrap(), target);
}

fn call(p: &Project, command: ClaudeCommand) -> pctx::domain::Result<Value> {
    adapter::execute(p, &AdapterCommand::Claude { command })
}
fn classified(p: &Project, command: ClaudeCommand, code: &str, exit: i32) {
    let error = call(p, command).unwrap_err();
    assert_eq!((error.code.as_str(), error.exit), (code, exit));
}
#[test]
fn direct_adapter_plan_input_source_and_binding_are_distinct() {
    let f = Fixture::new();
    let p = f.project();
    let planned = call(
        &p,
        ClaudeCommand::Plan {
            agent: "fixture".into(),
        },
    )
    .unwrap();
    let id = planned["plan_id"].as_str().unwrap().to_string();
    let file = p
        .control_dir
        .join("adapter-plans")
        .join(format!("{id}.json"));
    let bytes = fs::read(&file).unwrap();
    let before = f.state();
    classified(
        &p,
        ClaudeCommand::Install {
            plan: id.clone(),
            expect_hash: "wrong".into(),
        },
        "PLAN_MISMATCH",
        2,
    );
    // Pure hash refusal precedes lock/storage/config effects.
    assert_eq!(f.state(), before);
    assert!(!p.root.join(".claude").exists());
    fs::write(&file, b"changed bytes").unwrap();
    classified(
        &p,
        ClaudeCommand::Install {
            plan: id.clone(),
            expect_hash: id.clone(),
        },
        "PLAN_MISMATCH",
        4,
    );
    assert_eq!(fs::read(&file).unwrap(), b"changed bytes");
    assert!(!p.root.join(".claude").exists());
    fs::write(&file, &bytes).unwrap();
    for (field, value, exit) in [
        ("schema", serde_json::json!(99), 2),
        ("workspace", serde_json::json!("other"), 9),
    ] {
        let mut modified: Value = serde_json::from_slice(&bytes).unwrap();
        modified[field] = value;
        let modified = serde_json::to_vec(&modified).unwrap();
        let digest = pctx::domain::hash(&modified);
        let changed = p
            .control_dir
            .join("adapter-plans")
            .join(format!("{digest}.json"));
        fs::write(&changed, &modified).unwrap();
        classified(
            &p,
            ClaudeCommand::Install {
                plan: digest.clone(),
                expect_hash: digest,
            },
            "PLAN_MISMATCH",
            exit,
        );
        assert_eq!(fs::read(&changed).unwrap(), modified);
        assert!(!p.root.join(".claude").exists());
    }
    fs::create_dir(p.root.join(".claude")).unwrap();
    fs::write(p.root.join(".claude/settings.local.json"), b"{}").unwrap();
    let before_stale = f.state();
    classified(
        &p,
        ClaudeCommand::Install {
            plan: id.clone(),
            expect_hash: id,
        },
        "PLAN_STALE",
        4,
    );
    assert_eq!(f.state(), before_stale);
    assert_ne!(f.state(), before);
    let mut expired = p.clone();
    let original = Instant::now() - Duration::from_secs(1);
    expired.deadline = Some(Deadline::from_instant(original));
    classified(
        &expired,
        ClaudeCommand::Install {
            plan: "invalid".into(),
            expect_hash: "invalid".into(),
        },
        "TIMEOUT",
        7,
    );
    assert_eq!(expired.deadline.unwrap().instant(), original);
    assert_eq!(f.state(), before_stale);
}
#[test]
fn direct_adapter_schema_and_ownership_integrity_are_storage_seven() {
    let f = Fixture::new();
    let p = f.project();
    let path = f.base.join("event.json");
    fs::write(
        &path,
        serde_json::json!({"hook_event_name":"PermissionDenied","session_id":"native"}).to_string(),
    )
    .unwrap();
    let event = || ClaudeCommand::Event {
        agent: "fixture".into(),
        from_file: Some(path.clone()),
        idempotency_key: Some("fixture-key".into()),
        hook: false,
    };
    call(&p, event()).unwrap();
    let db = pctx::work::connect(&p).unwrap();
    db.execute_batch("ALTER TABLE adapter_schema RENAME TO saved_adapter_schema; CREATE TABLE adapter_schema(version INTEGER); INSERT INTO adapter_schema VALUES(99);").unwrap();
    drop(db);
    let before = f.state();
    classified(&p, event(), "DB_SCHEMA_TOO_NEW", 7);
    assert_eq!(f.state(), before);
    let db = pctx::work::connect(&p).unwrap();
    db.execute_batch("DELETE FROM adapter_schema;").unwrap();
    drop(db);
    let before = f.state();
    classified(&p, event(), "DB_SCHEMA_TOO_NEW", 7);
    assert_eq!(f.state(), before);
    let db = pctx::work::connect(&p).unwrap();
    db.execute_batch(
        "DROP TABLE adapter_schema; ALTER TABLE saved_adapter_schema RENAME TO adapter_schema;",
    )
    .unwrap();
    db.execute("INSERT INTO adapter_installs(workspace,plan,owned,config_hash,status) VALUES(?1,'fixture','[]',?2,'installed')",rusqlite::params![p.workspace_id,pctx::domain::hash([])]).unwrap();
    drop(db);
    let before = f.state();
    classified(
        &p,
        ClaudeCommand::Uninstall {
            plan: "fixture".into(),
            expect_config_hash: pctx::domain::hash([]),
        },
        "CONFIG_CONFLICT",
        7,
    );
    assert_eq!(f.state(), before);
}
#[test]
fn direct_adapter_receipt_conflicts_and_reconciliation_retain_nine() {
    let f = Fixture::new();
    let p = f.project();
    let path = f.base.join("event.json");
    fs::write(
        &path,
        serde_json::json!({"hook_event_name":"PermissionDenied","session_id":"native"}).to_string(),
    )
    .unwrap();
    let event = |key: &str| ClaudeCommand::Event {
        agent: "fixture".into(),
        from_file: Some(path.clone()),
        idempotency_key: Some(key.into()),
        hook: false,
    };
    let imported = call(&p, event("fixture-key")).unwrap();
    let before = f.state();
    assert_eq!(call(&p, event("fixture-key")).unwrap(), imported);
    assert_eq!(f.state(), before);
    fs::write(
        &path,
        serde_json::json!({"hook_event_name":"PermissionRequest","session_id":"native"})
            .to_string(),
    )
    .unwrap();
    let before = f.state();
    classified(&p, event("fixture-key"), "IDEMPOTENCY_CONFLICT", 9);
    assert_eq!(f.state(), before);
    fs::write(
        &path,
        serde_json::json!({"hook_event_name":"PreCompact","session_id":"native","trigger":"auto"})
            .to_string(),
    )
    .unwrap();
    classified(&p, event("unbound"), "BASELINE_MISMATCH", 9);
    // Original producer records an incomplete intent before binding failure. This
    // evidence does not imply rollback; replay must require reconciliation.
    let before = f.state();
    classified(&p, event("unbound"), "RECONCILIATION_REQUIRED", 9);
    assert_eq!(f.state(), before);
    classified(
        &p,
        ClaudeCommand::Uninstall {
            plan: "absent".into(),
            expect_config_hash: "fixture".into(),
        },
        "CONFIG_CONFLICT",
        9,
    );
}

#[test]
fn direct_adapter_corrupt_persisted_json_and_shapes_are_storage_errors() {
    let f = Fixture::new();
    let p = f.project();
    let path = f.base.join("event.json");
    fs::write(
        &path,
        serde_json::json!({"hook_event_name":"PermissionDenied","session_id":"native"}).to_string(),
    )
    .unwrap();
    let event = || ClaudeCommand::Event {
        agent: "fixture".into(),
        from_file: Some(path.clone()),
        idempotency_key: Some("fixture-key".into()),
        hook: false,
    };
    call(&p, event()).unwrap();
    let db = pctx::work::connect(&p).unwrap();
    let original: String = db
        .query_row(
            "SELECT result FROM adapter_receipts WHERE key='fixture-key'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(db);
    for corrupt in [
        "{",
        "[]",
        "{}",
        r#"{"event":"Other","hook_output":{}}"#,
        r#"{"event":"PermissionDenied","hook_output":[]}"#,
        r#"{"event":"PermissionDenied","hook_output":{},"incomplete":"false"}"#,
    ] {
        let db = pctx::work::connect(&p).unwrap();
        db.execute(
            "UPDATE adapter_receipts SET result=?1 WHERE key='fixture-key'",
            [corrupt],
        )
        .unwrap();
        drop(db);
        let before = f.state();
        classified(&p, event(), "DB_CORRUPT", 7);
        assert_eq!(f.state(), before);
    }
    let db = pctx::work::connect(&p).unwrap();
    db.execute(
        "UPDATE adapter_receipts SET result=?1 WHERE key='fixture-key'",
        [original],
    )
    .unwrap();
    drop(db);
    fs::create_dir(p.root.join(".claude")).unwrap();
    let settings = br#"{"hooks":{"SessionStart":[]}}"#;
    fs::write(p.root.join(".claude/settings.local.json"), settings).unwrap();
    let expected = pctx::domain::hash(settings);
    for corrupt in ["{", r#"{"SessionStart":{}}"#] {
        let db = pctx::work::connect(&p).unwrap();
        db.execute("INSERT OR REPLACE INTO adapter_installs(workspace,plan,owned,config_hash,status) VALUES(?1,'fixture',?2,?3,'installed')",rusqlite::params![p.workspace_id,corrupt,expected]).unwrap();
        drop(db);
        let before = f.state();
        classified(
            &p,
            ClaudeCommand::Uninstall {
                plan: "fixture".into(),
                expect_config_hash: expected.clone(),
            },
            "DB_CORRUPT",
            7,
        );
        assert_eq!(f.state(), before);
    }
}

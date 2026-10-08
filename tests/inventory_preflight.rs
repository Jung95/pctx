//! PCTX01-G03-R07: existing inventory grammar and original request priority.
use pctx::{
    deadline::Deadline,
    inventory::{self, InventoryCommand},
    project::{Config, Project, ProjectConfig, RootAnchor},
};
use serde_json::{Value, json};
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
#[test]
fn lexical_inventory_arguments_precede_project_and_response_effects() {
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
            for path in [
                "",
                "../profile.json",
                "/profile.json",
                "dir\\profile.json",
                "./profile.json",
            ] {
                for args in [
                    vec!["inventory", "profile", "--path", path],
                    vec!["inventory", "audit", "--registry", path],
                    vec!["inventory", "scan", "--profile", path],
                ] {
                    let mut command = Command::new(env!("CARGO_BIN_EXE_pctx"));
                    let output = command
                        .current_dir(&f.base)
                        .args(["--format", format, "--root"])
                        .arg(&f.root)
                        .env("PCTX_DATA_DIR", &f.data)
                        .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                        .arg("--output")
                        .arg(&response)
                        .args(&args)
                        .output()
                        .unwrap();
                    assert_eq!(output.status.code(), Some(5), "{args:?}: {output:?}");
                    assert!(output.stderr.is_empty());
                    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
                    assert_eq!(value["errors"][0]["code"], "PATH_OUTSIDE_ROOT");
                    assert_eq!(value["status"], "error");
                    assert!(value["project_id"].is_null());
                    assert_eq!(f.state(), before, "{args:?}");
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
fn direct_inventory_paths_precede_root_checks_and_preserve_original_expiry() {
    let f = Fixture::new();
    let mut p = f.project();
    // A mismatched captured root must be reached only for lexically valid requests.
    p.root_anchor = RootAnchor::capture(&f.base).unwrap();
    let before = f.state();
    for path in [
        "",
        "../profile.json",
        "/profile.json",
        "dir\\profile.json",
        "./profile.json",
    ] {
        for result in [
            inventory::profile(&p, path),
            inventory::audit(&p, path, None),
            inventory::scan(&p, Some(path), 1, 1),
            inventory::execute(&p, &InventoryCommand::Profile { path: path.into() }),
            inventory::execute(
                &p,
                &InventoryCommand::Audit {
                    registry: path.into(),
                    since: None,
                },
            ),
            inventory::execute(
                &p,
                &InventoryCommand::Scan {
                    profile: Some(path.into()),
                    max_files: 1,
                    max_bytes: 1,
                },
            ),
        ] {
            let error = result.unwrap_err();
            assert_eq!((error.code.as_str(), error.exit), ("PATH_OUTSIDE_ROOT", 5));
            assert_eq!(f.state(), before);
        }
    }
    assert_eq!(
        inventory::profile(&p, "valid.json").unwrap_err().code,
        "POLICY_DENIED"
    );
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for result in [
        inventory::profile(&p, "../profile.json"),
        inventory::audit(&p, "../profile.json", None),
        inventory::scan(&p, Some("../profile.json"), 1, 1),
        inventory::execute(
            &p,
            &InventoryCommand::Profile {
                path: "../profile.json".into(),
            },
        ),
        inventory::execute(
            &p,
            &InventoryCommand::Audit {
                registry: "../profile.json".into(),
                since: None,
            },
        ),
        inventory::execute(
            &p,
            &InventoryCommand::Scan {
                profile: Some("../profile.json".into()),
                max_files: 1,
                max_bytes: 1,
            },
        ),
    ] {
        assert_eq!(result.unwrap_err().code, "TIMEOUT");
        assert_eq!(p.deadline.unwrap().instant(), original);
        assert_eq!(f.state(), before);
    }
}
#[test]
fn expired_scan_limits_keep_original_timeout_priority() {
    let f = Fixture::new();
    let mut p = f.project();
    let before = f.state();
    for (files, bytes) in [(0, 1), (4097, 1), (1, 0), (1, 16777217)] {
        assert_eq!(
            inventory::scan(&p, Some("../profile.json"), files, bytes)
                .unwrap_err()
                .code,
            "INVALID_ARGUMENT"
        );
    }
    let original = Instant::now() - Duration::from_secs(1);
    p.deadline = Some(Deadline::from_instant(original));
    for (files, bytes) in [(0, 1), (4097, 1), (1, 0), (1, 16777217)] {
        for result in [
            inventory::scan(&p, Some("../profile.json"), files, bytes),
            inventory::execute(
                &p,
                &InventoryCommand::Scan {
                    profile: Some("../profile.json".into()),
                    max_files: files,
                    max_bytes: bytes,
                },
            ),
        ] {
            assert_eq!(result.unwrap_err().code, "TIMEOUT");
            assert_eq!(p.deadline.unwrap().instant(), original);
            assert_eq!(f.state(), before);
        }
    }
}
#[test]
fn shared_inventory_grammar_preserves_reader_normalization_and_scan_bounds() {
    for path in [
        "fixtures/profile.json",
        "fixtures/./profile.json",
        "fixtures//profile.json",
        "fixtures/profile.json/",
        "   ",
        ".git/profile.json",
    ] {
        for command in [
            InventoryCommand::Profile { path: path.into() },
            InventoryCommand::Audit {
                registry: path.into(),
                since: Some("unrestricted-baseline".into()),
            },
            InventoryCommand::Scan {
                profile: Some(path.into()),
                max_files: 4096,
                max_bytes: 16777216,
            },
        ] {
            inventory::validate_inventory_request(&command).unwrap();
        }
    }
    inventory::validate_inventory_request(&InventoryCommand::Scan {
        profile: None,
        max_files: 1,
        max_bytes: 1,
    })
    .unwrap();
    for (files, bytes) in [(0, 1), (4097, 1), (1, 0), (1, 16777217)] {
        let error = inventory::validate_inventory_request(&InventoryCommand::Scan {
            profile: Some("../profile.json".into()),
            max_files: files,
            max_bytes: bytes,
        })
        .unwrap_err();
        assert_eq!((error.code.as_str(), error.exit), ("INVALID_ARGUMENT", 2));
    }
}
#[test]
fn accepted_inventory_paths_preserve_profile_audit_scan_and_policy_contracts() {
    let f = Fixture::new();
    f.init();
    fs::create_dir(f.root.join("fixtures")).unwrap();
    fs::write(
        f.root.join("fixtures/profile.json"),
        json!({"schema_version":1,"id":"fixture","expectations":[],"roles":[]}).to_string(),
    )
    .unwrap();
    fs::write(
        f.root.join("fixtures/registry.json"),
        json!({"schema_version":1,"source_refs":[]}).to_string(),
    )
    .unwrap();
    let before = f.state();
    // Internal dot components and repeated separators are accepted by the existing reader.
    for path in [
        "fixtures/profile.json",
        "fixtures/./profile.json",
        "fixtures//profile.json",
    ] {
        let output = f.run(&["inventory", "profile", "--path", path]);
        assert!(output.status.success(), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["data"]["profile_id"], "fixture");
        assert_eq!(value["data"]["operation_authorized"], false);
        assert_eq!(value["data"]["profile_applied"], false);
        assert_eq!(f.state(), before);
    }
    let output = f.run(&[
        "inventory",
        "audit",
        "--registry",
        "fixtures/./registry.json",
    ]);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["documents_modified"], false);
    assert_eq!(value["data"]["tasks_created"], false);
    assert_eq!(f.state(), before);
    let output = f.run(&["inventory", "scan", "--profile", "fixtures//profile.json"]);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["profile"]["profile_id"], "fixture");
    assert_eq!(value["data"]["scripts_executed"], false);
    assert_eq!(f.state(), before);
    let output = f.run(&["inventory", "scan"]);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value["data"]["profile"].is_null());
    assert_eq!(value["data"]["scripts_executed"], false);
    assert_eq!(f.state(), before);
    let output = f.run(&[
        "inventory",
        "audit",
        "--registry",
        "fixtures/registry.json",
        "--since",
        "different-event",
    ]);
    assert_eq!(output.status.code(), Some(9), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], "BASELINE_MISMATCH");
    assert_eq!(f.state(), before);
    for args in [
        vec!["inventory", "profile", "--path", ".git/profile.json"],
        vec!["inventory", "audit", "--registry", ".git/registry.json"],
        vec!["inventory", "scan", "--profile", ".git/profile.json"],
    ] {
        let output = f.run(&args);
        assert_eq!(output.status.code(), Some(5), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["errors"][0]["code"], "POLICY_DENIED");
        assert_eq!(f.state(), before);
    }
}

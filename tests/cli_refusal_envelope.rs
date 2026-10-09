//! PCTX01-G02/G04/G05: native refusal envelope and parser matrix.
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
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
            .env("PCTX_USER_CONFIG", self.base.join("absent-config"))
            .env("PCTX_ACTOR", "owner");
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
}

fn error_document(o: &Output, code: &str, exit: i32, command: &str, established: Option<&Value>) {
    assert_eq!(o.status.code(), Some(exit), "{o:?}");
    assert!(o.stderr.is_empty(), "{o:?}");
    assert!(o.stdout.ends_with(b"\n"));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v.as_object().unwrap().len(), 12);
    assert_eq!(v["schema_version"], "1.0");
    assert_eq!(v["command"], command);
    assert_eq!(v["status"], "error");
    for key in ["project_id", "workspace_id"] {
        assert_eq!(
            v[key],
            established
                .map(|response| response[key].clone())
                .unwrap_or(Value::Null)
        );
    }
    assert!(v["generation_id"].is_null());
    assert_eq!(v["validation"]["mode"], "matched");
    assert_eq!(v["validation"]["scope"], serde_json::json!([]));
    chrono::DateTime::parse_from_rfc3339(v["validation"]["checked_at"].as_str().unwrap()).unwrap();
    assert_eq!(v["validation"]["workspace_atomic"], false);
    assert_eq!(
        v["coverage"],
        serde_json::json!({"status":"partial","reasons":[code]})
    );
    assert!(v["data"].is_null());
    assert_eq!(
        v["truncation"],
        serde_json::json!({"truncated":false,"reasons":[]})
    );
    assert_eq!(v["warnings"], serde_json::json!([]));
    assert_eq!(v["errors"].as_array().unwrap().len(), 1);
    assert_eq!(v["errors"][0].as_object().unwrap().len(), 3);
    assert_eq!(v["errors"][0]["code"], code);
    assert!(
        v["errors"][0]["message"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert_eq!(v["errors"][0]["retryable"], false);
}
#[test]
fn all_visible_leaf_unknown_options_refuse_before_project_and_output() {
    let registry: Value = serde_json::from_str(include_str!(
        "../docs/implementation/evidence/pctx01-admission-registry.json"
    ))
    .unwrap();
    let leaves: Vec<_> = registry["paths"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["leaf"] == true)
        .collect();
    assert_eq!(leaves.len(), 140);
    let f = Fixture::new();
    let response = f.base.join("response.json");
    for existing in [false, true] {
        if existing {
            fs::write(&response, "original response").unwrap();
        }
        for format in ["json", "compact"] {
            for leaf in &leaves {
                let mut c = f.command();
                c.args(["--no-color", "--output"]).arg(&response);
                // Replace the default JSON option rather than passing a duplicate global.
                if format == "compact" {
                    c = Command::new(env!("CARGO_BIN_EXE_pctx"));
                    c.current_dir(&f.base)
                        .args(["--format", format, "--no-color", "--root"])
                        .arg(&f.root)
                        .arg("--output")
                        .arg(&response)
                        .env("PCTX_DATA_DIR", &f.data)
                        .env("PCTX_USER_CONFIG", f.base.join("absent-config"));
                }
                c.args(leaf["path"].as_str().unwrap().split_whitespace());
                let o = c.arg("--pctx-invalid-fixture-option").output().unwrap();
                if format == "json" {
                    error_document(&o, "INVALID_ARGUMENT", 2, "arguments", None);
                } else {
                    assert_eq!(o.status.code(), Some(2), "{o:?}");
                    assert!(o.stdout.is_empty());
                    let text = std::str::from_utf8(&o.stderr).unwrap();
                    assert!(text.contains("unexpected argument"), "{text}");
                    assert!(!text.contains('\u{1b}'));
                }
                assert!(!f.root.exists() && !f.data.exists());
                if existing {
                    assert_eq!(fs::read(&response).unwrap(), b"original response");
                } else {
                    assert!(!response.exists());
                }
            }
        }
    }
}
#[test]
fn install_required_and_duplicate_options_keep_parser_envelope_and_no_effects() {
    let hash = "a".repeat(64);
    let forms = [
        vec![],
        vec!["--plan", &hash],
        vec!["--expect-hash", &hash],
        vec!["--plan"],
        vec!["--plan", &hash, "--expect-hash"],
        vec!["--plan", &hash, "--expect-hash", &hash, "--plan", &hash],
        vec![
            "--plan",
            &hash,
            "--expect-hash",
            &hash,
            "--expect-hash",
            &hash,
        ],
        vec!["--plan", &hash, "--expect-hash", &hash, "--epoch", "1"],
    ];
    for initialized in [false, true] {
        let f = Fixture::new();
        if initialized {
            f.init();
        }
        let response = f.base.join("response.json");
        fs::write(&response, "original response").unwrap();
        let before = f.state();
        for form in &forms {
            let o = f
                .command()
                .arg("--output")
                .arg(&response)
                .args(["adapter", "claude", "install"])
                .args(form)
                .output()
                .unwrap();
            error_document(&o, "INVALID_ARGUMENT", 2, "arguments", None);
            assert_eq!(f.state(), before);
            assert_eq!(fs::read(&response).unwrap(), b"original response");
        }
    }
}
#[test]
fn semantic_and_representation_refusals_do_not_claim_completed_execution() {
    for initialized in [false, true] {
        let f = Fixture::new();
        if initialized {
            f.init();
        }
        let before = f.state();
        for format in ["json", "compact", "markdown"] {
            let o = Command::new(env!("CARGO_BIN_EXE_pctx"))
                .current_dir(&f.base)
                .args(["--format", format, "--root"])
                .arg(&f.root)
                .env("PCTX_DATA_DIR", &f.data)
                .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                .args([
                    "adapter",
                    "claude",
                    "install",
                    "--plan",
                    "invalid",
                    "--expect-hash",
                    "invalid",
                ])
                .output()
                .unwrap();
            let code = if format == "markdown" {
                "INVALID_ARGUMENT"
            } else {
                "PLAN_MISMATCH"
            };
            error_document(&o, code, 2, "adapter", None);
            assert_eq!(f.state(), before);
        }
    }
}
fn find(root: &Path, name: &str) -> Option<PathBuf> {
    for e in fs::read_dir(root).unwrap() {
        let path = e.unwrap().path();
        if path.file_name().unwrap() == name {
            return Some(path);
        }
        if path.is_dir()
            && let Some(path) = find(&path, name)
        {
            return Some(path);
        }
    }
    None
}
#[test]
fn admitted_install_failures_keep_state_classification_and_response_destination() {
    let f = Fixture::new();
    f.init();
    let planned = f.run(&["adapter", "claude", "plan", "--agent", "fixture"]);
    assert_eq!(planned.status.code(), Some(0));
    let planned: Value = serde_json::from_slice(&planned.stdout).unwrap();
    let id = planned["data"]["plan_id"].as_str().unwrap().to_string();
    let file = find(&f.data, &format!("{id}.json")).unwrap();
    let original = fs::read(&file).unwrap();
    // Existing lock permits producer validation; this is not first-use rollback.
    fs::write(
        file.parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("adapter.lock"),
        "",
    )
    .unwrap();
    for (kind, code, exit) in [
        ("policy", "POLICY_DENIED", 5),
        ("bytes", "PLAN_MISMATCH", 4),
        ("schema", "PLAN_MISMATCH", 2),
        ("binding", "PLAN_MISMATCH", 9),
        ("config", "PLAN_STALE", 4),
        ("missing", "IO_ERROR", 7),
    ] {
        let mut hash = id.clone();
        let mut actor = "owner";
        match kind {
            "policy" => {
                actor = "agent:fixture";
            }
            "bytes" => {
                fs::write(&file, "changed bytes").unwrap();
            }
            "schema" | "binding" => {
                let mut v: Value = serde_json::from_slice(&original).unwrap();
                if kind == "schema" {
                    v["schema"] = serde_json::json!(99);
                } else {
                    v["workspace"] = serde_json::json!("other-workspace");
                }
                let bytes = serde_json::to_vec(&v).unwrap();
                hash = pctx::domain::hash(&bytes);
                fs::write(file.parent().unwrap().join(format!("{hash}.json")), bytes).unwrap();
            }
            "config" => {
                fs::create_dir(f.root.join(".claude")).unwrap();
                fs::write(f.root.join(".claude/settings.local.json"), "{}").unwrap();
            }
            "missing" => {
                hash = "b".repeat(64);
            }
            _ => unreachable!(),
        }
        let before = f.state();
        for format in ["json", "compact"] {
            for destination in ["stdout", "absent", "existing"] {
                let response = f
                    .base
                    .join(format!("response-{kind}-{format}-{destination}.json"));
                let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
                c.current_dir(&f.base)
                    .args(["--format", format, "--root"])
                    .arg(&f.root)
                    .env("PCTX_DATA_DIR", &f.data)
                    .env("PCTX_USER_CONFIG", f.base.join("absent-config"))
                    .env("PCTX_ACTOR", actor)
                    .args([
                        "adapter",
                        "claude",
                        "install",
                        "--plan",
                        &hash,
                        "--expect-hash",
                        &hash,
                    ]);
                if destination != "stdout" {
                    c.arg("--output").arg(&response);
                }
                if destination == "existing" {
                    fs::write(&response, "original response").unwrap();
                }
                let mut o = c.output().unwrap();
                if destination == "existing" {
                    assert_eq!(o.status.code(), Some(9), "{o:?}");
                    assert!(o.stdout.is_empty());
                    assert!(
                        std::str::from_utf8(&o.stderr)
                            .unwrap()
                            .contains("REVISION_CONFLICT")
                    );
                    assert_eq!(fs::read(&response).unwrap(), b"original response");
                } else {
                    if destination == "absent" {
                        assert!(o.stdout.is_empty());
                        o.stdout = fs::read(&response).unwrap();
                    }
                    error_document(&o, code, exit, "adapter", Some(&planned));
                }
                assert_eq!(f.state(), before);
            }
        }
        fs::write(&file, &original).unwrap();
    }
}

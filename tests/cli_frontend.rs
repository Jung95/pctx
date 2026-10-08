//! PCTX01 frontend contracts before project discovery or effects.
use serde_json::Value;
use std::{
    ffi::OsString,
    fs,
    process::{Command, Output},
};
struct Fixture {
    temp: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        Self {
            temp: tempfile::tempdir().unwrap(),
        }
    }
    fn run(&self, args: &[OsString]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(self.temp.path())
            .args(args)
            .env("PCTX_DATA_DIR", self.temp.path().join("data"))
            .env("PCTX_USER_CONFIG", self.temp.path().join("absent-config"))
            .output()
            .unwrap()
    }
    fn unchanged(&self) {
        assert_eq!(fs::read_dir(self.temp.path()).unwrap().count(), 0);
    }
}
fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
fn argument_error(output: &Output) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["schema_version"], "1.0");
    assert_eq!(v["status"], "error");
    assert_eq!(v["errors"][0]["code"], "INVALID_ARGUMENT");
    assert!(v["project_id"].is_null() && v["workspace_id"].is_null());
}
#[test]
fn parser_errors_respect_actual_json_option_and_do_not_load_project() {
    let f = Fixture::new();
    for values in [
        vec!["--format=json", "--unknown"],
        vec!["--format", "json", "--unknown"],
    ] {
        argument_error(&f.run(&args(&values)));
    }
    let output = f.run(&args(&[
        "--root",
        "json",
        "--format",
        "compact",
        "--unknown",
    ]));
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    let output = f.run(&args(&["run", "--unknown", "--", "--format=json"]));
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    f.unchanged();
}
#[cfg(unix)]
#[test]
fn non_utf8_argument_returns_structured_error_without_panic_or_effect() {
    use std::os::unix::ffi::OsStringExt;
    let f = Fixture::new();
    let mut values = args(&["--format", "json", "find"]);
    values.push(OsString::from_vec(vec![255, 254]));
    let output = f.run(&values);
    argument_error(&output);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("panicked"));
    f.unchanged();
}
#[test]
fn help_and_version_are_successful_without_project_access() {
    let f = Fixture::new();
    for values in [
        vec!["--help"],
        vec!["--version"],
        vec!["--format=json", "find", "--help"],
        vec!["session", "attach", "--help"],
    ] {
        let output = f.run(&args(&values));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert!(!output.stdout.is_empty());
    }
    f.unchanged();
}

#[test]
fn unsupported_ndjson_is_rejected_before_project_discovery_or_init() {
    let f = Fixture::new();
    for command in [vec!["init"], vec!["find", "needle"], vec!["status"]] {
        let mut values = args(&["--root", "absent-project", "--format", "ndjson"]);
        values.extend(args(&command));
        let output = f.run(&values);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["type"], "error");
        assert_eq!(error["data"]["code"], "INVALID_ARGUMENT");
    }
    f.unchanged();
}

#[test]
fn invalid_strict_queries_preserve_generation_database_sources_and_output_path() {
    use std::{collections::BTreeMap, path::Path};
    fn snapshot(path: &Path, base: &Path, result: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                snapshot(&path, base, result);
            } else {
                result.insert(
                    path.strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    pctx::domain::hash(fs::read(&path).unwrap()),
                );
            }
        }
    }
    let f = Fixture::new();
    fs::create_dir(f.temp.path().join("project")).unwrap();
    fs::write(
        f.temp.path().join("project/source.py"),
        "def before():\n    return 1\n",
    )
    .unwrap();
    for command in [vec!["init"], vec!["index", "update"]] {
        let mut values = args(&["--root", "project", "--format", "json"]);
        values.extend(args(&command));
        let output = f.run(&values);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    // A refresh would observe this edit and replace the active generation.
    fs::write(
        f.temp.path().join("project/source.py"),
        "def changed():\n    return 2\n",
    )
    .unwrap();
    let mut before = BTreeMap::new();
    snapshot(f.temp.path(), f.temp.path(), &mut before);
    for command in [
        vec![
            "find",
            "changed",
            "--freshness",
            "strict",
            "--limit",
            "1001",
        ],
        vec!["find", "[", "--regex", "--freshness", "strict"],
        vec!["find", "--query", "AND missing", "--freshness", "strict"],
        vec![
            "find",
            "--query",
            "changed",
            "--regex",
            "--freshness",
            "strict",
        ],
        vec!["find", "changed", "--scope", "../", "--freshness", "strict"],
        vec![
            "find",
            "changed",
            "--scope",
            "/outside",
            "--freshness",
            "strict",
        ],
        vec![
            "query",
            "--language",
            "python",
            "--kind",
            "function",
            "--freshness",
            "strict",
            "--limit",
            "1001",
        ],
    ] {
        let mut values = args(&[
            "--root",
            "project",
            "--format",
            "json",
            "--output",
            "rejected.json",
        ]);
        values.extend(args(&command));
        let output = f.run(&values);
        argument_error(&output);
        let mut after = BTreeMap::new();
        snapshot(f.temp.path(), f.temp.path(), &mut after);
        assert_eq!(
            before, after,
            "Rejected request changed durable state: {command:?}"
        );
        assert!(!f.temp.path().join("rejected.json").exists());
    }
}

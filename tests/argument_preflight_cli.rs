//! Pure argument rejection must precede project discovery and derived writes.
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn command(root: &Path, data: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
    c.args(["--format", "json", "--root"])
        .arg(root)
        .env("PCTX_DATA_DIR", data)
        .env("PCTX_USER_CONFIG", root.join("absent-config"));
    c
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, current: &Path, map: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        if !current.exists() {
            return;
        }
        map.insert(current.strip_prefix(root).unwrap().into(), None);
        for e in fs::read_dir(current).unwrap() {
            let e = e.unwrap();
            assert!(!e.file_type().unwrap().is_symlink());
            let path = e.path();
            if path.is_dir() {
                walk(root, &path, map);
            } else {
                map.insert(
                    path.strip_prefix(root).unwrap().into(),
                    Some(fs::read(path).unwrap()),
                );
            }
        }
    }
    let mut map = BTreeMap::new();
    walk(root, root, &mut map);
    map
}
fn invalid_cases() -> Vec<(Vec<&'static str>, i32, &'static str)> {
    vec![
        (vec!["extract"], 2, "INVALID_ARGUMENT"),
        (
            vec!["extract", "--location", "code.py"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["extract", "--location", "code.py:no"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["extract", "--location", "code.py:0"],
            2,
            "INVALID_ARGUMENT",
        ),
        (vec!["extract", "--path", "code.py"], 2, "INVALID_ARGUMENT"),
        (vec!["extract", "--line", "1"], 2, "INVALID_ARGUMENT"),
        (
            vec!["extract", "--location", "../outside:1"],
            5,
            "PATH_OUTSIDE_ROOT",
        ),
        (
            vec!["build", "--dependency-depth", "3", "--task", "fixture"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["build", "--detail", "unknown", "--task", "fixture"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["build", "--budget-tokens", "1", "--task", "fixture"],
            6,
            "CAPABILITY_UNAVAILABLE",
        ),
        (
            vec!["trace", "code.py", "--depth", "33"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["impact", "code.py", "--depth", "33"],
            2,
            "INVALID_ARGUMENT",
        ),
        (vec!["read"], 2, "INVALID_ARGUMENT"),
        (
            vec!["read", "code.py", "--lines", "0:2"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["read", "code.py", "--lines", "3:2"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["read", "code.py", "--lines", "1"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["read", "code.py", "--lines", "1:2:3"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["read", "code.py", "--symbol", "fake", "--lines", "1:2"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["read", "--symbol-name", "first"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["read", "--symbol", "fake", "--symbol-name", "first"],
            2,
            "INVALID_ARGUMENT",
        ),
        (
            vec!["read", "code.py", "--path", "other.py"],
            2,
            "INVALID_ARGUMENT",
        ),
        (vec!["read", "../outside"], 5, "PATH_OUTSIDE_ROOT"),
        (
            vec!["outline", "../outside", "--freshness", "strict"],
            2,
            "INVALID_ARGUMENT",
        ),
    ]
}
fn refusal(c: &mut Command, expected_exit: i32, expected_code: &str) {
    let out = c.output().unwrap();
    assert_eq!(out.status.code(), Some(expected_exit), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["errors"][0]["code"], expected_code, "{out:?}");
    assert_eq!(v["schema_version"], "1.0");
    assert!(v["data"].is_null());
    assert!(v["project_id"].is_null());
    assert!(v["workspace_id"].is_null());
    assert!(out.stderr.is_empty());
}
#[test]
fn malformed_read_build_extract_and_graph_inputs_refuse_before_discovery() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("missing");
    let data = t.path().join("data");
    let output = t.path().join("response.json");
    for (args, exit, code) in invalid_cases() {
        refusal(
            command(&root, &data)
                .args(["--output", output.to_str().unwrap()])
                .args(&args),
            exit,
            code,
        );
        assert!(!root.exists());
        assert!(!data.exists());
        assert!(!output.exists());
    }
}
#[test]
fn rejected_requests_do_not_refresh_dirty_index_or_write_response() {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir(&root).unwrap();
    assert!(
        command(&root, &data)
            .arg("init")
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::write(root.join("code.py"), "def first():\n    return 1\n").unwrap();
    assert!(
        command(&root, &data)
            .args(["index", "update"])
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::write(root.join("code.py"), "def changed():\n    return 2\n").unwrap();
    let before = (snapshot(&root), snapshot(&data));
    let output = root.join("response.json");
    for (args, exit, code) in invalid_cases() {
        refusal(
            command(&root, &data)
                .args(["--output", output.to_str().unwrap()])
                .args(&args),
            exit,
            code,
        );
        assert_eq!((snapshot(&root), snapshot(&data)), before, "{args:?}");
        assert!(!output.exists());
    }
}
#[test]
fn valid_read_aliases_line_clamping_and_outline_remain_available() {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir(&root).unwrap();
    assert!(
        command(&root, &data)
            .arg("init")
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::write(root.join("code.py"), "def first():\n    return 1\n").unwrap();
    assert!(
        command(&root, &data)
            .args(["index", "update"])
            .output()
            .unwrap()
            .status
            .success()
    );
    for args in [
        vec!["read", "code.py", "--lines", "1:999"],
        vec!["read", "--path", "code.py", "--lines", "1:2"],
        vec!["read", "--symbol-name", "first", "--path", "code.py"],
        vec!["outline", ".", "--freshness", "matched"],
    ] {
        let out = command(&root, &data).args(&args).output().unwrap();
        assert!(out.status.success(), "{args:?} {out:?}");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["status"], "ok");
        if args[0] == "read" {
            assert_eq!(v["data"]["path"], "code.py");
            assert!(v["data"]["text"].as_str().unwrap().contains("def first"));
        }
    }
}

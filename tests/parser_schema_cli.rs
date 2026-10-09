//! Execute refused cases derived from the frozen real parser, without allowing
//! an unrelated missing required argument to serve as the refusal oracle.
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        if path.is_dir() {
            out.insert(path.strip_prefix(base).unwrap().into(), None);
            for entry in fs::read_dir(path).unwrap() {
                walk(base, &entry.unwrap().path(), out);
            }
        } else {
            out.insert(
                path.strip_prefix(base).unwrap().into(),
                Some(fs::read(path).unwrap()),
            );
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn generated_parser_refusals_preserve_project_output_and_open_stdin() {
    let matrix: Value = serde_json::from_str(include_str!(
        "../docs/implementation/evidence/pctx01-parser-cases.json"
    ))
    .unwrap();
    assert_eq!(matrix["leaves"].as_array().unwrap().len(), 140);
    let refused: Vec<_> = matrix["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["accepted"] == false)
        .collect();
    assert!(!refused.is_empty());
    let mut combinations = 0;
    for initialized in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("project");
        let data = base.join("data");
        let response = base.join("response");
        let configured = || {
            let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
            c.current_dir(&base)
                .arg("--root")
                .arg(&root)
                .env("PCTX_DATA_DIR", &data)
                .env("PCTX_USER_CONFIG", base.join("absent-config"))
                .env("PCTX_ACTOR", "owner");
            c
        };
        if initialized {
            fs::create_dir(&root).unwrap();
            assert!(
                configured()
                    .args(["--format", "json", "init"])
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        for existing in [false, true] {
            if existing {
                fs::write(&response, b"unchanged response sentinel").unwrap();
            }
            for format in ["json", "compact"] {
                let before = snapshot(&base);
                for case in &refused {
                    let mut args: Vec<String> = case["argv"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap().into())
                        .collect();
                    let pack = case["route"] == "pack create";
                    if pack {
                        // This is the artifact path, never a second global response option.
                        if let Some(index) = args.iter().position(|a| a == "--output")
                            && index + 1 < args.len()
                            && args[index + 1] == "parser-fixture"
                        {
                            args[index + 1] = response.to_str().unwrap().into();
                        }
                    }
                    let mut command = configured();
                    command.args(["--format", format, "--no-color"]);
                    if !pack {
                        command.arg("--output").arg(&response);
                    }
                    let mut child = command
                        .args(&args)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .unwrap();
                    // Keep stdin open: any accidental producer read would block.
                    // This watchdog bounds the test, not the product query budget.
                    let end = Instant::now() + Duration::from_secs(5);
                    loop {
                        if child.try_wait().unwrap().is_some() {
                            break;
                        }
                        if Instant::now() >= end {
                            child.kill().unwrap();
                            let _ = child.wait();
                            panic!("Parser entered a blocking phase: {case}");
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    let output = child.wait_with_output().unwrap();
                    assert_eq!(output.status.code(), Some(2), "{case}: {output:?}");
                    if format == "json" {
                        assert!(output.stderr.is_empty(), "{case}: {output:?}");
                        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
                        assert_eq!(value["command"], "arguments", "{case}: {value}");
                        assert_eq!(value["status"], "error");
                        assert!(value["data"].is_null());
                        assert!(value["project_id"].is_null());
                        assert!(value["workspace_id"].is_null());
                        assert_eq!(
                            value["coverage"],
                            serde_json::json!({"status":"partial","reasons":["INVALID_ARGUMENT"]})
                        );
                        assert_eq!(value["errors"][0]["code"], "INVALID_ARGUMENT");
                        assert_eq!(value["errors"][0]["retryable"], false);
                        assert!(output.stdout.ends_with(b"\n"));
                    } else {
                        assert!(output.stdout.is_empty(), "{case}: {output:?}");
                        let diagnostic = std::str::from_utf8(&output.stderr).unwrap();
                        assert!(
                            !diagnostic.is_empty() && !diagnostic.contains('\u{1b}'),
                            "{case}: {diagnostic}"
                        );
                        let needles: &[&str] = match case["native_error_kind"].as_str().unwrap() {
                            "MissingRequiredArgument" | "MissingSubcommand" => {
                                &["required", "requires a subcommand"]
                            }
                            "InvalidValue" | "ValueValidation" => {
                                &["invalid value", "a value is required"]
                            }
                            "ArgumentConflict" => &["cannot be used", "may be supplied only once"],
                            "UnknownArgument" => &["unexpected argument"],
                            "TooManyValues" => &["unexpected value"],
                            "DisplayHelpOnMissingArgumentOrSubcommand" => &["Usage:"],
                            other => panic!("Unclassified refusal kind {other}: {case}"),
                        };
                        assert!(
                            needles.iter().any(|s| diagnostic.contains(s)),
                            "Unexpected refusal category: {case}: {diagnostic}"
                        );
                    }
                    assert_eq!(snapshot(&base), before, "Parser effects: {case}");
                    combinations += 1;
                }
            }
        }
    }
    assert_eq!(combinations, refused.len() * 8);
    eprintln!(
        "Qualified {} refused cases in {combinations} native CLI combinations; accepted controls remain parser-only",
        refused.len()
    );
}

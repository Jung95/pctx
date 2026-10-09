//! PCTX01-G06-D03: native producer families retain empty/absent/incomplete truth.
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture {
    temp: tempfile::TempDir,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        let f = Self { temp, root };
        f.check(&["init"], 0);
        f
    }
    fn run(&self, args: &[&str], format: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(&self.root)
            .args(["--root", self.root.to_str().unwrap(), "--format", format])
            .args(args)
            .env("PCTX_DATA_DIR", self.temp.path().join("data"))
            .env("PCTX_USER_CONFIG", self.temp.path().join("absent-config"))
            .env("PCTX_HOST_RESOURCE_DIR", self.temp.path().join("host"))
            .env("PCTX_ACTOR", "owner")
            .output()
            .unwrap()
    }
    fn check(&self, args: &[&str], code: i32) -> Value {
        self.check_format(args, code, "json")
    }
    fn check_format(&self, args: &[&str], code: i32, format: &str) -> Value {
        let out = self.run(args, format);
        assert_eq!(out.status.code(), Some(code), "{args:?}/{format}: {out:?}");
        assert!(out.stderr.is_empty(), "{args:?}: {out:?}");
        assert!(out.stdout.ends_with(b"\n"));
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            v["status"],
            if code == 0 {
                "ok"
            } else if code == 3 {
                "partial"
            } else {
                "error"
            }
        );
        assert!(v["project_id"].is_string() && v["workspace_id"].is_string());
        if code == 0 {
            assert!(v["errors"].as_array().unwrap().is_empty());
        }
        if code != 0 {
            assert_ne!(v["coverage"]["status"], "complete");
        }
        v
    }
    #[cfg(unix)]
    fn observed_output(&self, text: &str, code: i32) -> Value {
        fs::write(self.root.join("input.txt"), text).unwrap();
        let plan = self.check(&["trust", "plan", "--", "/bin/cat", "input.txt"], 0);
        self.check(
            &[
                "trust",
                "add",
                "--expect-hash",
                plan["data"]["fingerprint"].as_str().unwrap(),
                "--",
                "/bin/cat",
                "input.txt",
            ],
            0,
        );
        self.check(
            &[
                "run",
                "--exit-policy",
                "pctx",
                "--execution-timeout-ms",
                "15000",
                "--",
                "/bin/cat",
                "input.txt",
            ],
            code,
        )
    }
}

#[test]
fn frozen_families_empty_and_missing_are_distinct() {
    for format in ["json", "compact"] {
        let f = Fixture::new();
        assert_eq!(
            f.check_format(&["cache", "stats"], 0, format)["data"]["snapshots"],
            0
        );
        assert_eq!(
            f.check_format(&["status"], 0, format)["data"]["indexed_files"],
            Value::Null
        );
        let indexed = f.check(&["index", "update"], 0);
        assert_eq!(
            f.check_format(&["status"], 0, format)["data"]["indexed_files"],
            indexed["data"]["files"]
        );
        assert_eq!(
            f.check_format(&["board"], 0, format)["data"]["tasks"],
            json!([])
        );
        f.check_format(&["task", "show", "TASK-absent"], 6, format);
        assert_eq!(
            f.check_format(&["schedule", "list"], 0, format)["data"]["schedules"],
            json!([])
        );
        f.check_format(
            &["schedule", "inspect", "--namespace", "fixture", "absent"],
            6,
            format,
        );
        let inventory = f.check_format(&["inventory", "scan"], 0, format);
        assert_eq!(inventory["data"]["packages"], json!([]));
        assert_eq!(
            inventory["data"]["coverage"]["status"],
            "complete_for_static_subset"
        );
        f.check_format(
            &["inventory", "profile", "--path", "absent.json"],
            6,
            format,
        );
        f.check_format(
            &["output", "find", "OUT-absent", "--literal", "absent"],
            6,
            format,
        );
        f.check_format(
            &[
                "filter",
                "apply",
                "--filter",
                "absent",
                "--input",
                "absent.txt",
                "--child-exit",
                "0",
            ],
            6,
            format,
        );
        f.check_format(&["repo", "status"], 6, format);
        // Do not reuse the preceding non-Git TTL snapshot for the Git control.
        let git = Fixture::new();
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&git.root)
                .status()
                .unwrap()
                .success()
        );
        fs::write(git.root.join(".git/info/exclude"), ".pctx/\n").unwrap();
        let repo = git.check_format(&["repo", "status", "--fields", "dirty,counts"], 0, format);
        assert_eq!(repo["data"]["coverage"]["status"], "complete");
        assert_eq!(repo["data"]["items"]["dirty"], false);
        assert_eq!(repo["data"]["items"]["counts"]["untracked"], 0);
    }
}

#[test]
fn known_incomplete_inventory_and_corrupt_status_are_not_successful_empty() {
    for format in ["json", "compact"] {
        let f = Fixture::new();
        fs::write(f.root.join("package.json"), "{not json}").unwrap();
        let v = f.check_format(&["inventory", "scan"], 3, format);
        assert_eq!(v["data"]["coverage"]["status"], "partial");
        assert!(
            v["data"]["coverage"]["reasons"]
                .to_string()
                .contains("UNSUPPORTED_MANIFEST")
        );
        fs::remove_file(f.root.join("package.json")).unwrap();
        f.check(&["index", "update"], 0);
        let status = f.check(&["status"], 0);
        let path = status["data"]["local_storage"]["index"].as_str().unwrap();
        fs::write(path, b"not sqlite").unwrap();
        let v = f.check_format(&["status"], 7, format);
        assert!(!v["errors"].as_array().unwrap().is_empty());
        assert!(v["data"].get("indexed_files").is_none());
    }
}

#[test]
fn requested_json_filter_parse_failure_is_incomplete() {
    for format in ["json", "compact"] {
        let f = Fixture::new();
        fs::create_dir_all(f.root.join(".pctx/filters")).unwrap();
        fs::write(f.root.join(".pctx/filters/fixture.toml"), "schema_version=1\nid='fixture'\nversion='1.0.0'\npriority=1\nrules=[]\n[match]\nprogram='/bin/cat'\nargv_prefix=[]\nstream='both'\n[parse]\nkind='json'\n[render]\nmax_bytes=8192\nkeep_head_lines=12\nkeep_tail_lines=20\nshow_omission_counts=true\n").unwrap();
        fs::write(f.root.join("input.txt"), "").unwrap();
        let empty = f.check_format(
            &[
                "filter",
                "apply",
                "--filter",
                "fixture",
                "--input",
                "input.txt",
                "--child-exit",
                "0",
            ],
            0,
            format,
        );
        assert_eq!(empty["data"]["records"], json!([]));
        fs::write(f.root.join("input.txt"), "{not json}\n").unwrap();
        let partial = f.check_format(
            &[
                "filter",
                "apply",
                "--filter",
                "fixture",
                "--input",
                "input.txt",
                "--child-exit",
                "0",
            ],
            3,
            format,
        );
        assert_eq!(partial["data"]["parse_status"], "partial");
        assert_eq!(partial["data"]["execution_started"], false);
    }
}

#[cfg(unix)]
#[test]
fn saved_output_zero_matches_and_incomplete_capture_are_distinct_without_rerun() {
    let f = Fixture::new();
    let empty = f.observed_output("", 0);
    let id = empty["data"]["output_id"].as_str().unwrap();
    for format in ["json", "compact"] {
        let v = f.check_format(&["output", "find", id, "--literal", "absent"], 0, format);
        assert_eq!(v["data"]["records"], json!([]));
        assert_eq!(v["data"]["capture_complete"], true);
        assert_eq!(v["data"]["command_rerun"], false);
    }
    let partial = f.observed_output(&"p\n".repeat(140_000), 3);
    assert_eq!(partial["data"]["child_exit_code"], 0);
    assert_eq!(partial["data"]["capture_complete"], false);
    let id = partial["data"]["output_id"].as_str().unwrap();
    for format in ["json", "compact"] {
        for route in [
            vec!["output", "find", id, "--literal", "absent"],
            vec!["output", "show", id, "--view", "full", "--lines", "1:3"],
            vec!["output", "render", id, "--filter", "builtin"],
        ] {
            let v = f.check_format(&route, 3, format);
            assert_eq!(v["data"]["capture_complete"], false);
            assert_eq!(v["data"]["command_rerun"], false);
            if route[1] != "find" {
                assert_eq!(v["data"]["child_exit_code"], 0);
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn pack_optional_validation_differs_from_requested_but_unavailable_sources() {
    for format in ["json", "compact"] {
        let f = Fixture::new();
        fs::write(f.root.join("code.ts"), "export const value = 1;\n").unwrap();
        fs::write(f.root.join("task.json"), json!({"schema_version":1,"title":"Review","scope":["code.ts"],"acceptance":[],"checks":[]}).to_string()).unwrap();
        let task = f.check(&["task", "create", "--from-file", "task.json"], 0);
        let id = task["data"]["task_id"].as_str().unwrap();
        f.check_format(
            &["pack", "plan", "--task-id", id, "--scope", "absent/**"],
            2,
            format,
        );
        let plan = f.check(&["pack", "plan", "--task-id", id, "--scope", "code.ts"], 0);
        f.check(
            &[
                "pack",
                "create",
                "--plan",
                plan["data"]["plan_id"].as_str().unwrap(),
                "--expect-hash",
                plan["data"]["plan_hash"].as_str().unwrap(),
                "--output",
                "artifact.json",
            ],
            0,
        );
        let current = f.check_format(
            &["pack", "verify", "artifact.json", "--against", "current"],
            0,
            format,
        );
        assert_eq!(current["data"]["freshness"], "current");
        fs::remove_file(f.root.join("code.ts")).unwrap();
        let inspection = f.check_format(&["pack", "inspect", "artifact.json"], 0, format);
        assert_eq!(inspection["data"]["freshness"], "unknown");
        assert_eq!(inspection["data"]["source_validation"], json!([]));
        let incomplete = f.check_format(
            &["pack", "verify", "artifact.json", "--against", "current"],
            3,
            format,
        );
        assert_eq!(incomplete["data"]["integrity"], "verified");
        assert_eq!(incomplete["data"]["freshness"], "unknown");
        assert_eq!(
            incomplete["data"]["source_validation"][0]["freshness"],
            "unknown"
        );
        f.check_format(&["pack", "inspect", "absent.json"], 6, format);
    }
}

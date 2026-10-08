//! PCTX01 frontend outcomes from actual isolated native children.
#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture {
    temp: tempfile::TempDir,
    program: PathBuf,
}
impl Fixture {
    fn new(body: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("fixture");
        let f = Self { temp, program };
        let init = f.invoke(&["init"]);
        assert!(init.status.success(), "{init:?}");
        fs::write(&f.program, body).unwrap();
        fs::set_permissions(&f.program, fs::Permissions::from_mode(0o755)).unwrap();
        f
    }
    fn invoke(&self, args: &[&str]) -> Output {
        self.invoke_format(args, "json")
    }
    fn invoke_format(&self, args: &[&str], format: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(self.temp.path())
            .args([
                "--root",
                self.temp.path().to_str().unwrap(),
                "--format",
                format,
            ])
            .args(args)
            .env("PCTX_DATA_DIR", self.temp.path().join("data"))
            .env("PCTX_USER_CONFIG", self.temp.path().join("absent-config"))
            .env("PCTX_HOST_RESOURCE_DIR", self.temp.path().join("host"))
            .env("PCTX_ACTOR", "owner")
            .output()
            .unwrap()
    }
    fn trust(&self, mode: &str) {
        let program = self.program.to_str().unwrap();
        let result = self.invoke(&["trust", "plan", "--", program, mode]);
        assert!(result.status.success(), "{result:?}");
        let v = document(&result);
        let fingerprint = v["data"]["fingerprint"].as_str().unwrap();
        let result = self.invoke(&[
            "trust",
            "add",
            "--expect-hash",
            fingerprint,
            "--",
            program,
            mode,
        ]);
        assert!(result.status.success(), "{result:?}");
    }
    fn run(&self, mode: &str, policy: &str, timeout: &str) -> Output {
        self.invoke(&[
            "run",
            "--exit-policy",
            policy,
            "--execution-timeout-ms",
            timeout,
            "--",
            self.program.to_str().unwrap(),
            mode,
        ])
    }
    fn count(&self) -> PathBuf {
        self.temp.path().join("invocations")
    }
}
fn document(out: &Output) -> Value {
    assert!(out.stderr.is_empty(), "{out:?}");
    serde_json::from_slice(&out.stdout).unwrap()
}
const SCRIPT: &str = "#!/bin/sh\nprintf x >> invocations\nprintf 'child stdout\\n'\nprintf 'child stderr\\n' >&2\ncase \"$1\" in signal) kill -TERM $$;; timeout) exec /bin/sleep 5;; *) exit \"$1\";; esac\n";

#[test]
fn actual_child_codes_and_signal_are_distinct_from_wrapper_status() {
    for (mode, code, signal) in [
        ("0", Some(0), None),
        ("1", Some(1), None),
        ("2", Some(2), None),
        ("signal", None, Some(15)),
    ] {
        let f = Fixture::new(SCRIPT);
        f.trust(mode);
        for policy in ["child", "pctx"] {
            let out = f.run(mode, policy, "5000");
            let v = document(&out);
            let expected = if policy == "pctx" {
                0
            } else {
                code.unwrap_or_else(|| 128 + signal.unwrap())
            };
            assert_eq!(out.status.code(), Some(expected), "{v}");
            assert_eq!(v["status"], "ok", "{v}");
            assert_eq!(v["data"]["spawned"], true);
            assert_eq!(v["data"]["child_exit_code"], serde_json::json!(code));
            assert_eq!(v["data"]["signal"], serde_json::json!(signal));
            assert!(v["data"]["pctx_error"].is_null());
            assert_eq!(v["data"]["task_completion"], "not_evaluated");
            assert_eq!(v["data"]["test_result"], "not_evaluated");
            let id = v["data"]["output_id"].as_str().unwrap();
            let reread = f.invoke(&["output", "show", id, "--view", "full"]);
            assert!(reread.status.success(), "{reread:?}");
            let saved = document(&reread);
            assert_eq!(saved["data"]["command_rerun"], false);
            assert_eq!(saved["data"]["child_exit_code"], serde_json::json!(code));
            assert_eq!(saved["data"]["signal"], serde_json::json!(signal));
            assert!(
                saved["data"]["records"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["stream"] == "stderr" && r["text"] == "child stderr")
            );
        }
        assert_eq!(
            fs::read(f.count()).unwrap(),
            b"xx",
            "Exactly two explicit runs, no reread execution"
        );
    }
}
#[test]
fn actual_prelaunch_refusal_has_no_child_outcome_or_invocation() {
    let f = Fixture::new(SCRIPT);
    for policy in ["child", "pctx"] {
        let out = f.run("2", policy, "5000");
        let v = document(&out);
        assert_eq!(out.status.code(), Some(5), "{v}");
        assert_eq!(v["status"], "error");
        assert_eq!(v["data"]["spawned"], false);
        assert_eq!(v["data"]["termination"], "not_started");
        assert!(v["data"]["child_exit_code"].is_null());
        assert_eq!(v["data"]["pctx_error"], "OWNER_DECISION_REQUIRED");
    }
    assert!(!f.count().exists());
}
#[test]
fn actual_spawn_failure_never_becomes_success_or_child_exit() {
    let f = Fixture::new("#!/nonexistent-pctx-fixture-interpreter\n");
    f.trust("0");
    for policy in ["child", "pctx"] {
        let out = f.run("0", policy, "5000");
        let v = document(&out);
        assert_eq!(out.status.code(), Some(7), "{v}");
        assert_eq!(v["status"], "error");
        assert_eq!(v["data"]["spawned"], false);
        assert_eq!(v["data"]["termination"], "not_started");
        assert!(v["data"]["child_exit_code"].is_null());
        assert_eq!(v["data"]["pctx_error"], "SPAWN_FAILED");
        assert_eq!(v["errors"][0]["code"], "SPAWN_FAILED");
    }
}
#[test]
fn actual_execution_timeout_preserves_observation_but_returns_wrapper_error() {
    let f = Fixture::new(SCRIPT);
    f.trust("timeout");
    for policy in ["child", "pctx"] {
        let out = f.run("timeout", policy, "250");
        let v = document(&out);
        assert_eq!(out.status.code(), Some(7), "{v}");
        assert_eq!(v["status"], "error");
        assert_eq!(v["data"]["spawned"], true);
        assert_eq!(v["data"]["termination"], "timed_out");
        assert!(v["data"]["child_exit_code"].is_null());
        assert!(v["data"]["signal"].is_number());
        assert_eq!(v["data"]["pctx_error"], "TIMEOUT");
        assert_eq!(v["errors"][0]["code"], "TIMEOUT");
    }
    // A child may time out before its first script instruction. Do not infer execution
    // of the body from a successful native spawn.
    if f.count().exists() {
        assert!(fs::read(f.count()).unwrap().len() <= 2);
    }
}

#[test]
fn incomplete_capture_is_partial_without_replacing_child_success() {
    let f = Fixture::new("#!/bin/sh\nprintf x >> invocations\nprintf '\\377\\n'\nexit 0\n");
    f.trust("0");
    for policy in ["child", "pctx"] {
        let out = f.run("0", policy, "5000");
        let v = document(&out);
        assert_eq!(out.status.code(), Some(3), "{v}");
        assert_eq!(v["status"], "partial");
        assert_eq!(v["data"]["child_exit_code"], 0);
        assert_eq!(v["data"]["capture_complete"], false);
        assert!(v["data"]["pctx_error"].is_null());
        assert_eq!(v["data"]["test_result"], "not_evaluated");
    }
    assert_eq!(fs::read(f.count()).unwrap(), b"xx");
}

#[test]
fn publication_failure_preserves_child_outcome_and_never_retries() {
    use fs2::FileExt;
    let f = Fixture::new(SCRIPT);
    f.trust("0");
    let store = f.temp.path().join("data/outputs");
    fs::create_dir_all(&store).unwrap();
    let lock = fs::File::create(store.join("store.lock")).unwrap();
    lock.lock_exclusive().unwrap();
    for policy in ["child", "pctx"] {
        let out = f.run("0", policy, "5000");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        let warning: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(warning["code"], "OUTPUT_MEASUREMENT_UNRECORDED");
        assert_eq!(warning["delivery_written"], true);
        assert_eq!(warning["measurement_recorded"], "unknown");
        assert_eq!(out.status.code(), Some(7), "{v}");
        assert_eq!(v["status"], "error");
        assert_eq!(v["data"]["spawned"], true);
        assert_eq!(v["data"]["child_exit_code"], 0);
        assert_eq!(v["data"]["pctx_error"], "RESOURCE_BUSY");
        assert_eq!(v["data"]["raw_available"], false);
        assert_eq!(v["errors"][0]["code"], "RESOURCE_BUSY");
        let id = v["data"]["output_id"].as_str().unwrap();
        assert!(
            !store
                .join(v["workspace_id"].as_str().unwrap())
                .join(format!("{id}.json"))
                .exists()
        );
    }
    assert_eq!(fs::read(f.count()).unwrap(), b"xx");
}

#[test]
fn missing_project_is_prelaunch_failure_in_both_modes_without_files() {
    let temp = tempfile::tempdir().unwrap();
    for policy in ["child", "pctx"] {
        let out = Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(temp.path())
            .args([
                "--root",
                "missing",
                "--format",
                "json",
                "run",
                "--exit-policy",
                policy,
                "--",
                "fixture",
            ])
            .env("PCTX_DATA_DIR", temp.path().join("data"))
            .env("PCTX_USER_CONFIG", temp.path().join("absent-config"))
            .output()
            .unwrap();
        let v = document(&out);
        assert!(!out.status.success());
        assert_eq!(v["status"], "error");
        assert_eq!(v["data"]["spawned"], false);
        assert_eq!(v["data"]["termination"], "not_started");
        assert!(v["data"]["child_exit_code"].is_null());
        assert!(v["data"]["signal"].is_null());
        assert_eq!(v["data"]["pctx_error"], v["errors"][0]["code"]);
        assert!(v["data"]["output_id"].is_null());
    }
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn project_refusals_preserve_typed_exit_and_message_before_spawn() {
    for configured in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        if configured {
            fs::create_dir(temp.path().join(".pctx")).unwrap();
            fs::write(temp.path().join(".pctx/config.toml"), "invalid = [").unwrap();
        }
        let before = if configured {
            Some(fs::read(temp.path().join(".pctx/config.toml")).unwrap())
        } else {
            None
        };
        for policy in ["child", "pctx"] {
            let out = Command::new(env!("CARGO_BIN_EXE_pctx"))
                .current_dir(temp.path())
                .args([
                    "--root",
                    temp.path().to_str().unwrap(),
                    "--format",
                    "json",
                    "run",
                    "--exit-policy",
                    policy,
                    "--",
                    "fixture",
                ])
                .env("PCTX_DATA_DIR", temp.path().join("data"))
                .env("PCTX_USER_CONFIG", temp.path().join("absent-config"))
                .output()
                .unwrap();
            let v = document(&out);
            let (code, exit, message) = if configured {
                ("INVALID_CONFIG", 2, "Invalid project TOML or unknown field")
            } else {
                ("NOT_INITIALIZED", 6, "Run pctx init first")
            };
            assert_eq!(out.status.code(), Some(exit), "{v}");
            assert_eq!(v["errors"][0]["code"], code);
            assert_eq!(v["errors"][0]["message"], message);
            assert_eq!(v["data"]["pctx_error"], code);
            assert_eq!(v["data"]["spawned"], false);
            assert_eq!(v["data"]["termination"], "not_started");
            assert!(v["data"]["child_exit_code"].is_null());
            assert!(v["data"]["signal"].is_null());
        }
        assert!(!temp.path().join("data").exists());
        if let Some(before) = before {
            assert_eq!(
                fs::read(temp.path().join(".pctx/config.toml")).unwrap(),
                before
            );
            assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        } else {
            assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
        }
    }
}

#[test]
fn admitted_small_run_budgets_preserve_prelaunch_truth_without_artifact() {
    let f = Fixture::new(SCRIPT);
    let probe = f.invoke(&[
        "run",
        "--budget-bytes",
        "1",
        "--",
        f.program.to_str().unwrap(),
        "0",
    ]);
    assert_eq!(probe.status.code(), Some(2));
    let minimum = document(&probe)["data"]["minimum_budget_bytes"]
        .as_u64()
        .unwrap() as usize;
    for policy in ["child", "pctx"] {
        for limit in [minimum - 1, minimum, minimum + 1, 2000] {
            let out = f.invoke(&[
                "run",
                "--exit-policy",
                policy,
                "--budget-bytes",
                &limit.to_string(),
                "--",
                f.program.to_str().unwrap(),
                "0",
            ]);
            let v = document(&out);
            let (exit, code) = if limit < minimum {
                (2, "INVALID_ARGUMENT")
            } else {
                (8, "BUDGET_TOO_SMALL")
            };
            assert_eq!(out.status.code(), Some(exit), "{v}");
            assert_eq!(v["errors"][0]["code"], code);
            assert_eq!(v["data"]["spawned"], false, "{v}");
            assert_eq!(v["data"]["termination"], "not_started");
            assert!(v["data"]["child_exit_code"].is_null());
            assert!(v["data"]["signal"].is_null());
            assert_eq!(v["data"]["pctx_error"], code);
            assert!(v["data"]["output_id"].is_null());
            if limit >= minimum {
                assert!(
                    out.stdout.len() <= limit,
                    "{} > {limit}: {v}",
                    out.stdout.len()
                );
            }
        }
    }
    assert!(!f.count().exists());
    assert!(!f.temp.path().join("data/output-jobs").exists());
    assert!(!f.temp.path().join("data/outputs").exists());
}

#[test]
fn project_refusal_near_minimum_keeps_original_exit_and_fits() {
    let temp = tempfile::tempdir().unwrap();
    let invoke = |policy: &str, budget: &str| {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(temp.path())
            .args([
                "--root",
                temp.path().to_str().unwrap(),
                "--format",
                "json",
                "run",
                "--exit-policy",
                policy,
                "--budget-bytes",
                budget,
                "--",
                "fixture",
            ])
            .env("PCTX_DATA_DIR", temp.path().join("data"))
            .env("PCTX_USER_CONFIG", temp.path().join("absent-config"))
            .output()
            .unwrap()
    };
    let probe = invoke("child", "1");
    let minimum = document(&probe)["data"]["minimum_budget_bytes"]
        .as_u64()
        .unwrap() as usize;
    for policy in ["child", "pctx"] {
        for limit in [minimum, minimum + 1] {
            let out = invoke(policy, &limit.to_string());
            let v = document(&out);
            assert_eq!(out.status.code(), Some(6), "{v}");
            assert_eq!(v["errors"][0]["code"], "NOT_INITIALIZED");
            assert_eq!(v["data"]["pctx_error"], "NOT_INITIALIZED");
            assert_eq!(v["data"]["spawned"], false);
            assert_eq!(v["data"]["termination"], "not_started");
            assert!(
                out.stdout.len() <= limit,
                "{} > {limit}: {v}",
                out.stdout.len()
            );
        }
    }
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn final_serialized_budget_preserves_actual_child_truth_and_full_reread() {
    // Bidi formatting is retained as text and expands from three bytes to six in safe JSON.
    // Producer record admission alone cannot establish the delivered byte budget.
    let text = "\u{202e}".repeat(750);
    let body = format!(
        "#!/bin/sh\nprintf x >> invocations\nprintf '{text}\\n'\ncase \"$1\" in signal) kill -TERM $$;; *) exit \"$1\";; esac\n"
    );
    for (mode, code, signal) in [
        ("0", Some(0), None),
        ("2", Some(2), None),
        ("signal", None, Some(15)),
    ] {
        let f = Fixture::new(&body);
        f.trust(mode);
        let baseline = f.run(mode, "pctx", "5000");
        assert!(baseline.status.success(), "{baseline:?}");
        let initial = document(&baseline);
        assert!(
            baseline.stdout.len() > 5000,
            "Fixture must exercise final-byte trimming: {}",
            baseline.stdout.len()
        );
        assert_eq!(initial["data"]["records_included"], 1);
        for format in ["json", "compact"] {
            for policy in ["child", "pctx"] {
                let out = f.invoke_format(
                    &[
                        "run",
                        "--exit-policy",
                        policy,
                        "--execution-timeout-ms",
                        "5000",
                        "--budget-bytes",
                        "5000",
                        "--",
                        f.program.to_str().unwrap(),
                        mode,
                    ],
                    format,
                );
                let v = document(&out);
                let expected = if policy == "pctx" {
                    0
                } else {
                    code.unwrap_or_else(|| 128 + signal.unwrap())
                };
                assert_eq!(out.status.code(), Some(expected), "{v}");
                assert!(out.stdout.len() <= 5000, "{}: {v}", out.stdout.len());
                assert_eq!(v["status"], "ok", "{v}");
                assert_eq!(v["data"]["spawned"], true);
                assert_eq!(v["data"]["child_exit_code"], serde_json::json!(code));
                assert_eq!(v["data"]["signal"], serde_json::json!(signal));
                assert_eq!(v["data"]["capture_complete"], true);
                assert!(v["data"]["pctx_error"].is_null());
                assert_eq!(v["data"]["records_included"], 0, "{v}");
                assert_eq!(v["data"]["records_omitted"], 1);
                assert_eq!(v["data"]["test_result"], "not_evaluated");
                let id = v["data"]["output_id"].as_str().unwrap();
                let saved = f.invoke(&["output", "show", id, "--view", "full"]);
                assert!(saved.status.success(), "{saved:?}");
                let saved = document(&saved);
                assert_eq!(saved["data"]["command_rerun"], false);
                assert_eq!(saved["data"]["child_exit_code"], serde_json::json!(code));
                assert_eq!(saved["data"]["signal"], serde_json::json!(signal));
                assert_eq!(saved["data"]["records"][0]["text"], text);
            }
        }
        assert_eq!(fs::read(f.count()).unwrap(), b"xxxxx");
    }
}

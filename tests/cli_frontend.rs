//! PCTX01 frontend contracts before project discovery or effects.
#[cfg(unix)]
#[path = "support/delivery.rs"]
mod delivery;
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

#[test]
fn parser_diagnostics_mask_secrets_and_escape_control_characters() {
    let f = Fixture::new();
    let output = f.run(&args(&["--no-color", "ghp_abcdefghijklmnopqrst\u{1b}[31m"]));
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(!text.contains("ghp_abcdefghijklmnopqrst"));
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("REDACTED"));
    f.unchanged();
}

#[cfg(unix)]
#[test]
fn no_color_suppresses_help_on_a_real_terminal_including_nested_help() {
    use std::{
        io::Read,
        os::fd::{AsRawFd, FromRawFd},
        process::{Child, Stdio},
        time::{Duration, Instant},
    };
    struct Reap(Child);
    impl Drop for Reap {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn terminal(f: &Fixture, values: &[&str]) -> Vec<u8> {
        let mut master = -1;
        let mut slave = -1;
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
        // The child receives only its stdout/stderr copies, not the master.
        assert_eq!(
            unsafe { libc::fcntl(master, libc::F_SETFD, libc::FD_CLOEXEC) },
            0
        );
        assert_eq!(
            unsafe { libc::fcntl(slave, libc::F_SETFD, libc::FD_CLOEXEC) },
            0
        );
        let mut master = unsafe { fs::File::from_raw_fd(master) };
        let slave = unsafe { fs::File::from_raw_fd(slave) };
        let mut child = Reap(
            Command::new(env!("CARGO_BIN_EXE_pctx"))
                .current_dir(f.temp.path())
                .args(values)
                .env("TERM", "xterm-256color")
                .env("CLICOLOR_FORCE", "1")
                .env_remove("NO_COLOR")
                .env_remove("CLICOLOR")
                .stdin(Stdio::null())
                .stdout(slave.try_clone().unwrap())
                .stderr(slave)
                .spawn()
                .unwrap(),
        );
        let mut bytes = Vec::new();
        let mut chunk = [0; 4096];
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .expect("Terminal help exceeded ten seconds");
            let mut ready = libc::pollfd {
                fd: master.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let result = unsafe {
                libc::poll(
                    &mut ready,
                    1,
                    remaining.as_millis().min(i32::MAX as u128) as i32,
                )
            };
            if result < 0
                && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
            {
                continue;
            }
            assert!(
                result > 0,
                "Terminal help did not finish within ten seconds"
            );
            match master.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                Err(e) if e.raw_os_error() == Some(libc::EIO) => break,
                Err(e) => panic!("Terminal capture failed: {e}"),
            }
        }
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "Terminal child did not exit within ten seconds"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(status.success());
        bytes
    }
    let f = Fixture::new();
    let colored = terminal(&f, &["--help"]);
    assert!(
        colored.contains(&27),
        "Positive color control was not a colored terminal"
    );
    for values in [
        vec!["--no-color", "--help"],
        vec!["session", "attach", "--no-color", "--help"],
        vec!["--format=json", "--help"],
    ] {
        let plain = terminal(&f, &values);
        assert!(!plain.contains(&27));
    }
    f.unchanged();
}

#[test]
fn every_visible_help_path_describes_schema_and_effects_without_project_access() {
    let f = Fixture::new();
    let mut pending = vec![Vec::<String>::new()];
    let mut visited = 0;
    while let Some(path) = pending.pop() {
        let mut argv = path.clone();
        argv.extend(["--no-color".into(), "--help".into()]);
        let output = f.run(&argv.iter().map(Into::into).collect::<Vec<_>>());
        assert!(output.status.success(), "{path:?}");
        assert!(output.stderr.is_empty(), "{path:?}");
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("JSON envelope schema 1.0"), "{path:?}");
        assert!(text.contains("Effects:"), "{path:?}");
        assert!(!text.contains("unclassified:"), "{path:?}");
        assert!(!text.contains('\u{1b}'), "{path:?}");
        // Discover the public command tree from actual help, including flattened groups.
        if let Some(section) = text.split("Commands:\n").nth(1) {
            for line in section.lines().take_while(|line| !line.is_empty()) {
                let name = line.split_whitespace().next().unwrap();
                if name != "help" {
                    let mut next = path.clone();
                    next.push(name.into());
                    pending.push(next);
                }
            }
        }
        visited += 1;
    }
    assert!(
        visited > 100,
        "Command-tree traversal stopped early: {visited}"
    );
    for (path, required) in [
        (vec!["index", "gc"], "--apply writes by deleting"),
        (vec!["task", "complete"], "--dry-run only reads"),
        (vec!["adapter", "claude", "doctor"], "claude --version"),
        (vec!["context", "get"], "context receipt"),
        (vec!["session", "reconcile"], "does not mutate epochs"),
        (vec!["output", "render"], "never reruns"),
        (vec!["schedule", "inspect"], "--observe-native executes"),
        (vec!["schedule", "install"], "--apply-native executes"),
        (vec!["extract"], "refresh derived index"),
    ] {
        let mut argv = path;
        argv.push("--help");
        let output = f.run(&args(&argv));
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout).unwrap().contains(required));
    }
    f.unchanged();
}

#[test]
fn semantic_refusals_are_one_safe_json_document_for_every_requested_format() {
    let f = Fixture::new();
    for format in ["compact", "json", "markdown"] {
        for tail in [
            vec!["read", "code.py", "--lines", "0:2"],
            vec!["build", "--task", "fixture", "--detail", "unknown"],
            vec!["--timeout-ms", "0", "read", "code.py"],
        ] {
            let mut values = args(&[
                "--root",
                "absent-project",
                "--format",
                format,
                "--output",
                "refused-response.json",
                "--no-color",
            ]);
            values.extend(args(&tail));
            let output = f.run(&values);
            argument_error(&output);
            let documents = serde_json::Deserializer::from_slice(&output.stdout)
                .into_iter::<Value>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(documents.len(), 1);
            assert_eq!(output.stdout.last(), Some(&b'\n'));
            assert!(!output.stdout.contains(&27));
            f.unchanged();
        }
    }
}

#[test]
fn valid_minimum_capacity_bounds_semantic_and_timeout_refusals_without_writes() {
    let f = Fixture::new();
    let tiny = f.run(&args(&[
        "--format",
        "json",
        "build",
        "--task",
        "fixture",
        "--budget-bytes",
        "1",
    ]));
    argument_error(&tiny);
    let value: Value = serde_json::from_slice(&tiny.stdout).unwrap();
    let minimum = value["data"]["minimum_budget_bytes"].as_u64().unwrap() as usize;
    for format in ["compact", "json", "markdown"] {
        for tail in [
            vec!["--detail", "unknown"],
            vec!["--dependency-depth", "3"],
            vec!["--timeout-ms", "0"],
        ] {
            let mut values = args(&[
                "--root",
                "absent-project",
                "--format",
                format,
                "--output",
                "refused-response.json",
                "build",
                "--task",
                "fixture",
                "--budget-bytes",
                &minimum.to_string(),
            ]);
            values.extend(args(&tail));
            let output = f.run(&values);
            argument_error(&output);
            assert!(
                output.stdout.len() <= minimum,
                "{} > {minimum}",
                output.stdout.len()
            );
            f.unchanged();
        }
    }
}

#[cfg(unix)]
#[test]
fn undeliverable_parser_capacity_semantic_and_timeout_refusals_return_io_exit() {
    if !delivery::isolated_case(
        "undeliverable_parser_capacity_semantic_and_timeout_refusals_return_io_exit",
    ) {
        return;
    }
    let f = Fixture::new();
    for values in [
        vec!["--format", "json", "--unknown"],
        vec![
            "--format",
            "json",
            "build",
            "--task",
            "fixture",
            "--budget-bytes",
            "1",
        ],
        vec!["--format", "json", "read", "code.py", "--lines", "0:2"],
        vec!["--format", "json", "--timeout-ms", "0", "read", "code.py"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(f.temp.path())
            .args(&values)
            .env("PCTX_DATA_DIR", f.temp.path().join("data"))
            .env("PCTX_USER_CONFIG", f.temp.path().join("absent-config"))
            .stdout(delivery::closed_pipe())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(7), "{values:?}: {output:?}");
        assert!(output.stderr.is_empty());
        f.unchanged();
    }
}

#[test]
fn plain_parser_diagnostics_escape_bidi_c1_and_line_controls() {
    let f = Fixture::new();
    for control in ['\u{202e}', '\u{2066}', '\u{200f}', '\u{85}', '\u{2028}'] {
        let option = format!("--unknown-{control}-name");
        let output = f.run(&args(&["--no-color", &option]));
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(!diagnostic.contains(control), "{diagnostic:?}");
        assert!(
            diagnostic.contains(&format!("\\u{{{:04x}}}", control as u32)),
            "{diagnostic:?}"
        );
        f.unchanged();
    }
}

#[test]
fn unsupported_markdown_refuses_before_response_files_and_stream_dispatch() {
    for command in [
        vec!["init"],
        vec!["status"],
        vec!["index", "update"],
        vec!["find", "needle"],
        vec!["task", "list"],
        vec!["context", "get", "--task-id", "T001", "--session", "S001"],
        vec!["board"],
        vec!["board", "--watch"],
        vec!["activity"],
        vec!["activity", "--follow"],
        vec!["run", "--", "must-not-execute"],
    ] {
        for output_file in [false, true] {
            let f = Fixture::new();
            let mut values = args(&["--root", "absent-project", "--format", "markdown"]);
            if output_file {
                values.push("--output".into());
                values.push(f.temp.path().join("rejected.json").into_os_string());
            }
            values.extend(args(&command));
            let output = f.run(&values);
            assert!(
                !f.temp.path().join("rejected.json").exists(),
                "Rejected Markdown request wrote a response file: {command:?}"
            );
            argument_error(&output);
            let v: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                v["errors"][0]["message"], "Markdown supports build, outline, read and handoff",
                "{command:?}"
            );
            f.unchanged();
        }
    }
}

#[cfg(unix)]
#[test]
fn undeliverable_plain_parser_diagnostic_returns_io_exit_without_panic() {
    if !delivery::isolated_case(
        "undeliverable_plain_parser_diagnostic_returns_io_exit_without_panic",
    ) {
        return;
    }
    let f = Fixture::new();
    let normal = f.run(&args(&["--no-color", "--unknown"]));
    assert_eq!(normal.status.code(), Some(2));
    assert!(normal.stdout.is_empty());
    assert!(!normal.stderr.is_empty());
    let output = Command::new(env!("CARGO_BIN_EXE_pctx"))
        .current_dir(f.temp.path())
        .args(["--no-color", "--unknown"])
        .env("PCTX_DATA_DIR", f.temp.path().join("data"))
        .env("PCTX_USER_CONFIG", f.temp.path().join("absent-config"))
        .stderr(delivery::closed_pipe())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    assert!(output.stdout.is_empty());
    f.unchanged();
}

#[test]
fn root_version_is_exact_and_ignores_project_output_and_query_options() {
    let f = Fixture::new();
    for version in ["--version", "-V"] {
        for options_first in [false, true] {
            let globals = args(&[
                "--root",
                "absent-project",
                "--format",
                "markdown",
                "--timeout-ms",
                "0",
                "--output",
                "must-not-write",
                "--no-color",
            ]);
            let mut values = if options_first {
                globals.clone()
            } else {
                args(&[version])
            };
            values.extend(if options_first {
                args(&[version])
            } else {
                globals
            });
            let output = f.run(&values);
            assert_eq!(output.status.code(), Some(0), "{values:?}: {output:?}");
            assert_eq!(
                output.stdout,
                format!("pctx {}\n", env!("CARGO_PKG_VERSION")).as_bytes()
            );
            assert!(output.stderr.is_empty());
            f.unchanged();
        }
    }
}

#[test]
fn singular_global_duplicates_are_rejected_before_project_and_output_effects() {
    for values in [
        vec!["--format=json", "--format=json", "init"],
        vec!["--format=json", "checkpoint", "--format=json", "list"],
        vec![
            "--format=json",
            "--root",
            "first",
            "--root",
            "second",
            "init",
        ],
        vec![
            "--format=json",
            "--root",
            "first",
            "checkpoint",
            "list",
            "--root",
            "second",
        ],
        vec![
            "--format=json",
            "--timeout-ms",
            "10000",
            "--timeout-ms",
            "10000",
            "status",
        ],
        vec![
            "--format=json",
            "--timeout-ms",
            "10000",
            "checkpoint",
            "list",
            "--timeout-ms",
            "10000",
        ],
        vec![
            "--format=json",
            "--output",
            "first.json",
            "--output",
            "second.json",
            "init",
        ],
        vec![
            "--format=json",
            "--output",
            "first.json",
            "checkpoint",
            "list",
            "--output",
            "second.json",
        ],
        vec!["--format=json", "--no-color", "--no-color", "status"],
        vec![
            "--format=json",
            "--no-color",
            "checkpoint",
            "list",
            "--no-color",
        ],
    ] {
        let f = Fixture::new();
        let output = f.run(&args(&values));
        argument_error(&output);
        f.unchanged();
    }
}

#[test]
fn nested_global_positions_select_the_same_root_format_and_timeout() {
    let f = Fixture::new();
    fs::create_dir(f.temp.path().join("project")).unwrap();
    let init = f.run(&args(&["--root", "project", "--format=json", "init"]));
    assert_eq!(init.status.code(), Some(0), "{init:?}");
    let init: Value = serde_json::from_slice(&init.stdout).unwrap();
    let index = f.run(&args(&[
        "--root",
        "project",
        "--format=json",
        "index",
        "update",
    ]));
    assert_eq!(index.status.code(), Some(0), "{index:?}");
    for position in 0..3 {
        let mut values = args(&["checkpoint", "list"]);
        let globals = args(&[
            "--root",
            "project",
            "--format=json",
            "--timeout-ms",
            "10000",
            "--no-color",
        ]);
        values.splice(position..position, globals);
        let output = f.run(&values);
        assert_eq!(output.status.code(), Some(0), "{values:?}: {output:?}");
        assert!(output.stderr.is_empty());
        let v: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(v["project_id"], init["project_id"]);
        assert_eq!(v["workspace_id"], init["workspace_id"]);
        assert_eq!(v["command"], "checkpoint list");
        assert_eq!(v["status"], "ok");
        assert!(output.stdout.ends_with(b"\n"));
        let zero = args(&[
            "--root",
            "absent-project",
            "--format=json",
            "--timeout-ms",
            "0",
        ]);
        let mut values = args(&["checkpoint", "list"]);
        values.splice(position..position, zero);
        argument_error(&f.run(&values));
    }
}

#[test]
fn pack_artifact_output_is_required_once_and_works_at_every_command_depth() {
    let f = Fixture::new();
    fs::create_dir_all(f.temp.path().join("project/src")).unwrap();
    fs::write(
        f.temp.path().join("project/src/code.ts"),
        "export const value = 1;\n",
    )
    .unwrap();
    let task_path = f.temp.path().join("project/task.json");
    fs::write(&task_path, serde_json::json!({"schema_version":1,"title":"Inspect source","scope":["src/**"],"acceptance":[{"id":"AC1","description":"Inspected","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
    let call = |command: Vec<OsString>| {
        let mut values = args(&["--root", "project", "--format=json"]);
        values.extend(command);
        let output = f.run(&values);
        assert_eq!(output.status.code(), Some(0), "{values:?}: {output:?}");
        assert!(output.stderr.is_empty());
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    call(args(&["init"]));
    let mut create = args(&["task", "create", "--from-file"]);
    create.push(task_path.into_os_string());
    let task = call(create)["data"]["task_id"].as_str().unwrap().to_owned();
    let plan = call(args(&[
        "pack",
        "plan",
        "--task-id",
        &task,
        "--scope",
        "src",
        "--content",
        "metadata",
    ]));
    let plan_id = plan["data"]["plan_id"].as_str().unwrap();
    let plan_hash = plan["data"]["plan_hash"].as_str().unwrap();
    for position in 0..3 {
        let path = format!("artifacts/position-{position}");
        let mut values = args(&[
            "pack",
            "create",
            "--plan",
            plan_id,
            "--expect-hash",
            plan_hash,
        ]);
        values.splice(position..position, args(&["--output", &path]));
        let response = call(values);
        assert_eq!(response["command"], "pack");
        assert_eq!(response["status"], "ok");
        assert!(
            f.temp
                .path()
                .join("project")
                .join(&path)
                .join("manifest.json")
                .is_file()
        );
        assert!(
            f.temp
                .path()
                .join("project")
                .join(&path)
                .join("context.json")
                .is_file()
        );
    }
    let mut values = args(&[
        "--root",
        "project",
        "--format=json",
        "pack",
        "create",
        "--plan",
        plan_id,
        "--expect-hash",
        plan_hash,
    ]);
    argument_error(&f.run(&values));
    values.splice(0..0, args(&["--output", "artifacts/first"]));
    values.extend(args(&["--output", "artifacts/second"]));
    argument_error(&f.run(&values));
    assert!(!f.temp.path().join("project/artifacts/first").exists());
    assert!(!f.temp.path().join("project/artifacts/second").exists());
    let help = f.run(&args(&["pack", "create", "--help"]));
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("supply exactly one --output")
    );
}

#[test]
fn native_hook_invalid_transports_refuse_before_project_or_output_access() {
    for format in ["compact", "json"] {
        for input in [false, true] {
            for output in [false, true] {
                if !input && !output {
                    continue;
                }
                let f = Fixture::new();
                let mut argv = args(&[
                    "--root", "missing", "--format", format, "adapter", "claude", "event",
                    "--agent", "fixture", "--hook",
                ]);
                if input {
                    argv.extend(args(&["--from-file", "unread-input.json"]));
                }
                if output {
                    argv.push("--output".into());
                    argv.push(f.temp.path().join("response.json").into_os_string());
                }
                let result = f.run(&argv);
                argument_error(&result);
                f.unchanged();
            }
        }
    }
}

#[test]
fn parsed_run_preflight_refusals_attest_no_child_before_any_access() {
    for format in ["compact", "json"] {
        for refusal in ["capacity", "timeout", "representation"] {
            let f = Fixture::new();
            let mut argv = args(&[
                "--root",
                "missing",
                "--format",
                if refusal == "representation" {
                    "markdown"
                } else {
                    format
                },
                "--output",
            ]);
            argv.push(f.temp.path().join("refused.json").into_os_string());
            argv.push("run".into());
            if refusal == "capacity" {
                argv.extend(args(&["--budget-bytes", "1"]));
            } else if refusal == "timeout" {
                argv.extend(args(&["--timeout-ms", "0"]));
            }
            argv.extend(args(&["--", "fixture"]));
            let out = f.run(&argv);
            argument_error(&out);
            let v: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(v["data"]["spawned"], false);
            assert_eq!(v["data"]["termination"], "not_started");
            assert!(v["data"]["child_exit_code"].is_null());
            assert!(v["data"]["signal"].is_null());
            assert_eq!(v["data"]["pctx_error"], "INVALID_ARGUMENT");
            f.unchanged();
        }
    }
}

fn outline_document(out: &Output, format: &str) -> Value {
    if format == "markdown" {
        let text = std::str::from_utf8(&out.stdout).unwrap();
        let metadata = text
            .split("## Complete envelope metadata\n")
            .nth(1)
            .unwrap();
        let body = metadata
            .split("```json\n")
            .nth(1)
            .unwrap()
            .split("\n```")
            .next()
            .unwrap();
        serde_json::from_str(body).unwrap()
    } else {
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

#[test]
fn initialized_producer_errors_keep_identity_in_each_document_and_destination() {
    let f = Fixture::new();
    let init = f.run(&args(&["--format", "json", "init"]));
    assert!(init.status.success());
    let established: Value = serde_json::from_slice(&init.stdout).unwrap();
    for format in ["json", "compact", "markdown"] {
        for to_file in [false, true] {
            let destination = f.temp.path().join("failure-response");
            let mut request = args(&["--format", format, "read", "absent.txt"]);
            if to_file {
                request.extend([OsString::from("--output"), destination.clone().into()]);
            }
            let mut out = f.run(&request);
            assert_eq!(out.status.code(), Some(7), "{out:?}");
            assert!(out.stderr.is_empty());
            if to_file {
                assert!(out.stdout.is_empty());
                out.stdout = fs::read(&destination).unwrap();
                fs::remove_file(destination).unwrap();
            }
            let response = outline_document(&out, format);
            assert_eq!(response["status"], "error");
            assert_eq!(response["errors"][0]["code"], "IO_ERROR");
            for key in ["project_id", "workspace_id"] {
                assert_eq!(response[key], established[key], "{format}: lost {key}");
            }
            assert_eq!(response["coverage"]["status"], "partial");
        }
    }
}

#[test]
fn native_metadata_overflow_keeps_prelaunch_truth_and_declares_omitted_identity() {
    let f = Fixture::new();
    let init = f.run(&args(&["--format", "json", "init"]));
    assert!(init.status.success());
    let initialized: Value = serde_json::from_slice(&init.stdout).unwrap();
    let registry_path = f.temp.path().join("data/registry.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&registry_path).unwrap()).unwrap();
    let binding = registry["roots"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap();
    let old = binding["workspace_id"].as_str().unwrap().to_owned();
    let wide = format!("WS-{}", "w".repeat(237));
    fs::rename(
        f.temp.path().join("data/workspaces").join(&old),
        f.temp.path().join("data/workspaces").join(&wide),
    )
    .unwrap();
    binding["workspace_id"] = serde_json::json!(wide);
    fs::write(registry_path, serde_json::to_vec(&registry).unwrap()).unwrap();
    let probe = f.run(&args(&[
        "--format",
        "json",
        "run",
        "--budget-bytes",
        "1",
        "--",
        "unused-fixture-program",
    ]));
    let refusal: Value = serde_json::from_slice(&probe.stdout).unwrap();
    let limit = refusal["data"]["minimum_budget_bytes"].as_u64().unwrap() as usize;
    for format in ["compact", "json"] {
        for to_file in [false, true] {
            let destination = f.temp.path().join("bounded-response");
            let mut request = args(&[
                "--format",
                format,
                "run",
                "--budget-bytes",
                &limit.to_string(),
            ]);
            if to_file {
                request.extend([OsString::from("--output"), destination.clone().into()]);
            }
            request.extend(args(&["--", "unused-fixture-program"]));
            let out = f.run(&request);
            assert_eq!(out.status.code(), Some(8), "{out:?}");
            assert!(out.stderr.is_empty());
            let bytes = if to_file {
                assert!(out.stdout.is_empty());
                let bytes = fs::read(&destination).unwrap();
                fs::remove_file(destination).unwrap();
                bytes
            } else {
                out.stdout
            };
            assert!(bytes.len() <= limit, "{} > {limit}", bytes.len());
            let v: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(v["data"]["spawned"], false);
            assert_eq!(v["data"]["termination"], "not_started");
            assert!(v["data"]["child_exit_code"].is_null());
            assert!(v["data"]["output_id"].is_null());
            assert_eq!(v["project_id"], initialized["project_id"]);
            assert!(v["workspace_id"].is_null());
            assert!(
                v["truncation"]["metadata_omitted"]
                    .as_array()
                    .unwrap()
                    .contains(&serde_json::json!("workspace_id"))
            );
        }
    }
}

#[test]
fn read_final_document_budget_uses_emitted_format_and_preserves_excerpt_truth() {
    for source in ["\u{202e}".repeat(12000), "a".repeat(48000)] {
        let f = Fixture::new();
        fs::write(f.temp.path().join("body.txt"), &source).unwrap();
        assert!(f.run(&args(&["init"])).status.success());
        for format in ["compact", "json", "markdown"] {
            for (to_file, require_complete) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                let destination = f.temp.path().join("response");
                let mut request = args(&["--format", format, "read", "body.txt"]);
                if require_complete {
                    request.push(OsString::from("--require-complete"));
                }
                if to_file {
                    request.extend([OsString::from("--output"), destination.clone().into()]);
                }
                let out = f.run(&request);
                let bytes = if to_file {
                    assert!(out.stdout.is_empty());
                    fs::read(&destination).unwrap()
                } else {
                    out.stdout.clone()
                };
                assert!(out.stderr.is_empty(), "{out:?}");
                assert!(bytes.len() <= 65536, "{format}: {}", bytes.len());
                let delivered = Output {
                    stdout: bytes,
                    ..out
                };
                let v = outline_document(&delivered, format);
                for field in [
                    "schema_version",
                    "command",
                    "status",
                    "project_id",
                    "workspace_id",
                    "generation_id",
                    "validation",
                    "coverage",
                    "data",
                    "truncation",
                    "warnings",
                    "errors",
                ] {
                    assert!(v.get(field).is_some(), "missing {field}: {v}");
                }
                assert_eq!(v["schema_version"], "1.0");
                assert_eq!(v["command"], "read");
                assert_eq!(v["validation"]["workspace_atomic"], false);
                assert!(v["validation"]["scope"].is_array());
                assert!(v["validation"]["checked_at"].is_string());
                assert!(v["coverage"]["reasons"].is_array());
                assert!(v["warnings"].is_array());
                assert!(v["errors"].is_array());
                let text = v["data"]["text"].as_str().expect("retained source excerpt");
                assert!(!text.is_empty());
                assert!(source.starts_with(text));
                assert_eq!(
                    v["data"]["file_hash"],
                    pctx::domain::hash(source.as_bytes())
                );
                assert!(v["project_id"].is_string());
                assert!(v["workspace_id"].is_string());
                if text.len() < source.len() {
                    assert_eq!(delivered.status.code(), Some(3), "{v}");
                    assert_eq!(
                        v["status"],
                        if require_complete { "error" } else { "partial" }
                    );
                    if require_complete {
                        assert_eq!(v["errors"][0]["code"], "PARTIAL_RESULT");
                    }
                    assert_eq!(v["data"]["returned_excerpt"], true);
                    assert_eq!(v["data"]["truncated"], true);
                    assert_eq!(v["coverage"]["status"], "partial");
                    assert_eq!(v["truncation"]["truncated"], true);
                } else {
                    assert_eq!(delivered.status.code(), Some(0), "{v}");
                    assert_eq!(v["status"], "ok");
                }
                if to_file {
                    fs::remove_file(destination).unwrap();
                }
            }
        }
    }
}

#[test]
fn outline_empty_unsupported_and_partial_are_distinct_in_every_representation() {
    let f = Fixture::new();
    fs::create_dir(f.temp.path().join("project")).unwrap();
    let invoke = |values: Vec<OsString>| {
        let mut options = args(&["--root", "project"]);
        options.extend(values);
        f.run(&options)
    };
    assert!(invoke(args(&["--format", "json", "init"])).status.success());
    fs::create_dir(f.temp.path().join("project/mixed")).unwrap();
    fs::write(
        f.temp.path().join("project/empty.py"),
        "# no declarations\n",
    )
    .unwrap();
    fs::write(
        f.temp.path().join("project/unsupported.rb"),
        "puts 'ordinary text'\n",
    )
    .unwrap();
    fs::write(
        f.temp.path().join("project/partial.py"),
        "def unfinished(\n",
    )
    .unwrap();
    fs::write(
        f.temp.path().join("project/mixed/empty.py"),
        "# no declarations\n",
    )
    .unwrap();
    fs::write(
        f.temp.path().join("project/mixed/unsupported.rb"),
        "puts 1\n",
    )
    .unwrap();
    let update = invoke(args(&["--format", "json", "index", "update"]));
    assert!(update.status.success(), "{update:?}");
    for format in ["json", "compact", "markdown"] {
        for (path, exit, status, coverage) in [
            ("empty.py", 0, "ok", "complete"),
            ("unsupported.rb", 6, "error", "unsupported"),
            ("partial.py", 3, "partial", "partial"),
            ("mixed", 3, "partial", "partial"),
        ] {
            let out = invoke(args(&[
                "--format",
                format,
                "outline",
                path,
                "--freshness",
                "strict",
            ]));
            assert!(out.stderr.is_empty(), "{out:?}");
            let v = outline_document(&out, format);
            assert_eq!(out.status.code(), Some(exit), "{format}/{path}: {v}");
            assert_eq!(v["status"], status);
            assert_eq!(v["coverage"]["status"], coverage);
            assert_eq!(v["validation"]["mode"], "strict");
            assert!(v["project_id"].is_string() && v["workspace_id"].is_string());
            let files = v["data"]["files"].as_array().unwrap();
            assert_eq!(files.len(), if path == "mixed" { 2 } else { 1 });
            if path == "empty.py" {
                assert_eq!(files[0]["symbols"], serde_json::json!([]));
                assert_eq!(files[0]["coverage"]["status"], "complete");
                assert_eq!(v["errors"], serde_json::json!([]));
            } else if path == "unsupported.rb" {
                assert_eq!(files[0]["parse_status"], "unsupported");
                assert_eq!(files[0]["symbols"], serde_json::json!([]));
                assert_eq!(v["errors"][0]["code"], "CAPABILITY_UNAVAILABLE");
            } else if path == "mixed" {
                assert!(
                    files
                        .iter()
                        .any(|f| f["coverage"]["status"] == "unsupported")
                );
                assert!(files.iter().any(|f| f["coverage"]["status"] == "complete"));
            } else {
                assert_eq!(files[0]["parse_status"], "partial");
            }
        }
    }
    // Text search in an unsupported structural language remains valid.
    let out = invoke(args(&[
        "--format",
        "json",
        "find",
        "absent_fixture_word",
        "--kind",
        "text",
        "--scope",
        "unsupported.rb",
        "--freshness",
        "strict",
    ]));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["data"]["items"], serde_json::json!([]));
}

#[test]
fn partial_refresh_does_not_claim_the_requested_outline_is_wholly_unsupported() {
    let f = Fixture::new();
    fs::create_dir(f.temp.path().join("project")).unwrap();
    let invoke = |values: Vec<OsString>| {
        let mut options = args(&["--root", "project"]);
        options.extend(values);
        f.run(&options)
    };
    assert!(invoke(args(&["--format", "json", "init"])).status.success());
    fs::create_dir(f.temp.path().join("project/scope")).unwrap();
    fs::write(
        f.temp.path().join("project/scope/unsupported.rb"),
        "puts 1\n",
    )
    .unwrap();
    fs::File::create(f.temp.path().join("project/scope/oversized.py"))
        .unwrap()
        .set_len(1_048_577)
        .unwrap();
    for format in ["json", "compact", "markdown"] {
        let out = invoke(args(&[
            "--format",
            format,
            "outline",
            "scope",
            "--freshness",
            "strict",
        ]));
        assert!(out.stderr.is_empty(), "{out:?}");
        let v = outline_document(&out, format);
        assert_eq!(out.status.code(), Some(3), "{v}");
        assert_eq!(v["status"], "partial");
        assert_eq!(v["coverage"]["status"], "partial");
        assert_eq!(v["data"]["coverage"]["status"], "partial");
        assert!(
            v["data"]["coverage"]["refresh_reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "FILE_TOO_LARGE")
        );
        let files = v["data"]["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0]["coverage"]["status"], "unsupported");
        assert_eq!(v["errors"], serde_json::json!([]));
    }
}

#[test]
fn saved_output_invalid_arguments_refuse_before_project_and_response_access() {
    let cases = vec![
        vec!["output", "show", "../invalid"],
        vec!["output", "find", "invalid", "--literal", "x"],
        vec!["output", "render", "invalid"],
        vec![
            "output",
            "show",
            "OUT-fixture",
            "--view",
            "full",
            "--lines",
            "nonsense",
        ],
        vec![
            "output",
            "show",
            "OUT-fixture",
            "--view",
            "full",
            "--lines",
            "0:1",
        ],
        vec![
            "output",
            "show",
            "OUT-fixture",
            "--view",
            "full",
            "--lines",
            "2:1",
        ],
        vec![
            "output",
            "show",
            "OUT-fixture",
            "--view",
            "full",
            "--lines",
            "1:1002",
        ],
        vec!["output", "show", "OUT-fixture", "--lines", "1:2"],
        vec!["output", "show", "OUT-fixture", "--lines", "nonsense"],
        vec!["output", "show", "OUT-fixture", "--stream", "stdout"],
        vec!["output", "find", "OUT-fixture", "--literal", ""],
        vec![
            "output",
            "find",
            "OUT-fixture",
            "--literal",
            "x",
            "--limit",
            "0",
        ],
        vec![
            "output",
            "find",
            "OUT-fixture",
            "--literal",
            "x",
            "--limit",
            "1001",
        ],
    ];
    for format in ["json", "compact"] {
        for case in &cases {
            let f = Fixture::new();
            let mut values = args(&["--root", "missing", "--format", format, "--output"]);
            values.push(f.temp.path().join("response.json").into_os_string());
            values.extend(args(case));
            let out = f.run(&values);
            assert_eq!(
                out.status.code(),
                Some(2),
                "{case:?}: {}",
                String::from_utf8_lossy(&out.stdout)
            );
            argument_error(&out);
            f.unchanged();
        }
    }
}

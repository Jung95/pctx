//! Actual local Git fixtures; no repository conversion helper may run through query admission.
#![cfg(unix)]
use pctx::{deadline::Deadline, query_process};
use std::{
    fs,
    path::Path,
    process::Command,
    time::{Duration, SystemTime},
};
fn git(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_COUNT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .unwrap()
}
fn status(root: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .args([
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
            "--ignore-submodules=all",
        ])
        .current_dir(root)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap());
    command
}
#[test]
fn effective_clean_or_process_filter_declines_status_without_running_driver() {
    use std::os::unix::fs::PermissionsExt;
    for driver in ["clean", "process"] {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("project");
        fs::create_dir(&root).unwrap();
        assert!(git(&root, &["init", "--quiet"]).status.success());
        fs::write(root.join("input.txt"), "tracked\n").unwrap();
        // Older-than-index timestamps avoid racy-clean heuristics. A later fixed
        // timestamp plus equal-size bytes forces Git to compare converted content.
        let indexed_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
        fs::File::open(root.join("input.txt"))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(indexed_time))
            .unwrap();
        fs::write(root.join(".gitattributes"), "input.txt filter=fixture\n").unwrap();
        assert!(
            git(&root, &["add", "input.txt", ".gitattributes"])
                .status
                .success()
        );
        let marker = t.path().join("driver-ran");
        let helper = t.path().join("driver.sh");
        fs::write(
            &helper,
            format!(
                "#!/bin/sh\nprintf executed > '{}'\n/bin/cat\n",
                marker.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(
            git(
                &root,
                &[
                    "config",
                    &format!("filter.fixture.{driver}"),
                    helper.to_str().unwrap()
                ]
            )
            .status
            .success()
        );
        fs::write(root.join("input.txt"), "changed\n").unwrap();
        fs::File::open(root.join("input.txt"))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(indexed_time + Duration::from_secs(10)))
            .unwrap();
        assert_eq!(fs::metadata(root.join("input.txt")).unwrap().len(), 8);
        if driver == "clean" {
            // Confirm the fixture is an actual native conversion trigger, not a dead configuration.
            let native = git(&root, &["status", "--porcelain=v2", "-z"]);
            if !native.status.success() || !marker.exists() {
                let diagnostics = tempfile::Builder::new()
                    .prefix("pctx-filter-trigger-failure-")
                    .tempdir_in("/tmp")
                    .unwrap()
                    .keep();
                fs::write(diagnostics.join("stdout.bin"), &native.stdout).unwrap();
                fs::write(diagnostics.join("stderr.txt"), &native.stderr).unwrap();
                fs::write(
                    diagnostics.join("source.txt"),
                    fs::read(root.join("input.txt")).unwrap(),
                )
                .unwrap();
                panic!(
                    "Native Git did not exercise fixture filter; status={}, evidence={}",
                    native.status,
                    diagnostics.display()
                );
            }
            assert!(native.status.success());
            assert!(
                marker.exists(),
                "Native Git did not exercise the fixture filter"
            );
            fs::remove_file(&marker).unwrap();
        }
        let before = [
            fs::read(root.join("input.txt")).unwrap(),
            fs::read(root.join(".git/index")).unwrap(),
            fs::read(root.join(".git/config")).unwrap(),
        ];
        let result = query_process::output(
            status(&root),
            Deadline::from_millis(3000).unwrap(),
            1024 * 1024,
        )
        .unwrap_err();
        assert_eq!(result.code, "CAPABILITY_UNAVAILABLE");
        assert_eq!(result.exit, 6);
        assert!(
            !marker.exists(),
            "Readonly admission executed {driver} conversion"
        );
        let after = [
            fs::read(root.join("input.txt")).unwrap(),
            fs::read(root.join(".git/index")).unwrap(),
            fs::read(root.join(".git/config")).unwrap(),
        ];
        assert_eq!(before, after);
    }
}
#[test]
fn no_filter_match_is_normal_and_invalid_configuration_is_not_an_empty_observation() {
    let t = tempfile::tempdir().unwrap();
    assert!(git(t.path(), &["init", "--quiet"]).status.success());
    let output = query_process::output(
        status(t.path()),
        Deadline::from_millis(3000).unwrap(),
        1024 * 1024,
    )
    .unwrap();
    assert!(output.status.success());
    fs::write(t.path().join(".git/config"), "[broken\n").unwrap();
    let e = query_process::output(
        status(t.path()),
        Deadline::from_millis(3000).unwrap(),
        1024 * 1024,
    )
    .unwrap_err();
    assert_eq!(e.code, "SOURCE_UNAVAILABLE");
}
#[test]
fn filter_configuration_query_is_narrow_and_not_general_configuration_access() {
    let t = tempfile::tempdir().unwrap();
    assert!(git(t.path(), &["init", "--quiet"]).status.success());
    let mut allowed = Command::new("git");
    allowed
        .args(["config", "--null", "--get-regexp", r"^filter\."])
        .current_dir(t.path());
    assert_eq!(
        query_process::output(allowed, Deadline::from_millis(3000).unwrap(), 65536)
            .unwrap()
            .status
            .code(),
        Some(1)
    );
    let mut denied = Command::new("git");
    denied
        .args(["config", "user.name", "mutation"])
        .current_dir(t.path());
    assert_eq!(
        query_process::output(denied, Deadline::from_millis(3000).unwrap(), 65536)
            .unwrap_err()
            .code,
        "POLICY_DENIED"
    );
}

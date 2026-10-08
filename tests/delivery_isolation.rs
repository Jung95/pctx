//! PCTX01 test-harness ownership: a timed-out worker cannot leave its enrolled CLI alive.
#![cfg(unix)]
#[allow(dead_code)]
#[path = "support/delivery.rs"]
mod delivery;
use std::{
    fs,
    os::unix::process::ExitStatusExt,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const CASE: &str = "owned_worker_timeout_kills_enrolled_cli";

#[test]
fn owned_worker_timeout_kills_enrolled_cli() {
    if delivery::is_worker(CASE) {
        let marker = std::path::PathBuf::from(std::env::var_os("PCTX_TEST_NESTED_PID").unwrap());
        let base = marker.parent().unwrap();
        let root = base.join("project");
        let data = base.join("data");
        fs::create_dir(&root).unwrap();
        let invoke = |args: &[&str]| {
            Command::new(env!("CARGO_BIN_EXE_pctx"))
                .args(["--root", root.to_str().unwrap(), "--format", "json"])
                .args(args)
                .env("PCTX_DATA_DIR", &data)
                .env("PCTX_USER_CONFIG", base.join("absent-config"))
                .output()
                .unwrap()
        };
        let init = invoke(&["init"]);
        assert!(init.status.success(), "{init:?}");
        let agent = invoke(&["agent", "register", "--name", "fixture"]);
        assert!(agent.status.success(), "{agent:?}");
        let agent: serde_json::Value = serde_json::from_slice(&agent.stdout).unwrap();
        let agent = agent["data"]["agent_id"].as_str().unwrap();
        // The CLI's read/write FIFO stays open even if the worker dies.
        // Root-only cancellation therefore cannot masquerade as nested cleanup.
        let input = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(base.join("held-input"))
            .unwrap();
        let mut cli = Command::new(env!("CARGO_BIN_EXE_pctx"))
            .args([
                "--root",
                root.to_str().unwrap(),
                "adapter",
                "claude",
                "event",
                "--agent",
                agent,
                "--hook",
            ])
            .env("PCTX_DATA_DIR", &data)
            .env("PCTX_USER_CONFIG", base.join("absent-config"))
            .stdin(Stdio::from(input))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        assert!(cli.try_wait().unwrap().is_none());
        let group = unsafe { libc::getpgid(cli.id() as i32) };
        assert_eq!(group, unsafe { libc::getpgrp() });
        fs::write(&marker, format!("{} {group}", cli.id())).unwrap();
        loop {
            assert!(
                cli.try_wait().unwrap().is_none(),
                "Held-stdin CLI exited before cancellation"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let marker = temp.path().join("nested-pid");
    let fifo = std::ffi::CString::new(
        temp.path()
            .join("held-input")
            .as_os_str()
            .as_encoded_bytes(),
    )
    .unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let result = delivery::run_worker_observed(
        CASE,
        Duration::from_secs(5),
        &[("PCTX_TEST_NESTED_PID", &marker)],
        |worker_pid| {
            let values: Vec<i32> = fs::read_to_string(&marker)
                .unwrap()
                .split_whitespace()
                .map(|v| v.parse().unwrap())
                .collect();
            let pid = values[0];
            assert_eq!(
                values[1], worker_pid as i32,
                "Nested CLI must be enrolled in owned group"
            );
            let start = Instant::now();
            loop {
                if unsafe { libc::kill(pid, 0) } == -1 {
                    assert_eq!(
                        std::io::Error::last_os_error().raw_os_error(),
                        Some(libc::ESRCH)
                    );
                    break;
                }
                if start.elapsed() >= Duration::from_secs(2) {
                    // Preserve the failed assertion while cleaning the deliberately held
                    // owned group while its leader PID is still reserved, including
                    // during the guard-loss experiment.
                    assert_eq!(unsafe { libc::getpgid(pid) }, worker_pid as i32);
                    assert_eq!(
                        unsafe { libc::kill(-(worker_pid as i32), libc::SIGKILL) },
                        0
                    );
                    panic!(
                        "Nested CLI remains present after worker cleanup; owned group cancelled"
                    );
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        },
    );
    assert!(result.timed_out, "{:?}", result.output);
    assert_eq!(result.output.status.signal(), Some(libc::SIGKILL));
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(result.worker_pid as i32, &mut status, libc::WNOHANG) },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD),
        "Worker must already be reaped"
    );
}

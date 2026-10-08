//! Actual context input waits share the request deadline and cannot publish an index.
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
    database: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let data = temp.path().join("data");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("code.py"), "def needle():\n    return 1\n").unwrap();
        let mut f = Self {
            _temp: temp,
            root,
            data,
            database: PathBuf::new(),
        };
        for args in [["init"].as_slice(), ["index", "update"].as_slice()] {
            let o = f.command(args).output().unwrap();
            assert!(o.status.success(), "{}", diagnostics(&o));
        }
        f.database = fs::read_dir(f.data.join("workspaces"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
            .join("index.sqlite3");
        f
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.args(["--root", self.root.to_str().unwrap(), "--format", "json"])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_ACTOR", "owner")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env(
                "GIT_CONFIG_SYSTEM",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env_remove("PCTX_RUN_CAPABILITY");
        c
    }
    fn generation(&self) -> String {
        rusqlite::Connection::open(&self.database)
            .unwrap()
            .query_row("SELECT active_generation_id FROM workspace_meta", [], |r| {
                r.get(0)
            })
            .unwrap()
    }
}
fn diagnostics(o: &Output) -> String {
    format!(
        "status={} stdout={} stderr={}",
        o.status,
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}
struct Supervised(Option<Child>);
impl Supervised {
    fn finish(&mut self) -> Output {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.0.as_mut().unwrap().try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::yield_now();
        }
        if self.0.as_mut().unwrap().try_wait().unwrap().is_none() {
            self.0.as_mut().unwrap().kill().unwrap();
        }
        self.0.take().unwrap().wait_with_output().unwrap()
    }
}
impl Drop for Supervised {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            if let Ok(output) = child.wait_with_output()
                && std::thread::panicking()
            {
                eprintln!("Input fixture cleanup: {}", diagnostics(&output));
            }
        }
    }
}

#[test]
fn held_open_task_stdin_times_out_without_index_publication() {
    let f = Fixture::new();
    let generation = f.generation();
    let code = fs::read(f.root.join("code.py")).unwrap();
    let child = f
        .command(&["build", "--task-file", "-", "--timeout-ms", "400"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut process = Supervised(Some(child));
    // Nonempty input is insufficient without EOF. Keep the writer owned and
    // open until after the child exits: no delay/closing pipe makes it pass.
    let mut writer = process.0.as_mut().unwrap().stdin.take().unwrap();
    writer.write_all(b"Find needle\n").unwrap();
    let output = process.finish();
    let details = diagnostics(&output);
    assert_eq!(output.status.code(), Some(7), "{details}");
    let response: Value = serde_json::from_slice(&output.stdout).expect(&details);
    assert_eq!(response["errors"][0]["code"], "TIMEOUT", "{details}");
    assert_eq!(
        f.generation(),
        generation,
        "Input timeout published a replacement index"
    );
    assert_eq!(fs::read(f.root.join("code.py")).unwrap(), code);
    drop(writer);
}

#[cfg(unix)]
#[test]
fn fifo_task_path_is_rejected_without_waiting_for_a_writer_or_publishing() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let f = Fixture::new();
    let generation = f.generation();
    let fifo = f._temp.path().join("task.pipe");
    let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let child = f
        .command(&[
            "build",
            "--task-file",
            fifo.to_str().unwrap(),
            "--timeout-ms",
            "400",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = Supervised(Some(child)).finish();
    let details = diagnostics(&output);
    assert_eq!(output.status.code(), Some(2), "{details}");
    let response: Value = serde_json::from_slice(&output.stdout).expect(&details);
    assert_eq!(
        response["errors"][0]["code"], "INVALID_ARGUMENT",
        "{details}"
    );
    assert_eq!(f.generation(), generation);
}

#[test]
fn task_file_reader_preserves_utf8_and_rejects_oversize_invalid_and_expired_inputs() {
    use pctx::{
        deadline::Deadline,
        input::{MAX_TASK_BYTES, task_document},
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("task.txt");
    let text = "Find needle — 한글 🦀\r\n";
    fs::write(&path, text).unwrap();
    assert_eq!(
        task_document(path.to_str().unwrap(), Deadline::from_millis(1000).unwrap()).unwrap(),
        text
    );
    fs::write(&path, [0xff]).unwrap();
    assert_eq!(
        task_document(path.to_str().unwrap(), Deadline::from_millis(1000).unwrap())
            .unwrap_err()
            .code,
        "UNSUPPORTED_ENCODING"
    );
    fs::write(&path, vec![b'x'; MAX_TASK_BYTES + 1]).unwrap();
    assert_eq!(
        task_document(path.to_str().unwrap(), Deadline::from_millis(1000).unwrap())
            .unwrap_err()
            .code,
        "FILE_TOO_LARGE"
    );
    fs::write(&path, vec![b'x'; MAX_TASK_BYTES]).unwrap();
    assert_eq!(
        task_document(path.to_str().unwrap(), Deadline::from_millis(1000).unwrap())
            .unwrap()
            .len(),
        MAX_TASK_BYTES
    );
    let deadline = Deadline::from_millis(1).unwrap();
    while deadline.check().is_ok() {
        std::hint::spin_loop();
    }
    assert_eq!(
        task_document(temp.path().join("absent").to_str().unwrap(), deadline)
            .unwrap_err()
            .code,
        "TIMEOUT"
    );
}

#[cfg(unix)]
#[test]
fn unix_stdin_flags_probe() {
    if std::env::var_os("PCTX_TASK_INPUT_PROBE").is_none() {
        return;
    }
    let before = unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_GETFL) };
    assert!(before >= 0);
    let error =
        pctx::input::task_document("-", pctx::deadline::Deadline::from_millis(100).unwrap())
            .unwrap_err();
    assert_eq!(error.code, "TIMEOUT");
    let after = unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_GETFL) };
    assert_eq!(after, before, "Shared stdin flags were not restored");
    fs::write(
        std::env::var_os("PCTX_TASK_INPUT_PROBE").unwrap(),
        "restored",
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn stdin_flag_restoration_is_observed_in_a_separate_process() {
    let temp = tempfile::tempdir().unwrap();
    let proof = temp.path().join("restoration-proof");
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "unix_stdin_flags_probe", "--nocapture"])
        .env("PCTX_TASK_INPUT_PROBE", &proof)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut process = Supervised(Some(child));
    let writer = process.0.as_mut().unwrap().stdin.take().unwrap();
    let output = process.finish();
    assert!(output.status.success(), "{}", diagnostics(&output));
    assert_eq!(fs::read_to_string(proof).unwrap(), "restored");
    drop(writer);
}

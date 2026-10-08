use fs2::FileExt;
use pctx::{
    deadline::Deadline,
    project::{Config, Project, ProjectConfig, RootAnchor, detect_root_with_deadline},
    query_process,
};
use std::{
    fs,
    process::Command,
    time::{Duration, Instant},
};

fn fixture() -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap().join("project");
    fs::create_dir(&root).unwrap();
    let data = temp.path().join("data");
    let workspace = data.join("workspace");
    let control = data.join("control");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&control).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data,
        workspace_dir: workspace,
        control_dir: control,
        project_id: uuid::Uuid::new_v4().to_string(),
        workspace_id: "workspace".into(),
        coordination_id: "coordination".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: uuid::Uuid::new_v4().to_string(),
                name: "deadline fixture".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    (temp, p)
}
fn lock(path: &std::path::Path) -> fs::File {
    let f = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    f.lock_exclusive().unwrap();
    f
}
#[test]
fn expired_loading_and_detection_do_not_touch_nonexistent_project() {
    let t = tempfile::tempdir().unwrap();
    let absent = t.path().join("absent");
    let d = Deadline::from_millis(1).unwrap();
    std::thread::sleep(Duration::from_millis(5));
    assert_eq!(
        detect_root_with_deadline(Some(&absent), Some(d))
            .unwrap_err()
            .code,
        "TIMEOUT"
    );
    assert_eq!(
        Project::open_with_deadline(&absent, Some(d))
            .unwrap_err()
            .code,
        "TIMEOUT"
    );
    assert!(!absent.exists());
}
#[test]
fn connection_initialization_wait_respects_remaining_request_not_five_seconds() {
    let (_t, mut p) = fixture();
    let _owner = lock(&p.workspace_dir.join("connection-init.lock"));
    p.deadline = Some(Deadline::from_millis(100).unwrap());
    let started = Instant::now();
    assert_eq!(p.connect(false).unwrap_err().code, "TIMEOUT");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!p.index_db().exists());
}
#[test]
fn successive_database_connections_share_the_original_request_budget() {
    let (_t, mut p) = fixture();
    let first = lock(&p.workspace_dir.join("connection-init.lock"));
    let _second = lock(&p.control_dir.join("connection-init.lock"));
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(70));
        drop(first);
    });
    p.deadline = Some(Deadline::from_millis(250).unwrap());
    let started = Instant::now();
    p.connect(false).unwrap();
    release.join().unwrap();
    assert_eq!(p.connect(true).unwrap_err().code, "TIMEOUT");
    assert!(started.elapsed() >= Duration::from_millis(230));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!p.control_db().exists());
}
#[test]
fn expired_connection_does_not_even_create_initialization_lock() {
    let (_t, mut p) = fixture();
    p.deadline = Some(Deadline::from_millis(1).unwrap());
    std::thread::sleep(Duration::from_millis(5));
    assert_eq!(p.connect(false).unwrap_err().code, "TIMEOUT");
    assert!(!p.workspace_dir.join("connection-init.lock").exists());
    assert!(!p.index_db().exists());
}
#[test]
fn busy_sqlite_read_is_bounded_by_remaining_request() {
    let (_t, mut p) = fixture();
    let blocker = rusqlite::Connection::open(p.index_db()).unwrap();
    blocker
        .execute_batch("CREATE TABLE fixture(value TEXT); BEGIN EXCLUSIVE;")
        .unwrap();
    p.deadline = Some(Deadline::from_millis(100).unwrap());
    let started = Instant::now();
    let e = p.connect(false).unwrap_err();
    assert!(
        matches!(e.code.as_str(), "TIMEOUT" | "INDEX_BUSY"),
        "{}",
        e.code
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    blocker.execute_batch("ROLLBACK;").unwrap();
}
#[test]
fn query_supervisor_rejects_mutation_before_spawn() {
    let mut c = Command::new("git");
    c.arg("init");
    assert_eq!(
        query_process::output(c, Deadline::from_millis(1000).unwrap(), 4096)
            .unwrap_err()
            .code,
        "POLICY_DENIED"
    );
}
#[cfg(unix)]
fn fake_git(script: &str) -> (tempfile::TempDir, Command) {
    use std::os::unix::fs::PermissionsExt;
    let t = tempfile::tempdir().unwrap();
    let executable = t.path().join("git");
    fs::write(&executable, format!("#!/bin/sh\n{script}")).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let mut c = Command::new(executable);
    c.args(["rev-parse", "--is-inside-work-tree"]).env_clear();
    (t, c)
}
#[cfg(unix)]
fn preserve_eof_failure(root: &std::path::Path, payload: &serde_json::Value) -> std::path::PathBuf {
    let diagnostics = tempfile::Builder::new()
        .prefix("pctx-query-eof-failure-")
        .tempdir_in("/tmp")
        .unwrap()
        .keep();
    fs::write(
        diagnostics.join("observation.json"),
        serde_json::to_vec_pretty(payload).unwrap(),
    )
    .unwrap();
    for name in [
        "root.pid",
        "descendant.pid",
        "descendant.ready",
        "root.exiting",
        "root.started",
        "git",
    ] {
        if let Ok(bytes) = fs::read(root.join(name)) {
            fs::write(diagnostics.join(name), bytes).unwrap();
        }
    }
    diagnostics
}
#[cfg(unix)]
#[test]
fn query_deadline_includes_inherited_pipe_eof_after_root_exit() {
    let (t, mut c) = fake_git(
        "printf '%s' $$ > root.pid\n(printf descendant-ready > descendant.ready; exec /bin/sleep 60) &\nprintf '%s' $! > descendant.pid\nwhile [ ! -f descendant.ready ]; do /bin/sleep 0.005; done\nprintf root-exiting > root.exiting\nprintf root\nexit 0\n",
    );
    c.current_dir(t.path()).env("PATH", "/usr/bin:/bin");
    // This is a lifecycle fixture allowance, not a product latency acceptance.
    // The native run expired at the old 200ms cutoff before descendant.pid was
    // created, so that run could not certify the intended EOF phase. Reserve startup time and observe readiness
    // without resetting the original deadline or loosening the EOF requirement.
    let deadline = Deadline::from_millis(3000).unwrap();
    let started = Instant::now();
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        sender
            .send(query_process::output(c, deadline, 4096))
            .unwrap();
    });
    let mut root_exit_observed = false;
    let mut descendant_live_at_root_exit = false;
    let mut phases = Vec::new();
    let result = loop {
        if let Ok(result) = receiver.try_recv() {
            break result;
        }
        if !root_exit_observed
            && t.path().join("root.exiting").exists()
            && let (Ok(root), Ok(descendant)) = (
                fs::read_to_string(t.path().join("root.pid")),
                fs::read_to_string(t.path().join("descendant.pid")),
            )
            && let (Ok(root), Ok(descendant)) =
                (root.parse::<libc::id_t>(), descendant.parse::<i32>())
        {
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            // Observe exit while deliberately retaining the root PID. WNOWAIT
            // cannot consume the query supervisor's child status or permit PGID reuse.
            let rc = unsafe {
                libc::waitid(
                    libc::P_PID,
                    root,
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if rc == 0
                && info.si_signo != 0
                && info.si_code == libc::CLD_EXITED
                && deadline.check().is_ok()
            {
                root_exit_observed = true;
                descendant_live_at_root_exit = unsafe { libc::kill(descendant, 0) } == 0
                    && t.path().join("descendant.ready").exists();
                phases.push(format!(
                    "root exit observed at {}ms, descendant alive={descendant_live_at_root_exit}",
                    started.elapsed().as_millis()
                ));
            } else if rc != 0 {
                phases.push(format!(
                    "root observation: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        if started.elapsed() > Duration::from_secs(6) {
            let payload = serde_json::json!({"result":"supervisor_did_not_return","elapsed_ms":started.elapsed().as_millis(),"root_exit_observed":root_exit_observed,"descendant_live_at_root_exit":descendant_live_at_root_exit,"phases":phases});
            let diagnostics = preserve_eof_failure(t.path(), &payload);
            panic!(
                "Query supervisor exceeded deadline plus cleanup grace; evidence at {}",
                diagnostics.display()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    worker.join().unwrap();
    let valid_timeout = result.as_ref().is_err_and(|e| e.code == "TIMEOUT");
    if !valid_timeout
        || !root_exit_observed
        || !descendant_live_at_root_exit
        || started.elapsed() >= Duration::from_secs(5)
    {
        let payload = serde_json::json!({"result":format!("{result:?}"),"elapsed_ms":started.elapsed().as_millis(),"root_exit_observed":root_exit_observed,"descendant_live_at_root_exit":descendant_live_at_root_exit,"phases":phases});
        let diagnostics = preserve_eof_failure(t.path(), &payload);
        panic!(
            "Inherited EOF fixture did not reach/complete its required phase; actual evidence at {}: {payload}",
            diagnostics.display()
        );
    }
    assert!(root_exit_observed, "Root exit must precede timeout");
    assert!(
        descendant_live_at_root_exit,
        "A real descendant must hold inherited pipes after root exit"
    );
    assert_eq!(result.unwrap_err().code, "TIMEOUT");
    let pid: i32 = fs::read_to_string(t.path().join("descendant.pid"))
        .unwrap()
        .parse()
        .unwrap();
    // The real child inherited both pipes. Timeout kills the isolated process group.
    let limit = Instant::now() + Duration::from_secs(1);
    loop {
        let absent = unsafe { libc::kill(pid, 0) } == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH);
        #[cfg(target_os = "linux")]
        let zombie = fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|s| {
            s.rsplit_once(") ")
                .is_some_and(|(_, tail)| tail.starts_with('Z'))
        });
        #[cfg(not(target_os = "linux"))]
        let zombie = false;
        if absent || zombie {
            break;
        }
        if Instant::now() >= limit {
            let payload = serde_json::json!({"result":"descendant_remained_alive_after_timeout","pid":pid,"elapsed_ms":started.elapsed().as_millis()});
            let diagnostics = preserve_eof_failure(t.path(), &payload);
            panic!(
                "Inherited pipe child remained alive; evidence at {}",
                diagnostics.display()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[cfg(unix)]
#[test]
fn query_cannot_succeed_with_live_descendant_that_closed_both_streams() {
    let (t, mut c) = fake_git(
        "printf '%s' $$ > root.pid\n(printf descendant-ready > descendant.ready; exec /bin/sleep 60) </dev/null >/dev/null 2>&1 &\nprintf '%s' $! > descendant.pid\nwhile [ ! -f descendant.ready ]; do /bin/sleep 0.005; done\nprintf root-exiting > root.exiting\nprintf root\nexit 0\n",
    );
    c.current_dir(t.path()).env("PATH", "/usr/bin:/bin");
    // Exact fixture identities are cleaned on assertion failure as well as success.
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let identities = (
                fs::read_to_string(self.0.join("root.pid")),
                fs::read_to_string(self.0.join("descendant.pid")),
            );
            if let (Ok(root), Ok(child)) = identities
                && let (Ok(root), Ok(child)) = (root.parse::<i32>(), child.parse::<i32>())
                && unsafe { libc::getpgid(child) } == root
            {
                unsafe {
                    libc::kill(child, libc::SIGKILL);
                }
            }
        }
    }
    let _cleanup = Cleanup(t.path().to_path_buf());
    let deadline = Deadline::from_millis(3000).unwrap();
    let started = Instant::now();
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = sender.send(query_process::output(c, deadline, 4096));
    });
    let mut root_exit_with_live_child = false;
    let result = loop {
        if let Ok(result) = receiver.try_recv() {
            break result;
        }
        if t.path().join("root.exiting").exists()
            && let (Ok(root), Ok(child)) = (
                fs::read_to_string(t.path().join("root.pid")),
                fs::read_to_string(t.path().join("descendant.pid")),
            )
            && let (Ok(root), Ok(child)) = (root.parse::<libc::id_t>(), child.parse::<i32>())
        {
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let rc = unsafe {
                libc::waitid(
                    libc::P_PID,
                    root,
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if rc == 0
                && info.si_signo != 0
                && info.si_code == libc::CLD_EXITED
                && unsafe { libc::kill(child, 0) } == 0
                && deadline.check().is_ok()
            {
                root_exit_with_live_child = true;
            }
        }
        if started.elapsed() > Duration::from_secs(6) {
            let evidence = preserve_eof_failure(
                t.path(),
                &serde_json::json!({
                "phase":"closed_streams_supervisor_stuck", "root_exit_with_live_child":root_exit_with_live_child}),
            );
            panic!(
                "Closed-stream descendant supervisor stuck: {}",
                evidence.display()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    worker.join().unwrap();
    if !root_exit_with_live_child || !result.as_ref().is_err_and(|e| e.code == "TIMEOUT") {
        let evidence = preserve_eof_failure(
            t.path(),
            &serde_json::json!({
            "result":format!("{result:?}"), "root_exit_with_live_child":root_exit_with_live_child,
            "elapsed_ms":started.elapsed().as_millis()}),
        );
        panic!(
            "Live closed-stream child must prevent success: {}",
            evidence.display()
        );
    }
    let child: i32 = fs::read_to_string(t.path().join("descendant.pid"))
        .unwrap()
        .parse()
        .unwrap();
    let limit = Instant::now() + Duration::from_secs(1);
    loop {
        let absent = unsafe { libc::kill(child, 0) } == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH);
        #[cfg(target_os = "linux")]
        let zombie = fs::read_to_string(format!("/proc/{child}/stat")).is_ok_and(|s| {
            s.rsplit_once(") ")
                .is_some_and(|(_, tail)| tail.starts_with('Z'))
        });
        #[cfg(not(target_os = "linux"))]
        let zombie = false;
        if absent || zombie {
            break;
        }
        if Instant::now() >= limit {
            let evidence = preserve_eof_failure(
                t.path(),
                &serde_json::json!({
                "phase":"closed_streams_child_survived_cancel", "child":child}),
            );
            panic!(
                "Closed-stream child survived cancellation: {}",
                evidence.display()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(unix)]
#[test]
fn simultaneous_streams_are_collected_and_overflow_never_infers_success() {
    let (t, mut c) = fake_git(
        "printf '%s' $$ > root.pid\nprintf root-started > root.started\nprintf stdout\nprintf stderr >&2\nprintf root-exiting > root.exiting\n",
    );
    c.current_dir(t.path());
    let o =
        query_process::output(c, Deadline::from_millis(1000).unwrap(), 64).unwrap_or_else(|e| {
            let payload = serde_json::json!({"error_code":e.code,"diagnostic":e.message,
                "root_started":t.path().join("root.started").exists(),
                "root_exiting":t.path().join("root.exiting").exists()});
            let evidence = preserve_eof_failure(t.path(), &payload);
            panic!(
                "Simple builtin query failed; evidence at {}: {payload}",
                evidence.display()
            );
        });
    assert!(o.status.success());
    assert_eq!(o.stdout, b"stdout");
    assert_eq!(o.stderr, b"stderr");
    let (_t, c) = fake_git("printf '1234567890'\n");
    assert_eq!(
        query_process::output(c, Deadline::from_millis(1000).unwrap(), 5)
            .unwrap_err()
            .code,
        "PARTIAL_RESULT"
    );
}

#[test]
fn deadline_project_loading_child_entry() {
    let Some(root) = std::env::var_os("PCTX_D030_LOADING_ROOT").map(std::path::PathBuf::from)
    else {
        return;
    };
    let initialized = Project::init(&root).unwrap();
    let deadline = Deadline::from_millis(2000).unwrap();
    let loaded = Project::open_with_deadline(&root, Some(deadline)).unwrap();
    assert_eq!(loaded.project_id, initialized.project_id);
    assert_eq!(loaded.deadline.unwrap().instant(), deadline.instant());
    assert!(loaded.remaining(Duration::from_secs(5)).unwrap() < Duration::from_secs(2));
    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    std::env::set_current_dir(&nested).unwrap();
    let discovered = detect_root_with_deadline(None, Some(deadline)).unwrap();
    assert_eq!(discovered, fs::canonicalize(root).unwrap());
}
#[test]
fn actual_registered_project_loading_and_ancestor_discovery_keep_same_deadline() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    fs::create_dir(&root).unwrap();
    let result = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "deadline_project_loading_child_entry",
            "--nocapture",
        ])
        .env("PCTX_D030_LOADING_ROOT", &root)
        .env("PCTX_DATA_DIR", temp.path().join("data"))
        .env(
            "PCTX_USER_CONFIG",
            temp.path().join("absent-user-config.toml"),
        )
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
#[cfg(unix)]
#[test]
fn query_program_resolution_uses_captured_command_path() {
    let (t, _c) = fake_git("printf 'captured-path'\n");
    let mut c = Command::new("git");
    c.args(["rev-parse", "--is-inside-work-tree"])
        .env("PATH", t.path());
    let result = query_process::output(c, Deadline::from_millis(1000).unwrap(), 4096).unwrap();
    assert_eq!(result.stdout, b"captured-path");
}

#[cfg(unix)]
#[test]
fn relative_query_path_uses_child_cwd_and_skips_non_executable_path_entry() {
    use std::os::unix::fs::PermissionsExt;
    let t = tempfile::tempdir().unwrap();
    let cwd = t.path().join("child");
    fs::create_dir(&cwd).unwrap();
    let first = cwd.join("first");
    let second = cwd.join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    fs::write(first.join("git"), "#!/bin/sh\nprintf wrong\n").unwrap();
    fs::set_permissions(first.join("git"), fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(
        second.join("git"),
        "#!/bin/sh\nprintf '%s' \"$$\" > query-root.pid\nprintf child-cwd\n",
    )
    .unwrap();
    fs::set_permissions(second.join("git"), fs::Permissions::from_mode(0o700)).unwrap();
    let mut command = Command::new("git");
    command
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(&cwd)
        .env("PATH", "first:second");
    let started = Instant::now();
    let result = query_process::output(command, Deadline::from_millis(1000).unwrap(), 4096);
    if let Err(error) = &result {
        let elapsed = started.elapsed();
        let evidence = t.keep();
        fs::write(
            evidence.join("failure.txt"),
            format!("{error:?}\nelapsed={elapsed:?}\n"),
        )
        .unwrap();
        panic!(
            "Relative PATH query failed; evidence={}",
            evidence.display()
        );
    }
    assert_eq!(result.unwrap().stdout, b"child-cwd");
    let mut explicit = Command::new("./second/git");
    explicit
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(cwd)
        .env("PATH", ".");
    assert_eq!(
        query_process::output(explicit, Deadline::from_millis(1000).unwrap(), 4096)
            .unwrap()
            .stdout,
        b"child-cwd"
    );
}

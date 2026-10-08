//! Finite local Git/ps, adapter version and managed schedule observations.
//! No mutation commands or execution admission bypass.
use crate::{
    deadline::Deadline,
    domain::{Error, Result},
};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

#[cfg(unix)]
use std::process::Stdio;

fn error(code: &str, message: &str) -> Error {
    Error::new(
        code,
        message,
        match code {
            "PARTIAL_RESULT" => 3,
            "POLICY_DENIED" => 5,
            "CAPABILITY_UNAVAILABLE" | "SOURCE_UNAVAILABLE" => 6,
            _ => 7,
        },
    )
}
#[cfg(unix)]
#[derive(Default)]
struct QueryObservation {
    phase: &'static str,
    spawn_ms: u128,
    root_pid: u32,
    native_child: Option<String>,
    iterations: u64,
    stdout_eof: bool,
    stderr_eof: bool,
    stdout_bytes: usize,
    stderr_bytes: usize,
    root_exit_observed: bool,
    waitid_signo: i32,
    waitid_code: i32,
    group_probes: u64,
    group_has_others: Option<bool>,
}
#[cfg(unix)]
impl QueryObservation {
    fn timeout(&self, elapsed: Duration) -> Error {
        // Counts and lifecycle flags only: never argv, paths, or captured bytes.
        error(
            "TIMEOUT",
            &format!(
                "Request deadline expired (phase={}, elapsed_ms={}, spawn_ms={}, iterations={}, stdout_eof={}, stderr_eof={}, stdout_bytes={}, stderr_bytes={}, root_exit_observed={}, waitid_signo={}, waitid_code={}, group_probes={}, group_has_others={:?}, root_pid={}, native_child={:?})",
                self.phase,
                elapsed.as_millis(),
                self.spawn_ms,
                self.iterations,
                self.stdout_eof,
                self.stderr_eof,
                self.stdout_bytes,
                self.stderr_bytes,
                self.root_exit_observed,
                self.waitid_signo,
                self.waitid_code,
                self.group_probes,
                self.group_has_others,
                self.root_pid,
                self.native_child
            ),
        )
    }
}

#[cfg(target_os = "macos")]
fn mac_child_observation(pid: u32) -> String {
    // Only our unreaped child is queried. Fixed-size native records; no command
    // names, executable paths, environment, stack, or host-process inventory.
    let mut bsd = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    let bsd_size = std::mem::size_of::<libc::proc_bsdinfo>();
    let bsd_bytes = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            bsd.as_mut_ptr().cast(),
            bsd_size as i32,
        )
    };
    if bsd_bytes != bsd_size as i32 {
        return format!(
            "bsd_unknown(bytes={bsd_bytes},errno={:?})",
            std::io::Error::last_os_error().raw_os_error()
        );
    }
    let bsd = unsafe { bsd.assume_init() };
    if bsd.pbi_pid != pid || bsd.pbi_uid != unsafe { libc::geteuid() } {
        return "bsd_identity_unknown".into();
    }
    let mut task = std::mem::MaybeUninit::<libc::proc_taskinfo>::zeroed();
    let task_size = std::mem::size_of::<libc::proc_taskinfo>();
    let task_bytes = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTASKINFO,
            0,
            task.as_mut_ptr().cast(),
            task_size as i32,
        )
    };
    let base = format!(
        "status={},flags={},pgid_matches={},ppid_matches={},start_sec={},start_usec={}",
        bsd.pbi_status,
        bsd.pbi_flags,
        bsd.pbi_pgid == pid,
        bsd.pbi_ppid == std::process::id(),
        bsd.pbi_start_tvsec,
        bsd.pbi_start_tvusec
    );
    if task_bytes != task_size as i32 {
        return format!(
            "{base},task_unknown(bytes={task_bytes},errno={:?})",
            std::io::Error::last_os_error().raw_os_error()
        );
    }
    let task = unsafe { task.assume_init() };
    format!(
        "{base},threads={},running={},cpu_user={},cpu_system={},mach_calls={},unix_calls={},context_switches={}",
        task.pti_threadnum,
        task.pti_numrunning,
        task.pti_total_user,
        task.pti_total_system,
        task.pti_syscalls_mach,
        task.pti_syscalls_unix,
        task.pti_csw
    )
}

// Native group membership is observed while the exited root remains unreaped.
// The pinned root prevents PGID reuse. This does not contain descendants that
// deliberately escape the group; only exact admitted local observations use it.
#[cfg(unix)]
fn group_has_other_members(root: u32, deadline: Deadline) -> Result<bool> {
    deadline.check()?;
    #[cfg(target_os = "macos")]
    {
        // PROC_PGRP_ONLY returns bytes, including the root zombie. A full buffer
        // is ambiguous, so never interpret truncation as an empty group.
        let mut pids = vec![0i32; 32768];
        let bytes = unsafe {
            *libc::__error() = 0;
            libc::proc_listpids(
                2,
                root,
                pids.as_mut_ptr().cast(),
                (pids.len() * std::mem::size_of::<i32>()) as i32,
            )
        };
        deadline.check()?;
        if bytes <= 0
            || bytes as usize >= pids.len() * std::mem::size_of::<i32>()
            || !(bytes as usize).is_multiple_of(std::mem::size_of::<i32>())
        {
            return Err(error(
                "SOURCE_UNAVAILABLE",
                "Query process-group membership is unknown",
            ));
        }
        let members = &pids[..bytes as usize / std::mem::size_of::<i32>()];
        if !members.contains(&(root as i32)) {
            return Err(error(
                "SOURCE_UNAVAILABLE",
                "Pinned query root missing from group observation",
            ));
        }
        Ok(members.iter().any(|pid| *pid > 0 && *pid != root as i32))
    }
    #[cfg(target_os = "linux")]
    {
        let mut root_seen = false;
        let mut other_seen = false;
        for entry in std::fs::read_dir("/proc")? {
            deadline.check()?;
            let entry = entry?;
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            let path = entry.path().join("stat");
            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => {
                    return Err(error(
                        "SOURCE_UNAVAILABLE",
                        "Query process-group membership is inaccessible",
                    ));
                }
            };
            let Some(observed) = linux_read_stat(file)? else {
                continue;
            };
            let bytes = observed;
            deadline.check()?;
            let group = linux_stat_group(&bytes)
                .ok_or_else(|| error("SOURCE_UNAVAILABLE", "Invalid process-group observation"))?;
            if group == root {
                if pid == root {
                    root_seen = true;
                } else {
                    other_seen = true;
                }
            }
        }
        if !root_seen {
            return Err(error(
                "SOURCE_UNAVAILABLE",
                "Pinned query root missing from group observation",
            ));
        }
        // Include zombies conservatively: no successful empty-group claim while
        // any other member remains visible, even if its eventual reap is delayed.
        Ok(other_seen)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = root;
        Err(error(
            "CAPABILITY_UNAVAILABLE",
            "Native query group observation is unavailable",
        ))
    }
}

#[cfg(target_os = "linux")]
fn linux_read_stat(reader: impl Read) -> Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    if let Err(e) = reader.take(8193).read_to_end(&mut bytes) {
        // An opened proc descriptor stays bound to its original process. If
        // that process is reaped during enumeration, Linux reports ESRCH.
        // The pinned root must still be observed by the caller; other errors
        // remain failures rather than evidence of an empty group.
        if e.kind() == std::io::ErrorKind::NotFound || e.raw_os_error() == Some(libc::ESRCH) {
            return Ok(None);
        }
        return Err(error(
            "SOURCE_UNAVAILABLE",
            "Query process-group membership could not be read",
        ));
    }
    Ok(Some(bytes))
}

#[cfg(target_os = "linux")]
fn linux_stat_group(bytes: &[u8]) -> Option<u32> {
    if bytes.len() > 8192 {
        return None;
    }
    // comm is arbitrary bytes (and may include spaces/parentheses). Only the
    // numeric ASCII field after its final closing delimiter is interpreted.
    let end = bytes.windows(2).rposition(|pair| pair == b") ")?;
    let group = bytes[end + 2..]
        .split(|byte| byte.is_ascii_whitespace())
        .filter(|field| !field.is_empty())
        .nth(2)?;
    if group.is_empty() || !group.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(group).ok()?.parse().ok()
}

fn executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let Ok(path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        // Match execvp's executable admission instead of selecting a non-executable
        // earlier PATH entry and masking a later usable executable.
        unsafe { libc::access(path.as_ptr(), libc::X_OK) == 0 }
    }
    #[cfg(not(unix))]
    {
        true
    }
}
fn executable(
    program: &std::ffi::OsStr,
    path_setting: &std::ffi::OsStr,
    cwd: &Path,
    deadline: Deadline,
) -> Result<PathBuf> {
    let path = Path::new(program);
    if path.components().count() > 1 {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        if executable_file(&candidate) {
            return std::fs::canonicalize(candidate).map_err(Into::into);
        }
        return Err(error(
            "SOURCE_UNAVAILABLE",
            "Local query executable unavailable",
        ));
    }
    for directory in std::env::split_paths(path_setting) {
        deadline.check()?;
        let directory = if directory.is_absolute() {
            directory
        } else {
            cwd.join(directory)
        };
        let candidate = directory.join(path);
        if executable_file(&candidate) {
            return std::fs::canonicalize(candidate).map_err(Into::into);
        }
        #[cfg(windows)]
        if path.extension().is_none() {
            let candidate = candidate.with_extension("exe");
            if executable_file(&candidate) {
                return std::fs::canonicalize(candidate).map_err(Into::into);
            }
        }
    }
    Err(error(
        "SOURCE_UNAVAILABLE",
        "Local query executable unavailable",
    ))
}
fn managed_label(value: &str) -> bool {
    value
        .strip_prefix("org.pctx.schedule.")
        .is_some_and(|suffix| {
            suffix.len() == 24
                && suffix
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}
fn managed_observation(name: &str, args: &[&str]) -> bool {
    match name {
        "launchctl" => {
            #[cfg(target_os = "macos")]
            {
                if args.len() != 2 || args[0] != "print" {
                    return false;
                }
                let parts = args[1].split('/').collect::<Vec<_>>();
                parts.len() == 3
                    && parts[0] == "gui"
                    && parts[1] == unsafe { libc::getuid() }.to_string()
                    && managed_label(parts[2])
            }
            #[cfg(not(target_os = "macos"))]
            {
                false
            }
        }
        "systemctl" => {
            cfg!(target_os = "linux")
                && args.len() == 3
                && args[0] == "--user"
                && args[1] == "is-active"
                && args[2].strip_suffix(".timer").is_some_and(managed_label)
        }
        _ => false,
    }
}
fn validate(command: &Command, cwd: &Path, deadline: Deadline) -> Result<PathBuf> {
    let name = Path::new(command.get_program())
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let args = command
        .get_args()
        .map(|a| a.to_str().unwrap_or(""))
        .collect::<Vec<_>>();
    let permitted = match name {
        "git" => {
            let mut i = 0;
            while args.get(i) == Some(&"-c") {
                if !args.get(i + 1).is_some_and(|a| {
                    [
                        "core.fsmonitor=false",
                        "core.hooksPath=/dev/null",
                        "core.hooksPath=NUL",
                        "core.untrackedCache=false",
                        "core.pager=cat",
                    ]
                    .contains(a)
                }) {
                    return Err(error("POLICY_DENIED", "Unsafe query Git configuration"));
                }
                i += 2;
            }
            match args.get(i) {
                Some(&"rev-parse") => args[i + 1..].iter().all(|a| {
                    [
                        "--show-toplevel",
                        "--path-format=absolute",
                        "--git-common-dir",
                        "--is-inside-work-tree",
                    ]
                    .contains(a)
                }),
                Some(&"config") => args[i + 1..] == ["--null", "--get-regexp", r"^filter\."],
                Some(&"status") => args[i + 1..].iter().all(|a| {
                    [
                        "--porcelain=v2",
                        "-z",
                        "--branch",
                        "--untracked-files=all",
                        "--ignore-submodules=all",
                    ]
                    .contains(a)
                }),
                _ => false,
            }
        }
        "claude" => args == ["--version"],
        "launchctl" | "systemctl" => managed_observation(name, &args),
        "ps" => {
            args.len() == 4
                && args[0] == "-p"
                && args[1].parse::<u32>().is_ok_and(|pid| pid > 0)
                && args[2] == "-o"
                && args[3] == "lstart="
        }
        _ => false,
    };
    if !permitted {
        return Err(error(
            "POLICY_DENIED",
            "Only exact local Git/ps, adapter version or managed schedule queries may use query supervision",
        ));
    }
    let configured_path = command
        .get_envs()
        .find(|(key, _)| key.to_string_lossy().eq_ignore_ascii_case("PATH"));
    let inherited_path = std::env::var_os("PATH").unwrap_or_default();
    let path = match configured_path {
        Some((_, Some(path))) => path,
        Some((_, None)) => std::ffi::OsStr::new(""),
        None => &inherited_path,
    };
    executable(command.get_program(), path, cwd, deadline)
}

#[cfg(unix)]
fn drain<P: Read + std::os::fd::AsRawFd>(
    pipe: &mut P,
    bytes: &mut Vec<u8>,
    max: usize,
    deadline: Deadline,
) -> Result<bool> {
    let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut chunk = [0; 8192];
    loop {
        deadline.check()?;
        match pipe.read(&mut chunk) {
            Ok(0) => return Ok(true),
            Ok(n) => {
                if bytes.len().saturating_add(n) > max {
                    return Err(error("PARTIAL_RESULT", "Local query exceeded output bound"));
                }
                bytes.extend_from_slice(&chunk[..n]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
}
#[cfg(windows)]
fn drain<P: Read + std::os::windows::io::AsRawHandle>(
    pipe: &mut P,
    bytes: &mut Vec<u8>,
    max: usize,
    deadline: Deadline,
) -> Result<bool> {
    use windows_sys::Win32::{
        Foundation::{ERROR_BROKEN_PIPE, GetLastError},
        System::Pipes::PeekNamedPipe,
    };
    let mut chunk = [0; 8192];
    loop {
        deadline.check()?;
        let mut available = 0;
        if unsafe {
            PeekNamedPipe(
                pipe.as_raw_handle().cast(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        } == 0
        {
            if unsafe { GetLastError() } == ERROR_BROKEN_PIPE {
                return Ok(true);
            }
            return Err(std::io::Error::last_os_error().into());
        }
        if available == 0 {
            return Ok(false);
        }
        let capacity = chunk.len().min(available as usize);
        let count = pipe.read(&mut chunk[..capacity])?;
        if count == 0 {
            return Ok(true);
        }
        if bytes.len().saturating_add(count) > max {
            return Err(error("PARTIAL_RESULT", "Local query exceeded output bound"));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
}

/// Capture one whitelisted read-only query under one deadline, including pipe EOF.
/// The caller supplies the explicit environment; inherited variables are cleared.
/// Timeout cleanup has a separate bounded one-second observation grace period.
pub fn output(command: Command, deadline: Deadline, max_bytes: usize) -> Result<Output> {
    deadline.check()?;
    if max_bytes == 0 {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Query output bound must be positive",
            2,
        ));
    }
    let cwd = std::fs::canonicalize(match command.get_current_dir() {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir()?,
    })?;
    deadline.check()?;
    let executable = validate(&command, &cwd, deadline)?;
    deadline.check()?;
    let mut environment = command
        .get_envs()
        .filter_map(|(key, value)| value.map(|v| (key.to_os_string(), v.to_os_string())))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut args = command
        .get_args()
        .map(std::ffi::OsStr::to_os_string)
        .collect::<Vec<_>>();
    if Path::new(command.get_program())
        .file_stem()
        .is_some_and(|name| name == "git")
    {
        let prefix = [
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.pager=cat",
            "-c",
            "core.untrackedCache=false",
        ];
        args.splice(0..0, prefix.into_iter().map(std::ffi::OsString::from));
        for (key, value) in [
            ("GIT_CONFIG_NOSYSTEM", "1"),
            (
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            ),
            ("GIT_CONFIG_COUNT", "0"),
            ("GIT_OPTIONAL_LOCKS", "0"),
            ("GIT_PAGER", "cat"),
        ] {
            environment.insert(key.into(), value.into());
        }
    }

    if command.get_args().any(|arg| arg == "status") {
        // Git status may run clean/process filters while refreshing tracked files.
        // Suppressing a filter would change conversion/dirty semantics, so decline
        // the status query whenever its effective configuration defines a driver.
        let mut preflight = Command::new(&executable);
        preflight
            .args(["config", "--null", "--get-regexp", r"^filter\."])
            .current_dir(&cwd)
            .env_clear()
            .envs(&environment);
        let filters = output(preflight, deadline, max_bytes)?;
        if filters.status.success() {
            return Err(error(
                "CAPABILITY_UNAVAILABLE",
                "Git status with filter drivers requires a verified readonly backend",
            ));
        }
        if filters.status.code() != Some(1)
            || !filters.stdout.is_empty()
            || !filters.stderr.is_empty()
        {
            return Err(error(
                "SOURCE_UNAVAILABLE",
                "Git filter configuration could not be verified",
            ));
        }
        deadline.check()?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let mut command = Command::new(executable);
        command
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .envs(environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Standard group setup preserves PGID=PID while allowing native
            // spawn instead of forcing fork/exec for a pre_exec callback.
            .process_group(0);
        deadline.check()?;
        let started = std::time::Instant::now();
        let mut child = command.spawn()?;
        let mut observation = QueryObservation {
            phase: "spawn_returned",
            spawn_ms: started.elapsed().as_millis(),
            root_pid: child.id(),
            ..Default::default()
        };
        let mut root_identity_retained = true;
        let result = (|| {
            let mut stdout = child
                .stdout
                .take()
                .ok_or_else(|| error("SOURCE_UNAVAILABLE", "Missing query stdout"))?;
            let mut stderr = child
                .stderr
                .take()
                .ok_or_else(|| error("SOURCE_UNAVAILABLE", "Missing query stderr"))?;
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let (mut out_eof, mut err_eof) = (false, false);
            #[cfg(target_os = "macos")]
            let mut next_native_observation = started;
            loop {
                observation.iterations = observation.iterations.saturating_add(1);
                deadline.check()?;
                #[cfg(target_os = "macos")]
                if std::time::Instant::now() >= next_native_observation {
                    observation.phase = "native_child_observation";
                    observation.native_child = Some(mac_child_observation(child.id()));
                    next_native_observation = std::time::Instant::now() + Duration::from_millis(50);
                    deadline.check()?;
                }
                if !out_eof {
                    observation.phase = "stdout_drain";
                    out_eof = drain(
                        &mut stdout,
                        &mut out,
                        max_bytes.saturating_sub(err.len()),
                        deadline,
                    )?;
                }
                observation.stdout_eof = out_eof;
                observation.stdout_bytes = out.len();
                if !err_eof {
                    observation.phase = "stderr_drain";
                    err_eof = drain(
                        &mut stderr,
                        &mut err,
                        max_bytes.saturating_sub(out.len()),
                        deadline,
                    )?;
                }
                observation.stderr_eof = err_eof;
                observation.stderr_bytes = err.len();
                observation.phase = "root_exit_observation";
                // Observe without reaping: root exit and pipe EOF alone do not
                // prove descendants with closed streams have finished.
                let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
                let rc = unsafe {
                    libc::waitid(
                        libc::P_PID,
                        child.id(),
                        &mut info,
                        libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                    )
                };
                if rc != 0 {
                    let e = std::io::Error::last_os_error();
                    if e.raw_os_error() == Some(libc::ECHILD) {
                        root_identity_retained = false;
                    }
                    return Err(error(
                        "SOURCE_UNAVAILABLE",
                        "Query root exit could not be observed",
                    ));
                }
                observation.waitid_signo = info.si_signo;
                observation.waitid_code = info.si_code;
                observation.root_exit_observed = info.si_signo != 0;
                let group_empty = if observation.root_exit_observed && out_eof && err_eof {
                    observation.phase = "group_observation";
                    observation.group_probes = observation.group_probes.saturating_add(1);
                    let others = group_has_other_members(child.id(), deadline)?;
                    observation.group_has_others = Some(others);
                    !others
                } else {
                    false
                };
                if group_empty {
                    // No further signals are allowed after this transition.
                    root_identity_retained = false;
                    let status = child.try_wait()?.ok_or_else(|| {
                        error(
                            "SOURCE_UNAVAILABLE",
                            "Observed query root could not be reaped",
                        )
                    })?;
                    return Ok(Output {
                        status,
                        stdout: out,
                        stderr: err,
                    });
                }
                observation.phase = "wait_next_observation";
                std::thread::sleep(deadline.remaining()?.min(Duration::from_millis(5)));
            }
        })();
        let result = result.map_err(|e| {
            if e.code == "TIMEOUT" {
                observation.timeout(started.elapsed())
            } else {
                e
            }
        });
        if result.is_err() && root_identity_retained {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            let grace = std::time::Instant::now() + Duration::from_secs(1);
            while matches!(child.try_wait(), Ok(None)) && std::time::Instant::now() < grace {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        result
    }
    #[cfg(windows)]
    {
        use crate::windows_process::{NativeSpawnRequest, PreparedJob};
        let mut job = PreparedJob::create()?;
        deadline.check()?;
        let mut child = job.spawn_suspended(&NativeSpawnRequest {
            executable,
            args,
            environment,
            cwd,
        })?;
        let result = collect_windows(&mut child, deadline, max_bytes);
        if result.is_err() {
            let _ = child.reject_and_observe(1, Duration::from_secs(1));
        }
        result
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (command, executable, args, environment, cwd);
        Err(error(
            "CAPABILITY_UNAVAILABLE",
            "No finite native query backend",
        ))
    }
}

#[cfg(windows)]
fn collect_windows(
    child: &mut crate::windows_process::SuspendedChild,
    deadline: Deadline,
    max_bytes: usize,
) -> Result<Output> {
    use std::os::windows::process::ExitStatusExt;
    deadline.check()?;
    child.resume_finite_query()?;
    let mut stdout = child
        .take_stdout()
        .ok_or_else(|| error("SOURCE_UNAVAILABLE", "Missing query stdout"))?;
    let mut stderr = child
        .take_stderr()
        .ok_or_else(|| error("SOURCE_UNAVAILABLE", "Missing query stderr"))?;
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let (mut out_eof, mut err_eof) = (false, false);
    let mut code = None;
    loop {
        deadline.check()?;
        if !out_eof {
            out_eof = drain(
                &mut stdout,
                &mut out,
                max_bytes.saturating_sub(err.len()),
                deadline,
            )?;
        }
        if !err_eof {
            err_eof = drain(
                &mut stderr,
                &mut err,
                max_bytes.saturating_sub(out.len()),
                deadline,
            )?;
        }
        if code.is_none() {
            code = child.wait_exit(Duration::ZERO)?;
        }
        if let Some(code) = code
            && out_eof
            && err_eof
            && child.observe_containment()
                == crate::windows_process::ContainmentObservation::EmptyProven
        {
            return Ok(Output {
                status: std::process::ExitStatus::from_raw(code),
                stdout: out,
                stderr: err,
            });
        }
        std::thread::sleep(deadline.remaining()?.min(Duration::from_millis(5)));
    }
}

#[cfg(all(test, windows))]
mod native_tests {
    use super::*;
    use crate::windows_process::{ContainmentObservation, NativeSpawnRequest, PreparedJob};
    fn request(root: &Path, entry: &str) -> NativeSpawnRequest {
        NativeSpawnRequest {
            executable: std::env::current_exe().unwrap(),
            args: vec![
                "--exact".into(),
                format!("query_process::native_tests::{entry}").into(),
                "--nocapture".into(),
            ],
            environment: std::collections::BTreeMap::from([(
                "PCTX_D030_NATIVE_FIXTURE".into(),
                "1".into(),
            )]),
            cwd: root.to_path_buf(),
        }
    }
    #[test]
    fn held_pipe_child() {
        if std::env::var_os("PCTX_D030_NATIVE_FIXTURE").is_none() {
            return;
        }
        std::fs::write("descendant-observed", "native child").unwrap();
        std::thread::sleep(Duration::from_secs(60));
    }
    #[test]
    fn root_exits_with_inherited_pipes() {
        if std::env::var_os("PCTX_D030_NATIVE_FIXTURE").is_none() {
            return;
        }
        let request = request(&std::env::current_dir().unwrap(), "held_pipe_child");
        let _child = Command::new(request.executable)
            .args(request.args)
            .env("PCTX_D030_NATIVE_FIXTURE", "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .unwrap();
        let until = std::time::Instant::now() + Duration::from_secs(3);
        while !Path::new("descendant-observed").exists() {
            assert!(std::time::Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn finite_query_timeout_observes_owned_windows_cancellation_and_pipe_eof() {
        for entry in ["held_pipe_child", "root_exits_with_inherited_pipes"] {
            let root = tempfile::tempdir().unwrap();
            let mut job = PreparedJob::create().unwrap();
            let mut child = job.spawn_suspended(&request(root.path(), entry)).unwrap();
            let started = std::time::Instant::now();
            let result = collect_windows(&mut child, Deadline::from_millis(1000).unwrap(), 65536);
            assert_eq!(result.unwrap_err().code, "TIMEOUT");
            assert_eq!(
                child.reject_and_observe(1, Duration::from_secs(2)),
                ContainmentObservation::EmptyProven
            );
            assert!(started.elapsed() < Duration::from_secs(4));
            assert!(root.path().join("descendant-observed").exists());
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod linux_group_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn stat_group_ignores_non_utf8_comm_and_rejects_invalid_group() {
        assert_eq!(
            linux_stat_group(b"42 (fixture\xff ) inner) S 1 123 0"),
            Some(123)
        );
        assert_eq!(linux_stat_group(b"42 (fixture) S 1 12x 0"), None);
        assert_eq!(linux_stat_group(b"42 (fixture) S 1"), None);
    }

    #[test]
    fn opened_stat_of_reaped_process_is_absent_but_pinned_root_is_required() {
        let mut child = Command::new("/bin/sleep").arg("60").spawn().unwrap();
        let pid = child.id();
        let path = format!("/proc/{pid}/stat");
        let mut raw = std::fs::File::open(&path).unwrap();
        let observed = std::fs::File::open(&path).unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
        let error = raw.read_to_end(&mut Vec::new()).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
        assert!(linux_read_stat(observed).unwrap().is_none());
        let error = group_has_other_members(pid, Deadline::from_millis(3000).unwrap()).unwrap_err();
        assert_eq!(error.code, "SOURCE_UNAVAILABLE");
        assert_eq!(
            error.message,
            "Pinned query root missing from group observation"
        );
    }

    #[test]
    fn inaccessible_stat_remains_unknown_and_live_stat_bytes_are_preserved() {
        struct Denied;
        impl Read for Denied {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from_raw_os_error(libc::EACCES))
            }
        }
        let error = linux_read_stat(Denied).unwrap_err();
        assert_eq!(error.code, "SOURCE_UNAVAILABLE");
        assert_eq!(error.exit, 6);
        let bytes = std::fs::read("/proc/self/stat").unwrap();
        assert_eq!(linux_read_stat(bytes.as_slice()).unwrap().unwrap(), bytes);
    }

    #[test]
    fn non_utf8_named_fixture_child() {
        let Some(marker) = std::env::var_os("PCTX_QUERY_NON_UTF8_CHILD") else {
            return;
        };
        let name = b"fixture\xff\0";
        assert_eq!(
            unsafe { libc::prctl(libc::PR_SET_NAME, name.as_ptr(), 0, 0, 0) },
            0
        );
        std::fs::write(marker, b"ready").unwrap();
        std::thread::sleep(Duration::from_secs(60));
    }

    #[test]
    fn unrelated_native_non_utf8_process_name_does_not_block_query() {
        struct Cleanup(std::process::Child);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let root = tempfile::tempdir().unwrap();
        let marker = root.path().join("ready");
        let mut fixture = Command::new(std::env::current_exe().unwrap());
        fixture
            .args([
                "--exact",
                "query_process::linux_group_tests::non_utf8_named_fixture_child",
            ])
            .env("PCTX_QUERY_NON_UTF8_CHILD", &marker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _cleanup = Cleanup(fixture.spawn().unwrap());
        let until = std::time::Instant::now() + Duration::from_secs(3);
        while !marker.exists() {
            assert!(
                std::time::Instant::now() < until,
                "Native fixture did not become ready"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let executable = root.path().join("git");
        std::fs::write(&executable, b"#!/bin/sh\nprintf observed\n").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut query = Command::new(executable);
        query
            .args(["rev-parse", "--is-inside-work-tree"])
            .env_clear();
        let result = output(query, Deadline::from_millis(3000).unwrap(), 4096).unwrap();
        assert!(result.status.success());
        assert_eq!(result.stdout, b"observed");
    }
}

#[cfg(test)]
mod adapter_version_policy_tests {
    use super::*;

    #[test]
    fn version_observation_does_not_admit_inference_or_extra_arguments() {
        let root = tempfile::tempdir().unwrap();
        for args in [
            vec![],
            vec!["--version", "--help"],
            vec!["--print", "fixture"],
            vec!["--version=fixture"],
        ] {
            let mut command = Command::new("claude");
            command.args(&args);
            let error =
                validate(&command, root.path(), Deadline::from_millis(1000).unwrap()).unwrap_err();
            assert_eq!(error.code, "POLICY_DENIED", "{args:?}");
            assert_eq!(error.exit, 5);
        }
    }
}

#[cfg(test)]
mod managed_observation_policy_tests {
    use super::*;
    #[test]
    fn schedule_queries_reject_mutations_wildcards_and_extra_arguments() {
        let label = "org.pctx.schedule.0123456789abcdef01234567";
        assert!(managed_label(label));
        for value in [
            "org.pctx.schedule.*",
            "org.pctx.schedule.0123",
            "org.pctx.schedule.0123456789abcdef0123456G",
            "org.pctx.schedule.0123456789abcdef01234567/extra",
        ] {
            assert!(!managed_label(value));
        }
        let root = tempfile::tempdir().unwrap();
        for (program, args) in [
            ("launchctl", vec!["bootstrap", "gui/0", "fixture.plist"]),
            ("launchctl", vec!["print", "gui/0/*"]),
            (
                "launchctl",
                vec![
                    "print",
                    "gui/0/org.pctx.schedule.0123456789abcdef01234567",
                    "extra",
                ],
            ),
            (
                "systemctl",
                vec![
                    "--user",
                    "start",
                    "org.pctx.schedule.0123456789abcdef01234567.timer",
                ],
            ),
            ("systemctl", vec!["--user", "is-active", "*.timer"]),
            (
                "systemctl",
                vec![
                    "--user",
                    "is-active",
                    "org.pctx.schedule.0123456789abcdef01234567.timer",
                    "extra",
                ],
            ),
            ("systemctl", vec!["--user", "show-environment"]),
        ] {
            let mut command = Command::new(program);
            command.args(&args);
            let e =
                validate(&command, root.path(), Deadline::from_millis(1000).unwrap()).unwrap_err();
            assert_eq!(e.code, "POLICY_DENIED", "{program} {args:?}");
            assert_eq!(e.exit, 5);
        }
        #[cfg(target_os = "macos")]
        {
            let domain = format!("gui/{}/{label}", unsafe { libc::getuid() });
            assert!(managed_observation("launchctl", &["print", &domain]));
            assert!(!managed_observation(
                "launchctl",
                &["print", &domain, "extra"]
            ));
        }
        #[cfg(target_os = "linux")]
        assert!(managed_observation(
            "systemctl",
            &["--user", "is-active", &format!("{label}.timer")]
        ));
    }
}

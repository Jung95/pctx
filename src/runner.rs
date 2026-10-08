//! Registered checks reuse the shared output supervisor. Durable host slots never expire by time.
use crate::{
    domain::{Error, Result, hash, id, now},
    output,
    project::{Project, atomic_write, private_dir},
    reader, work,
};
use clap::Subcommand;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Subcommand)]
pub enum RunnerCommand {
    CheckPlan {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        run: Option<String>,
    },
    CheckRun {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        run: String,
        #[arg(long, default_value_t = 8192)]
        budget_bytes: usize,
    },
    Trust {
        #[arg(long)]
        key: String,
        #[arg(long)]
        expect_hash: String,
    },
    ResourceStatus,
    JobCancel {
        job: String,
    },
    HelperRequest {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        run: String,
        #[arg(long, default_value = "local")]
        mode: String,
        #[arg(long)]
        scope: Vec<String>,
        #[arg(long, default_value_t = 8192)]
        budget_bytes: usize,
    },
    HelperStatus {
        helper: String,
    },
    HelperCancel {
        helper: String,
    },
    HelperRelease {
        helper: String,
        #[arg(long)]
        evidence: String,
    },
    #[command(hide = true)]
    BridgeGuardian {
        #[arg(long)]
        fd: i32,
        #[arg(long)]
        lock_path: PathBuf,
    },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    checks: BTreeMap<String, Profile>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    argv: Vec<String>,
    #[serde(default)]
    auxiliary_provider: Option<Auxiliary>,
    #[serde(default)]
    bridge: Option<Bridge>,
    #[serde(default)]
    memory: MemoryPolicy,
    #[serde(default)]
    script_inputs: Vec<String>,
    #[serde(default = "root")]
    cwd: String,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    env_allowlist: Vec<String>,
    #[serde(default)]
    resources: Vec<String>,
    #[serde(default = "backend")]
    resource_backend: String,
    #[serde(default)]
    execution_timeout_ms: Option<u64>,
    #[serde(default)]
    heavy: bool,
    #[serde(default)]
    reporter: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Auxiliary {
    kind: String,
    workspace: PathBuf,
    scope: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Helper {
    schema_version: u32,
    helper_id: String,
    task_id: String,
    run_id: String,
    mode: String,
    scope: Vec<String>,
    workspace: String,
    execution_workspace: Option<String>,
    state: String,
    job_id: Option<String>,
    output_id: Option<String>,
    started: bool,
    created_at: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bridge {
    protocol: String,
    lock_path: PathBuf,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemoryPolicy {
    #[serde(default = "native")]
    source: String,
    #[serde(default = "memory_minimum")]
    minimum_available_bytes: u64,
    #[serde(default = "deny")]
    unknown: String,
    #[serde(default)]
    fixture_path: Option<String>,
}
fn native() -> String {
    "native".into()
}
fn deny() -> String {
    "deny".into()
}
fn memory_minimum() -> u64 {
    256 * 1024 * 1024
}
impl Default for MemoryPolicy {
    fn default() -> Self {
        Self {
            source: native(),
            minimum_available_bytes: memory_minimum(),
            unknown: deny(),
            fixture_path: None,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemoryObservation {
    schema_version: u32,
    source: String,
    available_bytes: Option<u64>,
    pressure: String,
    sampled_at: i64,
    #[serde(default)]
    measurement: String,
}
fn root() -> String {
    ".".into()
}
fn backend() -> String {
    "pctx".into()
}
#[derive(Clone, Serialize, Deserialize)]
struct Binding {
    schema_version: u32,
    key: String,
    profile: Profile,
    executable: String,
    executable_hash: String,
    script_hashes: BTreeMap<String, String>,
    execution_fingerprint: String,
    guardian_hash: Option<String>,
    workspace: String,
    policy: String,
    fingerprint: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Job {
    schema_version: u32,
    job_id: String,
    workspace: String,
    profile_fingerprint: String,
    resources: Vec<String>,
    state: String,
    pid: Option<u32>,
    process_group: Option<u32>,
    start_identity: Option<String>,
    boot_id: String,
    #[serde(default)]
    guardian_pid: Option<u32>,
    #[serde(default)]
    guardian_start: Option<String>,
    #[serde(default)]
    guardian_attached: bool,
    #[serde(default)]
    bridge_path_hash: Option<String>,
    created_at: i64,
    updated_at: i64,
}
#[cfg(unix)]
struct AdmissionDiagnostics {
    start: std::time::Instant,
    enabled: bool,
}
#[cfg(unix)]
impl AdmissionDiagnostics {
    fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
            enabled: std::env::var_os("PCTX_RUNNER_DIAGNOSTICS").is_some_and(|v| v == "1"),
        }
    }
    fn phase(&self, phase: &'static str) {
        if self.enabled {
            eprintln!(
                "PCTX-RUNNER-PHASE-v1 pid={} elapsed_ms={} phase={phase}",
                std::process::id(),
                self.start.elapsed().as_millis()
            );
        }
    }
}
fn error(code: &str, msg: &str, exit: i32) -> Error {
    Error::new(code, msg, exit)
}
fn owner() -> Result<()> {
    if std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()) != "owner" {
        return Err(error(
            "POLICY_DENIED",
            "Runner trust and cancellation require local owner",
            5,
        ));
    }
    Ok(())
}
// macOS exposes root-owned system aliases /var and /tmp. Resolve only those
// exact aliases; a user-created link anywhere below them remains forbidden.
fn system_path(path: &Path) -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        for (alias, target, link) in [
            ("/var", "/private/var", "private/var"),
            ("/tmp", "/private/tmp", "private/tmp"),
        ] {
            if let Ok(tail) = path.strip_prefix(alias) {
                if fs::symlink_metadata(alias)?.uid() != 0 || fs::read_link(alias)? != *link {
                    return Err(error(
                        "POLICY_DENIED",
                        "System path alias differs from trusted OS layout",
                        5,
                    ));
                }
                return Ok(Path::new(target).join(tail));
            }
        }
    }
    Ok(path.to_path_buf())
}
fn checked(path: &Path) -> Result<()> {
    let path = system_path(path)?;
    for parent in path.ancestors() {
        if fs::symlink_metadata(parent).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(error(
                "POLICY_DENIED",
                "Linked runner state/profile paths denied",
                5,
            ));
        }
    }
    Ok(())
}
fn profile(p: &Project, key: &str) -> Result<Binding> {
    let f = reader::read(p, ".pctx/runner.toml")?;
    if f.size_bytes > 65536 {
        return Err(error(
            "INVALID_CONFIG",
            "Execution profile exceeds 64 KiB",
            2,
        ));
    }
    let config: Config = toml::from_str(&f.text)
        .map_err(|_| error("INVALID_CONFIG", "Invalid inert runner TOML profile", 2))?;
    if config.schema_version != 1 {
        return Err(error(
            "INVALID_CONFIG",
            "Unsupported runner profile schema",
            2,
        ));
    }
    let mut spec = config.checks.get(key).cloned().ok_or_else(|| {
        error(
            "CAPABILITY_UNAVAILABLE",
            "Check has no registered execution profile",
            6,
        )
    })?;
    let target = if let Some(provider) = &spec.auxiliary_provider {
        if provider.kind != "local"
            || provider.scope.is_empty()
            || provider.scope.len() > 64
            || spec.heavy
            || spec.resources != vec!["aux-agent".to_owned()]
        {
            return Err(error(
                "POLICY_DENIED",
                "Auxiliary local provider requires exactly aux-agent and bounded scope; heavy/deploy denied",
                5,
            ));
        }
        let target = auxiliary_workspace(p, &provider.workspace)?;
        for path in &provider.scope {
            reader::authorize(&target, path)?;
        }
        Some(target)
    } else {
        None
    };
    let execution_project = target.as_ref().unwrap_or(p);
    if spec.argv.is_empty()
        || spec.argv.len() > 256
        || spec.argv.iter().map(String::len).sum::<usize>() > 16384
        || ![
            "pctx-json-v1",
            "jest-json-v1",
            "vitest-json-v1",
            "eslint-json-v1",
            "typescript-text-v1",
            "git-status-porcelain-v1-z",
            "git-log-nul-v1",
            "git-diff-unified-v1",
        ]
        .contains(&spec.reporter.as_str())
    {
        return Err(error(
            "INVALID_CONFIG",
            "Registered argv/reporter invalid",
            2,
        ));
    }
    if spec.argv.iter().any(|s| reader::redact(s).1) {
        return Err(error(
            "POLICY_DENIED",
            "Sensitive argv is forbidden in stored execution bindings",
            5,
        ));
    }
    if spec.cwd != "." {
        let path = reader::authorize(execution_project, &spec.cwd)?;
        if !path.is_dir() {
            return Err(error(
                "INVALID_CONFIG",
                "Registered cwd must be a project directory",
                2,
            ));
        }
    }
    if !["pctx", "legacy"].contains(&spec.resource_backend.as_str()) {
        return Err(error(
            "INVALID_CONFIG",
            "Unknown canonical resource provider",
            2,
        ));
    }
    for (name, value) in &spec.env {
        if !spec.env_allowlist.contains(name)
            || !["LANG", "LC_ALL", "LC_CTYPE", "TZ", "CI"].contains(&name.as_str())
            || reader::redact(value).1
            || value.len() > 1024
        {
            return Err(error(
                "POLICY_DENIED",
                "Registered environment exceeds nonsecret allowlist",
                5,
            ));
        }
    }
    spec.resources.sort();
    spec.resources.dedup();
    if spec
        .resources
        .iter()
        .any(|r| !["heavy-compute", "docs-publish", "aux-agent"].contains(&r.as_str()))
        || spec.heavy && !spec.resources.iter().any(|r| r == "heavy-compute")
    {
        return Err(error(
            "INVALID_CONFIG",
            "Heavy execution needs registered host heavy-compute resource",
            2,
        ));
    }
    if !["native", "fixture"].contains(&spec.memory.source.as_str())
        || !["deny", "owner_override"].contains(&spec.memory.unknown.as_str())
    {
        return Err(error(
            "INVALID_CONFIG",
            "Unknown memory observation policy",
            2,
        ));
    }
    let guardian_hash = if let Some(bridge) = &mut spec.bridge {
        if bridge.protocol != "fs2-guardian-v1" || !bridge.lock_path.is_absolute() {
            return Err(error(
                "CAPABILITY_UNAVAILABLE",
                "Unknown canonical bridge protocol",
                6,
            ));
        }
        bridge.lock_path = fs::canonicalize(&bridge.lock_path)?;
        checked(&bridge.lock_path)?;
        if !bridge.lock_path.is_file() {
            return Err(error(
                "INVALID_CONFIG",
                "Canonical lock must be an existing regular file",
                2,
            ));
        }
        Some(hash(fs::read(std::env::current_exe()?)?))
    } else {
        None
    };
    let native = output::registered_binding_at(execution_project, &spec.argv, &spec.cwd)?;
    let executable = native["executable"]
        .as_str()
        .ok_or_else(|| error("INVALID_CONFIG", "Execution binding missing executable", 2))?
        .to_owned();
    let executable_hash = native["executable_hash"].as_str().unwrap_or("").to_owned();
    let name = Path::new(&executable)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if [
        "cargo",
        "rustc",
        "npm",
        "pnpm",
        "yarn",
        "make",
        "cmake",
        "ninja",
        "gradle",
        "mvn",
        "go",
        "pytest",
        "swift",
        "xcodebuild",
    ]
    .contains(&name)
        && (!spec.heavy || !spec.resources.iter().any(|r| r == "heavy-compute"))
    {
        return Err(error(
            "POLICY_DENIED",
            "Known build/test executables require heavy-compute admission",
            5,
        ));
    }
    let mut scripts = BTreeMap::new();
    if spec.script_inputs.len() > 128 {
        return Err(error(
            "INVALID_CONFIG",
            "Too many registered script inputs",
            2,
        ));
    }
    for input in &spec.script_inputs {
        let file = reader::read(execution_project, input)?;
        scripts.insert(input.clone(), file.hash);
    }
    for arg in spec.argv.iter().skip(1) {
        if !arg.starts_with('-') && execution_project.root.join(&spec.cwd).join(arg).is_file() {
            let relative = if spec.cwd == "." {
                arg.clone()
            } else {
                format!("{}/{}", spec.cwd, arg)
            };
            let file = reader::read(execution_project, &relative)?;
            scripts.insert(relative, file.hash);
        }
    }
    for input in [
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "Cargo.toml",
        "Cargo.lock",
        "Makefile",
    ] {
        let relative = if spec.cwd == "." {
            input.to_owned()
        } else {
            format!("{}/{}", spec.cwd, input)
        };
        if execution_project.root.join(&relative).is_file() {
            scripts.insert(
                relative.clone(),
                reader::read(execution_project, &relative)?.hash,
            );
        }
    }
    let native = output::registered_binding_at(execution_project, &spec.argv, &spec.cwd)?;
    let execution_fingerprint = native["fingerprint"]
        .as_str()
        .ok_or_else(|| error("INVALID_CONFIG", "Execution binding missing fingerprint", 2))?
        .to_owned();
    let fingerprint = hash(serde_json::to_vec(
        &json!({"key":key,"profile":spec,"execution_fingerprint":execution_fingerprint,"guardian_hash":guardian_hash,"execution_workspace":execution_project.workspace_id,"execution_policy":execution_project.policy_hash(),"guardian_version":env!("CARGO_PKG_VERSION"),"scripts":scripts,"workspace":p.workspace_id,"policy":p.policy_hash()}),
    )?);
    Ok(Binding {
        schema_version: 1,
        key: key.into(),
        profile: spec,
        executable,
        executable_hash,
        script_hashes: scripts,
        execution_fingerprint,
        guardian_hash,
        workspace: p.workspace_id.clone(),
        policy: p.policy_hash(),
        fingerprint,
    })
}

fn trust_path(p: &Project, b: &Binding) -> PathBuf {
    p.data_dir
        .join("trust/runners")
        .join(format!("{}.json", b.fingerprint))
}
fn trusted(p: &Project, b: &Binding) -> Result<()> {
    let path = trust_path(p, b);
    checked(&path)?;
    let raw = fs::read(path).map_err(|_| {
        error(
            "OWNER_DECISION_REQUIRED",
            "Exact registered check fingerprint requires runner trust",
            5,
        )
    })?;
    let saved: Value = serde_json::from_slice(&raw)?;
    if saved["schema_version"] != 1
        || saved["fingerprint"] != b.fingerprint
        || saved["workspace"] != p.workspace_id
        || saved["policy"] != p.policy_hash()
        || saved["execution_fingerprint"] != b.execution_fingerprint
    {
        return Err(error(
            "CONFIG_CHANGED",
            "Registered executable/profile binding changed",
            9,
        ));
    }
    Ok(())
}
/// Authority comes from the current locally trusted profile, never child report fields.
pub(crate) fn current_check_binding(p: &Project, key: &str) -> Result<Value> {
    let b = profile(p, key)?;
    trusted(p, &b)?;
    Ok(json!({"authority":"trusted_local_runner_profile","key":key,
        "profile_fingerprint":b.fingerprint,"execution_fingerprint":b.execution_fingerprint,
        "environment_fingerprint":output::registered_environment_fingerprint(&b.profile.env)?,
        "cwd":b.profile.cwd,"workspace":p.workspace_id,"policy":p.policy_hash()}))
}
fn host_dir(p: &Project) -> Result<PathBuf> {
    let path = std::env::var_os("PCTX_HOST_RESOURCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| p.data_dir.join("host-resources"));
    if !path.is_absolute() {
        return Err(error(
            "INVALID_CONFIG",
            "Host resource directory must be absolute",
            2,
        ));
    }
    let path = system_path(&path)?;
    checked(&path)?;
    checked(&path.join("jobs"))?;
    checked(&path.join("slots"))?;
    private_dir(&path)?;
    private_dir(&path.join("jobs"))?;
    private_dir(&path.join("slots"))?;
    Ok(path)
}
fn slot(resource: &str) -> &str {
    match resource {
        "heavy-compute" | "docs-publish" => "exclusive-compute",
        _ => "aux-agent",
    }
}
fn boot_id() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(s) = fs::read_to_string("/proc/sys/kernel/random/boot_id") {
            return s.trim().into();
        }
    }
    #[cfg(target_os = "macos")]
    {
        let mut value = [0u8; 128];
        let mut len = value.len();
        let rc = unsafe {
            libc::sysctlbyname(
                c"kern.bootsessionuuid".as_ptr(),
                value.as_mut_ptr().cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if rc == 0 && len > 1 && len <= value.len() {
            let bytes = &value[..len];
            if let Ok(uuid) = std::str::from_utf8(bytes) {
                let uuid = uuid.trim_end_matches('\0');
                if !uuid.is_empty() {
                    return format!("macos_boot_session:{uuid}");
                }
            }
        }
        let mut boot = std::mem::MaybeUninit::<libc::timeval>::zeroed();
        let mut len = std::mem::size_of::<libc::timeval>();
        let rc = unsafe {
            libc::sysctlbyname(
                c"kern.boottime".as_ptr(),
                boot.as_mut_ptr().cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if rc == 0 && len == std::mem::size_of::<libc::timeval>() {
            let boot = unsafe { boot.assume_init() };
            if boot.tv_sec > 0 {
                return format!("macos_boot_time:{}:{}", boot.tv_sec, boot.tv_usec);
            }
        }
    }
    "unknown".into()
}
fn process_start(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let fields = stat
            .rsplit_once(") ")?
            .1
            .split_whitespace()
            .collect::<Vec<_>>();
        fields.get(19).map(|value| value.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
        let size = std::mem::size_of::<libc::proc_bsdinfo>();
        let bytes = unsafe {
            libc::proc_pidinfo(
                pid as i32,
                libc::PROC_PIDTBSDINFO,
                0,
                info.as_mut_ptr().cast(),
                size as i32,
            )
        };
        if bytes != size as i32 {
            return None;
        }
        let info = unsafe { info.assume_init() };
        if info.pbi_pid != pid
            || info.pbi_uid != unsafe { libc::geteuid() }
            || info.pbi_start_tvsec == 0
        {
            return None;
        }
        Some(format!(
            "{}:{}",
            info.pbi_start_tvsec, info.pbi_start_tvusec
        ))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}
fn job_path(dir: &Path, job: &str) -> Result<PathBuf> {
    if !job.starts_with("JOB-")
        || job.len() > 64
        || !job.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return Err(error("INVALID_ARGUMENT", "Invalid resource job ID", 2));
    }
    Ok(dir.join("jobs").join(format!("{job}.json")))
}
fn publish(dir: &Path, job: &Job) -> Result<()> {
    atomic_write(
        &job_path(dir, &job.job_id)?,
        &serde_json::to_vec(job)?,
        true,
    )
}
fn acquire(p: &Project, b: &Binding) -> Result<(PathBuf, Job)> {
    if b.profile.resource_backend == "legacy"
        && b.profile.bridge.is_none()
        && !b.profile.resources.is_empty()
    {
        return Err(error(
            "CAPABILITY_UNAVAILABLE",
            "Legacy canonical lock bridge is not verified; execution disabled",
            6,
        ));
    }
    let dir = host_dir(p)?;
    let mut resources = b
        .profile
        .resources
        .iter()
        .map(|r| slot(r).to_owned())
        .collect::<Vec<_>>();
    resources.sort();
    resources.dedup();
    let job = Job {
        schema_version: 1,
        job_id: id("JOB"),
        workspace: p.workspace_id.clone(),
        profile_fingerprint: b.fingerprint.clone(),
        resources: resources.clone(),
        state: "starting_or_unknown".into(),
        pid: None,
        process_group: None,
        start_identity: None,
        boot_id: boot_id(),
        guardian_pid: None,
        guardian_start: None,
        guardian_attached: false,
        bridge_path_hash: None,
        created_at: now(),
        updated_at: now(),
    };
    publish(&dir, &job)?;
    let mut obtained: Vec<PathBuf> = vec![];
    for resource in resources {
        let path = dir.join("slots").join(format!("{resource}.json"));
        match atomic_write(&path, &serde_json::to_vec(&job)?, false) {
            Ok(()) => obtained.push(path),
            Err(_) => {
                for path in obtained {
                    let _ = fs::remove_file(path);
                }
                let mut failed = job.clone();
                failed.state = "admission_blocked".into();
                publish(&dir, &failed)?;
                return Err(error(
                    "RESOURCE_BUSY",
                    "Host slot is occupied or owner unknown; no TTL takeover",
                    7,
                ));
            }
        }
    }
    Ok((dir, job))
}
fn spawned(dir: &Path, job: &mut Job, pid: u32) -> Result<()> {
    job.pid = Some(pid);
    job.process_group = Some(pid);
    job.start_identity = process_start(pid);
    job.state = "active_or_unknown".into();
    job.updated_at = now();
    publish(dir, job)?;
    for resource in &job.resources {
        atomic_write(
            &dir.join("slots").join(format!("{resource}.json")),
            &serde_json::to_vec(job)?,
            true,
        )?;
    }
    Ok(())
}
fn acknowledged_guardian(dir: &Path, job: &mut Job) -> Result<()> {
    job.guardian_attached = true;
    job.updated_at = now();
    publish(dir, job)?;
    for resource in &job.resources {
        let path = dir.join("slots").join(format!("{resource}.json"));
        let owner: Job = serde_json::from_slice(&fs::read(&path)?)?;
        if owner.job_id != job.job_id {
            return Err(error(
                "RESOURCE_OWNER_UNKNOWN",
                "Slot changed before guardian acknowledgment",
                7,
            ));
        }
        atomic_write(&path, &serde_json::to_vec(job)?, true)?;
    }
    Ok(())
}
fn group_gone(_job: &Job) -> bool {
    #[cfg(unix)]
    {
        if let Some(group) = _job.process_group {
            let rc = unsafe { libc::kill(-(group as i32), 0) };
            return rc == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH);
        }
    }
    false
}
fn release(dir: &Path, job: &mut Job) -> Result<bool> {
    if !group_gone(job) {
        job.state = "resource_owner_unknown".into();
        publish(dir, job)?;
        return Ok(false);
    }
    for resource in &job.resources {
        let path = dir.join("slots").join(format!("{resource}.json"));
        let current: Job = serde_json::from_slice(&fs::read(&path)?)?;
        if current.job_id != job.job_id {
            return Err(error(
                "RESOURCE_OWNER_UNKNOWN",
                "Host slot ownership changed",
                7,
            ));
        }
    }
    for resource in &job.resources {
        fs::remove_file(dir.join("slots").join(format!("{resource}.json")))?;
    }
    job.state = "finished".into();
    job.updated_at = now();
    publish(dir, job)?;
    Ok(true)
}
fn task_check(
    p: &Project,
    task_name: &str,
    key: &str,
    run: Option<&str>,
) -> Result<(String, work::CheckDefinition)> {
    let db = work::connect(p)?;
    let number = task_name
        .strip_prefix("T-")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(-1);
    let (task, definition): (String, String) = db
        .query_row(
            "SELECT id,definition FROM tasks WHERE id=?1 OR number=?2",
            rusqlite::params![task_name, number],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| error("TASK_NOT_FOUND", "Task not found", 6))?;
    let d: work::TaskDefinition = serde_json::from_str(&definition)?;
    let check = d
        .checks
        .into_iter()
        .find(|c| c.key == key)
        .ok_or_else(|| error("INVALID_ARGUMENT", "Check is not defined on task", 2))?;
    if let Some(run) = run {
        let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1 AND task=?2 AND workspace=?3 AND status='active' AND lease_until>?4)",rusqlite::params![run,task,p.workspace_id,now()],|r|r.get(0))?;
        if !valid {
            return Err(error(
                "LEASE_REVOKED",
                "Check requires active task run in current workspace",
                9,
            ));
        }
        let agent: String = db.query_row("SELECT agent FROM runs WHERE id=?1", [run], |row| {
            row.get(0)
        })?;
        crate::operations::ensure_claim_allowed_db(&db, &agent)?;
    }
    if !check.allowed_sources.iter().any(|s| s == "runner_observed") {
        return Err(error(
            "POLICY_DENIED",
            "Task check policy does not allow runner-observed evidence",
            5,
        ));
    }
    Ok((task, check))
}
fn planned(p: &Project, task: &str, key: &str, run: Option<&str>) -> Result<(Binding, Value)> {
    let (_, check) = task_check(p, task, key, run)?;
    let b = profile(p, key)?;
    let mut reasons = vec![];
    if trusted(p, &b).is_err() {
        reasons.push("owner_binding_required");
    }
    if b.profile.resource_backend == "legacy"
        && b.profile.bridge.is_none()
        && !b.profile.resources.is_empty()
    {
        reasons.push("legacy_bridge_unverified");
    }
    let plan = json!({"key":key,"fingerprint":b.fingerprint,"executable_hash":b.executable_hash,"script_hashes":b.script_hashes,"cwd":b.profile.cwd,"environment_keys":b.profile.env.keys().collect::<Vec<_>>(),"resources":b.profile.resources,"heavy":b.profile.heavy,"resource_provider":b.profile.resource_backend,"execution_timeout_ms":b.profile.execution_timeout_ms,"reporter":b.profile.reporter,"allowed_sources":check.allowed_sources,"output_paths":check.output_paths,"blocked_reasons":reasons,"execution_started":false,"active_run_verified":run.is_some(),"run_required_before_execution":true,"host_permission":"separate_required","memory_policy":b.profile.memory,"guardian_protocol":b.profile.bridge.as_ref().map(|b|&b.protocol),"guardian_executable_hash":b.guardian_hash});
    Ok((b, plan))
}
fn release_not_spawned(dir: &Path, job: &mut Job) -> Result<()> {
    for resource in &job.resources {
        let path = dir.join("slots").join(format!("{resource}.json"));
        let current: Job = serde_json::from_slice(&fs::read(&path)?)?;
        if current.job_id != job.job_id {
            return Err(error(
                "RESOURCE_OWNER_UNKNOWN",
                "Slot owner changed before spawn failure",
                7,
            ));
        }
    }
    for resource in &job.resources {
        fs::remove_file(dir.join("slots").join(format!("{resource}.json")))?;
    }
    job.state = "not_started".into();
    publish(dir, job)
}
pub fn execute(p: &Project, command: &RunnerCommand) -> Result<Value> {
    match command {
        RunnerCommand::BridgeGuardian { fd, lock_path } => guardian_main(*fd, lock_path),
        RunnerCommand::HelperRequest {
            task_id,
            key,
            run,
            mode,
            scope,
            budget_bytes,
        } => helper_request(p, task_id, key, run, mode, scope, *budget_bytes),
        RunnerCommand::HelperStatus { helper } => helper_status(p, helper),
        RunnerCommand::HelperCancel { helper } => helper_cancel(p, helper),
        RunnerCommand::HelperRelease { helper, evidence } => helper_release(p, helper, evidence),
        RunnerCommand::CheckPlan { task_id, key, run } => {
            Ok(planned(p, task_id, key, run.as_deref())?.1)
        }
        RunnerCommand::Trust { key, expect_hash } => {
            owner()?;
            let b = profile(p, key)?;
            if b.fingerprint != *expect_hash {
                return Err(error(
                    "CONFIG_CHANGED",
                    "Execution profile changed since plan",
                    9,
                ));
            }
            let path = trust_path(p, &b);
            checked(&path)?;
            private_dir(path.parent().unwrap())?;
            atomic_write(
                &path,
                &serde_json::to_vec(
                    &json!({"schema_version":1,"fingerprint":b.fingerprint,"workspace":b.workspace,"policy":b.policy,"execution_fingerprint":b.execution_fingerprint,"trusted_by":"local_owner"}),
                )?,
                false,
            )?;
            Ok(
                json!({"key":key,"fingerprint":b.fingerprint,"trusted":true,"execution_started":false,"host_permission":"separate_required"}),
            )
        }
        #[cfg(unix)]
        RunnerCommand::CheckRun {
            task_id,
            key,
            run,
            budget_bytes,
        } => {
            if *budget_bytes < 8192 {
                return Err(error(
                    "BUDGET_TOO_SMALL",
                    "Registered evidence metadata requires 8192-byte minimum before execution",
                    8,
                ));
            }
            let diagnostics = AdmissionDiagnostics::new();
            diagnostics.phase("planned_profile_begin");
            let (b, _) = planned(p, task_id, key, Some(run))?;
            diagnostics.phase("planned_profile_ready");
            if b.profile.auxiliary_provider.is_some() {
                return Err(error(
                    "POLICY_DENIED",
                    "Auxiliary providers require helper request with isolated workspace",
                    5,
                ));
            }
            trusted(p, &b)?;
            diagnostics.phase("current_profile_begin");
            let current = profile(p, key)?;
            diagnostics.phase("current_profile_ready");
            if current.fingerprint != b.fingerprint {
                return Err(error(
                    "CONFIG_CHANGED",
                    "Profile changed before execution",
                    9,
                ));
            }
            let memory = observe_memory(p, &b.profile.memory)?;
            if b.profile.heavy
                || b.profile
                    .resources
                    .iter()
                    .any(|r| r == "heavy-compute" || r == "aux-agent")
            {
                admit_memory(&memory, &b.profile.memory)?;
            }
            let check_binding = current_check_binding(p, key)?;
            let (dir, mut job) = acquire(p, &b)?;
            diagnostics.phase("host_slots_acquired");
            let begin = work::execute(
                p,
                &work::WorkCommand::Check {
                    command: work::CheckCommand::Begin {
                        task: task_id.clone(),
                        key: key.clone(),
                        run: run.clone(),
                    },
                },
            );
            let begin = match begin {
                Ok(v) => v,
                Err(e) => {
                    release_not_spawned(&dir, &mut job)?;
                    return Err(e);
                }
            };
            let check_id = begin["check_id"]
                .as_str()
                .ok_or_else(|| error("DB_ERROR", "Check begin did not return attempt identity", 7))?
                .to_owned();
            diagnostics.phase("final_profile_begin");
            let final_binding = match profile(p, key) {
                Ok(v) => v,
                Err(e) => {
                    work::record_runner_not_started(p, &check_id, &e.code)?;
                    release_not_spawned(&dir, &mut job)?;
                    return Err(e);
                }
            };
            diagnostics.phase("final_profile_ready");
            if final_binding.fingerprint != b.fingerprint {
                work::record_runner_not_started(p, &check_id, "CONFIG_CHANGED")?;
                release_not_spawned(&dir, &mut job)?;
                return Err(error(
                    "CONFIG_CHANGED",
                    "Profile or inputs changed after check began",
                    9,
                ));
            }
            diagnostics.phase("guardian_admission_begin");
            let guardian = match start_guardian(p, &b, &dir, &mut job) {
                Ok(v) => v,
                Err(e) => {
                    work::record_runner_not_started(p, &check_id, &e.code)?;
                    release_not_spawned(&dir, &mut job)?;
                    return Err(e);
                }
            }
            .map(|g| std::rc::Rc::new(std::cell::RefCell::new(g)));
            diagnostics.phase("guardian_admission_ready");
            let argv = b.profile.argv.clone();
            let request = output::RunRequest {
                task_id: Some(task_id.clone()),
                session: None,
                retain: "temporary".into(),
                execution_timeout_ms: b.profile.execution_timeout_ms,
                budget_bytes: budget_bytes.saturating_sub(1500),
                exit_policy: "pctx".into(),
                stdin: "closed".into(),
                argv,
            };
            let observed = output::run_registered_parsed(
                p,
                &request,
                &b.profile.cwd,
                &b.profile.env,
                &b.execution_fingerprint,
                &mut |pid| {
                    diagnostics.phase("child_spawned");
                    spawned(&dir, &mut job, pid)?;
                    if let Some(g) = guardian.as_ref() {
                        diagnostics.phase("guardian_attach_begin");
                        g.borrow_mut().attach(&job)?;
                        acknowledged_guardian(&dir, &mut job)?;
                        diagnostics.phase("guardian_durable_ack");
                    }
                    Ok(())
                },
                &mut || {
                    if let Some(g) = guardian.as_ref() {
                        g.borrow_mut().monitor()?;
                    }
                    Ok(())
                },
                &b.profile.reporter,
                &check_binding,
            );
            if job.pid.is_none()
                && let Some(g) = guardian.as_ref()
            {
                g.borrow_mut().stop_without_child()?;
            }
            let execution = match observed {
                Ok(value) => value,
                Err(e) => {
                    if job.pid.is_some() {
                        work::record_runner_execution_error(p, &check_id, &e.code)?;
                        if group_gone(&job)
                            && let Some(g) = guardian.as_ref()
                        {
                            g.borrow_mut().stop_without_child()?;
                        }
                        let _ = release(&dir, &mut job);
                    } else {
                        work::record_runner_not_started(p, &check_id, &e.code)?;
                        release_not_spawned(&dir, &mut job)?;
                    }
                    return Err(e);
                }
            };
            if group_gone(&job)
                && let Some(g) = guardian.as_ref()
                && let Err(e) = g.borrow_mut().wait_finished()
            {
                if let Some(output_id) = execution["output_id"].as_str() {
                    work::record_runner_unverified(p, &check_id, output_id)?;
                } else {
                    work::record_runner_execution_error(p, &check_id, &e.code)?;
                }
                return Err(e);
            }
            let released = if execution["spawned"] == false {
                release_not_spawned(&dir, &mut job)?;
                true
            } else {
                release(&dir, &mut job)?
            };
            let output_id = execution["output_id"].as_str();
            let report = match output_id {
                Some(output_id) => output::observed_report(p, output_id)?,
                None => None,
            };
            let evidence = if let (Some(output_id), Some(_)) = (output_id, report) {
                work::record_runner_report(p, &check_id, output_id)?
            } else if let Some(output_id) = output_id {
                work::record_runner_unverified(p, &check_id, output_id)?
            } else {
                work::record_runner_not_started(p, &check_id, "SPAWN_FAILED_NO_OUTPUT")?
            };
            Ok(
                json!({"job_id":job.job_id,"check_id":check_id,"execution":execution,"evidence":evidence,"memory_admission":memory,"memory_unknown_override":memory.available_bytes.is_none()&&b.profile.memory.unknown=="owner_override","resources_released":released,"resource_state":job.state,"host_permission":"separate_required"}),
            )
        }
        #[cfg(not(unix))]
        RunnerCommand::CheckRun { .. } => Err(error(
            "CAPABILITY_UNAVAILABLE",
            "Registered process supervision is supported only on verified Unix backend",
            6,
        )),
        RunnerCommand::ResourceStatus => {
            let dir = host_dir(p)?;
            let mut jobs = vec![];
            for resource in ["exclusive-compute", "aux-agent"] {
                let path = dir.join("slots").join(format!("{resource}.json"));
                if !path.exists() {
                    continue;
                }
                let mut job: Job = serde_json::from_slice(&fs::read(path)?)?;
                if job.boot_id != boot_id()
                    || job.pid.is_none()
                    || job
                        .pid
                        .is_some_and(|pid| process_start(pid) != job.start_identity)
                {
                    job.state = "resource_owner_unknown".into();
                }
                jobs.push(json!({"resource":resource,"job":job,"ttl_takeover_allowed":false,"automatic_release":false,"guardian_alive_verified":job.guardian_pid.is_some_and(|pid|job.guardian_start.is_some()&&process_start(pid)==job.guardian_start)}));
            }
            Ok(
                json!({"resources":jobs,"scope":"host","capacity_per_slot":1,"legacy_bridge":"per_job_registered_guardian_or_unconfigured"}),
            )
        }
        RunnerCommand::JobCancel { job: id } => {
            owner()?;
            let dir = host_dir(p)?;
            let path = job_path(&dir, id)?;
            let mut job: Job = serde_json::from_slice(&fs::read(path)?)?;
            if job.job_id != *id || job.schema_version != 1 {
                return Err(error(
                    "RESOURCE_OWNER_UNKNOWN",
                    "Job identity is invalid",
                    7,
                ));
            }
            if group_gone(&job) {
                let released = release(&dir, &mut job)?;
                return Ok(
                    json!({"job_id":id,"cancelled":false,"already_exited":true,"resources_released":released}),
                );
            }
            let pid = job.pid.ok_or_else(|| {
                error(
                    "RESOURCE_OWNER_UNKNOWN",
                    "No verifiable child identity; manual operator inspection required",
                    7,
                )
            })?;
            if job.boot_id == "unknown"
                || job.boot_id != boot_id()
                || job.start_identity.is_none()
                || process_start(pid).is_some_and(|actual| Some(actual) != job.start_identity)
            {
                return Err(error(
                    "RESOURCE_OWNER_UNKNOWN",
                    "Boot/process identity mismatch; refusing signals and unlock",
                    7,
                ));
            }
            #[cfg(unix)]
            {
                let group = job
                    .process_group
                    .ok_or_else(|| error("RESOURCE_OWNER_UNKNOWN", "Process group unknown", 7))?;
                unsafe {
                    libc::kill(-(group as i32), libc::SIGTERM);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
                if !group_gone(&job) {
                    unsafe {
                        libc::kill(-(group as i32), libc::SIGKILL);
                    }
                }
                let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while !group_gone(&job) && std::time::Instant::now() < until {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            let released = release(&dir, &mut job)?;
            Ok(
                json!({"job_id":id,"cancelled":released,"resources_released":released,"state":job.state}),
            )
        }
    }
}

fn observe_memory(p: &Project, policy: &MemoryPolicy) -> Result<MemoryObservation> {
    if policy.source == "fixture" {
        let path = policy.fixture_path.as_ref().ok_or_else(|| {
            error(
                "INVALID_CONFIG",
                "Synthetic memory observation requires an explicit fixture path",
                2,
            )
        })?;
        let f = reader::read(p, path)?;
        if f.size_bytes > 4096 {
            return Err(error("INVALID_CONFIG", "Memory fixture exceeds bound", 2));
        }
        let mut observation: MemoryObservation = serde_json::from_str(&f.text)?;
        if observation.schema_version != 1
            || observation.source != "fixture"
            || observation.sampled_at > now() + 1
            || now() - observation.sampled_at > 30
        {
            return Err(error(
                "MEMORY_UNKNOWN",
                "Synthetic observation is invalid or expired",
                7,
            ));
        }
        observation.measurement = "explicit_test_fixture_not_native_measurement".into();
        return Ok(observation);
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = fs::read_to_string("/proc/meminfo")
            && let Some(bytes) = text.lines().find_map(|line| {
                line.strip_prefix("MemAvailable:").and_then(|s| {
                    s.split_whitespace()
                        .next()?
                        .parse::<u64>()
                        .ok()?
                        .checked_mul(1024)
                })
            })
        {
            return Ok(MemoryObservation {
                schema_version: 1,
                source: "native".into(),
                available_bytes: Some(bytes),
                pressure: "observed_available".into(),
                sampled_at: now(),
                measurement: "linux_proc_MemAvailable_estimate".into(),
            });
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("/usr/bin/vm_stat")
            .env_clear()
            .env("LANG", "C")
            .output()
            && out.status.success()
            && let Ok(text) = String::from_utf8(out.stdout)
        {
            let page = text.lines().next().and_then(|s| {
                s.split("page size of ")
                    .nth(1)?
                    .split_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()
            });
            let free = text.lines().find_map(|s| {
                s.strip_prefix("Pages free:")?
                    .trim()
                    .trim_end_matches('.')
                    .parse::<u64>()
                    .ok()
            });
            let inactive = text.lines().find_map(|s| {
                s.strip_prefix("Pages inactive:")?
                    .trim()
                    .trim_end_matches('.')
                    .parse::<u64>()
                    .ok()
            });
            if let (Some(page), Some(free), Some(inactive)) = (page, free, inactive)
                && let Some(bytes) = free.checked_add(inactive).and_then(|n| n.checked_mul(page))
            {
                return Ok(MemoryObservation {
                    schema_version: 1,
                    source: "native".into(),
                    available_bytes: Some(bytes),
                    pressure: "reclaimable_estimate".into(),
                    sampled_at: now(),
                    measurement: "macos_vm_stat_free_plus_inactive_reclaimable_estimate".into(),
                });
            }
        }
    }
    Ok(MemoryObservation {
        schema_version: 1,
        source: "native".into(),
        available_bytes: None,
        pressure: "unknown".into(),
        sampled_at: now(),
        measurement: "native_measurement_unavailable".into(),
    })
}
fn admit_memory(observation: &MemoryObservation, policy: &MemoryPolicy) -> Result<()> {
    if ["critical", "high"].contains(&observation.pressure.as_str()) {
        return Err(error(
            "MEMORY_PRESSURE",
            "Observed host memory pressure defers resource admission",
            7,
        ));
    }
    match observation.available_bytes {
        Some(n) if n < policy.minimum_available_bytes => Err(error(
            "MEMORY_PRESSURE",
            "Observed available memory is below trusted admission threshold",
            7,
        )),
        None if policy.unknown != "owner_override" => Err(error(
            "MEMORY_UNKNOWN",
            "Available memory unknown; no automatic heavy admission",
            7,
        )),
        _ => Ok(()),
    }
}

#[cfg(unix)]
struct Guardian {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: std::process::ChildStdout,
    execution_group: Option<u32>,
}
#[cfg(not(unix))]
struct Guardian;
#[cfg(unix)]
fn guardian_line(pipe: &mut std::process::ChildStdout) -> Result<String> {
    use std::io::Read;
    use std::os::fd::AsRawFd;
    let fd = pipe.as_raw_fd();
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut bytes = vec![];
    loop {
        let mut one = [0u8; 1];
        match pipe.read(&mut one) {
            Ok(1) => {
                if one[0] == b'\n' {
                    return String::from_utf8(bytes).map_err(|_| {
                        error("RESOURCE_OWNER_UNKNOWN", "Invalid guardian handshake", 7)
                    });
                }
                bytes.push(one[0]);
                if bytes.len() > 512 {
                    return Err(error(
                        "RESOURCE_OWNER_UNKNOWN",
                        "Guardian handshake too large",
                        7,
                    ));
                }
            }
            Ok(0) => {
                return Err(error(
                    "RESOURCE_OWNER_UNKNOWN",
                    "Guardian ended before handshake",
                    7,
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e.into()),
            _ => {}
        }
        if std::time::Instant::now() >= end {
            return Err(error(
                "RESOURCE_OWNER_UNKNOWN",
                "Guardian handshake timeout",
                7,
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
#[cfg(unix)]
impl Guardian {
    fn attach(&mut self, job: &Job) -> Result<()> {
        use std::io::Write;
        self.execution_group = job.process_group;
        if job.start_identity.is_none() && !group_gone(job) {
            return Err(error(
                "RESOURCE_OWNER_UNKNOWN",
                "Native child start identity unavailable while group remains alive",
                7,
            ));
        }
        if self.child.try_wait()?.is_some() {
            return Err(error(
                "RESOURCE_OWNER_UNKNOWN",
                "Guardian exited before child receipt",
                7,
            ));
        }
        self.input.write_all(&serde_json::to_vec(job)?)?;
        self.input.write_all(b"\n")?;
        self.input.flush()?;
        match guardian_line(&mut self.output)?.as_str() {
            "PCTX-GUARDIAN-ATTACHED-v1" => (),
            "PCTX-GUARDIAN-REJECTED-PROOF-v1" => {
                return Err(error(
                    "RESOURCE_OWNER_UNKNOWN",
                    "Guardian rejected child receipt proof while retaining canonical lock",
                    7,
                ));
            }
            "PCTX-GUARDIAN-REJECTED-START-v1" => {
                return Err(error(
                    "RESOURCE_OWNER_UNKNOWN",
                    "Guardian rejected child start identity while retaining canonical lock",
                    7,
                ));
            }
            _ => {
                return Err(error(
                    "RESOURCE_OWNER_UNKNOWN",
                    "Guardian did not verify child receipt",
                    7,
                ));
            }
        }
        Ok(())
    }
    fn monitor(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_some() {
            let gone=self.execution_group.is_some_and(|group|unsafe{libc::kill(-(group as i32),0)}==-1&&std::io::Error::last_os_error().raw_os_error()==Some(libc::ESRCH));
            if !gone {
                return Err(error(
                    "RESOURCE_OWNER_UNKNOWN",
                    "Canonical lock guardian exited while child group remains",
                    7,
                ));
            }
        }
        Ok(())
    }
    fn stop_without_child(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
            self.child.wait()?;
        }
        Ok(())
    }
    fn wait_finished(&mut self) -> Result<()> {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while self.child.try_wait()?.is_none() {
            if std::time::Instant::now() > end {
                return Err(error(
                    "RESOURCE_OWNER_UNKNOWN",
                    "Guardian has not confirmed group termination",
                    7,
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }
}
#[cfg(not(unix))]
impl Guardian {
    fn monitor(&mut self) -> Result<()> {
        Ok(())
    }
    fn attach(&mut self, _: &Job) -> Result<()> {
        Err(error(
            "CAPABILITY_UNAVAILABLE",
            "No verified bridge on this host",
            6,
        ))
    }
    fn stop_without_child(&mut self) -> Result<()> {
        Ok(())
    }
    fn wait_finished(&mut self) -> Result<()> {
        Ok(())
    }
}
fn start_guardian(p: &Project, b: &Binding, dir: &Path, job: &mut Job) -> Result<Option<Guardian>> {
    let Some(bridge) = &b.profile.bridge else {
        return Ok(None);
    };
    #[cfg(not(unix))]
    {
        let _ = (p, dir, job, bridge);
        Err(error(
            "CAPABILITY_UNAVAILABLE",
            "Canonical bridge requires verified Unix process identity",
            6,
        ))
    }
    #[cfg(unix)]
    {
        use fs2::FileExt;
        use std::os::{
            fd::AsRawFd,
            unix::{fs::OpenOptionsExt, process::CommandExt},
        };
        use std::process::{Command, Stdio};
        let executable = std::env::current_exe()?;
        if Some(hash(fs::read(&executable)?)) != b.guardian_hash {
            return Err(error(
                "CONFIG_CHANGED",
                "Guardian executable or host identity unavailable",
                9,
            ));
        }
        if boot_id() == "unknown" {
            return Err(error(
                "CAPABILITY_UNAVAILABLE",
                "Native host boot identity cannot be observed; canonical bridge is disabled",
                6,
            ));
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&bridge.lock_path)?;
        lock.try_lock_exclusive()
            .map_err(|_| error("RESOURCE_BUSY", "Legacy canonical lock is occupied", 7))?;
        let fd = lock.as_raw_fd();
        let mut command = Command::new(executable);
        command
            .args([
                "--root",
                p.root
                    .to_str()
                    .ok_or_else(|| error("INVALID_CONFIG", "Non UTF-8 bridge root", 2))?,
                "runner",
                "bridge-guardian",
                "--fd",
                "197",
                "--lock-path",
            ])
            .arg(&bridge.lock_path)
            .env("PCTX_DATA_DIR", &p.data_dir)
            .env("PCTX_ACTOR", "owner")
            .env_remove("PCTX_RUN_CAPABILITY")
            .env_remove("PCTX_RUN_ID")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        unsafe {
            command.pre_exec(move || {
                if libc::dup2(fd, 197) < 0 || libc::fcntl(197, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::setpgid(0, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn()?;
        let input = child.stdin.take().ok_or_else(|| {
            error(
                "RESOURCE_OWNER_UNKNOWN",
                "Guardian receipt pipe unavailable",
                7,
            )
        })?;
        let output = child.stdout.take().ok_or_else(|| {
            error(
                "RESOURCE_OWNER_UNKNOWN",
                "Guardian handshake pipe unavailable",
                7,
            )
        })?;
        let mut guardian = Guardian {
            child,
            input,
            output,
            execution_group: None,
        };
        if !matches!(
            guardian_line(&mut guardian.output).as_deref(),
            Ok("PCTX-GUARDIAN-READY-v1")
        ) {
            let _ = guardian.stop_without_child();
            return Err(error(
                "RESOURCE_OWNER_UNKNOWN",
                "Canonical guardian admission handshake failed",
                7,
            ));
        }
        job.guardian_pid = Some(guardian.child.id());
        job.guardian_start = process_start(guardian.child.id());
        job.bridge_path_hash = Some(hash(bridge.lock_path.to_string_lossy().as_bytes()));
        publish(dir, job)?;
        // Parent drops its descriptor here; finite guardian remains sole mutex keeper.
        Ok(Some(guardian))
    }
}
fn guardian_main(fd: i32, path: &Path) -> Result<Value> {
    #[cfg(not(unix))]
    {
        let _ = (fd, path);
        Err(error(
            "CAPABILITY_UNAVAILABLE",
            "No verified guardian backend",
            6,
        ))
    }
    #[cfg(unix)]
    {
        use fs2::FileExt;
        use std::{
            io::{BufRead, Read, Write},
            os::{fd::FromRawFd, unix::fs::MetadataExt},
        };
        if fd != 197 {
            return Err(error(
                "INVALID_ARGUMENT",
                "Guardian descriptor contract rejected",
                2,
            ));
        }
        // This descriptor exists only through the reviewed parent's explicit dup2 contract.
        let lock = unsafe { fs::File::from_raw_fd(fd) };
        let metadata = lock.metadata()?;
        let target = fs::metadata(path)?;
        if !metadata.is_file() || metadata.dev() != target.dev() || metadata.ino() != target.ino() {
            return Err(error(
                "RESOURCE_OWNER_UNKNOWN",
                "Canonical descriptor identity mismatch",
                7,
            ));
        }
        lock.try_lock_exclusive()?;
        println!("PCTX-GUARDIAN-READY-v1");
        std::io::stdout().flush()?;
        let mut line = String::new();
        let count = std::io::BufReader::new(std::io::stdin().take(8193))
            .read_line(&mut line)
            .unwrap_or(0);
        let proof = if count > 0 && count <= 8192 {
            serde_json::from_str::<Job>(&line).ok()
        } else {
            None
        };
        let proof = proof.filter(|j| {
            j.schema_version == 1
                && j.boot_id != "unknown"
                && j.boot_id == boot_id()
                && j.pid == j.process_group
                && j.pid.is_some()
                && (j.start_identity.is_some() || group_gone(j))
                && j.guardian_pid == Some(std::process::id())
        });
        let Some(job) = proof else {
            // One bounded static protocol diagnostic, then retain the inherited
            // lock. Invalid/absent proof never authorizes release or success.
            let _ = std::io::stdout().write_all(b"PCTX-GUARDIAN-REJECTED-PROOF-v1\n");
            let _ = std::io::stdout().flush();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        };
        if let Some(pid) = job.pid
            && process_start(pid).is_some_and(|actual| Some(actual) != job.start_identity)
        {
            let _ = std::io::stdout().write_all(b"PCTX-GUARDIAN-REJECTED-START-v1\n");
            let _ = std::io::stdout().flush();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        let _ = std::io::stdout().write_all(b"PCTX-GUARDIAN-ATTACHED-v1\n");
        let _ = std::io::stdout().flush();
        while !group_gone(&job) {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        drop(lock);
        Ok(json!({"guardian":"finished","group_exit_proven":true}))
    }
}

fn inert_git_common(root: &Path, require_linked: bool) -> Result<PathBuf> {
    let entry = root.join(".git");
    checked(&entry)?;
    let metadata = fs::metadata(&entry)?;
    if metadata.is_dir() && !require_linked {
        return Ok(fs::canonicalize(entry)?);
    }
    if !metadata.is_file() || metadata.len() > 4096 {
        return Err(error(
            "WORKSPACE_MISMATCH",
            "Auxiliary root must be a linked Git worktree",
            9,
        ));
    }
    let text = fs::read_to_string(&entry)?;
    let value = text
        .trim()
        .strip_prefix("gitdir: ")
        .ok_or_else(|| error("WORKSPACE_MISMATCH", "Invalid inert worktree gitdir", 9))?;
    let gitdir = fs::canonicalize(root.join(value))?;
    checked(&gitdir)?;
    let common_file = gitdir.join("commondir");
    let backref = gitdir.join("gitdir");
    checked(&common_file)?;
    checked(&backref)?;
    if fs::metadata(&common_file)?.len() > 4096 || fs::metadata(&backref)?.len() > 4096 {
        return Err(error(
            "WORKSPACE_MISMATCH",
            "Git worktree metadata exceeds bound",
            9,
        ));
    }
    if fs::canonicalize(gitdir.join(fs::read_to_string(backref)?.trim()))?
        != fs::canonicalize(entry)?
    {
        return Err(error(
            "WORKSPACE_MISMATCH",
            "Git worktree backreference differs",
            9,
        ));
    }
    Ok(fs::canonicalize(
        gitdir.join(fs::read_to_string(common_file)?.trim()),
    )?)
}
fn auxiliary_workspace(p: &Project, path: &Path) -> Result<Project> {
    if !path.is_absolute() {
        return Err(error(
            "INVALID_CONFIG",
            "Registered auxiliary workspace must be absolute",
            2,
        ));
    }
    let target = Project::open(path)?;
    if target.root == p.root
        || target.workspace_id == p.workspace_id
        || target.project_id != p.project_id
        || target.coordination_id != p.coordination_id
        || inert_git_common(&target.root, true)? != inert_git_common(&p.root, false)?
    {
        return Err(error(
            "WORKSPACE_MISMATCH",
            "Auxiliary provider needs a separately registered linked workspace in this coordination",
            9,
        ));
    }
    Ok(target)
}
fn helper_path(p: &Project, helper: &str) -> Result<PathBuf> {
    if !helper.starts_with("HELP-")
        || helper.len() > 64
        || !helper
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return Err(error("INVALID_ARGUMENT", "Invalid helper ID", 2));
    }
    let dir = p.workspace_dir.join("helpers");
    checked(&dir)?;
    private_dir(&dir)?;
    Ok(dir.join(format!("{helper}.json")))
}
fn save_helper(p: &Project, h: &Helper) -> Result<()> {
    atomic_write(
        &helper_path(p, &h.helper_id)?,
        &serde_json::to_vec(h)?,
        true,
    )
}
fn load_helper(p: &Project, id: &str) -> Result<Helper> {
    let h: Helper = serde_json::from_slice(&fs::read(helper_path(p, id)?)?)?;
    if h.schema_version != 1 || h.helper_id != id || h.workspace != p.workspace_id {
        return Err(error(
            "WORKSPACE_MISMATCH",
            "Helper receipt belongs to another workspace",
            9,
        ));
    }
    Ok(h)
}
fn helper_request(
    p: &Project,
    task: &str,
    key: &str,
    run: &str,
    mode: &str,
    scope: &[String],
    budget: usize,
) -> Result<Value> {
    owner()?;
    let (task_id, _) = task_check(p, task, key, Some(run))?;
    if !["local", "cloud", "native"].contains(&mode) {
        return Err(error(
            "INVALID_ARGUMENT",
            "Unknown auxiliary provider mode",
            2,
        ));
    }
    if scope.len() > 64 || scope.iter().any(|s| reader::redact(s).1) {
        return Err(error(
            "INVALID_ARGUMENT",
            "Bounded nonsecret helper scope required",
            2,
        ));
    }
    let mut helper = Helper {
        schema_version: 1,
        helper_id: id("HELP"),
        task_id,
        run_id: run.into(),
        mode: mode.into(),
        scope: scope.to_vec(),
        workspace: p.workspace_id.clone(),
        execution_workspace: None,
        state: "queued_intent".into(),
        job_id: None,
        output_id: None,
        started: false,
        created_at: now(),
    };
    if mode != "local" {
        if scope.is_empty() {
            return Err(error(
                "INVALID_ARGUMENT",
                "Queued auxiliary intent requires explicit scope",
                2,
            ));
        }
        for path in scope {
            reader::authorize(p, path)?;
        }
        save_helper(p, &helper)?;
        return Ok(
            json!({"helper":helper,"provider_capability":"intent_only_no_launch_receipt","slot_allocated":false,"model_started":false}),
        );
    }
    if budget < 8192 {
        return Err(error(
            "BUDGET_TOO_SMALL",
            "Helper evidence requires minimum 8192 bytes",
            8,
        ));
    }
    let b = profile(p, key)?;
    trusted(p, &b)?;
    let provider = b.profile.auxiliary_provider.as_ref().ok_or_else(|| {
        error(
            "CAPABILITY_UNAVAILABLE",
            "No registered local auxiliary provider",
            6,
        )
    })?;
    if !scope.is_empty() && scope != provider.scope {
        return Err(error(
            "POLICY_DENIED",
            "Helper request scope differs from exact registered provider scope",
            5,
        ));
    }
    let target = auxiliary_workspace(p, &provider.workspace)?;
    helper.scope = provider.scope.clone();
    helper.execution_workspace = Some(target.workspace_id.clone());
    save_helper(p, &helper)?;
    let memory = observe_memory(p, &b.profile.memory)?;
    admit_memory(&memory, &b.profile.memory)?;
    let (dir, mut job) = acquire(p, &b)?;
    helper.job_id = Some(job.job_id.clone());
    helper.state = "starting_or_unknown".into();
    save_helper(p, &helper)?;
    if profile(p, key)?.fingerprint != b.fingerprint {
        release_not_spawned(&dir, &mut job)?;
        return Err(error(
            "CONFIG_CHANGED",
            "Auxiliary binding changed before execution",
            9,
        ));
    }
    let guardian = match start_guardian(p, &b, &dir, &mut job) {
        Ok(v) => v,
        Err(e) => {
            release_not_spawned(&dir, &mut job)?;
            return Err(e);
        }
    }
    .map(|g| std::rc::Rc::new(std::cell::RefCell::new(g)));
    let request = output::RunRequest {
        task_id: None,
        session: None,
        retain: "temporary".into(),
        execution_timeout_ms: b.profile.execution_timeout_ms,
        budget_bytes: budget.saturating_sub(1500),
        exit_policy: "pctx".into(),
        stdin: "closed".into(),
        argv: b.profile.argv.clone(),
    };
    let result = output::run_registered_monitored(
        &target,
        &request,
        &b.profile.cwd,
        &b.profile.env,
        &b.execution_fingerprint,
        &mut |pid| {
            spawned(&dir, &mut job, pid)?;
            helper.started = true;
            helper.state = "active_observed".into();
            save_helper(p, &helper)?;
            if let Some(g) = guardian.as_ref() {
                g.borrow_mut().attach(&job)?;
                acknowledged_guardian(&dir, &mut job)?;
            }
            Ok(())
        },
        &mut || {
            if let Some(g) = guardian.as_ref() {
                g.borrow_mut().monitor()?;
            }
            Ok(())
        },
    );
    if job.pid.is_none() {
        if let Some(g) = guardian.as_ref() {
            g.borrow_mut().stop_without_child()?;
        }
        release_not_spawned(&dir, &mut job)?;
    }
    let execution = match result {
        Ok(v) => v,
        Err(e) => {
            helper.state = "resource_owner_unknown".into();
            save_helper(p, &helper)?;
            return Err(e);
        }
    };
    if group_gone(&job)
        && let Some(g) = guardian.as_ref()
    {
        g.borrow_mut().wait_finished()?;
    }
    let released = if job.pid.is_none() {
        true
    } else {
        release(&dir, &mut job)?
    };
    helper.output_id = execution["output_id"].as_str().map(str::to_owned);
    helper.state = if released {
        if helper.started {
            "finished_observed"
        } else {
            "not_started"
        }
    } else {
        "resource_owner_unknown"
    }
    .into();
    save_helper(p, &helper)?;
    Ok(
        json!({"helper":helper,"execution":execution,"memory_admission":memory,"memory_unknown_override":memory.available_bytes.is_none()&&b.profile.memory.unknown=="owner_override","resources_released":released,"model_usage_observed":false,"model_start_status":"unknown_inside_registered_provider","provider_capability":"managed_local_process","capabilities_granted":["registered_argv_only"],"task_completion":"not_evaluated"}),
    )
}
fn helper_status(p: &Project, id: &str) -> Result<Value> {
    let h = load_helper(p, id)?;
    let job = if let Some(id) = &h.job_id {
        let dir = host_dir(p)?;
        Some(serde_json::from_slice::<Job>(&fs::read(job_path(
            &dir, id,
        )?)?)?)
    } else {
        None
    };
    Ok(json!({"helper":h,"job":job,"ttl_release_allowed":false,"provider_start_inferred":false}))
}
fn helper_cancel(p: &Project, id: &str) -> Result<Value> {
    owner()?;
    let mut h = load_helper(p, id)?;
    if let Some(job) = &h.job_id {
        let result = execute(p, &RunnerCommand::JobCancel { job: job.clone() })?;
        if result["resources_released"] == true {
            h.state = "cancelled_observed".into();
            save_helper(p, &h)?;
        }
        Ok(json!({"helper":h,"cancellation":result}))
    } else {
        h.state = "cancelled_intent".into();
        save_helper(p, &h)?;
        Ok(json!({"helper":h,"execution_started":false}))
    }
}
fn helper_release(p: &Project, id: &str, evidence: &str) -> Result<Value> {
    owner()?;
    let mut h = load_helper(p, id)?;
    if h.output_id.as_deref() != Some(evidence) {
        return Err(error(
            "POLICY_DENIED",
            "Helper release requires exact observed output receipt",
            5,
        ));
    }
    let dir = host_dir(p)?;
    let mut job: Job = serde_json::from_slice(&fs::read(job_path(
        &dir,
        h.job_id
            .as_ref()
            .ok_or_else(|| error("RESOURCE_OWNER_UNKNOWN", "Helper has no observed job", 7))?,
    )?)?)?;
    if job.state != "finished" && !release(&dir, &mut job)? {
        return Err(error(
            "RESOURCE_OWNER_UNKNOWN",
            "Helper process group exit unproven",
            7,
        ));
    }
    h.state = "released_with_receipt".into();
    save_helper(p, &h)?;
    Ok(json!({"helper":h,"resources_released":true}))
}

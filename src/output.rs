//! Explicit noninteractive execution and masked, expiring output artifacts.
//! Presentation is never completion evidence; registered runner owns host admission.
use crate::{
    deadline::Deadline,
    domain::{Error, Result, hash, now},
    project::{Project, atomic_write, private_dir},
    reader,
};
use clap::{Args, Subcommand};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use crate::domain::id;
#[cfg(unix)]
use std::process::{Command, Stdio};
#[cfg(any(unix, test))]
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(any(unix, test))]
const RECORD_LIMIT: usize = 256 * 1024;
const STREAM_LIMIT: usize = 8 * 1024 * 1024;
#[cfg(any(unix, test))]
const CAPTURE_RECORD_LIMIT: usize = 65_536;
#[cfg(any(unix, test))]
const TAIL_RECORD_LIMIT: usize = 64;
#[cfg(any(unix, test))]
const TAIL_BYTE_LIMIT: usize = 512 * 1024;
const PROJECT_LIMIT: u64 = 256 * 1024 * 1024;
const HOST_LIMIT: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Args)]
pub struct RunRequest {
    #[arg(long)]
    pub task_id: Option<String>,
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long, default_value="temporary", value_parser=["temporary","none"])]
    pub retain: String,
    #[arg(long)]
    pub execution_timeout_ms: Option<u64>,
    #[arg(long, default_value_t = 8192)]
    pub budget_bytes: usize,
    #[arg(long, default_value="child", value_parser=["child","pctx"])]
    pub exit_policy: String,
    #[arg(long, default_value="closed", value_parser=["closed"])]
    pub stdin: String,
    #[arg(last = true, required = true)]
    pub argv: Vec<String>,
}
#[derive(Debug, Subcommand)]
pub enum TrustCommand {
    /// Inspect exact executable, arguments and script hashes without executing.
    Plan {
        #[arg(last = true, required = true)]
        argv: Vec<String>,
    },
    /// Explicitly bind this exact non-heavy execution in private local user trust.
    Add {
        #[arg(long)]
        expect_hash: String,
        #[arg(last = true, required = true)]
        argv: Vec<String>,
    },
}
#[derive(Debug, Subcommand)]
pub enum OutputCommand {
    Show {
        id: String,
        #[arg(long, default_value="compact", value_parser=["compact","full"])]
        view: String,
        #[arg(long, value_parser=["stdout","stderr"])]
        stream: Option<String>,
        #[arg(long)]
        lines: Option<String>,
    },
    Find {
        id: String,
        #[arg(long)]
        literal: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
    Render {
        id: String,
        #[arg(long, default_value = "builtin")]
        filter: String,
    },
}
#[derive(Clone, Serialize, Deserialize)]
struct Binding {
    executable: String,
    executable_hash: String,
    scripts: BTreeMap<String, String>,
    argv_hash: String,
    workspace_id: String,
    policy_hash: String,
    fingerprint: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Record {
    stream: String,
    sequence: u64,
    text: String,
    redacted: bool,
}
#[cfg(any(unix, test))]
#[derive(Clone, Default, Serialize, Deserialize)]
struct Capture {
    records: Vec<Record>,
    tail: VecDeque<Record>,
    tail_bytes: usize,
    next_sequence: u64,
    stored_estimate: usize,
    captured_bytes: u64,
    normalized_bytes: u64,
    redacted_bytes: u64,
    omitted_bytes: u64,
    complete: bool,
    redacted: bool,
    io_error: bool,
}
#[derive(Serialize, Deserialize)]
struct Artifact {
    schema_version: u32,
    output_id: String,
    execution_id: String,
    workspace_id: String,
    policy_hash: String,
    input_fingerprint: String,
    #[serde(default)]
    check_binding: Option<Value>,
    #[serde(default)]
    parser_identity: String,
    #[serde(default)]
    input_manifest: BTreeMap<String, String>,
    created_at: i64,
    expires_at: i64,
    records_hash: String,
    records: Vec<Record>,
    captured_bytes: u64,
    normalized_bytes: u64,
    redacted_bytes: u64,
    omitted_bytes: u64,
    capture_complete: bool,
    retained: bool,
    spawned: bool,
    termination: String,
    child_exit_code: Option<i32>,
    signal: Option<i32>,
    pctx_error: Option<String>,
    input_stage: String,
    emitted_bytes: u64,
    retrieval_bytes: u64,
    delivery_attempts: u64,
}
fn err(code: &str, msg: &str, exit: i32) -> Error {
    Error::new(code, msg, exit)
}
// All saved-query phases use the caller's immutable deadline. Execution does
// not acquire a query budget; its timeout and child truth remain independent.
fn query_project(p: &Project) -> Result<Project> {
    let mut scoped = p.clone();
    if scoped.deadline.is_none() {
        scoped.deadline = Some(Deadline::from_millis(10_000)?);
    }
    scoped.check_deadline()?;
    Ok(scoped)
}
fn phase<T>(p: &Project, operation: impl FnOnce() -> Result<T>) -> Result<T> {
    p.check_deadline()?;
    let result = operation();
    p.check_deadline()?;
    result
}
fn output_dir(p: &Project) -> PathBuf {
    p.data_dir.join("outputs").join(&p.workspace_id)
}
fn safe_id(value: &str) -> Result<()> {
    if !value.starts_with("OUT-")
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(err("INVALID_ARGUMENT", "Invalid output identifier", 2));
    }
    Ok(())
}
fn line_range(lines: Option<&str>) -> Result<(usize, usize)> {
    let Some(lines) = lines else {
        return Ok((1, 80));
    };
    let (first, last) = lines
        .split_once(':')
        .ok_or_else(|| err("INVALID_ARGUMENT", "Lines must be A:B", 2))?;
    let first = first
        .parse::<usize>()
        .map_err(|_| err("INVALID_ARGUMENT", "Invalid line", 2))?;
    let last = last
        .parse::<usize>()
        .map_err(|_| err("INVALID_ARGUMENT", "Invalid line", 2))?;
    if first == 0 || last < first || last - first > 1000 {
        return Err(err(
            "INVALID_ARGUMENT",
            "Invalid or excessive line range",
            2,
        ));
    }
    Ok((first, last))
}

/// Pure saved-output argument admission shared by CLI and producer.
pub fn validate_output_request(command: &OutputCommand) -> Result<()> {
    let id = match command {
        OutputCommand::Show { id, .. }
        | OutputCommand::Find { id, .. }
        | OutputCommand::Render { id, .. } => id,
    };
    safe_id(id)?;
    match command {
        OutputCommand::Show {
            view,
            stream,
            lines,
            ..
        } => {
            if !matches!(view.as_str(), "compact" | "full") {
                return Err(err("INVALID_ARGUMENT", "Unknown output view", 2));
            }
            if view == "compact" && (stream.is_some() || lines.is_some()) {
                return Err(err(
                    "INVALID_ARGUMENT",
                    "Output selectors require --view full",
                    2,
                ));
            }
            if stream
                .as_ref()
                .is_some_and(|value| !matches!(value.as_str(), "stdout" | "stderr"))
            {
                return Err(err("INVALID_ARGUMENT", "Unknown output stream", 2));
            }
            line_range(lines.as_deref())?;
        }
        OutputCommand::Find { literal, limit, .. }
            if literal.is_empty() || *limit == 0 || *limit > 1000 =>
        {
            return Err(err(
                "INVALID_ARGUMENT",
                "Literal and bounded limit required",
                2,
            ));
        }
        _ => {}
    }
    Ok(())
}
fn checked_private(path: &Path) -> Result<()> {
    for c in path.ancestors() {
        if fs::symlink_metadata(c).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(err(
                "POLICY_DENIED",
                "Linked local artifact/trust path denied",
                5,
            ));
        }
    }
    Ok(())
}
fn artifact_path(p: &Project, value: &str) -> Result<PathBuf> {
    safe_id(value)?;
    let path = output_dir(p).join(format!("{value}.json"));
    checked_private(&path)?;
    Ok(path)
}
fn resolve(program: &str) -> Result<PathBuf> {
    let candidate = if Path::new(program).is_absolute() {
        PathBuf::from(program)
    } else if program.contains('/') || program.contains('\\') {
        return Err(err(
            "POLICY_DENIED",
            "Use an absolute executable path or PATH program",
            5,
        ));
    } else {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|d| d.join(program))
            .find(|f| f.is_file())
            .ok_or_else(|| err("CAPABILITY_UNAVAILABLE", "Executable not found", 6))?
    };
    let path = fs::canonicalize(candidate)?;
    let m = fs::metadata(&path)?;
    if !m.is_file() || m.len() > 128 * 1024 * 1024 {
        return Err(err(
            "POLICY_DENIED",
            "Executable is not a bounded regular file",
            5,
        ));
    }
    Ok(path)
}
fn classification(_argv: &[String], executable: &Path) -> Result<()> {
    let name = executable
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let denied = [
        "sh",
        "bash",
        "zsh",
        "dash",
        "fish",
        "cmd",
        "cmd.exe",
        "powershell",
        "pwsh",
        "python",
        "python3",
        "node",
        "ruby",
        "perl",
        "cargo",
        "rustc",
        "make",
        "cmake",
        "ninja",
        "xcodebuild",
        "npm",
        "pnpm",
        "yarn",
        "npx",
        "docker",
        "terraform",
        "az",
        "kubectl",
        "gh",
        "eas",
        "rtk",
        "tokf",
    ];
    if denied.iter().any(|n| {
        name == *n || name.starts_with(&format!("{n}-")) || name.starts_with(&format!("{n}."))
    }) {
        return Err(err(
            "CAPABILITY_UNAVAILABLE",
            "Shells, opaque interpreters, compressors and heavy/operational runners need a registered execution profile",
            6,
        ));
    }
    if name == "git" {
        return Err(err(
            "CAPABILITY_UNAVAILABLE",
            "Git helpers/configuration require a validated execution profile",
            6,
        ));
    }
    // Exact user trust is required for other programs; it is not broad host permission.
    Ok(())
}
fn binding(p: &Project, argv: &[String]) -> Result<Binding> {
    binding_inner(p, argv, false, ".")
}
fn validate_argv(argv: &[String]) -> Result<()> {
    if argv.is_empty() || argv.len() > 256 || argv.iter().map(String::len).sum::<usize>() > 65536 {
        return Err(err("INVALID_ARGUMENT", "Bounded nonempty argv required", 2));
    }
    Ok(())
}
/// Existing pure execution modes and argv grammar, without binding or authority checks.
pub fn validate_run_request(r: &RunRequest) -> Result<()> {
    if r.stdin != "closed"
        || !["temporary", "none"].contains(&r.retain.as_str())
        || !["child", "pctx"].contains(&r.exit_policy.as_str())
    {
        return Err(err("INVALID_ARGUMENT", "Unsupported run mode", 2));
    }
    validate_argv(&r.argv)
}
pub fn validate_trust_request(command: &TrustCommand) -> Result<()> {
    match command {
        TrustCommand::Plan { argv } | TrustCommand::Add { argv, .. } => validate_argv(argv),
    }
}
#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) enum HashPhase {
    ChunkRead,
    ChunkHashed,
}
#[cfg(test)]
type HashObserver = Box<dyn FnMut(HashPhase, usize, Option<Deadline>)>;
#[cfg(test)]
thread_local! {static HASH_OBSERVER:std::cell::RefCell<Option<HashObserver>>=const {std::cell::RefCell::new(None)};}
#[cfg(test)]
pub(crate) struct HashObserverGuard;
#[cfg(test)]
impl Drop for HashObserverGuard {
    fn drop(&mut self) {
        HASH_OBSERVER.with(|s| *s.borrow_mut() = None);
    }
}
#[cfg(test)]
pub(crate) fn observe_hash(
    callback: impl FnMut(HashPhase, usize, Option<Deadline>) + 'static,
) -> HashObserverGuard {
    HASH_OBSERVER.with(|s| {
        assert!(s.borrow().is_none());
        *s.borrow_mut() = Some(Box::new(callback));
    });
    HashObserverGuard
}
#[cfg(test)]
fn hash_observation(phase: HashPhase, bytes: usize, p: &Project) {
    HASH_OBSERVER.with(|s| {
        if let Some(callback) = s.borrow_mut().as_mut() {
            callback(phase, bytes, p.deadline);
        }
    });
}
/// External native executable domain, distinct from project-source policy.
/// Retains the regular-file authority throughout bounded streaming hashing.
pub(crate) fn hash_executable(p: &Project, path: &Path) -> Result<String> {
    hash_executable_with_limit_error(p, path, |message| err("POLICY_DENIED", message, 5))
}
pub(crate) fn hash_executable_with_limit(
    p: &Project,
    path: &Path,
    max_bytes: usize,
) -> Result<String> {
    hash_executable_bounded(p, path, max_bytes, |message| {
        err("POLICY_DENIED", message, 5)
    })
}
/// Domain-specific limit errors are constructed only at the actual size guard.
/// Authority, I/O, identity and timeout errors keep their original classification.
pub(crate) fn hash_executable_with_limit_error(
    p: &Project,
    path: &Path,
    limit_error: impl Fn(&'static str) -> Error,
) -> Result<String> {
    hash_executable_bounded(p, path, 128 * 1024 * 1024, limit_error)
}
fn hash_executable_bounded(
    p: &Project,
    path: &Path,
    max_bytes: usize,
    limit_error: impl Fn(&'static str) -> Error,
) -> Result<String> {
    use sha2::{Digest, Sha256};
    p.check_deadline()?;
    if max_bytes == 0 || max_bytes > 256 * 1024 * 1024 {
        return Err(err(
            "INVALID_ARGUMENT",
            "Unsupported executable hash bound",
            2,
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| err("POLICY_DENIED", "Invalid executable authority", 5))?;
    let leaf = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| err("POLICY_DENIED", "Invalid executable authority", 5))?;
    let anchor = crate::project::RootAnchor::capture(parent)?;
    p.check_deadline()?;
    let mut file = reader::anchored_open_deadline(parent, &anchor, leaf, p.deadline)?;
    let before = file.metadata()?;
    if !before.is_file() {
        return Err(err(
            "POLICY_DENIED",
            "Executable is not a bounded regular file",
            5,
        ));
    }
    if before.len() > max_bytes as u64 {
        return Err(limit_error("Executable is not a bounded regular file"));
    }
    let identity = same_file::Handle::from_file(file.try_clone()?)?;
    let mut sha = Sha256::new();
    let mut chunk = [0; 65536];
    let mut total = 0usize;
    loop {
        p.check_deadline()?;
        let count = file.read(&mut chunk);
        #[cfg(test)]
        if let Ok(count) = &count
            && *count > 0
        {
            hash_observation(HashPhase::ChunkRead, *count, p);
        }
        p.check_deadline()?;
        let count = count?;
        if count == 0 {
            break;
        }
        total += count;
        if total > max_bytes {
            return Err(limit_error("Executable exceeds size bound"));
        }
        sha.update(&chunk[..count]);
        #[cfg(test)]
        hash_observation(HashPhase::ChunkHashed, count, p);
    }
    p.check_deadline()?;
    let after = file.metadata()?;
    let reopened = reader::anchored_open_deadline(parent, &anchor, leaf, p.deadline)?;
    let current = reopened.metadata()?;
    if identity != same_file::Handle::from_file(reopened)?
        || before.len() != after.len()
        || before.len() != current.len()
        || before.modified().ok() != after.modified().ok()
        || before.modified().ok() != current.modified().ok()
    {
        return Err(err("CONFIG_CHANGED", "Executable changed while hashing", 9));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (before.ctime(), before.ctime_nsec()) != (after.ctime(), after.ctime_nsec())
            || (before.ctime(), before.ctime_nsec()) != (current.ctime(), current.ctime_nsec())
        {
            return Err(err("CONFIG_CHANGED", "Executable changed while hashing", 9));
        }
    }
    p.check_deadline()?;
    Ok(format!("{:x}", sha.finalize()))
}
fn binding_inner(p: &Project, argv: &[String], registered: bool, cwd: &str) -> Result<Binding> {
    p.check_deadline()?;
    validate_argv(argv)?;
    let executable = phase(p, || resolve(&argv[0]))?;
    if !registered {
        classification(argv, &executable)?;
    }
    let mut scripts = BTreeMap::new();
    for arg in argv.iter().skip(1) {
        p.check_deadline()?;
        let path = Path::new(arg);
        if path.is_absolute() && path.exists() {
            if !registered {
                return Err(err(
                    "POLICY_DENIED",
                    "Absolute file inputs require a registered execution profile",
                    5,
                ));
            }
            let relative = path.strip_prefix(&p.root).map_err(|_| {
                err(
                    "POLICY_DENIED",
                    "Registered absolute input is outside the project",
                    5,
                )
            })?;
            let relative = relative
                .to_str()
                .ok_or_else(|| err("INVALID_ARGUMENT", "Input path is not UTF-8", 2))?;
            let file = reader::read(p, relative)?;
            scripts.insert(relative.into(), file.hash);
            continue;
        }
        let input = if cwd == "." {
            arg.clone()
        } else {
            format!("{cwd}/{arg}")
        };
        if !arg.starts_with('-') && p.root.join(&input).is_file() {
            let f = reader::read(p, &input)?;
            scripts.insert(input, f.hash);
        }
    }
    let executable_hash = hash_executable(p, &executable)?;
    let argv_hash = hash(serde_json::to_vec(argv)?);
    let workspace_id = p.workspace_id.clone();
    let policy_hash = p.policy_hash();
    let fingerprint = hash(serde_json::to_vec(
        &json!({"executable":executable.to_string_lossy(),"executable_hash":executable_hash,"scripts":scripts,"argv_hash":argv_hash,"workspace":workspace_id,"policy":policy_hash,"cwd":cwd}),
    )?);
    p.check_deadline()?;
    Ok(Binding {
        executable: executable.to_string_lossy().into_owned(),
        executable_hash,
        scripts,
        argv_hash,
        workspace_id,
        policy_hash,
        fingerprint,
    })
}
pub fn trust(p: &Project, command: &TrustCommand) -> Result<Value> {
    if !matches!(command, TrustCommand::Plan { .. }) {
        return trust_inner(p, command);
    }
    let mut scope = p.clone();
    if scope.deadline.is_none() {
        scope.deadline = Some(crate::deadline::Deadline::from_millis(10000)?);
    }
    scope.check_deadline()?;
    let result = trust_inner(&scope, command);
    scope.check_deadline()?;
    result
}
fn trust_inner(p: &Project, command: &TrustCommand) -> Result<Value> {
    p.check_deadline()?;
    validate_trust_request(command)?;
    if matches!(command, TrustCommand::Add { .. })
        && std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()) != "owner"
    {
        return Err(err(
            "POLICY_DENIED",
            "Execution trust requires local owner authority",
            5,
        ));
    }
    let argv = match command {
        TrustCommand::Plan { argv } | TrustCommand::Add { argv, .. } => argv,
    };
    let b = binding(p, argv)?;
    if let TrustCommand::Add { expect_hash, .. } = command {
        if *expect_hash != b.fingerprint {
            return Err(err(
                "CONFIG_CHANGED",
                "Execution binding changed since plan",
                9,
            ));
        }
        let dir = p.data_dir.join("trust").join("executions");
        checked_private(&dir)?;
        private_dir(&dir)?;
        atomic_write(
            &dir.join(format!("{}.json", b.fingerprint)),
            &serde_json::to_vec(&b)?,
            true,
        )?;
    }
    Ok(
        json!({"fingerprint":b.fingerprint,"executable_hash":b.executable_hash,"script_hashes":b.scripts,"argv_hash":b.argv_hash,"workspace_id":b.workspace_id,"policy_hash":b.policy_hash,"trusted":matches!(command,TrustCommand::Add{..}),"execution_started":false,"classification":"explicit_non_heavy","host_permission":"separate"}),
    )
}
#[cfg(unix)]
fn validate_trust(p: &Project, b: &Binding) -> Result<()> {
    let path = p
        .data_dir
        .join("trust/executions")
        .join(format!("{}.json", b.fingerprint));
    checked_private(&path)?;
    let raw = fs::read(path).map_err(|_| {
        err(
            "OWNER_DECISION_REQUIRED",
            "Exact executable/argv/script binding requires explicit trust plan and add",
            5,
        )
    })?;
    let old: Binding =
        crate::domain::stored_json_bytes(&raw, "Stored execution trust JSON is invalid")?;
    if old.fingerprint != b.fingerprint
        || old.executable_hash != b.executable_hash
        || old.scripts != b.scripts
        || old.argv_hash != b.argv_hash
        || old.workspace_id != p.workspace_id
        || old.policy_hash != p.policy_hash()
    {
        return Err(err(
            "CONFIG_CHANGED",
            "Execution trust binding no longer matches",
            9,
        ));
    }
    Ok(())
}
#[cfg(any(unix, test))]
fn normalize(bytes: &[u8]) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    // Remove ANSI controls before masking, including split escape sequences in a record.
    static ANSI: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let ansi = ANSI.get_or_init(|| {
        regex::Regex::new(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\))")
            .expect("fixed ANSI pattern")
    });
    let clean = ansi.replace_all(text, "");
    Some(
        clean
            .chars()
            .filter(|c| *c == '\t' || !c.is_control())
            .collect(),
    )
}
#[cfg(any(unix, test))]
fn push_record(c: &mut Capture, bytes: &[u8], stream: &str, nul_framed: bool) {
    let sequence = c.next_sequence;
    c.next_sequence = c.next_sequence.saturating_add(1);
    let normalized = if nul_framed {
        std::str::from_utf8(bytes).ok().map(str::to_owned)
    } else {
        normalize(bytes)
    };
    if let Some(text) = normalized {
        c.normalized_bytes += text.len() as u64 + u64::from(!nul_framed);
        if text.contains("-----BEGIN") && text.contains("PRIVATE KEY-----") {
            c.omitted_bytes += bytes.len() as u64;
            c.complete = false;
            return;
        }
        let (text, redacted) = if text.is_empty() {
            (text, false)
        } else {
            reader::redact(&text)
        };
        // The denominator is observed masked output, including bounded-storage omissions.
        c.redacted_bytes = c
            .redacted_bytes
            .saturating_add(text.len() as u64 + u64::from(!nul_framed));
        c.redacted |= redacted;
        let cost = text.len() + 128;
        let record = Record {
            stream: stream.into(),
            sequence,
            text,
            redacted,
        };
        if c.records.len() < CAPTURE_RECORD_LIMIT
            && c.stored_estimate + cost <= STREAM_LIMIT - TAIL_BYTE_LIMIT
        {
            c.stored_estimate += cost;
            c.records.push(record);
        } else {
            c.complete = false;
            c.tail_bytes += cost;
            c.tail.push_back(record);
            while c.tail.len() > TAIL_RECORD_LIMIT || c.tail_bytes > TAIL_BYTE_LIMIT {
                if let Some(omitted) = c.tail.pop_front() {
                    c.tail_bytes = c.tail_bytes.saturating_sub(omitted.text.len() + 128);
                    c.omitted_bytes = c
                        .omitted_bytes
                        .saturating_add(omitted.text.len() as u64 + 1);
                }
            }
        }
    } else {
        c.omitted_bytes += bytes.len() as u64;
        c.complete = false;
    }
}

#[cfg(test)]
fn capture<R: Read + Send + 'static>(
    pipe: R,
    stream: &'static str,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<Capture> {
    capture_framed(pipe, stream, stop, false)
}
#[cfg(any(unix, test))]
fn capture_framed<R: Read + Send + 'static>(
    mut pipe: R,
    stream: &'static str,
    stop: Arc<AtomicBool>,
    nul_framed: bool,
) -> thread::JoinHandle<Capture> {
    thread::spawn(move || {
        let mut c = Capture {
            complete: true,
            ..Default::default()
        };
        let mut record = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut dropping = false;
        let mut pem_block = false;
        let mut stopped_at = None;
        loop {
            if stop.load(Ordering::Relaxed) && stopped_at.is_none() {
                stopped_at = Some(Instant::now());
            }
            match pipe.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    c.captured_bytes += n as u64;
                    for byte in &chunk[..n] {
                        if (nul_framed && *byte == 0)
                            || (!nul_framed && (*byte == b'\n' || *byte == b'\r'))
                        {
                            if nul_framed && !dropping {
                                record.push(0);
                            }
                            if !dropping {
                                let marker = String::from_utf8_lossy(&record);
                                if marker.contains("-----BEGIN")
                                    && marker.contains("PRIVATE KEY-----")
                                {
                                    pem_block = true;
                                }
                                if pem_block {
                                    c.omitted_bytes += record.len() as u64;
                                    c.complete = false;
                                    if marker.contains("-----END")
                                        && marker.contains("PRIVATE KEY-----")
                                    {
                                        pem_block = false;
                                    }
                                } else {
                                    push_record(&mut c, &record, stream, nul_framed);
                                }
                            }
                            record.clear();
                            dropping = false;
                        } else if dropping {
                            c.omitted_bytes += 1;
                        } else if record.len() >= RECORD_LIMIT {
                            if std::str::from_utf8(&record).is_ok_and(|s| {
                                s.contains("-----BEGIN") && s.contains("PRIVATE KEY-----")
                            }) {
                                pem_block = true;
                            }
                            c.omitted_bytes += record.len() as u64 + 1;
                            record.clear();
                            dropping = true;
                            c.complete = false;
                        } else {
                            record.push(*byte);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if stopped_at.is_some_and(|t| t.elapsed() > Duration::from_millis(250)) {
                        c.complete = false;
                        break;
                    }
                    thread::sleep(Duration::from_millis(2));
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    c.complete = false;
                    c.io_error = true;
                    break;
                }
            }
        }
        if !dropping && !record.is_empty() {
            if pem_block {
                c.omitted_bytes += record.len() as u64;
                c.complete = false;
            } else {
                push_record(&mut c, &record, stream, nul_framed);
            }
        }
        c.records.extend(c.tail.drain(..));
        c
    })
}
#[cfg(unix)]
fn nonblocking<T: std::os::fd::AsRawFd>(pipe: &T) -> Result<()> {
    let fd = pipe.as_raw_fd();
    // SAFETY: valid owned pipe descriptor; these fcntl operations do not transfer ownership.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(err("IO_ERROR", "Cannot configure bounded pipe capture", 7));
    }
    Ok(())
}
fn total_size(p: &Project, path: &Path) -> Result<u64> {
    let entries = phase(p, || Ok(fs::read_dir(path)?))?;
    let mut size = 0u64;
    for entry in entries {
        p.check_deadline()?;
        let entry = phase(p, || Ok(entry?))?;
        let kind = phase(p, || Ok(entry.file_type()?))?;
        let bytes = if kind.is_dir() {
            total_size(p, &entry.path())?
        } else if kind.is_file() {
            phase(p, || Ok(entry.metadata()?.len()))?
        } else {
            0
        };
        size = size.saturating_add(bytes);
    }
    p.check_deadline()?;
    Ok(size)
}
// Explicit unlock prevents a concurrent fork in another thread from briefly
// retaining an inherited open-file-description lock after this scope returns.
struct UnlockGuard(fs::File);
impl Drop for UnlockGuard {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}
fn save(p: &Project, a: &Artifact) -> Result<()> {
    phase(p, || save_inner(p, a))
}
fn save_inner(p: &Project, a: &Artifact) -> Result<()> {
    p.check_deadline()?;
    let top = p.data_dir.join("outputs");
    checked_private(&top)?;
    private_dir(&top)?;
    private_dir(&output_dir(p))?;
    let lock_path = top.join("store.lock");
    checked_private(&lock_path)?;
    let mut opts = fs::OpenOptions::new();
    opts.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let lock = UnlockGuard(opts.open(lock_path)?);
    lock.0
        .try_lock_exclusive()
        .map_err(|_| err("RESOURCE_BUSY", "Output publication is busy", 7))?;
    let path = artifact_path(p, &a.output_id)?;
    let bytes = phase(p, || Ok(serde_json::to_vec(a)?))?;
    let previous = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if total_size(p, &output_dir(p))?.saturating_sub(previous) + bytes.len() as u64 > PROJECT_LIMIT
        || total_size(p, &top)?.saturating_sub(previous) + bytes.len() as u64 > HOST_LIMIT
    {
        return Err(err(
            "OUTPUT_PARTIAL",
            "Output store quota reached; captured body is unavailable",
            3,
        ));
    }
    phase(p, || atomic_write(&path, &bytes, true))?;
    Ok(())
}
fn load(p: &Project, value: &str) -> Result<Artifact> {
    phase(p, || load_inner(p, value))
}
fn load_inner(p: &Project, value: &str) -> Result<Artifact> {
    use std::io::Read;
    const MAX_BYTES: usize = 64 * 1024 * 1024;
    let path = phase(p, || artifact_path(p, value))?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let mut file = phase(p, || {
        options
            .open(&path)
            .map_err(|_| err("OUTPUT_EXPIRED", "Output artifact unavailable", 6))
    })?;
    let metadata = phase(p, || Ok(file.metadata()?))?;
    if !metadata.is_file() {
        return Err(err(
            "POLICY_DENIED",
            "Output artifact must be a regular file",
            5,
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::{fs::MetadataExt, io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_TYPE_DISK, GetFileType,
        };
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || unsafe { GetFileType(file.as_raw_handle().cast()) } != FILE_TYPE_DISK
        {
            return Err(err(
                "POLICY_DENIED",
                "Output artifact is not a regular disk file",
                5,
            ));
        }
    }
    if metadata.len() > MAX_BYTES as u64 {
        return Err(err(
            "OUTPUT_PARTIAL",
            "Output artifact exceeds safe read limit",
            3,
        ));
    }
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 65536];
    loop {
        let capacity = chunk.len().min(MAX_BYTES + 1 - bytes.len());
        let count = phase(p, || Ok(file.read(&mut chunk[..capacity])?))?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > MAX_BYTES {
            return Err(err(
                "OUTPUT_PARTIAL",
                "Output artifact exceeds safe read limit",
                3,
            ));
        }
    }
    let a: Artifact = phase(p, || {
        crate::domain::stored_json_bytes(&bytes, "Stored output artifact JSON is invalid")
    })?;
    if a.schema_version != 1 || a.output_id != value || a.workspace_id != p.workspace_id {
        return Err(err(
            "POLICY_DENIED",
            "Output identity or workspace mismatch",
            5,
        ));
    }
    if a.policy_hash != p.policy_hash() {
        return Err(err(
            "POLICY_DENIED",
            "Output policy changed; old records require refiltering",
            5,
        ));
    }
    if a.expires_at <= now() {
        return Err(err("OUTPUT_EXPIRED", "Output retention has expired", 6));
    }
    if a.records_hash != phase(p, || Ok(hash(serde_json::to_vec(&a.records)?)))? {
        return Err(err("OUTPUT_PARTIAL", "Output integrity check failed", 3));
    }
    Ok(a)
}
fn typed_report(a: &Artifact) -> Option<Value> {
    if !a.capture_complete || a.pctx_error.is_some() {
        return None;
    }
    let text = a
        .records
        .iter()
        .filter(|r| r.stream == "stdout")
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if text.len() > STREAM_LIMIT {
        return None;
    }
    let r: crate::work::CheckReport = serde_json::from_str(&text).ok()?;
    if r.schema_version != 1
        || r.check_key.trim().is_empty()
        || r.producer.trim().is_empty()
        || r.finished_at < r.started_at
        || r.passed
            .checked_add(r.failed)
            .and_then(|n| n.checked_add(r.skipped))
            != Some(r.tests)
        || !["passed", "failed", "cancelled", "timed_out", "unverified"]
            .contains(&r.result.as_str())
        || a.child_exit_code != Some(r.exit_code)
    {
        return None;
    }
    Some(
        json!({"parser":"pctx-check-report-v1","summary":{"tests":r.tests,"passed":r.passed,"failed":r.failed,"errors":r.errors,"skipped":r.skipped},"reported_result":r.result,"report_origin":"child_stdout_claim","report_digest":hash(text.as_bytes()),"gate_evidence":false}),
    )
}
fn compact(p: &Project, a: &Artifact, budget: usize) -> Result<Value> {
    p.check_deadline()?;
    let mut protected = Vec::new();
    let mut normal = Vec::new();
    let protect =
        regex::Regex::new(r"(?i)error|failed|failure|warning|denied|timeout|panic").unwrap();
    for r in &a.records {
        p.check_deadline()?;
        if protect.is_match(&r.text) {
            protected.push(r)
        } else {
            normal.push(r)
        }
    }
    let total = a.records.len();
    let mut included = Vec::new();
    let mut used = 0;
    // Reserve metadata/envelope first; UTF-8 is never byte-truncated.
    let room = budget.saturating_sub(2500);
    for r in protected.iter().chain(normal.iter().take(12)) {
        p.check_deadline()?;
        let value =
            json!({"stream":r.stream,"sequence":r.sequence,"text":r.text,"redacted":r.redacted});
        let cost = serde_json::to_vec(&value)
            .map(|v| v.len())
            .unwrap_or(room + 1);
        if used + cost <= room {
            used += cost;
            included.push(value);
        }
    }
    let mut value = json!({"execution_id":a.execution_id,"output_id":a.output_id,"spawned":a.spawned,"termination":a.termination,"child_exit_code":a.child_exit_code,"signal":a.signal,"pctx_error":a.pctx_error,"parse_status":"unsupported","capture_complete":a.capture_complete,"redaction_applied":true,"redaction_changed_content":a.records.iter().any(|r|r.redacted),"input_stage":a.input_stage,"raw_available":a.retained && a.expires_at>now(),"raw_semantics":"redacted_uncompressed","records":included,"records_included":included.len(),"records_omitted":total.saturating_sub(included.len()),"protected_records":protected.len(),"captured_bytes":a.captured_bytes,"normalized_bytes":a.normalized_bytes,"redacted_bytes":a.redacted_bytes,"baseline_completeness":if a.capture_complete{"complete"}else{"partial"},"omitted_bytes":a.omitted_bytes,"query_ref":format!("pctx output show {} --view full",a.output_id),"evidence_origin":"runner_observed","task_completion":"not_evaluated","test_result":"not_evaluated","budget_bytes":budget});
    let report = phase(p, || Ok(typed_report(a)))?;
    if let Some(report) = report {
        value["parse_status"] = json!("complete");
        value["typed_report"] = report;
    }
    if !a.parser_identity.is_empty()
        && a.parser_identity != "unsupported"
        && a.parser_identity != "pctx-json-v1"
    {
        let records = a
            .records
            .iter()
            .take(8193)
            .map(|r| json!({"stream":r.stream,"sequence":r.sequence,"text":r.text}))
            .collect::<Vec<_>>();
        p.check_deadline()?;
        let parsed = crate::parsers::parse(
            &a.parser_identity,
            &records,
            a.child_exit_code,
            &a.termination,
            a.capture_complete && a.pctx_error.is_none(),
        );
        p.check_deadline()?;
        value["parse_status"] = parsed["parse_status"].clone();
        value["parser"] = parsed;
        let mut omitted = serde_json::Map::new();
        // Rendering omissions are distinct from incomplete parsing and capture.
        let parser_room = budget.saturating_sub(2500) / 2;
        while serde_json::to_vec(&value["parser"]).is_ok_and(|v| v.len() > parser_room) {
            p.check_deadline()?;
            let key = ["diagnostics", "items", "reasons"]
                .into_iter()
                .filter(|key| {
                    value["parser"][*key]
                        .as_array()
                        .is_some_and(|v| !v.is_empty())
                })
                .max_by_key(|key| {
                    serde_json::to_vec(&value["parser"][*key])
                        .map(|v| v.len())
                        .unwrap_or(0)
                });
            let Some(key) = key else {
                break;
            };
            value["parser"][key].as_array_mut().unwrap().pop();
            let count = omitted.get(key).and_then(Value::as_u64).unwrap_or(0) + 1;
            omitted.insert(key.into(), json!(count));
        }
        value["parser"]["presentation_omissions"] = Value::Object(omitted);
        value["parser"]["query_ref"] =
            json!(format!("pctx output show {} --view full", a.output_id));
    }
    p.check_deadline()?;
    Ok(value)
}
pub fn run(p: &Project, r: &RunRequest) -> Result<Value> {
    run_inner(p, r, ".", &BTreeMap::new(), None, None, None, None, None)
}

/// Frontend observation preserves admission failures separately from child truth.
/// The callback attests native spawn; post-spawn failures cannot claim not_started.
pub fn run_cli(p: &Project, r: &RunRequest) -> Value {
    let mut spawned = false;
    let result = {
        let mut observed = |_pid| {
            spawned = true;
            Ok(())
        };
        run_inner(
            p,
            r,
            ".",
            &BTreeMap::new(),
            None,
            Some(&mut observed),
            None,
            None,
            None,
        )
    };
    match result {
        Ok(data) => data,
        Err(error) => json!({
            "spawned":spawned,
            "termination":if spawned { "unknown" } else { "not_started" },
            "child_exit_code":null,
            "signal":null,
            "pctx_error":error.code,
            "processing_error":error,
            "processing_exit":error.exit,
            "raw_available":false,
            "task_completion":"not_evaluated",
            "test_result":"not_evaluated"
        }),
    }
}

/// Processing failures take precedence over shell exit propagation.
pub fn execution_error(data: &Value) -> Option<Error> {
    let code = data["pctx_error"].as_str()?;
    let exit = data["processing_exit"].as_i64().unwrap_or(7) as i32;
    let message = data["processing_error"]["message"]
        .as_str()
        .unwrap_or("Execution processing failed");
    Some(Error::new(code, message, exit))
}
pub(crate) fn registered_environment_fingerprint(
    environment: &BTreeMap<String, String>,
) -> Result<String> {
    registered_environment_fingerprint_with_path(
        environment,
        &std::env::var_os("PATH").unwrap_or_default(),
    )
}
fn registered_environment_fingerprint_with_path(
    environment: &BTreeMap<String, String>,
    path: &std::ffi::OsStr,
) -> Result<String> {
    let mut effective = BTreeMap::from([
        ("LANG".to_string(), "C.UTF-8".to_string()),
        ("GIT_PAGER".into(), "cat".into()),
        ("PAGER".into(), "cat".into()),
        ("GIT_EXTERNAL_DIFF".into(), "".into()),
        ("GIT_CONFIG_COUNT".into(), "0".into()),
    ]);
    effective.extend(environment.clone());
    Ok(hash(serde_json::to_vec(
        &json!({"environment":effective,"path_bytes_hash":hash(path.as_encoded_bytes())}),
    )?))
}
pub(crate) fn registered_binding_at(p: &Project, argv: &[String], cwd: &str) -> Result<Value> {
    Ok(serde_json::to_value(binding_inner(p, argv, true, cwd)?)?)
}
pub(crate) fn run_registered_monitored(
    p: &Project,
    r: &RunRequest,
    cwd: &str,
    environment: &BTreeMap<String, String>,
    expected_binding: &str,
    on_spawn: &mut dyn FnMut(u32) -> Result<()>,
    on_poll: &mut dyn FnMut() -> Result<()>,
) -> Result<Value> {
    run_inner(
        p,
        r,
        cwd,
        environment,
        Some(expected_binding),
        Some(on_spawn),
        Some(on_poll),
        None,
        None,
    )
}
// Parser selection is presentation-only and bound by the registered profile.
#[allow(clippy::too_many_arguments)]
#[cfg(unix)]
pub(crate) fn run_registered_parsed(
    p: &Project,
    r: &RunRequest,
    cwd: &str,
    environment: &BTreeMap<String, String>,
    expected_binding: &str,
    on_spawn: &mut dyn FnMut(u32) -> Result<()>,
    on_poll: &mut dyn FnMut() -> Result<()>,
    parser: &str,
    check_binding: &Value,
) -> Result<Value> {
    run_inner(
        p,
        r,
        cwd,
        environment,
        Some(expected_binding),
        Some(on_spawn),
        Some(on_poll),
        Some(parser),
        Some(check_binding),
    )
}
#[allow(clippy::too_many_arguments)]
fn run_inner(
    p: &Project,
    r: &RunRequest,
    cwd: &str,
    environment: &BTreeMap<String, String>,
    expected_binding: Option<&str>,
    on_spawn: Option<&mut dyn FnMut(u32) -> Result<()>>,
    on_poll: Option<&mut dyn FnMut() -> Result<()>>,
    parser_identity: Option<&str>,
    check_binding: Option<&Value>,
) -> Result<Value> {
    p.check_deadline()?;
    if r.budget_bytes < 3000 {
        return Err(err(
            "BUDGET_TOO_SMALL",
            "Run metadata requires at least 3000 bytes before execution",
            8,
        ));
    }
    validate_run_request(r)?;
    #[cfg(not(unix))]
    {
        let _ = (
            p,
            cwd,
            environment,
            expected_binding,
            on_spawn,
            on_poll,
            parser_identity,
            check_binding,
        );
        Err(err(
            "CAPABILITY_UNAVAILABLE",
            "Supervised manual capture is currently verified only on Unix",
            6,
        ))
    }
    #[cfg(unix)]
    {
        let mut on_spawn = on_spawn;
        let mut on_poll = on_poll;
        let registered = expected_binding.is_some();
        let execution_path = std::env::var_os("PATH").unwrap_or_default();
        let b = binding_inner(p, &r.argv, registered, cwd)?;
        if let Some(expected) = expected_binding {
            if b.fingerprint != expected {
                return Err(err(
                    "CONFIG_CHANGED",
                    "Registered binding changed before spawn",
                    9,
                ));
            }
        } else {
            validate_trust(p, &b)?;
        }
        if let Some(receipt) = check_binding {
            let key = receipt["key"]
                .as_str()
                .ok_or_else(|| err("INVALID_CONFIG", "Missing check binding key", 2))?;
            if crate::runner::current_check_binding(p, key)? != *receipt
                || receipt["execution_fingerprint"] != b.fingerprint
                || receipt["cwd"] != cwd
                || receipt["environment_fingerprint"]
                    != registered_environment_fingerprint_with_path(environment, &execution_path)?
            {
                return Err(err(
                    "CONFIG_CHANGED",
                    "Trusted runner environment changed before spawn",
                    9,
                ));
            }
        }
        let input_manifest = reader::manifest(p)?;
        let input_fingerprint = hash(serde_json::to_vec(&input_manifest)?);
        let execution_id = id("EXEC");
        let output_id = id("OUT");
        let job_dir = p.data_dir.join("output-jobs");
        checked_private(&job_dir)?;
        private_dir(&job_dir)?;
        let job_path = job_dir.join(format!("{execution_id}.json"));
        atomic_write(
            &job_path,
            &serde_json::to_vec(
                &json!({"execution_id":execution_id,"state":"starting","workspace_id":p.workspace_id,"input_fingerprint":input_fingerprint,"resource_admission":if registered{"registered_profile_external_resource_ledger"}else{"non_heavy_manual"}}),
            )?,
            false,
        )?;
        let mut cmd = Command::new(&b.executable);
        cmd.args(&r.argv[1..])
            .current_dir(&p.root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();
        // Git metadata reads must not invoke external pagers, diff helpers or textconv.
        cmd.env("PATH", &execution_path)
            .env("LANG", "C.UTF-8")
            .env("GIT_PAGER", "cat")
            .env("PAGER", "cat")
            .env("GIT_EXTERNAL_DIFF", "")
            .env("GIT_CONFIG_COUNT", "0");
        for (key, value) in environment {
            cmd.env(key, value);
        }
        #[cfg(unix)]
        let cwd_handle = if cwd == "." {
            fs::File::open(&p.root)?
        } else {
            reader::secure_open(p, cwd)?
        };
        #[cfg(unix)]
        {
            use std::{os::fd::AsRawFd, os::unix::process::CommandExt};
            let fd = cwd_handle.as_raw_fd();
            cmd.process_group(0);
            unsafe {
                cmd.pre_exec(move || {
                    if libc::fchdir(fd) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        // Manifest reads and admission setup can outlive the initial binding check.
        // Revalidate at the last parent-side boundary before admitting a child.
        let final_binding = (|| -> Result<()> {
            let current = binding_inner(p, &r.argv, registered, cwd)?;
            if current.fingerprint != b.fingerprint {
                return Err(err(
                    "CONFIG_CHANGED",
                    "Execution binding changed before spawn",
                    9,
                ));
            }
            if !registered {
                validate_trust(p, &current)?;
            }
            if let Some(receipt) = check_binding {
                let key = receipt["key"]
                    .as_str()
                    .ok_or_else(|| err("INVALID_CONFIG", "Missing check binding key", 2))?;
                if crate::runner::current_check_binding(p, key)? != *receipt
                    || receipt["execution_fingerprint"] != current.fingerprint
                    || receipt["cwd"] != cwd
                    || receipt["environment_fingerprint"]
                        != registered_environment_fingerprint_with_path(
                            environment,
                            &execution_path,
                        )?
                {
                    return Err(err(
                        "CONFIG_CHANGED",
                        "Trusted runner environment changed before spawn",
                        9,
                    ));
                }
            }
            Ok(())
        })();
        if let Err(e) = final_binding {
            atomic_write(
                &job_path,
                &serde_json::to_vec(
                    &json!({"execution_id":execution_id,"state":"not_started","spawned":false,"termination":"not_started","pctx_error":e.code,"task_completion":"not_evaluated"}),
                )?,
                true,
            )?;
            return Err(e);
        }
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => {
                let data = json!({"execution_id":execution_id,"spawned":false,"termination":"not_started","child_exit_code":null,"pctx_error":"SPAWN_FAILED","task_completion":"not_evaluated"});
                let _ = atomic_write(&job_path, &serde_json::to_vec(&data)?, true);
                return Ok(data);
            }
        };
        let pid = child.id();
        if let Some(callback) = on_spawn.as_mut()
            && let Err(e) = callback(pid)
        {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
        // If this parent dies, this durable receipt remains active_or_unknown; TTL never frees resources.
        let _ = atomic_write(
            &job_path,
            &serde_json::to_vec(
                &json!({"execution_id":execution_id,"pid":pid,"process_group":pid,"state":"active_or_unknown","started_at":now(),"resource_admission":if registered{"registered_profile_external_resource_ledger"}else{"non_heavy_manual"}}),
            )?,
            true,
        );
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| err("IO_ERROR", "Missing child stdout pipe", 7))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| err("IO_ERROR", "Missing child stderr pipe", 7))?;
        #[cfg(unix)]
        if nonblocking(&stdout)
            .and_then(|_| nonblocking(&stderr))
            .is_err()
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(err("IO_ERROR", "Pipe capture setup failed after spawn", 7));
        }
        let stop = Arc::new(AtomicBool::new(false));
        let nul_framed = matches!(
            parser_identity,
            Some("git-status-porcelain-v1-z" | "git-log-nul-v1")
        );
        let out_thread = capture_framed(stdout, "stdout", stop.clone(), nul_framed);
        let err_thread = capture_framed(stderr, "stderr", stop.clone(), false);
        let start = Instant::now();
        let mut timed_out = false;
        let mut monitor_error = None;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if let Some(callback) = on_poll.as_mut()
                && let Err(e) = callback()
            {
                monitor_error = Some(e);
                #[cfg(unix)]
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                break child.wait()?;
            }
            if r.execution_timeout_ms
                .is_some_and(|ms| start.elapsed() >= Duration::from_millis(ms))
            {
                timed_out = true;
                #[cfg(unix)]
                {
                    // SAFETY: signal targets the new child process group, never this parent group.
                    unsafe {
                        libc::kill(-(pid as i32), libc::SIGTERM);
                    }
                }
                thread::sleep(Duration::from_millis(100));
                #[cfg(unix)]
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                break child.wait()?;
            }
            thread::sleep(Duration::from_millis(5));
        };
        stop.store(true, Ordering::Relaxed);
        let out = out_thread
            .join()
            .map_err(|_| err("IO_ERROR", "Stdout capture panicked", 7))?;
        let stderr = err_thread
            .join()
            .map_err(|_| err("IO_ERROR", "Stderr capture panicked", 7))?;
        let mut records = out.records;
        records.extend(stderr.records);
        // Bound serialized record storage too: JSON escaping must not expand a 16MiB
        // logical capture into an unbounded artifact.
        let mut serialized_size = records
            .iter()
            .map(|r| {
                serde_json::to_vec(r)
                    .map(|v| v.len() + 1)
                    .unwrap_or(RECORD_LIMIT)
            })
            .sum::<usize>();
        let mut store_omitted = 0u64;
        while serialized_size + 4096 > 16 * 1024 * 1024 {
            if let Some(record) = records.pop() {
                serialized_size = serialized_size.saturating_sub(
                    serde_json::to_vec(&record)
                        .map(|v| v.len() + 1)
                        .unwrap_or(0),
                );
                store_omitted += record.text.len() as u64;
            } else {
                break;
            }
        }
        let retained = r.retain != "none";
        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            status.signal()
        };
        #[cfg(not(unix))]
        let signal: Option<i32> = None;
        let termination = if timed_out {
            "timed_out"
        } else if signal.is_some() {
            "signaled"
        } else {
            "exited"
        }
        .to_string();
        let mut a = Artifact {
            schema_version: 1,
            output_id: output_id.clone(),
            execution_id: execution_id.clone(),
            workspace_id: p.workspace_id.clone(),
            policy_hash: p.policy_hash(),
            input_fingerprint,
            check_binding: check_binding.cloned(),
            parser_identity: parser_identity.unwrap_or("unsupported").into(),
            input_manifest,
            created_at: now(),
            expires_at: now() + 86400,
            records_hash: hash(serde_json::to_vec(&records)?),
            records,
            captured_bytes: out.captured_bytes + stderr.captured_bytes,
            normalized_bytes: out.normalized_bytes + stderr.normalized_bytes,
            redacted_bytes: out.redacted_bytes + stderr.redacted_bytes,
            omitted_bytes: out.omitted_bytes + stderr.omitted_bytes + store_omitted,
            capture_complete: out.complete && stderr.complete && store_omitted == 0,
            retained,
            spawned: true,
            termination,
            child_exit_code: status.code(),
            signal,
            pctx_error: if let Some(error) = monitor_error.as_ref() {
                Some(error.code.clone())
            } else if timed_out {
                Some("TIMEOUT".into())
            } else if out.io_error || stderr.io_error {
                Some("CAPTURE_FAILED".into())
            } else {
                None
            },
            input_stage: "captured".into(),
            emitted_bytes: 0,
            retrieval_bytes: 0,
            delivery_attempts: 0,
        };
        // Execution presentation retains its existing bounded-record contract;
        // a query budget never replaces the completed child's exit truth.
        let mut execution_presentation = p.clone();
        execution_presentation.deadline = None;
        let mut data = compact(&execution_presentation, &a, r.budget_bytes)?;
        data["exit_policy"] = json!(r.exit_policy);
        if let Some(error) = monitor_error {
            data["processing_exit"] = json!(error.exit);
            data["processing_error"] = json!(error);
        }
        if !retained {
            a.records.clear();
            a.records_hash = hash(serde_json::to_vec(&a.records)?);
        }
        if let Err(e) = save(p, &a) {
            data["pctx_error"] = json!(e.code);
            data["processing_exit"] = json!(e.exit);
            data["processing_error"] = json!(e);
            data["raw_available"] = json!(false);
        }
        let _ = atomic_write(
            &job_path,
            &serde_json::to_vec(
                &json!({"execution_id":execution_id,"pid":pid,"state":if a.capture_complete{"child_exited"}else{"child_exited_descendants_unknown"},"termination":a.termination,"child_exit_code":a.child_exit_code,"signal":a.signal}),
            )?,
            true,
        );
        Ok(data)
    }
}

/// Called by the frontend with the exact bytes actually emitted (including its envelope).
/// Emitted is an observation, not proof of receipt or provider token/cost usage.
pub fn record_delivery(p: &Project, value: &str, bytes: u64, kind: &str) -> Result<()> {
    phase(p, || record_delivery_inner(p, value, bytes, kind))
}
fn record_delivery_inner(p: &Project, value: &str, bytes: u64, kind: &str) -> Result<()> {
    p.check_deadline()?;
    let metrics = p.data_dir.join("output-metrics.lock");
    checked_private(&metrics)?;
    let mut opts = fs::OpenOptions::new();
    opts.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let lock = UnlockGuard(opts.open(metrics)?);
    lock.0
        .try_lock_exclusive()
        .map_err(|_| err("RESOURCE_BUSY", "Output measurement is busy", 7))?;
    let mut a = load(p, value)?;
    if kind == "retrieval" {
        a.retrieval_bytes = a.retrieval_bytes.saturating_add(bytes);
    } else if kind == "compact" {
        a.emitted_bytes = a.emitted_bytes.saturating_add(bytes);
    } else {
        return Err(err("INVALID_ARGUMENT", "Unknown output delivery stage", 2));
    }
    a.delivery_attempts = a.delivery_attempts.saturating_add(1);
    save(p, &a)
}
pub fn output(p: &Project, command: &OutputCommand) -> Result<Value> {
    let scoped = query_project(p)?;
    phase(&scoped, || output_inner(&scoped, command))
}
fn output_inner(p: &Project, command: &OutputCommand) -> Result<Value> {
    validate_output_request(command)?;
    let value = match command {
        OutputCommand::Show { id, .. }
        | OutputCommand::Find { id, .. }
        | OutputCommand::Render { id, .. } => id,
    };
    let a = load(p, value)?;
    if !a.retained {
        return Err(err(
            "OUTPUT_EXPIRED",
            "Retention was disabled for this execution",
            6,
        ));
    }
    match command {
        OutputCommand::Show {
            view,
            stream,
            lines,
            ..
        } => {
            if view == "compact" {
                return compact(p, &a, 8192);
            }
            let (first, last) = line_range(lines.as_deref())?;
            let mut records = Vec::new();
            let mut total = 0usize;
            for record in &a.records {
                p.check_deadline()?;
                if stream.as_ref().is_some_and(|s| s != &record.stream) {
                    continue;
                }
                total += 1;
                let line = record.sequence as usize + 1;
                if line >= first && line <= last {
                    records.push(json!({"stream":record.stream,"sequence":record.sequence,"text":record.text,"redacted":record.redacted}));
                }
            }
            Ok(
                json!({"output_id":a.output_id,"execution_id":a.execution_id,"view":"full","raw_semantics":"redacted_uncompressed","records":records,"total_records":total,"omitted_records":total.saturating_sub(records.len()),"capture_complete":a.capture_complete,"omitted_bytes":a.omitted_bytes,"child_exit_code":a.child_exit_code,"signal":a.signal,"spawned":a.spawned,"pctx_error":a.pctx_error,"termination":a.termination,"command_rerun":false,"delivery_kind":"retrieval"}),
            )
        }
        OutputCommand::Find { literal, limit, .. } => {
            let mut records = Vec::new();
            for record in &a.records {
                p.check_deadline()?;
                if record.text.contains(literal) {
                    records.push(record.clone());
                }
                if records.len() >= *limit {
                    break;
                }
            }
            Ok(
                json!({"output_id":a.output_id,"records":records,"command_rerun":false,"capture_complete":a.capture_complete,"delivery_kind":"retrieval"}),
            )
        }
        OutputCommand::Render { filter, .. } => {
            let mut v = if filter == "builtin" {
                compact(p, &a, 8192)?
            } else {
                let mut offset = 0;
                let mut records = Vec::new();
                for record in &a.records {
                    p.check_deadline()?;
                    let start = offset;
                    offset += record.text.len() + 1;
                    records.push(crate::filters::FilterRecord {
                        stream: record.stream.clone(),
                        sequence: record.sequence,
                        text: record.text.clone(),
                        start_byte: start,
                        end_byte: offset,
                    });
                }
                p.check_deadline()?;
                crate::filters::apply_observed(
                    p,
                    filter,
                    &records,
                    a.child_exit_code,
                    &a.termination,
                )?
            };
            v["output_id"] = json!(a.output_id);
            v["execution_id"] = json!(a.execution_id);
            v["child_exit_code"] = json!(a.child_exit_code);
            v["signal"] = json!(a.signal);
            v["termination"] = json!(a.termination);
            v["pctx_error"] = json!(a.pctx_error);
            v["capture_complete"] = json!(a.capture_complete);
            v["omitted_bytes"] = json!(a.omitted_bytes);
            v["command_rerun"] = json!(false);
            v["delivery_kind"] = json!("compact");
            Ok(v)
        }
    }
}
pub fn savings(p: &Project) -> Result<Value> {
    let scoped = query_project(p)?;
    phase(&scoped, || savings_inner(&scoped))
}
fn savings_inner(p: &Project) -> Result<Value> {
    let mut samples = Vec::new();
    let mut baseline = 0u64;
    let mut emitted = 0u64;
    let mut retrieval = 0u64;
    let entries = phase(p, || match fs::read_dir(output_dir(p)) {
        Ok(entries) => Ok(Some(entries)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    })?;
    if let Some(entries) = entries {
        for entry in entries {
            p.check_deadline()?;
            let entry = phase(p, || Ok(entry?))?;
            let Some(value) = entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_owned)
            else {
                continue;
            };
            if !value.starts_with("OUT-") {
                continue;
            }
            let artifact = match load(p, &value) {
                Ok(artifact) => Some(artifact),
                Err(error) if error.code == "TIMEOUT" => return Err(error),
                Err(_) => None,
            };
            if let Some(a) = artifact {
                if a.input_stage == "already_compacted" {
                    samples.push(
                        json!({"output_id":value,"baseline":"unknown","input_stage":a.input_stage}),
                    );
                    continue;
                }
                if a.delivery_attempts == 0 {
                    samples.push(json!({"output_id":value,"metric_status":"not_emitted","redacted_uncompressed_bytes":a.redacted_bytes}));
                    continue;
                }
                baseline += a.redacted_bytes;
                emitted += a.emitted_bytes;
                retrieval += a.retrieval_bytes;
                samples.push(json!({"output_id":value,"redacted_uncompressed_bytes":a.redacted_bytes,"emitted_compact_bytes":a.emitted_bytes,"emitted_retrieval_bytes":a.retrieval_bytes,"delivery_attempts":a.delivery_attempts,"capture_complete":a.capture_complete}));
            }
        }
    }
    p.check_deadline()?;
    let net = baseline as i128 - emitted as i128 - retrieval as i128;
    Ok(
        json!({"scope":"workspace","metric":"observed_emitted_bytes","samples":samples,"redacted_uncompressed_bytes":baseline,"emitted_compact_bytes":emitted,"emitted_retrieval_bytes":retrieval,"net_bytes_saved":net as i64,"net_percent":if baseline==0 {Value::Null}else{json!(net as f64*100.0/baseline as f64)},"tokenizer_tokens":"unknown","provider_usage":"unknown","subscription_quota":"unknown","api_cost":"unknown","delivery_receipt":"not_observed","metrics_available":emitted+retrieval>0}),
    )
}

pub(crate) fn observed_check_binding(p: &Project, output_id: &str) -> Result<Option<Value>> {
    Ok(load(p, output_id)?.check_binding)
}

#[cfg(unix)]
pub(crate) fn observed_report(
    p: &Project,
    output_id: &str,
) -> Result<Option<crate::work::CheckReport>> {
    let artifact = load(p, output_id)?;
    if typed_report(&artifact).is_none() || artifact.termination != "exited" {
        return Ok(None);
    }
    let text = artifact
        .records
        .iter()
        .filter(|r| r.stream == "stdout")
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let mut report: crate::work::CheckReport = serde_json::from_str(&text)?;
    report.source = "runner_observed".into();
    Ok(Some(report))
}

/// Recognized location candidates retain per-file execution-input hashes.
pub fn diagnostic_locations(p: &Project, output_id: &str) -> Result<Value> {
    let artifact = load(p, output_id)?;
    if !artifact.retained {
        return Err(err("OUTPUT_EXPIRED", "Output records not retained", 6));
    }
    let pattern = regex::Regex::new(r#"([^\s:"'()]+):(\d+)(?::(\d+))?"#)
        .map_err(|_| err("PARSER_FAILED", "Location parser unavailable", 3))?;
    let mut locations = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut skipped = 0;
    for record in &artifact.records {
        for capture in pattern.captures_iter(&record.text) {
            let path = &capture[1];
            let Ok(line) = capture[2].parse::<usize>() else {
                skipped += 1;
                continue;
            };
            if line == 0 || reader::policy_allows(p, path).is_err() || reader::redact(path).1 {
                skipped += 1;
                continue;
            }
            if !seen.insert((path.to_string(), line)) {
                continue;
            }
            let Some(expected) = artifact.input_manifest.get(path) else {
                return Err(err(
                    "STALE_RESULT",
                    "Diagnostic source has no execution-input hash",
                    4,
                ));
            };
            if reader::read(p, path)?.hash != *expected {
                return Err(err(
                    "STALE_RESULT",
                    "Diagnostic source changed after execution",
                    4,
                ));
            }
            if locations.len() >= 1000 {
                return Err(err(
                    "PARTIAL_RESULT",
                    "Diagnostic location count exceeds bound",
                    3,
                ));
            }
            locations.push(json!({"path":path,"line":line,"source_hash":expected,"stream":record.stream,"record_sequence":record.sequence,"location_semantics":"reported_candidate"}));
        }
    }
    Ok(
        json!({"output_id":output_id,"locations":locations,"capture_complete":artifact.capture_complete,"unrecognized_or_denied_locations":skipped,"parser":"path_line_candidates","command_rerun":false}),
    )
}

#[cfg(unix)]
pub(crate) fn unverified_report(p: &Project, output_id: &str) -> Result<crate::work::CheckReport> {
    let artifact = load(p, output_id)?;
    let result = if artifact.termination == "timed_out" {
        "timed_out"
    } else if artifact.child_exit_code.is_some_and(|c| c != 0) || artifact.signal.is_some() {
        "failed"
    } else {
        "unverified"
    };
    Ok(crate::work::CheckReport {
        schema_version: 1,
        check_key: String::new(),
        producer: "pctx-supervisor".into(),
        source: "runner_observed".into(),
        exit_code: artifact.child_exit_code.unwrap_or(128),
        tests: 0,
        passed: 0,
        failed: 0,
        errors: 0,
        skipped: 0,
        result: result.into(),
        started_at: artifact.created_at,
        finished_at: artifact.created_at,
        environment: json!({"test_counts_status":"unknown","report_parse_status":"incomplete_or_unsupported","observed_termination":artifact.termination,"capture_complete":artifact.capture_complete,"signal":artifact.signal}),
    })
}

#[cfg(test)]
mod capture_tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn registered_native_callback_failures_preserve_processing_truth() {
        use crate::project::{Config, ProjectConfig, RootAnchor};
        use std::cell::Cell;
        struct NativeCleanup<'a>(&'a Cell<u32>);
        impl Drop for NativeCleanup<'_> {
            fn drop(&mut self) {
                let pid = self.0.get() as i32;
                if pid == 0 {
                    return;
                }
                let mut status = 0;
                // A still-owned, unreaped direct child reserves this PID before group cancellation.
                if unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) } == 0 {
                    unsafe {
                        libc::kill(-pid, libc::SIGKILL);
                        libc::kill(pid, libc::SIGKILL);
                        libc::waitpid(pid, &mut status, 0);
                    }
                }
            }
        }
        for spawn_failure in [true, false] {
            let temp = tempfile::tempdir().unwrap();
            let base = fs::canonicalize(temp.path()).unwrap();
            let root = base.join("project");
            let data = base.join("data");
            fs::create_dir_all(&root).unwrap();
            let p = Project {
                deadline: None,
                root_anchor: RootAnchor::capture(&root).unwrap(),
                root,
                data_dir: data.clone(),
                workspace_dir: data.join("workspace"),
                control_dir: data.join("control"),
                project_id: "fixture".into(),
                workspace_id: "ws".into(),
                coordination_id: "coord".into(),
                config: Config {
                    schema_version: 1,
                    project: ProjectConfig {
                        id: "fixture".into(),
                        name: "fixture".into(),
                    },
                    index: Default::default(),
                    policy: Default::default(),
                    search: Default::default(),
                    context: Default::default(),
                    roles: Default::default(),
                },
            };
            fs::create_dir_all(&p.workspace_dir).unwrap();
            fs::create_dir_all(&p.control_dir).unwrap();
            fs::write(
                p.root.join("fixture.sh"),
                "printf x >> invocation\nexec /bin/sleep 30\n",
            )
            .unwrap();
            let r = RunRequest {
                task_id: None,
                session: None,
                retain: "temporary".into(),
                execution_timeout_ms: Some(5000),
                budget_bytes: 8192,
                exit_policy: "pctx".into(),
                stdin: "closed".into(),
                argv: vec!["/bin/sh".into(), "fixture.sh".into()],
            };
            let binding = registered_binding_at(&p, &r.argv, ".").unwrap();
            let pid = Cell::new(0);
            let _cleanup = NativeCleanup(&pid);
            let polls = Cell::new(0);
            let result = run_registered_monitored(
                &p,
                &r,
                ".",
                &BTreeMap::new(),
                binding["fingerprint"].as_str().unwrap(),
                &mut |native_pid| {
                    pid.set(native_pid);
                    assert_eq!(
                        unsafe { libc::getpgid(native_pid as i32) },
                        native_pid as i32
                    );
                    // Distinct callback fixture readiness limit; not a change to any startup regression budget.
                    let end = Instant::now() + Duration::from_secs(2);
                    while fs::read(p.root.join("invocation")).ok().as_deref() != Some(b"x") {
                        if Instant::now() >= end {
                            return Err(err("TIMEOUT", "Fixture body did not start", 7));
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    if spawn_failure {
                        Err(err("IO_ERROR", "Spawn observer publication refused", 7))
                    } else {
                        Ok(())
                    }
                },
                &mut || {
                    polls.set(polls.get() + 1);
                    Err(err("CONFIG_CHANGED", "Monitor binding changed", 9))
                },
            );
            assert_ne!(pid.get(), 0);
            // Both paths return only after this native direct child is reaped.
            let mut status = 0;
            assert_eq!(
                unsafe { libc::waitpid(pid.get() as i32, &mut status, libc::WNOHANG) },
                -1
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
            assert_eq!(fs::read(p.root.join("invocation")).unwrap(), b"x");
            if spawn_failure {
                let error = result.unwrap_err();
                assert_eq!(error.code, "IO_ERROR");
                assert_eq!(error.exit, 7);
                assert_eq!(polls.get(), 0);
            } else {
                let data = result.unwrap();
                assert_eq!(data["spawned"], true);
                assert_eq!(data["termination"], "signaled");
                assert_eq!(data["signal"], libc::SIGKILL);
                assert!(data["child_exit_code"].is_null());
                let error = execution_error(&data).unwrap();
                assert_eq!(error.code, "CONFIG_CHANGED");
                assert_eq!(error.exit, 9);
                assert_eq!(error.message, "Monitor binding changed");
                assert_eq!(polls.get(), 1);
            }
        }
    }
    struct Fragmented {
        data: Vec<u8>,
        offset: usize,
    }
    impl Read for Fragmented {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            let n = 7.min(b.len()).min(self.data.len() - self.offset);
            b[..n].copy_from_slice(&self.data[self.offset..self.offset + n]);
            self.offset += n;
            Ok(n)
        }
    }
    #[test]
    fn split_secret_and_ansi_never_survive_normalization() {
        let data = b"\x1b[31merror ghp_abcdefghijklmnop123456789\x1b[0m\rprogress 50%\n".to_vec();
        let c = capture(
            Fragmented { data, offset: 0 },
            "stderr",
            Arc::new(AtomicBool::new(false)),
        )
        .join()
        .unwrap();
        assert_eq!(c.records[0].text, "error [REDACTED]");
        assert_eq!(c.records[1].text, "progress 50%");
        assert!(c.redacted);
        assert!(c.complete);
    }
    #[test]
    fn oversize_binary_invalid_utf8_and_record_counts_are_bounded() {
        let mut data = vec![b'x'; RECORD_LIMIT + 100];
        data.extend_from_slice(b"\n\x00hidden\n\xffinvalid\nerror retained\n");
        let c = capture(
            std::io::Cursor::new(data),
            "stdout",
            Arc::new(AtomicBool::new(false)),
        )
        .join()
        .unwrap();
        assert!(!c.complete);
        assert!(c.omitted_bytes >= RECORD_LIMIT as u64);
        assert_eq!(c.records.len(), 1);
        assert_eq!(c.records[0].text, "error retained");
        let c = capture(
            std::io::Cursor::new(vec![b'\n'; 100000]),
            "stdout",
            Arc::new(AtomicBool::new(false)),
        )
        .join()
        .unwrap();
        assert!(c.records.len() <= CAPTURE_RECORD_LIMIT + TAIL_RECORD_LIMIT);
        assert!(!c.complete);
    }
    #[test]
    fn incomplete_private_key_never_reaches_records() {
        let c = capture(
            std::io::Cursor::new(b"-----BEGIN PRIVATE KEY-----\nsensitivevalue"),
            "stdout",
            Arc::new(AtomicBool::new(false)),
        )
        .join()
        .unwrap();
        assert!(!c.complete);
        assert!(c.records.is_empty());
    }
    #[test]
    fn registered_git_nul_capture_preserves_filename_controls_and_record_boundaries() {
        let bytes = b" M path with\nnewline\0R  new\0old\0".to_vec();
        let c = capture_framed(
            Fragmented {
                data: bytes.clone(),
                offset: 0,
            },
            "stdout",
            Arc::new(AtomicBool::new(false)),
            true,
        )
        .join()
        .unwrap();
        assert!(c.complete);
        assert_eq!(c.records.len(), 3);
        assert_eq!(
            c.records
                .iter()
                .map(|r| r.text.as_str())
                .collect::<String>()
                .as_bytes(),
            bytes
        );
        assert_eq!(c.redacted_bytes, bytes.len() as u64);
        let records = c
            .records
            .iter()
            .map(|r| json!({"stream":r.stream,"sequence":r.sequence,"text":r.text}))
            .collect::<Vec<_>>();
        let parsed = crate::parsers::parse(
            "git-status-porcelain-v1-z",
            &records,
            Some(0),
            "exited",
            true,
        );
        assert_eq!(parsed["parse_status"], "complete");
        assert_eq!(parsed["items"].as_array().unwrap().len(), 2);
        // NUL preservation belongs only to the explicitly registered Git parser.
        let ordinary = capture(
            std::io::Cursor::new(bytes),
            "stdout",
            Arc::new(AtomicBool::new(false)),
        )
        .join()
        .unwrap();
        assert!(!ordinary.complete);
    }
}

//! Offline Claude protocol bridge. Never reads native transcripts or executes a model.
use crate::{
    domain::{Error, Result, hash, now},
    project::{Project, atomic_write, private_dir},
    quota::{self, Observation, QuotaCommand, UsageBatch},
    reader,
    session::{self, SessionCommand},
    work,
};
use clap::Subcommand;
use fs2::FileExt;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};
const LIMIT: usize = 128 * 1024;
#[derive(Debug, Subcommand)]
pub enum AdapterCommand {
    Claude {
        #[command(subcommand)]
        command: ClaudeCommand,
    },
}
#[derive(Debug, Subcommand)]
pub enum ClaudeCommand {
    Doctor,
    Plan {
        #[arg(long)]
        agent: String,
    },
    Install {
        #[arg(long)]
        plan: String,
        #[arg(long)]
        expect_hash: String,
    },
    Uninstall {
        #[arg(long)]
        plan: String,
        #[arg(long)]
        expect_config_hash: String,
    },
    Verify,
    Statusline {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        pool: String,
        #[arg(long)]
        session: String,
        #[arg(long)]
        epoch: i64,
        #[arg(long)]
        observed_at: String,
        #[arg(long)]
        window_start: String,
        #[arg(long)]
        window_end: String,
        #[arg(long)]
        counter_epoch: String,
        #[arg(long)]
        idempotency_key: String,
    },
    Event {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        from_file: Option<PathBuf>,
        #[arg(long)]
        idempotency_key: Option<String>,
        #[arg(long)]
        hook: bool,
    },
    ProtocolFixture {
        #[arg(long)]
        from_file: PathBuf,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: u32,
    project: String,
    workspace: String,
    agent: String,
    config_hash: String,
    additions: Value,
}
fn err(code: &str, s: &str) -> Error {
    Error::new(code, s, if code == "INVALID_ARGUMENT" { 2 } else { 9 })
}
fn label(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 256 || s.chars().any(|c| c.is_control()) || reader::redact(s).1 {
        return Err(err(
            "INVALID_ARGUMENT",
            "Invalid or sensitive adapter identity",
        ));
    }
    Ok(())
}
fn owner() -> Result<()> {
    if std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()) != "owner" {
        return Err(err(
            "POLICY_DENIED",
            "Adapter bindings and configuration require owner authority",
        ));
    }
    Ok(())
}
#[cfg(unix)]
fn safe_open(path: &Path) -> Result<fs::File> {
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut dir = fs::File::open("/")?;
    let parts: Vec<_> = absolute
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(Ok(s)),
            std::path::Component::RootDir => None,
            _ => Some(Err(err("PATH_DENIED", "Noncanonical adapter input path"))),
        })
        .collect::<Result<Vec<_>>>()?;
    for (i, part) in parts.iter().enumerate() {
        let name =
            std::ffi::CString::new(part.as_bytes()).map_err(|_| err("PATH_DENIED", "NUL path"))?;
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | if i + 1 < parts.len() {
                libc::O_DIRECTORY
            } else {
                0
            };
        let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        dir = unsafe { fs::File::from_raw_fd(fd) };
    }
    Ok(dir)
}
#[cfg(not(unix))]
fn safe_open(path: &Path) -> Result<fs::File> {
    for ancestor in path.ancestors() {
        if fs::symlink_metadata(ancestor)?.file_type().is_symlink() {
            return Err(err("PATH_DENIED", "Symlink adapter path"));
        }
    }
    Ok(fs::File::open(path)?)
}
fn bounded(path: Option<&Path>) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    match path {
        Some(path) => {
            if fs::symlink_metadata(path)?.file_type().is_symlink() {
                return Err(err("PATH_DENIED", "Symlink adapter input"));
            }
            safe_open(path)?
                .take((LIMIT + 1) as u64)
                .read_to_end(&mut data)?;
        }
        None => {
            std::io::stdin()
                .take((LIMIT + 1) as u64)
                .read_to_end(&mut data)?;
        }
    }
    if data.len() > LIMIT {
        return Err(err("BUDGET_EXCEEDED", "Adapter input exceeds 128 KiB"));
    }
    Ok(data)
}
fn acquire(p: &Project) -> Result<fs::File> {
    private_dir(&p.control_dir)?;
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let lock = options.open(p.control_dir.join("adapter.lock"))?;
    lock.lock_exclusive()?;
    Ok(lock)
}
fn config(p: &Project) -> Result<(PathBuf, Vec<u8>, Value)> {
    let dir = p.root.join(".claude");
    if dir.exists() && fs::symlink_metadata(&dir)?.file_type().is_symlink() {
        return Err(err(
            "PATH_DENIED",
            "Claude configuration directory is a symlink",
        ));
    }
    let path = dir.join("settings.local.json");
    let bytes = if path.exists() {
        bounded(Some(&path))?
    } else {
        Vec::new()
    };
    let value = if bytes.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&bytes)
            .map_err(|_| err("INVALID_ARGUMENT", "Invalid Claude configuration JSON"))?
    };
    if !value.is_object() {
        return Err(err(
            "INVALID_ARGUMENT",
            "Claude configuration must be an object",
        ));
    }
    Ok((path, bytes, value))
}
fn additions(agent: &str) -> Value {
    let mut out = json!({});
    for event in ["SessionStart", "PreCompact", "SessionEnd", "PreToolUse"] {
        out[event] = json!([{ "hooks":[{"type":"command","command":format!("pctx adapter claude event --hook --agent '{}'",agent.replace('\'',"'\\''")),"timeout":5}]}]);
    }
    out
}
fn merge(mut config: Value, add: &Value) -> Result<Value> {
    if config.get("hooks").is_none() {
        config["hooks"] = json!({});
    }
    let hooks = config["hooks"]
        .as_object_mut()
        .ok_or_else(|| err("CONFIG_CONFLICT", "Existing hooks must be an object"))?;
    for (event, groups) in add.as_object().unwrap() {
        let values = hooks
            .entry(event.clone())
            .or_insert(json!([]))
            .as_array_mut()
            .ok_or_else(|| err("CONFIG_CONFLICT", "Existing hook event must be an array"))?;
        for group in groups.as_array().unwrap() {
            if !values.contains(group) {
                values.push(group.clone());
            }
        }
    }
    Ok(config)
}
fn db(p: &Project) -> Result<Connection> {
    let mut db = work::connect(p)?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='adapter_schema')",
        [],
        |r| r.get(0),
    )?;
    if exists {
        let count: i64 = db.query_row("SELECT count(*) FROM adapter_schema", [], |r| r.get(0))?;
        if count != 1 {
            return Err(err(
                "DB_SCHEMA_TOO_NEW",
                "Adapter schema must have exactly one version",
            ));
        }
        let v: i64 = db.query_row("SELECT version FROM adapter_schema", [], |r| r.get(0))?;
        if v == 2 {
            return Ok(db);
        }
        if v != 1 {
            return Err(err("DB_SCHEMA_TOO_NEW", "Unsupported adapter schema"));
        }
    }
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if exists {
        let v: i64 = tx.query_row("SELECT version FROM adapter_schema", [], |r| r.get(0))?;
        if ![1, 2].contains(&v) {
            return Err(err(
                "DB_SCHEMA_TOO_NEW",
                "Adapter migration version changed",
            ));
        }
    }
    tx.execute_batch("CREATE TABLE IF NOT EXISTS adapter_schema(version INTEGER PRIMARY KEY CHECK(version IN (1,2))); INSERT INTO adapter_schema SELECT 1 WHERE NOT EXISTS(SELECT 1 FROM adapter_schema); CREATE TABLE IF NOT EXISTS adapter_bindings(workspace TEXT NOT NULL,agent TEXT NOT NULL,native TEXT NOT NULL,session TEXT NOT NULL,pending INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(workspace,agent,native)); CREATE TABLE IF NOT EXISTS adapter_capsules(id TEXT PRIMARY KEY,session TEXT NOT NULL,metadata TEXT NOT NULL,created INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS adapter_receipts(workspace TEXT NOT NULL,key TEXT NOT NULL,digest TEXT NOT NULL,result TEXT NOT NULL,created INTEGER NOT NULL,PRIMARY KEY(workspace,key)); CREATE TABLE IF NOT EXISTS adapter_installs(workspace TEXT NOT NULL,plan TEXT NOT NULL,owned TEXT NOT NULL,config_hash TEXT NOT NULL,status TEXT NOT NULL,PRIMARY KEY(workspace,plan)); UPDATE adapter_schema SET version=2;")?;
    tx.commit()?;
    Ok(db)
}
fn parse(bytes: &[u8]) -> Result<Value> {
    let raw: Value = serde_json::from_slice(bytes)
        .map_err(|_| err("INVALID_ARGUMENT", "Malformed hook JSON"))?;
    let event = raw
        .get("hook_event_name")
        .and_then(Value::as_str)
        .ok_or_else(|| err("INVALID_ARGUMENT", "Hook event name required"))?;
    if ![
        "SessionStart",
        "PreCompact",
        "PostCompact",
        "SessionEnd",
        "PreToolUse",
        "Stop",
        "PostToolUse",
        "PostToolUseFailure",
        "PermissionRequest",
        "PermissionDenied",
        "SubagentStart",
        "SubagentStop",
        "TaskCompleted",
    ]
    .contains(&event)
    {
        return Err(err(
            "CAPABILITY_UNAVAILABLE",
            "Unknown hook event; incompatible feature disabled",
        ));
    }
    let native = raw
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or_else(|| err("INVALID_ARGUMENT", "Native session identity required"))?;
    label(native)?;
    let mut safe = json!({"hook_event_name":event,"session_id":native});
    for key in ["source", "trigger", "permission_mode", "tool_name"] {
        if let Some(s) = raw.get(key).and_then(Value::as_str) {
            label(s)?;
            safe[key] = json!(s);
        }
    }
    if event == "SessionStart"
        && !matches!(
            safe["source"].as_str(),
            Some("startup" | "resume" | "clear" | "compact")
        )
    {
        return Err(err("CAPABILITY_UNAVAILABLE", "Unknown SessionStart source"));
    }
    if matches!(event, "PreCompact" | "PostCompact")
        && !matches!(safe["trigger"].as_str(), Some("manual" | "auto"))
    {
        return Err(err("CAPABILITY_UNAVAILABLE", "Unknown compact trigger"));
    }
    Ok(safe)
}
fn import(p: &Project, agent: &str, path: Option<&Path>, key: Option<&str>) -> Result<Value> {
    owner()?;
    label(agent)?;
    let bytes = bounded(path)?;
    let safe = parse(&bytes)?;
    if safe["hook_event_name"] == "PreToolUse" {
        let raw: Value = serde_json::from_slice(&bytes)?;
        return protection(&safe, &raw);
    }
    let digest = hash([agent.as_bytes(), &[0], bytes.as_slice()].concat());
    let key = key.unwrap_or(&digest);
    label(key)?;
    let _lock = acquire(p)?;
    let db = db(p)?;
    if let Some((d, r)) = db
        .query_row(
            "SELECT digest,result FROM adapter_receipts WHERE workspace=? AND key=?",
            params![p.workspace_id, key],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?
    {
        if d != digest {
            return Err(err(
                "IDEMPOTENCY_CONFLICT",
                "Hook key reused with different input",
            ));
        }
        let previous: Value = serde_json::from_str(&r)?;
        if previous["incomplete"] == true {
            return Err(err(
                "RECONCILIATION_REQUIRED",
                "Previous event interrupted; inspect session before retrying",
            ));
        }
        if let (Some(sid), Some(recorded)) = (
            previous["session"]["session_id"].as_str(),
            previous["session"]["context_epoch"].as_i64(),
        ) {
            let current: Option<i64> = db
                .query_row("SELECT epoch FROM pctx_sessions WHERE id=?", [sid], |r| {
                    r.get(0)
                })
                .optional()?;
            if current != Some(recorded) {
                return Err(err(
                    "BASELINE_MISMATCH",
                    "Stored receipt epoch is stale; no session mutation repeated",
                ));
            }
        }
        return Ok(previous);
    }
    db.execute(
        "INSERT INTO adapter_receipts(workspace,key,digest,result,created) VALUES(?,?,?,?,?)",
        params![
            p.workspace_id,
            key,
            digest,
            json!({"incomplete":true}).to_string(),
            now()
        ],
    )?;
    let native = safe["session_id"].as_str().unwrap();
    let event = safe["hook_event_name"].as_str().unwrap();
    let binding:Option<(String,bool)>=db.query_row("SELECT session,pending FROM adapter_bindings WHERE workspace=? AND agent=? AND native=?",params![p.workspace_id,agent,native],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let mut result = json!({"event":event,"source_authority":"explicit-local-import","live_transport_verified":false,"hook_output":{},"acknowledged":false,"task_completed":false,"host_permission_granted":false});
    match event {
        "SessionStart" => {
            let state = if safe["source"] == "compact" {
                let (s, pending) = binding.as_ref().ok_or_else(|| {
                    err(
                        "BASELINE_MISMATCH",
                        "Compact start requires an existing binding",
                    )
                })?;
                if *pending {
                    let epoch: i64 =
                        db.query_row("SELECT epoch FROM pctx_sessions WHERE id=?", [s], |r| {
                            r.get(0)
                        })?;
                    json!({"session_id":s,"context_epoch":epoch,"full_required":true})
                } else {
                    session::session(
                        p,
                        &SessionCommand::Boundary {
                            session: s.clone(),
                            reason: "claude-compact-start".into(),
                        },
                    )?
                }
            } else {
                session::session(
                    p,
                    &SessionCommand::Attach {
                        agent: agent.into(),
                        runtime: "claude-code".into(),
                        workspace: "current".into(),
                        native_session: Some(native.into()),
                        role: None,
                        account_pool: None,
                        adapter_version: "claude-protocol-v1".into(),
                    },
                )?
            };
            let sid = state["session_id"].as_str().unwrap();
            db.execute("INSERT INTO adapter_bindings(workspace,agent,native,session,pending) VALUES(?,?,?,?,0) ON CONFLICT(workspace,agent,native) DO UPDATE SET session=excluded.session,pending=0",params![p.workspace_id,agent,native,sid])?;
            result["session"] = state;
        }
        "PreCompact" => {
            let (sid, _) = binding.ok_or_else(|| {
                err(
                    "BASELINE_MISMATCH",
                    "PreCompact requires SessionStart binding",
                )
            })?;
            let epoch: i64 =
                db.query_row("SELECT epoch FROM pctx_sessions WHERE id=?", [&sid], |r| {
                    r.get(0)
                })?;
            let metadata = json!({"schema_version":1,"session_id":sid,"context_epoch":epoch,"workspace_id":p.workspace_id,"policy_hash":p.policy_hash(),"source_inventory":"deferred_until_explicit_context_get","next_action":"Obtain and acknowledge a full context packet; paused work requires explicit owner resume","approval_restore":false,"lease_restore":false});
            let state = session::session(
                p,
                &SessionCommand::Boundary {
                    session: sid.clone(),
                    reason: "claude-precompact".into(),
                },
            )?;
            db.execute(
                "UPDATE adapter_bindings SET pending=1 WHERE workspace=? AND agent=? AND native=?",
                params![p.workspace_id, agent, native],
            )?;
            let cap_id = hash(metadata.to_string());
            db.execute("INSERT OR IGNORE INTO adapter_capsules(id,session,metadata,created) VALUES(?,?,?,?)",params![cap_id,sid,metadata.to_string(),now()])?;
            result["capsule_id"] = json!(cap_id);
            result["session"] = state;
        }
        "PostCompact" => {
            result["capability"] = json!("documented; installed support unknown");
            result["full_required"] = json!(true);
        }
        "SessionEnd" => {
            if let Some((sid, _)) = binding {
                result["session"] = session::session(
                    p,
                    &SessionCommand::Suspend {
                        session: sid,
                        reason: "claude-session-end".into(),
                    },
                )?;
            }
        }
        _ => {
            result["observation_only"] = json!(true);
        }
    }
    db.execute(
        "UPDATE adapter_receipts SET result=? WHERE workspace=? AND key=? AND digest=?",
        params![result.to_string(), p.workspace_id, key, digest],
    )?;
    Ok(result)
}

#[cfg(unix)]
fn publish_config(p: &Project, expected: &[u8], desired: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::os::fd::{AsRawFd, FromRawFd};
    let dir = safe_open(&p.root.join(".claude"))?;
    let target = std::ffi::CString::new("settings.local.json").unwrap();
    let name =
        std::ffi::CString::new(format!(".pctx-adapter-{}", crate::domain::id("tmp"))).unwrap();
    let read = || -> Result<Vec<u8>> {
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                target.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(Vec::new());
            }
            return Err(e.into());
        }
        let mut data = Vec::new();
        unsafe { fs::File::from_raw_fd(fd) }
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut data)?;
        Ok(data)
    };
    if read()? != expected {
        return Err(err("PLAN_STALE", "Pinned configuration changed"));
    }
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut file = unsafe { fs::File::from_raw_fd(fd) };
    let result = (|| -> Result<()> {
        file.write_all(desired)?;
        file.sync_all()?;
        if read()? != expected {
            return Err(err("PLAN_STALE", "Configuration edited during staging"));
        }
        if unsafe {
            libc::renameat(
                dir.as_raw_fd(),
                name.as_ptr(),
                dir.as_raw_fd(),
                target.as_ptr(),
            )
        } < 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        dir.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), 0) };
    }
    result
}
#[cfg(not(unix))]
fn publish_config(_p: &Project, _expected: &[u8], _desired: &[u8]) -> Result<()> {
    Err(err(
        "CAPABILITY_UNAVAILABLE",
        "Pinned settings publication requires supported host filesystem implementation",
    ))
}

pub fn execute(p: &Project, c: &AdapterCommand) -> Result<Value> {
    let AdapterCommand::Claude { command } = c;
    match command {
        ClaudeCommand::Doctor => {
            let version = Command::new("claude")
                .arg("--version")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| {
                    reader::redact(&String::from_utf8_lossy(&o.stdout))
                        .0
                        .trim()
                        .to_string()
                });
            Ok(
                json!({"runtime":"claude-code","installed":version.is_some(),"version":version,"hooks_documented":["SessionStart","PreCompact","PostCompact","SessionEnd"],"installed_hook_support":if version.is_some(){json!("unknown")}else{json!(false)},"fixture_protocol":"claude-protocol-v1","account_live_check":"unknown","host_permissions":"independent","automatic_shell_wrapping":false,"managed_scheduler_receipt":"missing","keep_awake":"missing"}),
            )
        }
        ClaudeCommand::Plan { agent } => {
            owner()?;
            label(agent)?;
            let (_, bytes, cfg) = config(p)?;
            let add = additions(agent);
            merge(cfg, &add)?;
            let plan = Plan {
                schema: 1,
                project: p.project_id.clone(),
                workspace: p.workspace_id.clone(),
                agent: agent.clone(),
                config_hash: hash(&bytes),
                additions: add.clone(),
            };
            let data = serde_json::to_vec(&plan)?;
            let digest = hash(&data);
            let dir = p.control_dir.join("adapter-plans");
            if fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(err("PATH_DENIED", "Symlink plan directory"));
            }
            private_dir(&dir)?;
            atomic_write(&dir.join(format!("{digest}.json")), &data, false).or_else(|e| {
                if dir.join(format!("{digest}.json")).exists() {
                    Ok(())
                } else {
                    Err(e)
                }
            })?;
            Ok(
                json!({"plan_id":digest,"plan_hash":digest,"target":".claude/settings.local.json","config_hash":plan.config_hash,"additions":add,"permission_changes":[],"installed_support":"unknown","requires_explicit_install":true}),
            )
        }
        ClaudeCommand::Install { plan, expect_hash } => {
            owner()?;
            let _lock = acquire(p)?;
            if plan.len() != 64
                || !plan.bytes().all(|b| b.is_ascii_hexdigit())
                || plan != expect_hash
            {
                return Err(err("PLAN_MISMATCH", "Exact plan hash required"));
            }
            let bytes = bounded(Some(
                &p.control_dir
                    .join("adapter-plans")
                    .join(format!("{plan}.json")),
            ))?;
            if hash(&bytes) != *expect_hash {
                return Err(err("PLAN_MISMATCH", "Plan bytes changed"));
            }
            let plan: Plan = serde_json::from_slice(&bytes)?;
            if plan.schema != 1
                || plan.project != p.project_id
                || plan.workspace != p.workspace_id
                || plan.additions != additions(&plan.agent)
            {
                return Err(err("PLAN_MISMATCH", "Plan identity or additions changed"));
            }
            let (path, bytes, cfg) = config(p)?;
            if hash(&bytes) != plan.config_hash {
                return Err(err("PLAN_STALE", "Configuration changed since planning"));
            }
            let owned = owned_additions(&cfg, &plan.additions);
            let desired = serde_json::to_vec_pretty(&merge(cfg, &plan.additions)?)?;
            if desired.len() > LIMIT {
                return Err(err("BUDGET_EXCEEDED", "Merged settings exceed 128 KiB"));
            }
            let install_db = db(p)?;
            install_db.execute("INSERT INTO adapter_installs(workspace,plan,owned,config_hash,status) VALUES(?,?,?,?,'prepared') ON CONFLICT(workspace,plan) DO NOTHING",params![p.workspace_id,expect_hash,owned.to_string(),hash(&desired)])?;
            if !path.parent().unwrap().exists() {
                fs::create_dir(path.parent().unwrap())?;
            }
            let (_, current, _) = config(p)?;
            if current != bytes {
                return Err(err("PLAN_STALE", "Configuration changed before publishing"));
            }
            publish_config(p, &bytes, &desired)?;
            install_db.execute(
                "UPDATE adapter_installs SET status='installed' WHERE workspace=? AND plan=?",
                params![p.workspace_id, expect_hash],
            )?;
            Ok(
                json!({"installed":true,"target":".claude/settings.local.json","config_hash":hash(desired),"live_verified":false,"permission_changes":[]}),
            )
        }
        ClaudeCommand::Uninstall {
            plan,
            expect_config_hash,
        } => uninstall(p, plan, expect_config_hash),
        ClaudeCommand::Statusline {
            task_id,
            from_file,
            pool,
            session,
            epoch,
            observed_at,
            window_start,
            window_end,
            counter_epoch,
            idempotency_key,
        } => statusline(
            p,
            task_id,
            from_file,
            pool,
            session,
            *epoch,
            observed_at,
            window_start,
            window_end,
            counter_epoch,
            idempotency_key,
        ),
        ClaudeCommand::Verify => {
            let (_, bytes, cfg) = config(p)?;
            Ok(
                json!({"config_hash":hash(bytes),"hooks_present":cfg.get("hooks").is_some(),"live_verified":false,"native_rule_loading":"unknown","host_permission_granted":false}),
            )
        }
        ClaudeCommand::Event {
            agent,
            from_file,
            idempotency_key,
            hook,
        } => {
            if *hook && from_file.is_some() {
                return Err(err("INVALID_ARGUMENT", "Hook transport accepts stdin only"));
            }
            let mut value = import(p, agent, from_file.as_deref(), idempotency_key.as_deref())?;
            value["hook_transport"] = json!(hook);
            Ok(value)
        }
        ClaudeCommand::ProtocolFixture { from_file } => {
            let safe = parse(&bounded(Some(from_file))?)?;
            Ok(
                json!({"fixture_valid":true,"metadata":safe,"mutated":false,"live_verified":false,"acknowledged":false,"hook_output":{}}),
            )
        }
    }
}

fn owned_additions(config: &Value, additions: &Value) -> Value {
    let mut owned = json!({});
    for (event, groups) in additions.as_object().unwrap() {
        let original = config
            .get("hooks")
            .and_then(|h| h.get(event))
            .and_then(Value::as_array);
        let items: Vec<Value> = groups
            .as_array()
            .unwrap()
            .iter()
            .filter(|g| !original.is_some_and(|v| v.contains(g)))
            .cloned()
            .collect();
        if !items.is_empty() {
            owned[event] = json!(items);
        }
    }
    owned
}
fn uninstall(p: &Project, plan: &str, expected: &str) -> Result<Value> {
    owner()?;
    label(plan)?;
    let _lock = acquire(p)?;
    let db = db(p)?;
    let record: Option<(String, String, String)> = db
        .query_row(
            "SELECT owned,config_hash,status FROM adapter_installs WHERE workspace=? AND plan=?",
            params![p.workspace_id, plan],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (owned, published, status) = record.ok_or_else(|| {
        err(
            "CONFIG_CONFLICT",
            "No installation ownership receipt for this plan",
        )
    })?;
    let (_, bytes, mut cfg) = config(p)?;
    if hash(&bytes) != expected {
        return Err(err(
            "PLAN_STALE",
            "Current settings hash does not match explicit uninstall expectation",
        ));
    }
    if status == "removed" {
        return Ok(json!({"removed":false,"already_removed":true,"permission_changes":[]}));
    }
    if status != "installed" && hash(&bytes) != published {
        return Err(err(
            "RECONCILIATION_REQUIRED",
            "Interrupted installation does not match published settings",
        ));
    }
    let owned: Value = serde_json::from_str(&owned)?;
    for (event, groups) in owned
        .as_object()
        .ok_or_else(|| err("CONFIG_CONFLICT", "Malformed ownership receipt"))?
    {
        let existing = cfg
            .get_mut("hooks")
            .and_then(|h| h.get_mut(event))
            .and_then(Value::as_array_mut)
            .ok_or_else(|| {
                err(
                    "CONFIG_CONFLICT",
                    "Owned hook event was modified; no settings changed",
                )
            })?;
        for group in groups.as_array().unwrap() {
            let at = existing.iter().position(|v| v == group).ok_or_else(|| {
                err(
                    "CONFIG_CONFLICT",
                    "Owned hook was modified; no settings changed",
                )
            })?;
            existing.remove(at);
        }
    }
    let desired = serde_json::to_vec_pretty(&cfg)?;
    if desired.len() > LIMIT {
        return Err(err("BUDGET_EXCEEDED", "Settings exceed 128 KiB"));
    }
    publish_config(p, &bytes, &desired)?;
    db.execute(
        "UPDATE adapter_installs SET status='removed' WHERE workspace=? AND plan=?",
        params![p.workspace_id, plan],
    )?;
    Ok(
        json!({"removed":true,"config_hash":hash(desired),"permission_changes":[],"global_settings_changed":false}),
    )
}
fn protection(safe: &Value, raw: &Value) -> Result<Value> {
    let name = safe["tool_name"]
        .as_str()
        .ok_or_else(|| err("INVALID_ARGUMENT", "PreToolUse tool_name required"))?;
    let mut decision = None;
    let mut reason = "Direct or opaque command requires explicit host approval and registered PCTX runner admission";
    if ["Read", "Grep", "Glob"].contains(&name) {
    } else if name == "Bash" {
        let command = raw
            .get("tool_input")
            .and_then(|v| v.get("command"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let words: Vec<_> = command.split_whitespace().collect();
        let opaque = command.is_empty()
            || command.len() > 16384
            || command
                .chars()
                .any(|c| c.is_control() || ";|&<>`$\\\"'(){}".contains(c));
        if words
            .iter()
            .any(|w| ["rm", "sudo", "deploy", "shutdown", "reboot", "mkfs", "dd"].contains(w))
            || words
                .windows(2)
                .any(|w| w == ["git", "push"] || w == ["git", "reset"])
        {
            decision = Some("deny");
            reason = "Dangerous direct command denied; use an explicit registered operation and independent host permission";
        } else if opaque
            || ![
                "pwd",
                "git status",
                "git status --short",
                "git diff",
                "git diff --stat",
                "git log --oneline",
            ]
            .contains(&command.trim())
        {
            decision = Some("ask");
        }
    } else {
        decision = Some("ask");
    }
    let output=decision.map(|d|json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":d,"permissionDecisionReason":reason}})).unwrap_or(json!({}));
    Ok(
        json!({"event":"PreToolUse","hook_output":output,"source_authority":"explicit-local-import","live_transport_verified":false,"registered_runner_receipt_verified":false,"argv_rewritten":false,"command_executed":false,"host_permission_granted":false,"acknowledged":false,"protection":"advisory_host_permission_flow"}),
    )
}
#[allow(clippy::too_many_arguments)]
fn statusline(
    p: &Project,
    task: &str,
    path: &Path,
    pool: &str,
    session: &str,
    epoch: i64,
    observed: &str,
    start: &str,
    end: &str,
    counter: &str,
    key: &str,
) -> Result<Value> {
    owner()?;
    for value in [task, pool, session, counter, key] {
        label(value)?;
    }
    let raw: Value = serde_json::from_slice(&bounded(Some(path))?)
        .map_err(|_| err("INVALID_ARGUMENT", "Malformed status-line JSON"))?;
    let version = raw["version"]
        .as_str()
        .ok_or_else(|| err("CAPABILITY_UNAVAILABLE", "Status-line version required"))?;
    let patch = version
        .strip_prefix("2.1.")
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| (211..=284).contains(v))
        .ok_or_else(|| {
            err(
                "CAPABILITY_UNAVAILABLE",
                "Unverified status-line version schema; collector disabled",
            )
        })?;
    let _ = patch;
    let native = raw["session_id"]
        .as_str()
        .ok_or_else(|| err("INVALID_ARGUMENT", "Native status-line session required"))?;
    label(native)?;
    let db = work::connect(p)?;
    let bound: Option<(Option<String>, i64, String)> = db
        .query_row(
            "SELECT native_id,epoch,workspace FROM pctx_sessions WHERE id=?",
            [session],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if !bound.is_some_and(|(id, e, w)| {
        id.as_deref() == Some(native) && e == epoch && w == p.workspace_id
    }) {
        return Err(err(
            "BASELINE_MISMATCH",
            "Status-line native identity, PCTX session and current epoch must match",
        ));
    }
    let model = raw["model"]["id"]
        .as_str()
        .ok_or_else(|| err("INVALID_ARGUMENT", "Status-line model identity required"))?;
    label(model)?;
    let base =
        |metric: &str, unit: &str, kind: &str, amount: Option<f64>, status: &str| Observation {
            observation_id: format!("adapter-{}", hash(format!("{key}:{metric}"))),
            pool_id: pool.into(),
            provider: "anthropic".into(),
            model: model.into(),
            metric: metric.into(),
            unit: unit.into(),
            source: "statusline".into(),
            collector: "explicit-claude-statusline-import".into(),
            source_revision: version.into(),
            observed_at: observed.into(),
            window_id: format!("session-{session}:{start}:{end}"),
            window_start: start.into(),
            window_end: end.into(),
            reset_at: None,
            timezone: Some("UTC".into()),
            status: status.into(),
            amount,
            kind: kind.into(),
            request_id: None,
            session_id: Some(session.into()),
            context_epoch: Some(epoch),
            counter_epoch: Some(counter.into()),
            task_id: Some(task.into()),
            role: None,
            pricing_table_version: if metric == "api_cost" {
                Some("claude-client-list-price-unverified".into())
            } else {
                None
            },
            counter_origin_zero: false,
            workload: "execution".into(),
        };
    let cost = raw.get("cost").and_then(|v| v.get("total_cost_usd"));
    let pct = raw
        .get("context_window")
        .and_then(|v| v.get("used_percentage"));
    for value in [cost, pct].into_iter().flatten() {
        if !value.is_null() && !value.is_number() {
            return Err(err(
                "INVALID_ARGUMENT",
                "Status-line counters must be numeric or null",
            ));
        }
    }
    let cost = cost.and_then(Value::as_f64);
    let pct = pct.and_then(Value::as_f64);
    if pct.is_some_and(|v| !v.is_finite() || !(0.0..=100.0).contains(&v)) {
        return Err(err("INVALID_ARGUMENT", "Context percentage outside 0..100"));
    }
    let batch = UsageBatch {
        schema_version: 1,
        observations: vec![base(
            "api_cost",
            "USD",
            "cumulative",
            cost,
            if cost.is_some() {
                "estimate"
            } else {
                "unknown"
            },
        )],
    };
    private_dir(&p.control_dir)?;
    let temporary = p.control_dir.join(format!(
        ".adapter-usage-{}.json",
        crate::domain::id("import")
    ));
    atomic_write(&temporary, &serde_json::to_vec(&batch)?, false)?;
    let imported = quota::execute(
        p,
        &QuotaCommand::Ingest {
            from_file: temporary.clone(),
            idempotency_key: format!("adapter-statusline-{key}"),
        },
    );
    let cleanup = fs::remove_file(&temporary);
    let mut result = imported?;
    cleanup?;
    result["source_authentication"] = json!("explicit_local_import_unverified");
    result["actual_native_collection"] = json!(false);
    result["context_snapshot"] = json!({"used_percentage":pct,"status":if pct.is_some(){"reported_snapshot"}else{"unknown"},"persisted_as_usage":false});
    result["token_counters_imported"] = json!(false);
    result["context_tokens_are_session_cumulative"] = json!(false);
    Ok(result)
}

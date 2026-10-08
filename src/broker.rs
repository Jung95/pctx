//! Workspace- and permission-scoped local Git snapshots with cross-process refresh admission.
use crate::{
    domain::{Error, Result, hash, now},
    project::Project,
};
use clap::Subcommand;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Subcommand)]
pub enum RepoCommand {
    Status {
        #[arg(long, default_value = "branch,dirty,head")]
        fields: String,
        #[arg(long, default_value = "current")]
        workspace: String,
    },
}
#[derive(Debug, Subcommand)]
pub enum CacheCommand {
    Stats,
}
#[derive(Clone)]
struct Snapshot {
    value: Value,
    observed: i64,
    generation: i64,
    status: String,
    revision: String,
}
fn error(code: &str, message: &str, exit: i32) -> Error {
    Error::new(code, message, exit)
}
fn millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
fn connect(p: &Project) -> Result<Connection> {
    let db = p.connect(true)?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS broker_snapshots(key TEXT PRIMARY KEY,workspace TEXT NOT NULL,policy TEXT NOT NULL,scope TEXT NOT NULL,generation INTEGER NOT NULL,observed_ms INTEGER NOT NULL,status TEXT NOT NULL,revision TEXT NOT NULL,value TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS broker_refresh_jobs(key TEXT PRIMARY KEY,generation INTEGER NOT NULL,pid INTEGER NOT NULL,start_identity TEXT NOT NULL,claimed_ms INTEGER NOT NULL,owner_active INTEGER NOT NULL,refresh_count INTEGER NOT NULL);")?;
    Ok(db)
}
fn permission_scope() -> String {
    hash(std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()))
}
fn key(p: &Project) -> String {
    hash(serde_json::to_vec(&json!({"project":p.project_id,"workspace":p.workspace_id,"coordination":p.coordination_id,"policy":p.policy_hash(),"permission":permission_scope(),"query":"git-status-v1"})).unwrap_or_default())
}
fn snapshot(db: &Connection, key: &str) -> Result<Option<Snapshot>> {
    let row:Option<(String,i64,i64,String,String)>=db.query_row("SELECT value,observed_ms,generation,status,revision FROM broker_snapshots WHERE key=?1",[key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
    row.map(|(value, observed, generation, status, revision)| {
        Ok(Snapshot {
            value: crate::domain::stored_json(&value, "Stored broker snapshot JSON is invalid")?,
            observed,
            generation,
            status,
            revision,
        })
    })
    .transpose()
}
fn valid(s: &Snapshot) -> bool {
    matches!(
        s.status.as_str(),
        "ok_nonempty" | "ok_empty" | "unsupported"
    ) && millis() >= s.observed
        && millis() - s.observed < 2000
}
fn identity(pid: u32, deadline: Instant) -> Option<String> {
    #[cfg(unix)]
    {
        let mut command = Command::new("ps");
        command
            .args(["-p", &pid.to_string(), "-o", "lstart="])
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("LC_ALL", "C");
        let output = crate::query_process::output(
            command,
            crate::deadline::Deadline::from_instant(deadline),
            65536,
        )
        .ok()?;
        if !output.status.success() {
            return None;
        }
        let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
        if value.is_empty() { None } else { Some(value) }
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, deadline);
        None
    }
}
fn proven_dead(pid: u32, started: &str, deadline: Instant) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: signal zero checks existence without delivering a signal.
        if unsafe { libc::kill(pid as i32, 0) } == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            return true;
        }
        if started.is_empty() {
            return false;
        }
        identity(pid, deadline).is_some_and(|current| current != started)
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, started, deadline);
        false
    }
}
fn claim(db: &mut Connection, key: &str, deadline: Instant) -> Result<Option<i64>> {
    let transaction = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if snapshot(&transaction, key)?.as_ref().is_some_and(valid) {
        return Ok(None);
    }
    let existing:Option<(i64,u32,String,bool)>=transaction.query_row("SELECT generation,pid,start_identity,owner_active FROM broker_refresh_jobs WHERE key=?1",[key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    if let Some((_, pid, ref started, true)) = existing
        && !proven_dead(pid, started, deadline)
    {
        return Ok(None);
    }
    let generation = existing.as_ref().map(|j| j.0 + 1).unwrap_or(1);
    let pid = std::process::id();
    let started = identity(pid, deadline).unwrap_or_default();
    crate::deadline::Deadline::from_instant(deadline).check()?;
    transaction.execute("INSERT INTO broker_refresh_jobs(key,generation,pid,start_identity,claimed_ms,owner_active,refresh_count) VALUES(?1,?2,?3,?4,?5,1,1) ON CONFLICT(key) DO UPDATE SET generation=excluded.generation,pid=excluded.pid,start_identity=excluded.start_identity,claimed_ms=excluded.claimed_ms,owner_active=1,refresh_count=refresh_count+1",params![key,generation,pid,started,millis()])?;
    transaction.commit()?;
    Ok(Some(generation))
}
fn git(p: &Project, args: &[&str], deadline: Instant) -> Result<Vec<u8>> {
    if Instant::now() >= deadline {
        return Err(error("TIMEOUT", "Local Git query timed out", 7));
    }
    let mut command = Command::new("git");
    command
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.untrackedCache=false",
            "-c",
            "core.pager=cat",
        ])
        .args(args)
        .current_dir(&p.root)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_PAGER", "cat")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = crate::query_process::output(
        command,
        crate::deadline::Deadline::from_instant(deadline),
        8 * 1024 * 1024,
    )?;
    if !output.status.success() {
        return Err(error(
            "SOURCE_UNAVAILABLE",
            "Local Git query failed; no empty result inferred",
            7,
        ));
    }
    Ok(output.stdout)
}
fn marker_exists(p: &Project) -> bool {
    p.root
        .ancestors()
        .any(|r| fs::symlink_metadata(r.join(".git")).is_ok())
}
fn parse_status(bytes: &[u8]) -> Result<Value> {
    let mut branch = Value::Null;
    let mut head = Value::Null;
    let mut tracked = 0usize;
    let mut untracked = 0usize;
    let mut conflicts = 0usize;
    let mut renamed = 0usize;
    let mut records = bytes.split(|b| *b == 0);
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        if record.starts_with(b"# ") {
            if let Some(value) = record.strip_prefix(b"# branch.oid ") {
                let value = std::str::from_utf8(value)
                    .map_err(|_| error("PARTIAL_RESULT", "Invalid Git branch metadata", 3))?;
                if value != "(initial)" {
                    if ![40, 64].contains(&value.len())
                        || !value.bytes().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err(error("PARTIAL_RESULT", "Invalid Git HEAD identity", 3));
                    }
                    head = json!(value);
                }
            }
            if let Some(value) = record.strip_prefix(b"# branch.head ") {
                let value = std::str::from_utf8(value)
                    .map_err(|_| error("PARTIAL_RESULT", "Invalid Git branch metadata", 3))?;
                if value != "(detached)" {
                    branch = json!(crate::reader::redact(value).0);
                }
            }
        } else if record.starts_with(b"? ") {
            untracked += 1;
        } else if record.starts_with(b"! ") {
            continue;
        } else if record.starts_with(b"1 ") {
            if record.splitn(9, |b| *b == b' ').count() != 9 {
                return Err(error(
                    "PARTIAL_RESULT",
                    "Malformed tracked Git status record",
                    3,
                ));
            }
            tracked += 1;
        } else if record.starts_with(b"2 ") {
            if record.splitn(10, |b| *b == b' ').count() != 10 {
                return Err(error(
                    "PARTIAL_RESULT",
                    "Malformed renamed Git status record",
                    3,
                ));
            }
            tracked += 1;
            renamed += 1;
            if records.next().is_none() {
                return Err(error("PARTIAL_RESULT", "Missing rename source record", 3));
            }
        } else if record.starts_with(b"u ") {
            if record.splitn(11, |b| *b == b' ').count() != 11 {
                return Err(error(
                    "PARTIAL_RESULT",
                    "Malformed unmerged Git status record",
                    3,
                ));
            }
            tracked += 1;
            conflicts += 1;
        } else {
            return Err(error("PARTIAL_RESULT", "Unsupported Git status record", 3));
        }
    }
    Ok(
        json!({"branch":branch,"head":head,"dirty":tracked+untracked>0,"counts":{"tracked":tracked,"untracked":untracked,"conflicts":conflicts,"renamed":renamed}}),
    )
}
fn refresh(p: &Project, deadline: Instant) -> Result<(Value, String)> {
    if !marker_exists(p) {
        return Ok((
            json!({"capability":"unsupported","reason":"no_git_repository"}),
            "unsupported".into(),
        ));
    }
    match git(p, &["rev-parse", "--is-inside-work-tree"], deadline) {
        Ok(bytes) if bytes == b"true\n" => {}
        Ok(_) => {
            return Ok((
                json!({"capability":"unsupported","reason":"not_a_worktree"}),
                "unsupported".into(),
            ));
        }
        Err(e) => return Err(e),
    }
    let bytes = git(
        p,
        &[
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
            "--ignore-submodules=all",
        ],
        deadline,
    )?;
    let data = parse_status(&bytes)?;
    Ok((data, "ok_nonempty".into()))
}
fn publish(
    db: &mut Connection,
    p: &Project,
    key: &str,
    generation: i64,
    value: Value,
    status: &str,
) -> Result<bool> {
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let current: Option<i64> = tx
        .query_row(
            "SELECT generation FROM broker_refresh_jobs WHERE key=?1 AND owner_active=1",
            [key],
            |r| r.get(0),
        )
        .optional()?;
    if current != Some(generation) {
        return Ok(false);
    }
    let revision = hash(serde_json::to_vec(&value)?);
    let observed = millis();
    tx.execute("INSERT INTO broker_snapshots(key,workspace,policy,scope,generation,observed_ms,status,revision,value) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(key) DO UPDATE SET generation=excluded.generation,observed_ms=excluded.observed_ms,status=excluded.status,revision=excluded.revision,value=excluded.value",params![key,p.workspace_id,p.policy_hash(),permission_scope(),generation,observed,status,revision,value.to_string()])?;
    tx.execute(
        "UPDATE broker_refresh_jobs SET owner_active=0 WHERE key=?1 AND generation=?2",
        params![key, generation],
    )?;
    p.check_deadline()?;
    tx.commit()?;
    Ok(true)
}
fn response(s: Option<Snapshot>, fields: &[&str], cache: &str, pending: bool) -> Value {
    let mut selected = serde_json::Map::new();
    if let Some(s) = &s {
        for field in fields {
            if let Some(value) = s.value.get(*field) {
                selected.insert((*field).into(), value.clone());
            }
        }
        if s.status == "unsupported" {
            selected.insert("capability".into(), json!("unsupported"));
            selected.insert("reason".into(), s.value["reason"].clone());
        }
    }
    let status = s
        .as_ref()
        .map(|s| s.status.as_str())
        .unwrap_or("unavailable");
    json!({"items":selected,"source_status":status,"source_revision":s.as_ref().map(|s|&s.revision),"source_generation":s.as_ref().map(|s|s.generation),"observed_at":s.as_ref().and_then(|s|chrono::DateTime::from_timestamp_millis(s.observed)).map(|t|t.to_rfc3339()),"age_ms":s.as_ref().map(|s|millis().saturating_sub(s.observed).max(0)),"cache_status":cache,"refresh_status":if pending{"refresh_pending"}else{"idle"},"coverage":{"status":if matches!(status,"ok_nonempty"|"ok_empty"){"complete"}else if status=="unsupported"{"unsupported"}else{"partial"},"reasons":if status=="unsupported"{vec!["git_unavailable_for_workspace"]}else{vec![]}},"authority":"git_local_observation","freshness":if s.as_ref().is_some_and(valid){"current"}else{"stale"},"permission_scope":permission_scope(),"workspace_atomic":false,"network":"not_used"})
}
fn repo_fields(command: &RepoCommand) -> Result<Vec<&str>> {
    let RepoCommand::Status { fields, workspace } = command;
    if workspace != "current" {
        return Err(error(
            "INVALID_ARGUMENT",
            "Only the current workspace can be queried",
            2,
        ));
    }
    let fields = fields.split(',').collect::<Vec<_>>();
    if fields.is_empty()
        || fields
            .iter()
            .any(|f| !matches!(*f, "branch" | "dirty" | "head" | "counts"))
    {
        return Err(error(
            "INVALID_ARGUMENT",
            "Unknown repository status field",
            2,
        ));
    }
    Ok(fields)
}
/// Pure existing workspace/field grammar for CLI and producer admission.
pub fn validate_repo_request(command: &RepoCommand) -> Result<()> {
    repo_fields(command).map(|_| ())
}
/// Ordinary local query deadline from §11; TTL is a freshness bound, not a wait bound.
pub fn repo(p: &Project, command: &RepoCommand) -> Result<Value> {
    repo_with_timeout(p, command, Duration::from_secs(10))
}

/// Wait for the current shared refresh without stealing a live owner's claim.
/// Expiry returns an explicitly pending last-known snapshot, including when unavailable.
pub fn repo_with_timeout(p: &Project, command: &RepoCommand, timeout: Duration) -> Result<Value> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .filter(|_| !timeout.is_zero())
        .ok_or_else(|| {
            error(
                "INVALID_ARGUMENT",
                "Repository query timeout must be positive and bounded",
                2,
            )
        })?;
    let deadline = p
        .deadline
        .map(|shared| shared.instant().min(deadline))
        .unwrap_or(deadline);
    let mut scoped = p.clone();
    scoped.deadline = Some(crate::deadline::Deadline::from_instant(deadline));
    let p = &scoped;
    let fields = repo_fields(command)?;
    let key = key(p);
    let mut db = connect(p)?;
    loop {
        if let Some(s) = snapshot(&db, &key)?.filter(valid) {
            return Ok(response(Some(s), &fields, "hit", false));
        }
        if Instant::now() >= deadline {
            return Ok(response(snapshot(&db, &key)?, &fields, "last_known", true));
        }
        // Do not let repeated SQLite contention reset the caller's remaining wait.
        db.busy_timeout(deadline.saturating_duration_since(Instant::now()))?;
        if let Some(generation) = claim(&mut db, &key, deadline)? {
            // No SQLite transaction is held over the Git process.
            match refresh(p, deadline) {
                Ok((data, status)) => {
                    if publish(&mut db, p, &key, generation, data, &status)? {
                        return Ok(response(snapshot(&db, &key)?, &fields, "refreshed", false));
                    }
                }
                Err(e) => {
                    db.execute("UPDATE broker_refresh_jobs SET owner_active=0 WHERE key=?1 AND generation=?2",params![key,generation])?;
                    return Err(e);
                }
            }
        }
        thread::sleep(
            Duration::from_millis(10).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}
pub fn stats(p: &Project) -> Result<Value> {
    let db = connect(p)?;
    let scope = permission_scope();
    let count: i64 = db.query_row(
        "SELECT count(*) FROM broker_snapshots WHERE workspace=?1 AND policy=?2 AND scope=?3",
        params![p.workspace_id, p.policy_hash(), scope],
        |r| r.get(0),
    )?;
    let attempts: i64 = db.query_row(
        "SELECT coalesce(sum(refresh_count),0) FROM broker_refresh_jobs WHERE key=?1",
        [key(p)],
        |r| r.get(0),
    )?;
    let active: i64 = db.query_row(
        "SELECT count(*) FROM broker_refresh_jobs WHERE key=?1 AND owner_active=1",
        [key(p)],
        |r| r.get(0),
    )?;
    Ok(
        json!({"scope":"current_workspace_permission","snapshots":count,"upstream_refreshes":attempts,"refresh_jobs_active_or_unknown":active,"ttl_ms":2000,"source_code_persisted":false,"external_provider":"not_implemented","observed_at":now()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nul_names_rename_and_unborn_metadata_are_machine_parsed() {
        let value=parse_status(b"# branch.oid (initial)\0# branch.head main\0? space and\nnewline\0? leading # filename\0").unwrap();
        assert_eq!(value["head"], Value::Null);
        assert_eq!(value["counts"]["untracked"], 2);
        assert_eq!(value["dirty"], true);
        let value =
            parse_status(b"2 R. N... 100644 100644 100644 abc abc R100 new path\0old\npath\0")
                .unwrap();
        assert_eq!(value["counts"]["renamed"], 1);
        assert!(parse_status(b"u invalid\0").is_err());
    }
}

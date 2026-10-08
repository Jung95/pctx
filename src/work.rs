//! Durable local task coordination. The control database is independent of the index.
use crate::{
    domain::{Error, Result, hash, id, now},
    project::Project,
};
use clap::Subcommand;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug, Clone, Subcommand)]
pub enum WorkCommand {
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    Check {
        #[command(subcommand)]
        command: CheckCommand,
    },
    Control {
        #[command(subcommand)]
        command: ControlCommand,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum TaskCommand {
    Create {
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    List,
    Show {
        task: String,
    },
    Edit {
        task: String,
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        expect_revision: i64,
    },
    Ready {
        task: String,
        #[arg(long)]
        expect_revision: Option<i64>,
    },
    Assign {
        task: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        expect_revision: Option<i64>,
    },
    Reassign {
        task: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        expect_revision: Option<i64>,
    },
    Start {
        task: String,
        #[arg(long, default_value = "current")]
        workspace: String,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        expect_revision: Option<i64>,
    },
    Claim {
        task: String,
        #[arg(long)]
        agent: String,
        #[arg(long, default_value = "current")]
        workspace: String,
        #[arg(long)]
        expect_revision: Option<i64>,
    },
    Block {
        task: String,
        #[arg(long)]
        reason: String,
    },
    Pause {
        task: String,
        #[arg(long)]
        reason: String,
    },
    Resume {
        task: String,
    },
    Cancel {
        task: String,
        #[arg(long)]
        reason: String,
    },
    Reopen {
        task: String,
        #[arg(long)]
        reason: String,
    },
    Criterion {
        #[command(subcommand)]
        command: CriterionCommand,
    },
    Submit {
        task: String,
        #[arg(long)]
        run: String,
        #[arg(long)]
        expect_revision: Option<i64>,
    },
    Review {
        task: String,
        #[arg(long)]
        approve: bool,
        #[arg(long, default_value = "owner")]
        actor: String,
        #[arg(long, default_value = "")]
        note: String,
    },
    Complete {
        task: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        expect_revision: Option<i64>,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum CriterionCommand {
    Accept {
        task: String,
        #[arg(long)]
        criterion: String,
        #[arg(long)]
        evidence: String,
        #[arg(long, default_value = "")]
        note: String,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum AgentCommand {
    Register {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "agent")]
        kind: String,
        #[arg(long, default_value_t = 1)]
        concurrency_limit: u32,
    },
    List,
    Show {
        agent: String,
    },
    Report {
        #[arg(long)]
        run: String,
        #[arg(long)]
        lease_epoch: i64,
        #[arg(long)]
        report_seq: i64,
        #[arg(long)]
        idempotency_key: String,
        #[arg(long)]
        stage: String,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        estimate_percent: Option<u8>,
    },
    Heartbeat {
        #[arg(long)]
        run: String,
        #[arg(long)]
        lease_epoch: i64,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum CheckCommand {
    Plan {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        run: Option<String>,
    },
    Run {
        key: Option<String>,
        #[arg(long = "key", required_unless_present = "key", conflicts_with = "key")]
        registered_key: Option<String>,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        run: String,
        #[arg(long, default_value_t = 8192)]
        budget_bytes: usize,
    },
    Begin {
        task: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        run: String,
    },
    Record {
        check: String,
        #[arg(long)]
        from_file: PathBuf,
    },
    List {
        #[arg(long)]
        task: Option<String>,
    },
    Show {
        check: String,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum ControlCommand {
    Backup {
        #[arg(long)]
        output: PathBuf,
    },
    Restore {
        #[arg(long)]
        input: PathBuf,
    },
}
fn one() -> u32 {
    1
}
fn yes() -> bool {
    true
}
fn full() -> Vec<String> {
    vec!["**".into()]
}
fn codes() -> Vec<i32> {
    vec![0]
}
fn sources() -> Vec<String> {
    vec!["external_report".into()]
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskDefinition {
    pub schema_version: u32,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "priority")]
    pub priority: String,
    pub scope: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub acceptance: Vec<Criterion>,
    pub checks: Vec<CheckDefinition>,
    #[serde(default)]
    pub review_policy: ReviewPolicy,
}
fn priority() -> String {
    "P2".into()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub id: String,
    pub description: String,
    #[serde(default = "yes")]
    pub required: bool,
    #[serde(default = "one")]
    pub weight: u32,
    #[serde(default)]
    pub evidence_check_keys: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckDefinition {
    pub key: String,
    pub kind: String,
    #[serde(default = "yes")]
    pub required: bool,
    #[serde(default = "full")]
    pub input_scope: Vec<String>,
    #[serde(default)]
    pub output_paths: Vec<String>,
    #[serde(default)]
    pub minimum_executed_tests: Option<u64>,
    #[serde(default = "codes")]
    pub success_exit_codes: Vec<i32>,
    #[serde(default = "sources")]
    pub allowed_sources: Vec<String>,
    #[serde(default)]
    pub require_report_attachment: bool,
    #[serde(default)]
    pub not_applicable_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewPolicy {
    #[serde(default = "yes")]
    pub required: bool,
    #[serde(default = "human")]
    pub reviewer_kind: String,
}
fn human() -> String {
    "human".into()
}
impl Default for ReviewPolicy {
    fn default() -> Self {
        Self {
            required: true,
            reviewer_kind: human(),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckReport {
    pub schema_version: u32,
    pub check_key: String,
    pub producer: String,
    pub source: String,
    pub exit_code: i32,
    pub tests: u64,
    pub passed: u64,
    pub failed: u64,
    pub errors: u64,
    pub skipped: u64,
    pub result: String,
    pub started_at: i64,
    pub finished_at: i64,
    #[serde(default)]
    pub environment: Value,
}

pub fn connect(project: &Project) -> Result<Connection> {
    let db = project.connect(true)?;
    let initialized: i64 = db.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='work_meta'",
        [],
        |r| r.get(0),
    )?;
    if initialized > 0 {
        return Ok(db);
    }
    db.execute_batch("BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS work_meta(id INTEGER PRIMARY KEY CHECK(id=1),revision INTEGER NOT NULL);
INSERT OR IGNORE INTO work_meta VALUES(1,0);
CREATE TABLE IF NOT EXISTS tasks(id TEXT PRIMARY KEY,number INTEGER UNIQUE NOT NULL,state TEXT NOT NULL,revision INTEGER NOT NULL,definition_revision INTEGER NOT NULL,definition TEXT NOT NULL,agent TEXT,workspace TEXT,reason TEXT,resume_state TEXT,submission TEXT,completion TEXT);
CREATE TABLE IF NOT EXISTS agents(id TEXT PRIMARY KEY,name TEXT UNIQUE NOT NULL,kind TEXT NOT NULL,concurrency_limit INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS runs(id TEXT PRIMARY KEY,task TEXT NOT NULL REFERENCES tasks(id),agent TEXT NOT NULL REFERENCES agents(id),workspace TEXT NOT NULL,epoch INTEGER NOT NULL,status TEXT NOT NULL,lease_until INTEGER NOT NULL,last_seen INTEGER NOT NULL,last_progress INTEGER NOT NULL,seq INTEGER NOT NULL DEFAULT 0,stage TEXT NOT NULL DEFAULT 'planning',summary TEXT NOT NULL DEFAULT '',estimate INTEGER);
CREATE UNIQUE INDEX IF NOT EXISTS active_run ON runs(task) WHERE status='active';
CREATE TABLE IF NOT EXISTS checks(id TEXT PRIMARY KEY,task TEXT NOT NULL REFERENCES tasks(id),key TEXT NOT NULL,run TEXT NOT NULL REFERENCES runs(id),definition_revision INTEGER NOT NULL,target TEXT NOT NULL,policy TEXT NOT NULL,status TEXT NOT NULL,report TEXT,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS acceptances(task TEXT NOT NULL,criterion TEXT NOT NULL,evidence TEXT NOT NULL,definition_revision INTEGER NOT NULL,PRIMARY KEY(task,criterion));
CREATE TABLE IF NOT EXISTS reviews(id TEXT PRIMARY KEY,task TEXT NOT NULL,submission TEXT NOT NULL,actor TEXT NOT NULL,approved INTEGER NOT NULL,note TEXT NOT NULL,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS events(seq INTEGER PRIMARY KEY AUTOINCREMENT,entity TEXT NOT NULL,type TEXT NOT NULL,payload TEXT NOT NULL,created INTEGER NOT NULL);
CREATE TRIGGER IF NOT EXISTS events_no_update BEFORE UPDATE ON events BEGIN SELECT RAISE(ABORT,'append-only events'); END;
CREATE TRIGGER IF NOT EXISTS events_no_delete BEFORE DELETE ON events BEGIN SELECT RAISE(ABORT,'append-only events'); END;
CREATE TABLE IF NOT EXISTS receipts(key TEXT PRIMARY KEY,request_hash TEXT NOT NULL,response TEXT NOT NULL);
PRAGMA user_version=1; COMMIT;")?;
    Ok(db)
}
fn conflict(code: &str, msg: &str) -> Error {
    Error::new(code, msg, 9)
}
fn invalid(msg: &str) -> Error {
    Error::new("INVALID_ARGUMENT", msg, 2)
}
fn parse_file<T: serde::de::DeserializeOwned>(p: &PathBuf) -> Result<T> {
    let m = std::fs::metadata(p)?;
    if m.len() > 1048576 {
        return Err(invalid("Input JSON exceeds one MiB"));
    }
    let mut value: Value = serde_json::from_slice(&std::fs::read(p)?)?;
    sanitize(&mut value);
    Ok(serde_json::from_value(value)?)
}
fn sanitize(value: &mut Value) {
    match value {
        Value::String(s) => *s = crate::reader::redact(s).0,
        Value::Array(a) => {
            for v in a {
                sanitize(v);
            }
        }
        Value::Object(m) => {
            for (key, v) in m {
                if [
                    "password",
                    "secret",
                    "api_key",
                    "token",
                    "credential",
                    "authorization",
                ]
                .contains(&key.to_lowercase().as_str())
                {
                    *v = json!("[REDACTED]");
                } else {
                    sanitize(v);
                }
            }
        }
        _ => {}
    }
}
fn glob(patterns: &[String]) -> Result<globset::GlobSet> {
    let mut b = globset::GlobSetBuilder::new();
    for p in patterns {
        if p.starts_with('/') || p.split('/').any(|s| s == "..") {
            return Err(invalid("Scopes must be project-relative"));
        }
        b.add(globset::Glob::new(p).map_err(|_| invalid("Invalid scope glob"))?);
    }
    b.build().map_err(|_| invalid("Invalid scope"))
}
fn validate(d: &TaskDefinition) -> Result<()> {
    if d.schema_version != 1
        || d.title.trim().is_empty()
        || d.scope.is_empty()
        || !["P0", "P1", "P2", "P3"].contains(&d.priority.as_str())
    {
        return Err(invalid("Invalid task definition"));
    }
    glob(&d.scope)?;
    let mut keys = BTreeSet::new();
    for c in &d.checks {
        if c.key.is_empty()
            || !keys.insert(&c.key)
            || !["test", "lint", "typecheck", "manual", "not_applicable"].contains(&c.kind.as_str())
        {
            return Err(invalid("Invalid or duplicate check"));
        }
        glob(&c.input_scope)?;
        glob(&c.output_paths)?;
        if c.allowed_sources.iter().any(|source| {
            !["external_report", "manual_claim", "runner_observed"].contains(&source.as_str())
        }) {
            return Err(invalid("Unsupported check evidence source"));
        }
        if c.kind == "not_applicable"
            && c.not_applicable_reason
                .as_ref()
                .is_none_or(|s| s.trim().is_empty())
        {
            return Err(invalid("Check exemption needs a reason"));
        }
    }
    let mut criteria = BTreeSet::new();
    for c in &d.acceptance {
        if c.id.is_empty()
            || c.weight == 0
            || !criteria.insert(&c.id)
            || c.evidence_check_keys.iter().any(|k| !keys.contains(k))
        {
            return Err(invalid("Invalid criterion or evidence check key"));
        }
    }
    Ok(())
}
#[derive(Clone)]
struct Task {
    id: String,
    number: i64,
    state: String,
    revision: i64,
    def_rev: i64,
    def: TaskDefinition,
    agent: Option<String>,
    workspace: Option<String>,
    submission: Option<Value>,
    completion: Option<Value>,
}
fn task(db: &Connection, name: &str) -> Result<Task> {
    let number = name
        .strip_prefix("T-")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(-1);
    let row=db.query_row("SELECT id,number,state,revision,definition_revision,definition,agent,workspace,submission,completion FROM tasks WHERE id=?1 OR number=?2",params![name,number],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?,r.get::<_,Option<String>>(6)?,r.get::<_,Option<String>>(7)?,r.get::<_,Option<String>>(8)?,r.get::<_,Option<String>>(9)?))).optional()?.ok_or_else(||Error::new("TASK_NOT_FOUND","Task does not exist",6))?;
    Ok(Task {
        id: row.0,
        number: row.1,
        state: row.2,
        revision: row.3,
        def_rev: row.4,
        def: serde_json::from_str(&row.5)?,
        agent: row.6,
        workspace: row.7,
        submission: row.8.map(|s| serde_json::from_str(&s)).transpose()?,
        completion: row.9.map(|s| serde_json::from_str(&s)).transpose()?,
    })
}
fn expect(t: &Task, r: Option<i64>) -> Result<()> {
    if r.is_some_and(|r| r != t.revision) {
        Err(conflict("REVISION_CONFLICT", "Task revision changed"))
    } else {
        Ok(())
    }
}
fn event(db: &Connection, entity: &str, kind: &str, payload: &Value) -> Result<Value> {
    db.execute(
        "INSERT INTO events(entity,type,payload,created) VALUES(?1,?2,?3,?4)",
        params![entity, kind, payload.to_string(), now()],
    )?;
    let seq = db.last_insert_rowid();
    db.execute("UPDATE work_meta SET revision=revision+1 WHERE id=1", [])?;
    let rev: i64 = db.query_row("SELECT revision FROM work_meta", [], |r| r.get(0))?;
    Ok(json!({"event_seq":seq,"control_revision":rev}))
}
fn changed(db: &Connection, t: &Task, state: &str, kind: &str) -> Result<Value> {
    let n = db.execute(
        "UPDATE tasks SET state=?1,revision=revision+1 WHERE id=?2 AND revision=?3",
        params![state, t.id, t.revision],
    )?;
    if n != 1 {
        return Err(conflict("REVISION_CONFLICT", "Task revision changed"));
    }
    let e = event(
        db,
        &t.id,
        kind,
        &json!({"state":state,"task_revision":t.revision+1}),
    )?;
    Ok(
        json!({"task_id":t.id,"display_id":format!("T-{}",t.number),"state":state,"task_revision":t.revision+1,"event":e}),
    )
}
fn agent_id(db: &Connection, name: &str) -> Result<String> {
    db.query_row(
        "SELECT id FROM agents WHERE id=?1 OR name=?1",
        [name],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| Error::new("AGENT_NOT_FOUND", "Agent not registered", 6))
}
fn dependencies(db: &Connection, t: &Task) -> Result<()> {
    for dep in &t.def.dependencies {
        if task(db, dep)?.state != "done" {
            return Err(Error::new(
                "COMPLETION_GATE_FAILED",
                "Required predecessor is incomplete",
                10,
            ));
        }
    }
    Ok(())
}
fn issue_capability(project: &Project, run: &str, epoch: i64) -> Result<()> {
    let dir = project.workspace_dir.join("credentials");
    crate::project::private_dir(&dir)?;
    let credential =
        json!({"run_id":run,"lease_epoch":epoch,"capability":uuid::Uuid::new_v4().to_string()});
    crate::project::atomic_write(
        &dir.join(format!("{run}.json")),
        &serde_json::to_vec(&credential)?,
        false,
    )
}
fn authenticate(project: &Project, run: &str, epoch: i64) -> Result<()> {
    if std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()) == "owner" {
        return Ok(());
    }
    let supplied = std::env::var("PCTX_RUN_CAPABILITY")
        .map_err(|_| Error::new("POLICY_DENIED", "Agent writes require run capability", 5))?;
    let bytes = std::fs::read(
        project
            .workspace_dir
            .join("credentials")
            .join(format!("{run}.json")),
    )
    .map_err(|_| Error::new("POLICY_DENIED", "Run capability unavailable", 5))?;
    let credential: Value = serde_json::from_slice(&bytes)?;
    if credential["run_id"] != run
        || credential["lease_epoch"] != epoch
        || credential["capability"] != supplied
    {
        return Err(Error::new(
            "POLICY_DENIED",
            "Run capability does not match",
            5,
        ));
    }
    Ok(())
}
fn owner() -> Result<()> {
    if std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()) != "owner" {
        return Err(Error::new(
            "POLICY_DENIED",
            "Command requires local owner authority",
            5,
        ));
    }
    Ok(())
}
fn lease(
    db: &Connection,
    run: &str,
    epoch: Option<i64>,
    project: &Project,
) -> Result<(String, String, i64)> {
    let (task, agent, ws, ep, status, until): (String, String, String, i64, String, i64) = db
        .query_row(
            "SELECT task,agent,workspace,epoch,status,lease_until FROM runs WHERE id=?1",
            [run],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| conflict("LEASE_REVOKED", "Run does not exist"))?;
    if status != "active" || epoch.is_some_and(|e| e != ep) {
        return Err(conflict("LEASE_REVOKED", "Run lease was revoked"));
    }
    if ws != project.workspace_id {
        return Err(conflict(
            "LEASE_REVOKED",
            "Run belongs to a different workspace",
        ));
    }
    if until <= now() {
        return Err(conflict(
            "LEASE_EXPIRED",
            "Run lease expired; explicitly resume",
        ));
    }
    authenticate(project, run, ep)?;
    let actor = std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into());
    if actor != "owner" {
        let matches: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE id=?1 AND (id=?2 OR name=?2))",
            params![agent, actor],
            |r| r.get(0),
        )?;
        if !matches {
            return Err(Error::new(
                "POLICY_DENIED",
                "Actor does not own the run capability",
                5,
            ));
        }
    }
    Ok((task, agent, ep))
}
fn fingerprint(
    project: &Project,
    d: &TaskDefinition,
    check: Option<&CheckDefinition>,
) -> Result<String> {
    let inventory = crate::reader::inventory(project, false)?;
    if !inventory.skipped.is_empty() {
        return Err(Error::new(
            "PARTIAL_RESULT",
            "Verification inputs are incomplete",
            3,
        ));
    }
    let outputs = glob(&check.map(|c| c.output_paths.clone()).unwrap_or_default())?;
    let scope = glob(&check.map(|c| c.input_scope.clone()).unwrap_or_else(full))?;
    let required = glob(&d.scope)?;
    let mut files = std::collections::BTreeMap::new();
    for path in inventory.paths {
        let is_config = path == ".pctx/config.toml"
            || path.ends_with("lock")
            || path.ends_with("lock.json")
            || path == "Cargo.toml"
            || path == "package.json";
        if outputs.is_match(&path) {
            if required.is_match(&path) || is_config {
                return Err(invalid(
                    "Declared output overlaps required source or configuration",
                ));
            }
            continue;
        }
        if scope.is_match(&path) || required.is_match(&path) || is_config {
            files.insert(path.clone(), crate::reader::read(project, &path)?.hash);
        }
    }
    Ok(hash(serde_json::to_vec(
        &json!({"files":files,"definition":d,"policy":project.policy_hash(),"workspace":project.workspace_id}),
    )?))
}
fn evidence_set(db: &Connection, t: &Task) -> Result<Value> {
    let mut s =
        db.prepare("SELECT id,key,status,target,report FROM checks WHERE task=?1 ORDER BY rowid")?;
    let checks=s.query_map([&t.id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"key":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"target":r.get::<_,String>(3)?,"report":r.get::<_,Option<String>>(4)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    let mut s=db.prepare("SELECT criterion,evidence,definition_revision FROM acceptances WHERE task=?1 ORDER BY criterion")?;
    let accept=s.query_map([&t.id],|r|Ok(json!({"criterion":r.get::<_,String>(0)?,"evidence":r.get::<_,String>(1)?,"definition_revision":r.get::<_,i64>(2)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(json!({"checks":checks,"acceptances":accept}))
}
fn check_environment_current(project: &Project, key: &str, stored: Option<&str>) -> bool {
    let Some(data) = stored.and_then(|s| serde_json::from_str::<Value>(s).ok()) else {
        return false;
    };
    // External reports remain explicitly allowed assertions, without local execution authority.
    match data["report"]["source"].as_str() {
        Some("external_report") => true,
        Some("runner_observed") => crate::runner::current_check_binding(project, key)
            .is_ok_and(|current| data["check_binding"] == current),
        _ => false,
    }
}
fn required_environments_current(db: &Connection, project: &Project, t: &Task) -> Result<bool> {
    for c in t
        .def
        .checks
        .iter()
        .filter(|c| c.required && c.kind != "not_applicable")
    {
        let report: Option<Option<String>> = db
            .query_row(
                "SELECT report FROM checks WHERE task=?1 AND key=?2 ORDER BY rowid DESC LIMIT 1",
                params![t.id, c.key],
                |r| r.get(0),
            )
            .optional()?;
        if !check_environment_current(project, &c.key, report.flatten().as_deref()) {
            return Ok(false);
        }
    }
    Ok(true)
}
fn gates(db: &Connection, project: &Project, t: &Task) -> Result<Value> {
    let mut failures = vec![];
    if t.state != "in_review" {
        failures.push("task_not_in_review".to_string());
    }
    if dependencies(db, t).is_err() {
        failures.push("dependencies_incomplete".into());
    }
    let target = fingerprint(project, &t.def, None)?;
    let evidence = evidence_set(db, t)?;
    let evidence_hash = hash(evidence.to_string());
    if let Some(sub) = &t.submission {
        if sub["target"] != target
            || sub["definition_revision"] != t.def_rev
            || sub["evidence_hash"] != evidence_hash
        {
            failures.push("submission_stale".into());
        }
        let approvals:i64=db.query_row("SELECT count(*) FROM reviews WHERE task=?1 AND submission=?2 AND approved=1 AND actor='owner'",params![t.id,sub["id"].as_str().unwrap_or("")],|r|r.get(0))?;
        if approvals == 0 {
            failures.push("owner_review_required".into());
        }
    } else {
        failures.push("submission_missing".into());
    }
    for c in &t.def.acceptance {
        if c.required {
            let accepted:i64=db.query_row("SELECT count(*) FROM acceptances WHERE task=?1 AND criterion=?2 AND definition_revision=?3",params![t.id,c.id,t.def_rev],|r|r.get(0))?;
            if accepted == 0 {
                failures.push(format!("criterion:{}", c.id));
            }
        }
    }
    for c in &t.def.checks {
        if !c.required || c.kind == "not_applicable" {
            continue;
        }
        let latest:Option<(String,String,i64,String,Option<String>)>=db.query_row("SELECT status,target,definition_revision,policy,report FROM checks WHERE task=?1 AND key=?2 ORDER BY rowid DESC LIMIT 1",params![t.id,c.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
        let current = fingerprint(project, &t.def, Some(c))?;
        if !latest.is_some_and(|(s, h, r, p, report)| {
            s == "passed"
                && h == current
                && r == t.def_rev
                && p == project.policy_hash()
                && check_environment_current(project, &c.key, report.as_deref())
        }) {
            failures.push(format!("check:{}", c.key));
        }
    }
    let active: i64 = db.query_row(
        "SELECT count(*) FROM runs WHERE task=?1 AND status='active'",
        [&t.id],
        |r| r.get(0),
    )?;
    if active > 0 {
        failures.push("active_run".into());
    }
    Ok(
        json!({"passed":failures.is_empty(),"failures":failures,"target":target,"evidence_hash":evidence_hash}),
    )
}
pub fn execute(project: &Project, command: &WorkCommand) -> Result<Value> {
    if let WorkCommand::Check {
        command: CheckCommand::Plan { task_id, key, run },
    } = command
    {
        return crate::runner::execute(
            project,
            &crate::runner::RunnerCommand::CheckPlan {
                task_id: task_id.clone(),
                key: key.clone(),
                run: run.clone(),
            },
        );
    }
    if let WorkCommand::Check {
        command:
            CheckCommand::Run {
                key,
                registered_key,
                task_id,
                run,
                budget_bytes,
            },
    } = command
    {
        let key = key
            .as_ref()
            .or(registered_key.as_ref())
            .ok_or_else(|| invalid("Check key is required"))?;
        return crate::runner::execute(
            project,
            &crate::runner::RunnerCommand::CheckRun {
                task_id: task_id.clone(),
                key: key.clone(),
                run: run.clone(),
                budget_bytes: *budget_bytes,
            },
        );
    }
    if let WorkCommand::Control { command } = command {
        return control_command(project, command);
    }
    let mut db = connect(project)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let data = match command {
        WorkCommand::Task { command } => task_command(project, &tx, command)?,
        WorkCommand::Agent { command } => agent_command(project, &tx, command)?,
        WorkCommand::Check { command } => check_command(project, &tx, command)?,
        WorkCommand::Control { .. } => unreachable!("control commands handled before transaction"),
    };
    tx.commit()?;
    Ok(data)
}
fn task_command(project: &Project, db: &Connection, command: &TaskCommand) -> Result<Value> {
    if !matches!(
        command,
        TaskCommand::List | TaskCommand::Show { .. } | TaskCommand::Submit { .. }
    ) {
        owner()?;
    }
    match command {
        TaskCommand::Create {
            from_file,
            idempotency_key,
        } => {
            let d: TaskDefinition = parse_file(from_file)?;
            validate(&d)?;
            let request = hash(serde_json::to_vec(&d)?);
            if let Some(key) = idempotency_key
                && let Some((h, response)) = db
                    .query_row(
                        "SELECT request_hash,response FROM receipts WHERE key=?1",
                        [key],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                    )
                    .optional()?
            {
                if h != request {
                    return Err(conflict(
                        "IDEMPOTENCY_CONFLICT",
                        "Key used with different task definition",
                    ));
                }
                return Ok(serde_json::from_str(&response)?);
            }
            let number: i64 =
                db.query_row("SELECT coalesce(max(number),0)+1 FROM tasks", [], |r| {
                    r.get(0)
                })?;
            let task_id = id("TASK");
            db.execute("INSERT INTO tasks(id,number,state,revision,definition_revision,definition) VALUES(?1,?2,'backlog',1,1,?3)",params![task_id,number,serde_json::to_string(&d)?])?;
            let e = event(
                db,
                &task_id,
                "task_created",
                &json!({"definition_revision":1}),
            )?;
            let v = json!({"task_id":task_id,"display_id":format!("T-{number}"),"task_revision":1,"event":e});
            if let Some(key) = idempotency_key {
                db.execute(
                    "INSERT INTO receipts VALUES(?1,?2,?3)",
                    params![key, request, v.to_string()],
                )?;
            }
            Ok(v)
        }
        TaskCommand::List => board_db(project, db),
        TaskCommand::Show { task: name } => task_view(project, db, &task(db, name)?),
        TaskCommand::Edit {
            task: name,
            from_file,
            expect_revision,
        } => {
            let t = task(db, name)?;
            expect(&t, Some(*expect_revision))?;
            let d: TaskDefinition = parse_file(from_file)?;
            validate(&d)?;
            if t.state == "done" || t.state == "cancelled" {
                return Err(conflict("REVISION_CONFLICT", "Reopen task before editing"));
            }
            for dep in &d.dependencies {
                let predecessor = task(db, dep)?;
                if predecessor.id == t.id || reaches(db, &predecessor, &t.id, &mut BTreeSet::new())?
                {
                    return Err(conflict("DEPENDENCY_CYCLE", "Dependency cycle rejected"));
                }
            }
            db.execute("UPDATE tasks SET definition=?1,definition_revision=definition_revision+1,submission=NULL WHERE id=?2",params![serde_json::to_string(&d)?,t.id])?;
            changed(db, &t, &t.state, "task_definition_changed")
        }
        TaskCommand::Ready {
            task: name,
            expect_revision,
        } => {
            let t = task(db, name)?;
            expect(&t, *expect_revision)?;
            if !["backlog", "blocked", "paused"].contains(&t.state.as_str()) {
                return Err(conflict("REVISION_CONFLICT", "Task cannot enter ready"));
            }
            validate(&t.def)?;
            dependencies(db, &t)?;
            changed(db, &t, "ready", "task_ready")
        }
        TaskCommand::Assign {
            task: name,
            agent,
            expect_revision,
        }
        | TaskCommand::Reassign {
            task: name,
            agent,
            expect_revision,
            ..
        } => {
            let t = task(db, name)?;
            expect(&t, *expect_revision)?;
            if ["done", "cancelled"].contains(&t.state.as_str()) {
                return Err(conflict(
                    "REVISION_CONFLICT",
                    "Terminal task requires reopen before assignment",
                ));
            }
            let a = agent_id(db, agent)?;
            let reassign = matches!(command, TaskCommand::Reassign { .. });
            if let TaskCommand::Reassign { reason, .. } = command
                && reason.trim().is_empty()
            {
                return Err(invalid("Reassignment needs reason"));
            }
            let active: i64 = db.query_row(
                "SELECT count(*) FROM runs WHERE task=?1 AND status='active'",
                [&t.id],
                |r| r.get(0),
            )?;
            if active > 0 && !reassign {
                return Err(conflict(
                    "TASK_ALREADY_CLAIMED",
                    "Use explicit reassign for an active task",
                ));
            }
            db.execute("UPDATE runs SET status='interrupted',epoch=epoch+1 WHERE task=?1 AND status='active'",[&t.id])?;
            db.execute("UPDATE tasks SET agent=?1 WHERE id=?2", params![a, t.id])?;
            changed(
                db,
                &t,
                if reassign { "ready" } else { &t.state },
                "task_assigned",
            )
        }
        TaskCommand::Start {
            task: name,
            workspace,
            agent,
            expect_revision,
        } => start(
            project,
            db,
            name,
            workspace,
            agent.as_deref(),
            *expect_revision,
        ),
        TaskCommand::Claim {
            task: name,
            workspace,
            agent,
            expect_revision,
        } => start(project, db, name, workspace, Some(agent), *expect_revision),
        TaskCommand::Block { task: name, reason }
        | TaskCommand::Pause { task: name, reason }
        | TaskCommand::Cancel { task: name, reason }
        | TaskCommand::Reopen { task: name, reason } => {
            let t = task(db, name)?;
            if reason.trim().is_empty() {
                return Err(invalid("State transition requires reason"));
            }
            let state = match command {
                TaskCommand::Block { .. } => "blocked",
                TaskCommand::Pause { .. } => "paused",
                TaskCommand::Cancel { .. } => "cancelled",
                _ => "ready",
            };
            if matches!(command, TaskCommand::Reopen { .. }) {
                if !["done", "cancelled"].contains(&t.state.as_str()) {
                    return Err(conflict(
                        "REVISION_CONFLICT",
                        "Only terminal tasks can reopen",
                    ));
                }
            } else if ["done", "cancelled"].contains(&t.state.as_str()) {
                return Err(conflict(
                    "REVISION_CONFLICT",
                    "Terminal task requires reopen",
                ));
            } else if state == "blocked"
                && !["ready", "in_progress", "in_review"].contains(&t.state.as_str())
                || state == "paused" && !["in_progress", "blocked"].contains(&t.state.as_str())
            {
                return Err(conflict("REVISION_CONFLICT", "Invalid state transition"));
            }
            let state = if matches!(command, TaskCommand::Reopen { .. }) && t.state == "cancelled" {
                "backlog"
            } else {
                state
            };
            db.execute(
                "UPDATE tasks SET reason=?1,resume_state=?2 WHERE id=?3",
                params![crate::reader::redact(reason).0, t.state, t.id],
            )?;
            if state == "cancelled" {
                db.execute("UPDATE runs SET status='interrupted',epoch=epoch+1 WHERE task=?1 AND status='active'",[&t.id])?;
            }
            changed(db, &t, state, "task_state_changed")
        }
        TaskCommand::Resume { task: name } => {
            let t = task(db, name)?;
            if !["paused", "blocked", "in_progress"].contains(&t.state.as_str()) {
                return Err(conflict(
                    "REVISION_CONFLICT",
                    "Task is not paused or blocked",
                ));
            }
            let active: Option<(String, i64)> = db
                .query_row(
                    "SELECT id,lease_until FROM runs WHERE task=?1 AND status='active'",
                    [&t.id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if t.state == "in_progress" && active.as_ref().is_some_and(|(_, until)| *until > now())
            {
                return Err(conflict("REVISION_CONFLICT", "Run lease is still active"));
            }
            dependencies(db, &t)?;
            if active.as_ref().is_some_and(|(_, until)| *until <= now()) {
                db.execute("UPDATE runs SET status='interrupted',epoch=epoch+1 WHERE task=?1 AND status='active'",[&t.id])?;
            }
            let state = if active.is_some_and(|(_, until)| until > now()) {
                "in_progress"
            } else {
                "ready"
            };
            changed(db, &t, state, "task_resumed")
        }
        TaskCommand::Criterion {
            command:
                CriterionCommand::Accept {
                    task: name,
                    criterion,
                    evidence,
                    ..
                },
        } => {
            let t = task(db, name)?;
            let c = t
                .def
                .acceptance
                .iter()
                .find(|c| &c.id == criterion)
                .ok_or_else(|| invalid("Unknown criterion"))?;
            let (key, status, rev): (String, String, i64) = db
                .query_row(
                    "SELECT key,status,definition_revision FROM checks WHERE id=?1 AND task=?2",
                    params![evidence, t.id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?
                .ok_or_else(|| invalid("Criterion evidence does not exist"))?;
            let stored: Option<String> =
                db.query_row("SELECT report FROM checks WHERE id=?1", [evidence], |r| {
                    r.get(0)
                })?;
            if status != "passed"
                || rev != t.def_rev
                || !c.evidence_check_keys.contains(&key)
                || !check_environment_current(project, &key, stored.as_deref())
            {
                return Err(Error::new(
                    "CHECK_EVIDENCE_STALE",
                    "Evidence does not satisfy criterion",
                    10,
                ));
            }
            db.execute("INSERT INTO acceptances VALUES(?1,?2,?3,?4) ON CONFLICT(task,criterion) DO UPDATE SET evidence=excluded.evidence,definition_revision=excluded.definition_revision",params![t.id,criterion,evidence,t.def_rev])?;
            changed(db, &t, &t.state, "criterion_accepted")
        }
        TaskCommand::Submit {
            task: name,
            run,
            expect_revision,
        } => {
            let t = task(db, name)?;
            expect(&t, *expect_revision)?;
            let (run_task, agent, _) = lease(db, run, None, project)?;
            if t.state != "in_progress" || run_task != t.id {
                return Err(conflict("REVISION_CONFLICT", "Run cannot submit this task"));
            }
            let target = fingerprint(project, &t.def, None)?;
            let evidence = evidence_set(db, &t)?;
            let sub = json!({"id":id("SUB"),"target":target,"definition_revision":t.def_rev,"evidence_hash":hash(evidence.to_string()),"evidence":evidence,"agent":agent,"workspace":project.workspace_id,"submitted_at":now()});
            db.execute(
                "UPDATE tasks SET submission=?1 WHERE id=?2",
                params![sub.to_string(), t.id],
            )?;
            db.execute("UPDATE runs SET status='finished' WHERE id=?1", [run])?;
            let change = changed(db, &t, "in_review", "task_submitted")?;
            Ok(json!({"submission":sub,"change":change}))
        }
        TaskCommand::Review {
            task: name,
            approve,
            actor,
            note,
        } => {
            let t = task(db, name)?;
            if t.state != "in_review" {
                return Err(conflict("REVISION_CONFLICT", "Task not in review"));
            }
            let sub = t
                .submission
                .as_ref()
                .ok_or_else(|| conflict("REVISION_CONFLICT", "Submission missing"))?;
            if actor != "owner" {
                return Err(Error::new(
                    "POLICY_DENIED",
                    "Default policy requires independent human owner review",
                    5,
                ));
            }
            if sub["target"] != fingerprint(project, &t.def, None)?
                || sub["evidence_hash"] != hash(evidence_set(db, &t)?.to_string())
                || sub["definition_revision"] != t.def_rev
            {
                return Err(Error::new(
                    "CHECK_EVIDENCE_STALE",
                    "Submission changed before review",
                    10,
                ));
            }
            db.execute(
                "INSERT INTO reviews VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    id("REVIEW"),
                    t.id,
                    sub["id"].as_str(),
                    actor,
                    approve,
                    crate::reader::redact(note).0,
                    now()
                ],
            )?;
            changed(
                db,
                &t,
                if *approve { "in_review" } else { "ready" },
                "task_reviewed",
            )
        }
        TaskCommand::Complete {
            task: name,
            dry_run,
            expect_revision,
        } => {
            let t = task(db, name)?;
            expect(&t, *expect_revision)?;
            let gate = gates(db, project, &t)?;
            if *dry_run {
                return Ok(gate);
            }
            if gate["passed"] != true {
                return Err(Error::new("COMPLETION_GATE_FAILED", gate.to_string(), 10));
            }
            let completion =
                json!({"submission":t.submission,"actor":"owner","completed_at":now(),"gate":gate});
            db.execute(
                "UPDATE tasks SET completion=?1 WHERE id=?2",
                params![completion.to_string(), t.id],
            )?;
            changed(db, &t, "done", "task_completed")
        }
    }
}
fn reaches(db: &Connection, t: &Task, target: &str, seen: &mut BTreeSet<String>) -> Result<bool> {
    if !seen.insert(t.id.clone()) {
        return Ok(false);
    }
    for dep in &t.def.dependencies {
        let p = task(db, dep)?;
        if p.id == target || reaches(db, &p, target, seen)? {
            return Ok(true);
        }
    }
    Ok(false)
}
fn start(
    project: &Project,
    db: &Connection,
    name: &str,
    workspace: &str,
    agent: Option<&str>,
    revision: Option<i64>,
) -> Result<Value> {
    let t = task(db, name)?;
    expect(&t, revision)?;
    let active: i64 = db.query_row(
        "SELECT count(*) FROM runs WHERE task=?1 AND status='active'",
        [&t.id],
        |r| r.get(0),
    )?;
    if active != 0 {
        return Err(conflict(
            "TASK_ALREADY_CLAIMED",
            "Task already has an active run",
        ));
    }
    if !["ready", "in_progress"].contains(&t.state.as_str()) {
        return Err(conflict("REVISION_CONFLICT", "Task is not ready"));
    }
    if workspace != "current" && workspace != project.workspace_id {
        return Err(invalid("Start must target current workspace"));
    }
    dependencies(db, &t)?;
    let a = match agent {
        Some(a) => agent_id(db, a)?,
        None => t
            .agent
            .clone()
            .ok_or_else(|| invalid("Assign an agent first"))?,
    };
    crate::operations::ensure_claim_allowed_db(db, &a)?;
    if t.agent.as_ref().is_some_and(|assigned| assigned != &a) {
        return Err(conflict(
            "REVISION_CONFLICT",
            "Agent is not assigned to task",
        ));
    }
    let count: i64 = db.query_row(
        "SELECT count(*) FROM runs WHERE agent=?1 AND status='active'",
        [&a],
        |r| r.get(0),
    )?;
    let limit: i64 = db.query_row(
        "SELECT concurrency_limit FROM agents WHERE id=?1",
        [&a],
        |r| r.get(0),
    )?;
    if count >= limit {
        return Err(conflict(
            "TASK_ALREADY_CLAIMED",
            "Agent concurrency limit reached",
        ));
    }
    let epoch: i64 = db.query_row(
        "SELECT coalesce(max(epoch),0)+1 FROM runs WHERE task=?1",
        [&t.id],
        |r| r.get(0),
    )?;
    let run = id("RUN");
    db.execute("INSERT INTO runs(id,task,agent,workspace,epoch,status,lease_until,last_seen,last_progress) VALUES(?1,?2,?3,?4,?5,'active',?6,?7,?7)",params![run,t.id,a,project.workspace_id,epoch,now()+180,now()])?;
    db.execute(
        "UPDATE tasks SET agent=?1,workspace=?2 WHERE id=?3",
        params![a, project.workspace_id, t.id],
    )?;
    issue_capability(project, &run, epoch)?;
    let change = changed(db, &t, "in_progress", "run_started")?;
    Ok(json!({"run_id":run,"lease_epoch":epoch,"lease_until":now()+180,"change":change}))
}
fn agent_command(project: &Project, db: &Connection, command: &AgentCommand) -> Result<Value> {
    match command {
        AgentCommand::Register {
            name,
            kind,
            concurrency_limit,
        } => {
            owner()?;
            if crate::reader::redact(name).1
                || name.trim().is_empty()
                || !["agent", "human"].contains(&kind.as_str())
                || *concurrency_limit == 0
            {
                return Err(invalid("Invalid agent registration"));
            }
            let a = id("AGENT");
            db.execute(
                "INSERT INTO agents VALUES(?1,?2,?3,?4)",
                params![a, name, kind, concurrency_limit],
            )
            .map_err(|_| conflict("REVISION_CONFLICT", "Agent alias already registered"))?;
            let e = event(
                db,
                &a,
                "agent_registered",
                &json!({"name":name,"kind":kind}),
            )?;
            Ok(json!({"agent_id":a,"name":name,"event":e}))
        }
        AgentCommand::List => {
            let mut s =
                db.prepare("SELECT id,name,kind,concurrency_limit FROM agents ORDER BY name")?;
            let agents=s.query_map([],|r|Ok(json!({"agent_id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"kind":r.get::<_,String>(2)?,"concurrency_limit":r.get::<_,i64>(3)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
            Ok(json!({"agents":agents}))
        }
        AgentCommand::Show { agent } => {
            let a = agent_id(db, agent)?;
            let (name, kind): (String, String) =
                db.query_row("SELECT name,kind FROM agents WHERE id=?1", [&a], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?;
            Ok(json!({"agent_id":a,"name":name,"kind":kind}))
        }
        AgentCommand::Heartbeat { run, lease_epoch } => {
            lease(db, run, Some(*lease_epoch), project)?;
            db.execute(
                "UPDATE runs SET last_seen=?1,lease_until=?2 WHERE id=?3",
                params![now(), now() + 180, run],
            )?;
            Ok(json!({"run_id":run,"last_seen_at":now(),"lease_until":now()+180}))
        }
        AgentCommand::Report {
            run,
            lease_epoch,
            report_seq,
            idempotency_key,
            stage,
            summary,
            estimate_percent,
        } => {
            let request = hash(serde_json::to_vec(
                &json!({"run":run,"epoch":lease_epoch,"seq":report_seq,"stage":stage,"summary":summary,"estimate":estimate_percent}),
            )?);
            if let Some((h, response)) = db
                .query_row(
                    "SELECT request_hash,response FROM receipts WHERE key=?1",
                    [idempotency_key],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?
            {
                if h != request {
                    return Err(conflict(
                        "IDEMPOTENCY_CONFLICT",
                        "Report key used with different payload",
                    ));
                }
                return Ok(serde_json::from_str(&response)?);
            }
            let (task_id, _, _) = lease(db, run, Some(*lease_epoch), project)?;
            if summary.len() > 4096
                || estimate_percent.is_some_and(|v| v > 100)
                || ![
                    "planning",
                    "implementing",
                    "testing",
                    "waiting",
                    "submitting",
                ]
                .contains(&stage.as_str())
            {
                return Err(invalid("Invalid progress report"));
            }
            let seq: i64 = db.query_row("SELECT seq FROM runs WHERE id=?1", [run], |r| r.get(0))?;
            if *report_seq <= seq {
                return Err(conflict(
                    "OUT_OF_ORDER_REPORT",
                    "Report sequence must increase",
                ));
            }
            db.execute("UPDATE runs SET seq=?1,stage=?2,summary=?3,estimate=?4,last_seen=?5,last_progress=?5,lease_until=?6 WHERE id=?7",params![report_seq,stage,crate::reader::redact(summary).0,estimate_percent,now(),now()+180,run])?;
            let e = event(
                db,
                &task_id,
                "progress_reported",
                &json!({"run_id":run,"report_seq":report_seq,"stage":stage,"summary":crate::reader::redact(summary).0,"estimate_percent":estimate_percent}),
            )?;
            let v = json!({"run_id":run,"report_seq":report_seq,"event":e});
            db.execute(
                "INSERT INTO receipts VALUES(?1,?2,?3)",
                params![idempotency_key, request, v.to_string()],
            )?;
            Ok(v)
        }
    }
}
fn check_command(project: &Project, db: &Connection, command: &CheckCommand) -> Result<Value> {
    match command {
 CheckCommand::Plan{..}|CheckCommand::Run{..}=>unreachable!("Runner dispatch precedes control transaction"),
 CheckCommand::Begin{task:name,key,run}=>{let t=task(db,name)?;let (run_task,_,_)=lease(db,run,None,project)?;if run_task!=t.id{return Err(conflict("LEASE_REVOKED","Run belongs to another task"));}let c=t.def.checks.iter().find(|c|&c.key==key).ok_or_else(||invalid("Check key not defined"))?;let target=fingerprint(project,&t.def,Some(c))?;let check=id("CHECK");db.execute("INSERT INTO checks VALUES(?1,?2,?3,?4,?5,?6,?7,'running',NULL,?8)",params![check,t.id,key,run,t.def_rev,target,project.policy_hash(),now()])?;let e=event(db,&t.id,"check_begun",&json!({"check_id":check,"key":key,"target":target}))?;Ok(json!({"check_id":check,"artifact_fingerprint":target,"event":e}))},
 CheckCommand::Record{check,from_file}=>{let report:CheckReport=parse_file(from_file)?;if report.source=="runner_observed"{return Err(invalid("External report cannot claim runner-observed provenance"));}record_check_report(project,db,check,report,false,None)},
 CheckCommand::List{task:name}=>{let task_id=name.as_ref().map(|name|task(db,name).map(|t|t.id)).transpose()?;let mut s=db.prepare("SELECT id,task,key,status,target,report FROM checks WHERE ?1 IS NULL OR task=?1 ORDER BY rowid")?;let checks=s.query_map([task_id],|r|Ok(json!({"check_id":r.get::<_,String>(0)?,"task_id":r.get::<_,String>(1)?,"key":r.get::<_,String>(2)?,"result":r.get::<_,String>(3)?,"target":r.get::<_,String>(4)?,"report":r.get::<_,Option<String>>(5)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;Ok(json!({"checks":checks}))},
 CheckCommand::Show{check}=>db.query_row("SELECT id,task,key,status,target,report FROM checks WHERE id=?1",[check],|r|Ok(json!({"check_id":r.get::<_,String>(0)?,"task_id":r.get::<_,String>(1)?,"key":r.get::<_,String>(2)?,"result":r.get::<_,String>(3)?,"target":r.get::<_,String>(4)?,"report":r.get::<_,Option<String>>(5)?}))).optional()?.ok_or_else(||invalid("Check not found")),
}
}
fn task_view(project: &Project, db: &Connection, t: &Task) -> Result<Value> {
    let run:Option<Value>=db.query_row("SELECT id,epoch,status,lease_until,last_seen,last_progress,stage,summary,estimate FROM runs WHERE task=?1 ORDER BY rowid DESC LIMIT 1",[&t.id],|r|{let status:String=r.get(2)?;let until:i64=r.get(3)?;let seen:i64=r.get(4)?;Ok(json!({"run_id":r.get::<_,String>(0)?,"lease_epoch":r.get::<_,i64>(1)?,"status":status,"activity_status":if status=="active"&&until<=now(){"lease_expired"}else if status=="active"&&now()-seen>90{"stale"}else{&status},"lease_until":until,"last_seen_at":r.get::<_,i64>(4)?,"last_progress_at":r.get::<_,i64>(5)?,"stage":r.get::<_,String>(6)?,"summary":r.get::<_,String>(7)?,"estimate_percent":r.get::<_,Option<i64>>(8)?}))}).optional()?;
    let total: u64 = t
        .def
        .acceptance
        .iter()
        .filter(|c| c.required)
        .map(|c| u64::from(c.weight))
        .sum();
    let mut accepted = 0u64;
    for c in t.def.acceptance.iter().filter(|c| c.required) {
        let n:i64=db.query_row("SELECT count(*) FROM acceptances WHERE task=?1 AND criterion=?2 AND definition_revision=?3",params![t.id,c.id,t.def_rev],|r|r.get(0))?;
        if n > 0 {
            accepted += u64::from(c.weight);
        }
    }
    let validity = if t.state == "done" {
        if t.completion
            .as_ref()
            .and_then(|c| c["gate"]["target"].as_str())
            .is_some_and(|h| fingerprint(project, &t.def, None).is_ok_and(|current| h == current))
            && required_environments_current(db, project, t)?
        {
            "current"
        } else {
            "outdated"
        }
    } else {
        "not_completed"
    };
    let mut checks = Vec::new();
    for required in &t.def.checks {
        let latest:Option<(String,String,i64,String,Option<String>)>=db.query_row("SELECT status,target,definition_revision,policy,report FROM checks WHERE task=?1 AND key=?2 ORDER BY rowid DESC LIMIT 1",params![t.id,required.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
        let result = match latest {
            Some((status, target, revision, policy, report))
                if (status != "passed"
                    || check_environment_current(project, &required.key, report.as_deref()))
                    && revision == t.def_rev
                    && policy == project.policy_hash()
                    && fingerprint(project, &t.def, Some(required))
                        .is_ok_and(|current| current == target) =>
            {
                status
            }
            Some(_) => "stale".into(),
            None => "unknown".into(),
        };
        checks.push(json!({"key":required.key,"result":result}));
    }
    Ok(
        json!({"task_id":t.id,"display_id":format!("T-{}",t.number),"title":t.def.title,"state":t.state,"task_revision":t.revision,"definition_revision":t.def_rev,"definition":t.def,"agent_id":t.agent,"workspace_id":t.workspace,"run":run,"checks":checks,"checklist":{"accepted_weight":accepted,"required_weight":total,"percent":if total==0{None}else{Some(100.0*accepted as f64/total as f64)}},"completion_validity":validity,"submission":t.submission}),
    )
}
fn board_db(project: &Project, db: &Connection) -> Result<Value> {
    let mut s = db.prepare("SELECT id FROM tasks ORDER BY number")?;
    let ids = s
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let tasks = ids
        .into_iter()
        .map(|id| task_view(project, db, &task(db, &id)?))
        .collect::<Result<Vec<_>>>()?;
    let seq: i64 = db.query_row("SELECT coalesce(max(seq),0) FROM events", [], |r| r.get(0))?;
    Ok(
        json!({"coordination_id":project.coordination_id,"as_of_seq":seq,"tasks":tasks,"done_count":tasks.iter().filter(|t|t["state"]=="done").count(),"valid_done_count":tasks.iter().filter(|t|t["completion_validity"]=="current").count()}),
    )
}
pub fn board(project: &Project) -> Result<Value> {
    let mut db = connect(project)?;
    let tx = db.transaction()?;
    let data = board_db(project, &tx)?;
    tx.commit()?;
    Ok(data)
}
pub fn activity(project: &Project, since: i64) -> Result<Value> {
    if since < 0 {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Sequence cannot be negative",
            2,
        ));
    }
    let db = connect(project)?;
    let mut s = db.prepare(
        "SELECT seq,entity,type,payload,created FROM events WHERE seq>?1 ORDER BY seq LIMIT 1000",
    )?;
    let rows = s
        .query_map([since], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let events=rows.into_iter().map(|(seq,entity,kind,payload,time)|Ok(json!({"schema_version":"1.0","event_seq":seq,"entity_id":entity,"type":kind,"data":serde_json::from_str::<Value>(&payload)?,"received_at":time}))).collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"events":events,"next_cursor":events.last().map(|e|e["event_seq"].clone()).unwrap_or(json!(since)),"has_more":events.len()==1000}),
    )
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupArchive {
    schema_version: u32,
    project_id: String,
    source_coordination_id: String,
    created_at: i64,
    database_hash: String,
    database: Vec<u8>,
}
fn temporary_snapshot(project: &Project) -> PathBuf {
    project
        .control_dir
        .join(format!(".{}.sqlite3", id("snapshot")))
}
fn control_command(project: &Project, command: &ControlCommand) -> Result<Value> {
    owner()?;
    match command {
        ControlCommand::Backup { output } => {
            let source = connect(project)?;
            let path = temporary_snapshot(project);
            let result = (|| -> Result<Value> {
                let mut destination = Connection::open(&path)?;
                {
                    let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
                    backup.run_to_completion(128, std::time::Duration::from_millis(5), None)?;
                }
                // Shared-query refresh tables are derived caches, never portable control originals.
                let mut stmt = destination.prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name GLOB 'broker_*'",
                )?;
                let derived = stmt
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                drop(stmt);
                for table in derived {
                    destination
                        .execute_batch(&format!("DROP TABLE \"{}\"", table.replace('"', "\"\"")))?;
                }
                // Credentials live outside SQLite. Strip arbitrary environment data from portable metadata.
                destination.execute("UPDATE checks SET report=json_remove(report,'$.report.environment') WHERE report IS NOT NULL",[])?;
                let schedule_v2: bool = destination.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schedule_bindings')", [], |r| r.get(0))?;
                if schedule_v2 {
                    destination.execute_batch("DELETE FROM schedule_bindings; UPDATE schedule_installations SET metadata=json_object('provider',provider,'plan_hash',plan_hash,'former_state',state,'requires_reapproval',json('true')),state='unknown_restored';")?;
                }
                destination.execute_batch("PRAGMA journal_mode=DELETE; VACUUM;")?;
                drop(destination);
                let bytes = std::fs::read(&path)?;
                let archive = BackupArchive {
                    schema_version: 1,
                    project_id: project.project_id.clone(),
                    source_coordination_id: project.coordination_id.clone(),
                    created_at: now(),
                    database_hash: hash(&bytes),
                    database: bytes,
                };
                crate::project::atomic_write(output, &serde_json::to_vec(&archive)?, false)?;
                Ok(
                    json!({"output":output,"database_hash":archive.database_hash,"source_coordination_id":project.coordination_id,"credentials_included":false,"workspace_remap_required":true}),
                )
            })();
            let _ = std::fs::remove_file(&path);
            result
        }
        ControlCommand::Restore { input } => {
            if std::fs::metadata(input)?.len() > 268435456 {
                return Err(invalid("Control archive exceeds 256 MiB"));
            }
            let archive: BackupArchive = serde_json::from_slice(&std::fs::read(input)?)?;
            if archive.schema_version != 1
                || archive.project_id != project.project_id
                || archive.database_hash != hash(&archive.database)
            {
                return Err(Error::new(
                    "INVALID_ARCHIVE",
                    "Backup schema, project, or checksum mismatch",
                    2,
                ));
            }
            let coordination = id("COORD");
            let destination = project.data_dir.join("controls").join(&coordination);
            crate::project::private_dir(&destination)?;
            let path = destination.join("control.sqlite3");
            let result = (|| -> Result<Value> {
                crate::project::atomic_write(&path, &archive.database, false)?;
                let db = Connection::open(&path)?;
                let integrity: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
                let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
                if integrity != "ok" || version != 1 {
                    return Err(Error::new(
                        "INVALID_ARCHIVE",
                        "Backup database failed integrity/schema validation",
                        2,
                    ));
                }
                let tables = [
                    "work_meta",
                    "tasks",
                    "agents",
                    "runs",
                    "checks",
                    "acceptances",
                    "reviews",
                    "events",
                    "receipts",
                    "sqlite_sequence",
                    "pctx_session_schema",
                    "pctx_sessions",
                    "pctx_context_emissions",
                    "pctx_context_acks",
                    "pctx_session_events",
                    "pctx_session_capsules",
                    "ops_roles",
                    "ops_silences",
                    "ops_decisions",
                    "ops_requesters",
                    "ops_messages",
                    "ops_message_acks",
                    "ops_receipts",
                    "ops_events",
                    "quota_schema",
                    "quota_observations",
                    "quota_receipts",
                    "quota_reservations",
                    "quota_events",
                    "schedule_schema",
                    "schedule_definitions",
                    "schedule_occurrences",
                    "schedule_receipts",
                    "schedule_events",
                    "schedule_bindings",
                    "schedule_runs",
                    "schedule_installations",
                    "adapter_schema",
                    "adapter_bindings",
                    "adapter_receipts",
                    "adapter_capsules",
                    "adapter_installs",
                ];
                let mut q = db.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
                let names = q
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                if names.iter().any(|n| !tables.contains(&n.as_str()))
                    || tables[..9]
                        .iter()
                        .any(|n| !names.iter().any(|name| name == n))
                {
                    return Err(Error::new(
                        "INVALID_ARCHIVE",
                        "Unrecognized control database schema",
                        2,
                    ));
                }
                drop(q);
                let session_tables = &tables[10..16];
                let has_sessions = names.iter().any(|name| name == "pctx_session_schema");
                if session_tables
                    .iter()
                    .any(|table| names.iter().any(|name| name == table))
                {
                    if !session_tables
                        .iter()
                        .all(|table| names.iter().any(|name| name == table))
                    {
                        return Err(Error::new(
                            "INVALID_ARCHIVE",
                            "Incomplete session schema in backup",
                            2,
                        ));
                    }
                    let version: i64 =
                        db.query_row("SELECT version FROM pctx_session_schema", [], |r| r.get(0))?;
                    if version != 1 {
                        return Err(Error::new(
                            "INVALID_ARCHIVE",
                            "Unsupported session schema in backup",
                            2,
                        ));
                    }
                }
                let ops_tables = &tables[16..24];
                let has_ops = names.iter().any(|name| name == "ops_roles");
                if ops_tables
                    .iter()
                    .any(|table| names.iter().any(|name| name == table))
                    && !ops_tables
                        .iter()
                        .all(|table| names.iter().any(|name| name == table))
                {
                    return Err(Error::new(
                        "INVALID_ARCHIVE",
                        "Incomplete operations schema in backup",
                        2,
                    ));
                }
                let quota_tables = &tables[24..29];
                let has_quota = names.iter().any(|name| name == "quota_schema");
                if quota_tables
                    .iter()
                    .any(|table| names.iter().any(|name| name == table))
                {
                    if !quota_tables
                        .iter()
                        .all(|table| names.iter().any(|name| name == table))
                    {
                        return Err(Error::new(
                            "INVALID_ARCHIVE",
                            "Incomplete quota schema in backup",
                            2,
                        ));
                    }
                    let version: i64 =
                        db.query_row("SELECT version FROM quota_schema", [], |r| r.get(0))?;
                    if version != 1 {
                        return Err(Error::new("INVALID_ARCHIVE", "Unsupported quota schema", 2));
                    }
                }
                let schedule_tables = &tables[29..37];
                let has_schedule = names.iter().any(|name| name == "schedule_schema");
                let mut schedule_version = 0;
                if schedule_tables
                    .iter()
                    .any(|table| names.iter().any(|name| name == table))
                {
                    if !has_schedule {
                        return Err(Error::new(
                            "INVALID_ARCHIVE",
                            "Incomplete schedule schema",
                            2,
                        ));
                    }
                    schedule_version =
                        db.query_row("SELECT version FROM schedule_schema", [], |r| r.get(0))?;
                    let required = match schedule_version {
                        1 => &schedule_tables[..5],
                        2 => schedule_tables,
                        _ => {
                            return Err(Error::new(
                                "INVALID_ARCHIVE",
                                "Unsupported schedule schema",
                                2,
                            ));
                        }
                    };
                    if !required
                        .iter()
                        .all(|table| names.iter().any(|name| name == table))
                        || (schedule_version == 1
                            && schedule_tables[5..]
                                .iter()
                                .any(|table| names.iter().any(|name| name == table)))
                    {
                        return Err(Error::new(
                            "INVALID_ARCHIVE",
                            "Incomplete or inconsistent schedule schema",
                            2,
                        ));
                    }
                }
                let adapter_tables = &tables[37..];
                let has_adapter = names.iter().any(|name| name == "adapter_schema");
                if adapter_tables
                    .iter()
                    .any(|table| names.iter().any(|name| name == table))
                {
                    let version: i64 =
                        db.query_row("SELECT version FROM adapter_schema", [], |r| r.get(0))?;
                    let required = if version == 1 {
                        &adapter_tables[..4]
                    } else if version == 2 {
                        adapter_tables
                    } else {
                        return Err(Error::new(
                            "INVALID_ARCHIVE",
                            "Unsupported adapter schema",
                            2,
                        ));
                    };
                    if !required
                        .iter()
                        .all(|table| names.iter().any(|name| name == table))
                    {
                        return Err(Error::new(
                            "INVALID_ARCHIVE",
                            "Incomplete adapter schema",
                            2,
                        ));
                    }
                }
                let mut q = db.prepare("SELECT name FROM sqlite_master WHERE type='trigger'")?;
                let triggers = q
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                drop(q);
                for trigger in triggers {
                    db.execute_batch(&format!(
                        "DROP TRIGGER \"{}\"",
                        trigger.replace('"', "\"\"")
                    ))?;
                }
                db.execute_batch("BEGIN IMMEDIATE; UPDATE runs SET status='interrupted',epoch=epoch+1,lease_until=0 WHERE status='active'; UPDATE tasks SET submission=NULL WHERE state<>'done'; DELETE FROM receipts; UPDATE work_meta SET revision=revision+1 WHERE id=1; CREATE TRIGGER events_no_update BEFORE UPDATE ON events BEGIN SELECT RAISE(ABORT,'append-only events'); END; CREATE TRIGGER events_no_delete BEFORE DELETE ON events BEGIN SELECT RAISE(ABORT,'append-only events'); END;")?;
                if has_sessions {
                    db.execute_batch("CREATE TRIGGER pctx_session_events_no_update BEFORE UPDATE ON pctx_session_events BEGIN SELECT RAISE(ABORT,'append-only events'); END; CREATE TRIGGER pctx_session_events_no_delete BEFORE DELETE ON pctx_session_events BEGIN SELECT RAISE(ABORT,'append-only events'); END;")?;
                    db.execute("INSERT INTO pctx_session_events(session,kind,metadata,created) SELECT id,'control_restored',json_object('previous_epoch',epoch,'context_epoch',epoch+1,'ack_inherited',json('false'),'lease_restored',json('false')),?1 FROM pctx_sessions",[now()])?;
                    db.execute_batch("UPDATE pctx_sessions SET epoch=epoch+1,status=CASE WHEN status='active' THEN 'suspended' ELSE status END; DELETE FROM pctx_context_acks;")?;
                }
                if has_ops {
                    db.execute_batch("UPDATE ops_decisions SET state='expired',expires_at=0,provenance=NULL WHERE state='approved'; DELETE FROM ops_receipts; DELETE FROM ops_message_acks; UPDATE ops_messages SET state='queued' WHERE state IN ('dispatched','delivered','acknowledged'); CREATE TRIGGER ops_events_no_update BEFORE UPDATE ON ops_events BEGIN SELECT RAISE(ABORT,'append-only operations events'); END; CREATE TRIGGER ops_events_no_delete BEFORE DELETE ON ops_events BEGIN SELECT RAISE(ABORT,'append-only operations events'); END;")?;
                    db.execute("INSERT INTO ops_events(entity,kind,actor,payload,created) VALUES(?1,'control_restored','owner',?2,?3)",params![coordination,json!({"grants_invalidated":true,"pause_and_silence_preserved":true,"delivery_receipts_invalidated":true}).to_string(),now()])?;
                }
                if has_quota {
                    db.execute(
                        "UPDATE quota_reservations SET status='invalidated' WHERE status='active'",
                        [],
                    )?;
                    db.execute_batch("CREATE TRIGGER quota_events_no_update BEFORE UPDATE ON quota_events BEGIN SELECT RAISE(ABORT,'append-only events'); END; CREATE TRIGGER quota_events_no_delete BEFORE DELETE ON quota_events BEGIN SELECT RAISE(ABORT,'append-only events'); END;")?;
                    let cutoff: i64 = db.query_row(
                        "SELECT coalesce(max(rowid),0) FROM quota_observations",
                        [],
                        |r| r.get(0),
                    )?;
                    db.execute("INSERT INTO quota_events(entity,kind,metadata,created) VALUES(?1,'control_restored',?2,?3)",params![coordination,json!({"reservations_invalidated":true,"pause_and_silence_preserved":true,"observations_are_historical":true,"historical_observation_rowid":cutoff}).to_string(),now()])?;
                }
                if has_schedule {
                    if schedule_version == 2 {
                        db.execute_batch("DELETE FROM schedule_bindings; UPDATE schedule_runs SET state='interrupted_unknown',error='control_restored',finished=NULL WHERE state='running'; UPDATE schedule_occurrences SET state='interrupted_unknown' WHERE state='running'; UPDATE schedule_installations SET metadata=json_object('provider',provider,'plan_hash',plan_hash,'former_state',state,'requires_reapproval',json('true')),state='unknown_restored';")?;
                    }
                    db.execute_batch("CREATE TRIGGER schedule_events_no_update BEFORE UPDATE ON schedule_events BEGIN SELECT RAISE(ABORT,'append-only schedule events');END; CREATE TRIGGER schedule_events_no_delete BEFORE DELETE ON schedule_events BEGIN SELECT RAISE(ABORT,'append-only schedule events');END;")?;
                    db.execute("INSERT INTO schedule_events(namespace,schedule,kind,metadata,created) VALUES('control',?1,'control_restored',?2,?3)",params![coordination,json!({"registrations":"unknown","automatic_install":false,"pause_and_occurrences_preserved":true}).to_string(),now()])?;
                }
                if has_adapter {
                    db.execute_batch(
                        "DELETE FROM adapter_bindings; DELETE FROM adapter_receipts;",
                    )?;
                }
                let e = event(
                    &db,
                    &coordination,
                    "control_restored",
                    &json!({"source_coordination_id":archive.source_coordination_id,"workspace_remap_required":true,"leases_invalidated":true}),
                )?;
                db.execute_batch("COMMIT;")?;
                Ok(
                    json!({"coordination_id":coordination,"source_coordination_id":archive.source_coordination_id,"workspace_remap_required":true,"attached":false,"leases_invalidated":true,"event":e}),
                )
            })();
            if result.is_err() {
                let _ = std::fs::remove_dir_all(&destination);
            }
            result
        }
    }
}

fn record_check_report(
    project: &Project,
    db: &Connection,
    check: &str,
    mut report: CheckReport,
    trusted_runner: bool,
    output_ref: Option<&str>,
) -> Result<Value> {
    report.producer = crate::reader::redact(&report.producer).0;
    let sanitized = crate::reader::redact(&report.environment.to_string()).0;
    report.environment = serde_json::from_str(&sanitized).unwrap_or(json!({"redacted":true}));
    let (task_id, key, run, rev, target, policy, status): (
        String,
        String,
        String,
        i64,
        String,
        String,
        String,
    ) = db
        .query_row(
            "SELECT task,key,run,definition_revision,target,policy,status FROM checks WHERE id=?1",
            [check],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| invalid("Check attempt not found"))?;
    lease(db, &run, None, project)?;
    if status != "running" {
        return Err(conflict("REVISION_CONFLICT", "Check result is immutable"));
    }
    let t = task(db, &task_id)?;
    let c = t
        .def
        .checks
        .iter()
        .find(|c| c.key == key)
        .ok_or_else(|| invalid("Check definition removed"))?;
    if report.schema_version != 1
        || report.check_key != key
        || report.producer.trim().is_empty()
        || report.finished_at < report.started_at
        || !["passed", "failed", "cancelled", "timed_out", "unverified"]
            .contains(&report.result.as_str())
        || report
            .passed
            .checked_add(report.failed)
            .and_then(|v| v.checked_add(report.skipped))
            != Some(report.tests)
    {
        return Err(invalid("Invalid check report schema or counts"));
    }
    let check_binding = if trusted_runner {
        output_ref
            .map(|output| crate::output::observed_check_binding(project, output))
            .transpose()?
            .flatten()
    } else {
        None
    };
    let environment_stale = trusted_runner
        && check_binding.as_ref().is_some_and(|binding| {
            binding["key"] != key
                || !crate::runner::current_check_binding(project, &key)
                    .is_ok_and(|current| current == *binding)
        });
    let result = if rev != t.def_rev
        || target != fingerprint(project, &t.def, Some(c))?
        || policy != project.policy_hash()
        || environment_stale
    {
        "stale"
    } else if (trusted_runner && check_binding.is_none())
        || !c.allowed_sources.contains(&report.source)
        || !(report.source == "external_report"
            || (trusted_runner && report.source == "runner_observed"))
        || report.source == "manual_claim"
        || (c.require_report_attachment && output_ref.is_none())
    {
        "unverified"
    } else if ["cancelled", "timed_out", "unverified"].contains(&report.result.as_str()) {
        &report.result
    } else if report.failed > 0
        || report.errors > 0
        || !c.success_exit_codes.contains(&report.exit_code)
        || report.result == "failed"
    {
        "failed"
    } else if c.kind == "test"
        && report.passed + report.failed < c.minimum_executed_tests.unwrap_or(1).max(1)
    {
        "unverified"
    } else {
        "passed"
    };
    let data = json!({"report":report,"report_digest":hash(serde_json::to_vec(&report)?),"report_environment_digest":hash(serde_json::to_vec(&report.environment)?),"environment_fingerprint":check_binding.as_ref().map(|b|b["environment_fingerprint"].clone()),"environment_authority":if trusted_runner && check_binding.is_some(){"trusted_local_runner_profile"}else if trusted_runner{"runner_observation_binding_missing"}else{"external_report_claim_not_locally_verified"},"check_binding":check_binding,"output_id":output_ref});
    db.execute(
        "UPDATE checks SET status=?1,report=?2 WHERE id=?3",
        params![result, data.to_string(), check],
    )?;
    let e = event(
        db,
        &task_id,
        "check_recorded",
        &json!({"check_id":check,"result":result}),
    )?;
    Ok(json!({"check_id":check,"result":result,"event":e}))
}
#[cfg(unix)]
pub(crate) fn record_runner_report(
    project: &Project,
    check: &str,
    output_id: &str,
) -> Result<Value> {
    let report = crate::output::observed_report(project, output_id)?.ok_or_else(|| {
        Error::new(
            "PARTIAL_RESULT",
            "Complete observed typed report unavailable",
            3,
        )
    })?;
    let mut db = connect(project)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let result = record_check_report(project, &tx, check, report, true, Some(output_id))?;
    tx.commit()?;
    Ok(result)
}

#[cfg(unix)]
pub(crate) fn record_runner_unverified(
    project: &Project,
    check: &str,
    output_id: &str,
) -> Result<Value> {
    let mut report = crate::output::unverified_report(project, output_id)?;
    let mut db = connect(project)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    report.check_key = tx.query_row("SELECT key FROM checks WHERE id=?1", [check], |r| r.get(0))?;
    let result = record_check_report(project, &tx, check, report, true, Some(output_id))?;
    tx.commit()?;
    Ok(result)
}

pub(crate) fn authenticated_actor(project: &Project, db: &Connection) -> Result<Option<String>> {
    let actor = std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into());
    if actor == "owner" {
        return Ok(None);
    }
    let run = std::env::var("PCTX_RUN_ID").map_err(|_| {
        Error::new(
            "POLICY_DENIED",
            "Session operations require active run identity and capability",
            5,
        )
    })?;
    let (_, agent, _) = lease(db, &run, None, project)?;
    Ok(Some(agent))
}

#[cfg(unix)]
pub(crate) fn record_runner_not_started(
    project: &Project,
    check: &str,
    reason: &str,
) -> Result<Value> {
    let mut db = connect(project)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (task_id, status): (String, String) =
        tx.query_row("SELECT task,status FROM checks WHERE id=?1", [check], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    if status != "running" {
        return Err(conflict(
            "REVISION_CONFLICT",
            "Check attempt already finalized",
        ));
    }
    let data = json!({"source":"runner_admission","execution_started":false,"test_counts_status":"unknown","reason":crate::reader::redact(reason).0,"gate_evidence":false});
    tx.execute(
        "UPDATE checks SET status='unverified',report=?1 WHERE id=?2",
        params![data.to_string(), check],
    )?;
    let e = event(
        &tx,
        &task_id,
        "check_not_started",
        &json!({"check_id":check,"reason":data["reason"]}),
    )?;
    tx.commit()?;
    Ok(
        json!({"check_id":check,"result":"unverified","execution_started":false,"reason":data["reason"],"event":e}),
    )
}

#[cfg(unix)]
pub(crate) fn record_runner_execution_error(
    project: &Project,
    check: &str,
    reason: &str,
) -> Result<Value> {
    let mut db = connect(project)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (task_id, status): (String, String) =
        tx.query_row("SELECT task,status FROM checks WHERE id=?1", [check], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    if status != "running" {
        return Err(conflict(
            "REVISION_CONFLICT",
            "Check attempt already finalized",
        ));
    }
    let data = json!({"source":"runner_supervision_failure","execution_started":true,"child_exit_status":"unknown","test_counts_status":"unknown","reason":crate::reader::redact(reason).0,"gate_evidence":false});
    tx.execute(
        "UPDATE checks SET status='unverified',report=?1 WHERE id=?2",
        params![data.to_string(), check],
    )?;
    let e = event(
        &tx,
        &task_id,
        "check_execution_error",
        &json!({"check_id":check,"execution_started":true,"reason":data["reason"]}),
    )?;
    tx.commit()?;
    Ok(
        json!({"check_id":check,"result":"unverified","execution_started":true,"reason":data["reason"],"event":e}),
    )
}

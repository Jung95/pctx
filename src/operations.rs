//! Durable local decisions and mailbox; no external delivery or model wake is inferred.
use crate::{
    domain::{Error, Result, hash, id, now},
    project::Project,
    reader, work,
};
use clap::Subcommand;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
#[derive(Debug, Clone, Subcommand)]
pub enum OperationCommand {
    Role {
        #[command(subcommand)]
        command: RoleCommand,
    },
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    Decision {
        #[command(subcommand)]
        command: DecisionCommand,
    },
    Owner {
        #[command(subcommand)]
        command: OwnerCommand,
    },
    Message {
        #[command(subcommand)]
        command: MessageCommand,
    },
    Inbox {
        #[command(subcommand)]
        command: InboxCommand,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum RoleCommand {
    Pause {
        role: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long)]
        recipient: Option<String>,
    },
    Resume {
        role: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long)]
        recipient: Option<String>,
    },
    List,
}
#[derive(Debug, Clone, Subcommand)]
pub enum PolicyCommand {
    Evaluate {
        #[arg(long)]
        action_file: PathBuf,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum DecisionCommand {
    Request {
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        task_id: Option<String>,
    },
    Record {
        id: String,
        #[arg(long)]
        from_file: PathBuf,
    },
    Show {
        id: String,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum OwnerCommand {
    Queue {
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum MessageCommand {
    Send {
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        to_role: Option<String>,
        #[arg(long)]
        to_agent: Option<String>,
        #[arg(long)]
        to_session: Option<String>,
        #[arg(long)]
        task_id: Option<String>,
    },
    Ack {
        id: String,
        #[arg(long)]
        session: String,
    },
    Resolve {
        id: String,
        #[arg(long)]
        evidence: String,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum InboxCommand {
    Read {
        #[arg(long)]
        session: String,
        #[arg(long, default_value_t = 0)]
        since_seq: i64,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub schema_version: u32,
    pub kind: String,
    pub resource: String,
    pub environment: String,
    pub scope: Vec<String>,
    pub actor: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    pub amount: Option<u64>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub artifact_sha: Option<String>,
    #[serde(default)]
    pub argv: Vec<String>,
    pub reversible: bool,
    pub cost_known: bool,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRequest {
    pub schema_version: u32,
    pub action: Action,
    pub reason: String,
    #[serde(default)]
    pub impact: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub recommendation: Option<String>,
    #[serde(default)]
    pub deadline: Option<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRecord {
    pub schema_version: u32,
    pub approve: bool,
    pub owner_evidence: String,
    pub expires_at: i64,
    #[serde(default)]
    pub note: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub schema_version: u32,
    #[serde(rename = "type")]
    pub kind: String,
    pub topic: String,
    pub body: String,
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub source_revision: Option<i64>,
    pub idempotency_key: String,
}
fn invalid(s: &str) -> Error {
    Error::new("INVALID_ARGUMENT", s, 2)
}
fn denied(s: &str) -> Error {
    Error::new("POLICY_DENIED", s, 5)
}
fn owner() -> Result<()> {
    if actor() == "owner" {
        Ok(())
    } else {
        Err(denied("Trusted local owner operator required"))
    }
}
fn actor() -> String {
    std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into())
}
fn label(s: &str) -> Result<()> {
    if s.trim().is_empty() || s.len() > 256 || s.contains('\0') || reader::redact(s).1 {
        return Err(invalid("Invalid or sensitive identity label"));
    }
    Ok(())
}
fn sanitize(v: &mut Value) {
    match v {
        Value::String(s) => *s = reader::redact(s).0,
        Value::Array(a) => {
            for v in a {
                sanitize(v)
            }
        }
        Value::Object(o) => {
            for (k, v) in o {
                if [
                    "password",
                    "token",
                    "secret",
                    "api_key",
                    "authorization",
                    "credential",
                ]
                .contains(&k.to_lowercase().as_str())
                {
                    *v = json!("[REDACTED]")
                } else {
                    sanitize(v)
                }
            }
        }
        _ => {}
    }
}
fn raw_input<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T> {
    if std::fs::metadata(path)?.len() > 65536 {
        return Err(invalid("Operations input exceeds 64 KiB"));
    }
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn input<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T> {
    if std::fs::metadata(path)?.len() > 65536 {
        return Err(invalid("Operations input exceeds 64 KiB"));
    }
    let mut v: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    sanitize(&mut v);
    Ok(serde_json::from_value(v)?)
}
fn connect(p: &Project) -> Result<Connection> {
    let mut db = work::connect(p)?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='ops_roles' AND type='table')",
        [],
        |r| r.get(0),
    )?;
    if exists {
        return Ok(db);
    }
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS ops_roles(role TEXT PRIMARY KEY,paused INTEGER NOT NULL,reason TEXT NOT NULL,actor TEXT NOT NULL,updated INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS ops_silences(role TEXT NOT NULL,recipient TEXT NOT NULL,topic TEXT NOT NULL,active INTEGER NOT NULL,reason TEXT NOT NULL,actor TEXT NOT NULL,updated INTEGER NOT NULL,PRIMARY KEY(role,recipient,topic));
CREATE TABLE IF NOT EXISTS ops_decisions(id TEXT PRIMARY KEY,fingerprint TEXT UNIQUE NOT NULL,action TEXT NOT NULL,policy TEXT NOT NULL,state TEXT NOT NULL,reason TEXT NOT NULL,request TEXT NOT NULL,task TEXT,owner_evidence TEXT,expires_at INTEGER,provenance TEXT,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS ops_requesters(decision TEXT NOT NULL REFERENCES ops_decisions(id),actor TEXT NOT NULL,created INTEGER NOT NULL,PRIMARY KEY(decision,actor));
CREATE TABLE IF NOT EXISTS ops_messages(seq INTEGER PRIMARY KEY AUTOINCREMENT,id TEXT UNIQUE NOT NULL,sender TEXT NOT NULL,recipient_kind TEXT NOT NULL,recipient TEXT NOT NULL,task TEXT,payload TEXT NOT NULL,state TEXT NOT NULL,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS ops_message_acks(message TEXT NOT NULL REFERENCES ops_messages(id),session TEXT NOT NULL,epoch INTEGER NOT NULL,created INTEGER NOT NULL,PRIMARY KEY(message,session,epoch));
CREATE TABLE IF NOT EXISTS ops_receipts(actor TEXT NOT NULL,key TEXT NOT NULL,request_hash TEXT NOT NULL,response TEXT NOT NULL,PRIMARY KEY(actor,key));
CREATE TABLE IF NOT EXISTS ops_events(seq INTEGER PRIMARY KEY AUTOINCREMENT,entity TEXT NOT NULL,kind TEXT NOT NULL,actor TEXT NOT NULL,payload TEXT NOT NULL,created INTEGER NOT NULL);
CREATE TRIGGER IF NOT EXISTS ops_events_no_update BEFORE UPDATE ON ops_events BEGIN SELECT RAISE(ABORT,'append-only operations events'); END;
CREATE TRIGGER IF NOT EXISTS ops_events_no_delete BEFORE DELETE ON ops_events BEGIN SELECT RAISE(ABORT,'append-only operations events'); END;")?;
    tx.commit()?;
    Ok(db)
}
fn event(db: &Connection, entity: &str, kind: &str, payload: &Value) -> Result<()> {
    db.execute(
        "INSERT INTO ops_events(entity,kind,actor,payload,created) VALUES(?1,?2,?3,?4,?5)",
        params![entity, kind, actor(), payload.to_string(), now()],
    )?;
    Ok(())
}
fn authenticate(p: &Project, db: &Connection) -> Result<String> {
    let a = actor();
    if a == "owner" {
        return Ok(a);
    }
    let run = std::env::var("PCTX_RUN_ID")
        .map_err(|_| denied("Agent mutation requires run identity and capability"))?;
    let token = std::env::var("PCTX_RUN_CAPABILITY")
        .map_err(|_| denied("Agent mutation requires run capability"))?;
    let row:Option<(String,String,i64,i64,String)>=db.query_row("SELECT r.agent,r.workspace,r.epoch,r.lease_until,r.status FROM runs r JOIN agents a ON a.id=r.agent WHERE r.id=?1 AND (a.id=?2 OR a.name=?2)",params![run,a],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
    let (agent, workspace, epoch, until, status) =
        row.ok_or_else(|| denied("Actor does not own run"))?;
    if status != "active" || until <= now() || workspace != p.workspace_id {
        return Err(Error::new(
            "LEASE_REVOKED",
            "Agent run is inactive, expired, or in another workspace",
            9,
        ));
    }
    let credential: Value = serde_json::from_slice(
        &std::fs::read(
            p.workspace_dir
                .join("credentials")
                .join(format!("{run}.json")),
        )
        .map_err(|_| denied("Run credential unavailable"))?,
    )?;
    if credential["run_id"] != run
        || credential["lease_epoch"] != epoch
        || credential["capability"] != token
    {
        return Err(denied("Run capability mismatch"));
    }
    Ok(agent)
}
fn validate_action(a: &mut Action) -> Result<()> {
    if a.schema_version != 1 {
        return Err(invalid("Unsupported action schema"));
    }
    for s in [&a.kind, &a.resource, &a.environment, &a.actor] {
        label(s)?;
    }
    if a.scope.is_empty()
        || a.scope.iter().any(|s| {
            s.is_empty()
                || s.starts_with('/')
                || s.contains('\\')
                || s.split('/').any(|c| c == "..")
        })
    {
        return Err(invalid("Action needs normalized bounded scope"));
    }
    if a.scope.iter().any(|s| reader::redact(s).1) {
        return Err(invalid("Sensitive approval scope denied"));
    }
    for field in [&a.role, &a.topic, &a.currency].into_iter().flatten() {
        label(field)?;
    }
    a.scope.sort();
    a.scope.dedup();
    if a.argv.len() > 256 || a.argv.iter().map(String::len).sum::<usize>() > 16384 {
        return Err(invalid("Action argv exceeds limit"));
    }
    if a.argv.iter().any(|s| reader::redact(s).1) {
        return Err(invalid("Sensitive argv cannot be an approval binding"));
    }
    if let Some(sha) = &a.artifact_sha
        && (sha.len() != 64 || !sha.bytes().all(|c| c.is_ascii_hexdigit()))
    {
        return Err(invalid("Artifact SHA must be SHA-256"));
    }
    Ok(())
}
fn fingerprint(p: &Project, a: &Action) -> Result<String> {
    Ok(hash(serde_json::to_vec(
        &json!({"action":a,"policy":p.policy_hash(),"coordination":p.coordination_id}),
    )?))
}
fn paused(db: &Connection, role: &str) -> Result<bool> {
    Ok(db
        .query_row("SELECT paused FROM ops_roles WHERE role=?1", [role], |r| {
            r.get::<_, bool>(0)
        })
        .optional()?
        .unwrap_or(false))
}
fn recipient_key(db: &Connection, value: &str) -> Result<String> {
    Ok(db
        .query_row(
            "SELECT id FROM agents WHERE id=?1 OR name=?1",
            [value],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .unwrap_or_else(|| value.into()))
}
fn silenced(db: &Connection, role: &str, recipient: &str, topic: &str) -> Result<bool> {
    Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM ops_silences WHERE active=1 AND (role=?1 OR role='*') AND (recipient=?2 OR recipient='*') AND topic=?3)",params![role,recipient_key(db,recipient)?,topic],|r|r.get(0))?)
}
/// Hook for task claim: pause is durable policy, independent of task and lease state.
pub fn ensure_claim_allowed(p: &Project, agent: &str) -> Result<()> {
    let db = connect(p)?;
    ensure_claim_allowed_db(&db, agent)
}
pub fn ensure_claim_allowed_db(db: &Connection, agent: &str) -> Result<()> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='ops_roles' AND type='table')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(());
    }
    let alias: Option<String> = db
        .query_row(
            "SELECT name FROM agents WHERE id=?1 OR name=?1",
            [agent],
            |r| r.get(0),
        )
        .optional()?;
    if paused(db, agent)?
        || alias
            .as_ref()
            .is_some_and(|alias| paused(db, alias).unwrap_or(true))
    {
        return Err(Error::new("ROLE_PAUSED", "Agent or role is paused", 5));
    }
    let sessions: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='pctx_sessions')",
        [],
        |r| r.get(0),
    )?;
    if sessions {
        let mut s=db.prepare("SELECT role FROM pctx_sessions WHERE agent IN (SELECT id FROM agents WHERE id=?1 OR name=?1) AND role IS NOT NULL")?;
        for role in s.query_map([agent], |r| r.get::<_, String>(0))? {
            if paused(db, &role?)? {
                return Err(Error::new(
                    "ROLE_PAUSED",
                    "Agent's registered role is paused",
                    5,
                ));
            }
        }
    }
    Ok(())
}
fn evaluate(p: &Project, db: &Connection, a: &Action) -> Result<Value> {
    let mut reasons = vec![];
    if ensure_claim_allowed_db(db, &a.actor).is_err() {
        reasons.push("role_paused");
    }
    for scope in &a.scope {
        if reader::authorize(p, scope).is_err_and(|e| e.exit == 5) {
            reasons.push("security_scope_denied");
        }
    }
    if let Some(role) = a.role.as_deref()
        && p.config
            .roles
            .get(role)
            .and_then(toml::Value::as_table)
            .and_then(|t| t.get("denied_capabilities"))
            .and_then(toml::Value::as_array)
            .is_some_and(|values| values.iter().any(|v| v.as_str() == Some(&a.kind)))
    {
        reasons.push("explicit_deny");
    }
    if p.config.policy.network == "deny"
        && ["network", "external_publish", "external_api", "deploy"].contains(&a.kind.as_str())
    {
        reasons.push("network_denied");
    }
    let has_sessions: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='pctx_sessions')",
        [],
        |r| r.get(0),
    )?;
    if has_sessions {
        let mut stmt=db.prepare("SELECT DISTINCT role FROM pctx_sessions WHERE role IS NOT NULL AND agent IN (SELECT id FROM agents WHERE id=?1 OR name=?1)")?;
        for role in stmt.query_map([&a.actor], |r| r.get::<_, String>(0))? {
            let role = role?;
            if paused(db, &role)? {
                reasons.push("role_paused");
            }
            if let Some(topic) = &a.topic
                && silenced(db, &role, &a.actor, topic)?
            {
                reasons.push("topic_silenced");
            }
            if p.config
                .roles
                .get(&role)
                .and_then(toml::Value::as_table)
                .and_then(|t| t.get("denied_capabilities"))
                .and_then(toml::Value::as_array)
                .is_some_and(|values| values.iter().any(|v| v.as_str() == Some(&a.kind)))
            {
                reasons.push("explicit_deny");
            }
        }
    }
    if a.role
        .as_deref()
        .is_some_and(|role| paused(db, role).unwrap_or(true))
    {
        reasons.push("role_paused");
    }
    if let Some(topic) = &a.topic
        && silenced(db, a.role.as_deref().unwrap_or("*"), &a.actor, topic)?
    {
        reasons.push("topic_silenced");
    }
    if !a.cost_known
        || a.amount.is_none()
        || a.amount.is_some_and(|n| n > 0)
            && a.currency.as_ref().is_none_or(|s| s.trim().is_empty())
    {
        reasons.push("unknown_cost");
    }
    if a.scope.is_empty() {
        reasons.push("unknown_scope");
    }
    let f = fingerprint(p, a)?;
    let grant:Option<(String,String,Option<i64>,Option<String>)>=db.query_row("SELECT id,state,expires_at,provenance FROM ops_decisions WHERE fingerprint=?1 AND policy=?2",params![f,p.policy_hash()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    let allowed = reasons.is_empty()
        && grant
            .as_ref()
            .is_some_and(|(_, state, expires, provenance)| {
                state == "approved"
                    && expires.is_some_and(|e| e > now())
                    && provenance.as_deref() == Some("trusted_local_operator")
            });
    if !allowed && reasons.is_empty() {
        reasons.push("no_current_matching_owner_grant");
    }
    Ok(
        json!({"action_fingerprint":f,"internal_policy_allowed":allowed,"decision_id":grant.map(|g|g.0),"reasons":reasons,"host_permission":"separate_required","execution_started":false}),
    )
}
#[derive(Clone)]
struct RecipientSession {
    agent: String,
    role: Option<String>,
    epoch: i64,
    status: String,
}
fn session(db: &Connection, p: &Project, sid: &str) -> Result<RecipientSession> {
    db.query_row(
        "SELECT agent,role,epoch,status FROM pctx_sessions WHERE id=?1 AND workspace=?2",
        params![sid, p.workspace_id],
        |r| {
            Ok(RecipientSession {
                agent: r.get(0)?,
                role: r.get(1)?,
                epoch: r.get(2)?,
                status: r.get(3)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| {
        Error::new(
            "SESSION_NOT_FOUND",
            "Recipient session is not registered in current workspace",
            6,
        )
    })
}
fn task_id(db: &Connection, name: Option<&str>) -> Result<Option<String>> {
    name.map(|name| {
        let number = name
            .strip_prefix("T-")
            .and_then(|n| n.parse::<i64>().ok())
            .unwrap_or(-1);
        db.query_row(
            "SELECT id FROM tasks WHERE id=?1 OR number=?2",
            params![name, number],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::new("TASK_NOT_FOUND", "Task not found", 6))
    })
    .transpose()
}
fn decision_view(db: &Connection, decision: &str) -> Result<Value> {
    let (action,state,reason,request,evidence,expires,provenance):(String,String,String,String,Option<String>,Option<i64>,Option<String>)=db.query_row("SELECT action,state,reason,request,owner_evidence,expires_at,provenance FROM ops_decisions WHERE id=?1",[decision],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?.ok_or_else(||Error::new("DECISION_NOT_FOUND","Decision not found",6))?;
    let mut s = db.prepare("SELECT actor FROM ops_requesters WHERE decision=?1 ORDER BY actor")?;
    let requesters = s
        .query_map([decision], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(
        json!({"decision_id":decision,"action":serde_json::from_str::<Value>(&action)?,"state":if state=="approved"&&expires.is_none_or(|n|n<=now()){"expired"}else{&state},"reason":reason,"request":serde_json::from_str::<Value>(&request)?,"owner_evidence":evidence,"expires_at":expires,"provenance":provenance,"requesters":requesters,"host_permission":"separate_required"}),
    )
}
pub fn execute(p: &Project, command: &OperationCommand) -> Result<Value> {
    let mut db = connect(p)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let value = match command {
        OperationCommand::Role { command } => match command {
            RoleCommand::List => {
                owner()?;
                let mut s = tx.prepare(
                    "SELECT role,paused,reason,actor,updated FROM ops_roles ORDER BY role",
                )?;
                let roles=s.query_map([],|r|Ok(json!({"role":r.get::<_,String>(0)?,"paused":r.get::<_,bool>(1)?,"reason":r.get::<_,String>(2)?,"actor":r.get::<_,String>(3)?,"updated_at":r.get::<_,i64>(4)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
                let mut s=tx.prepare("SELECT role,recipient,topic,active FROM ops_silences ORDER BY role,recipient,topic")?;
                let silences=s.query_map([],|r|Ok(json!({"role":r.get::<_,String>(0)?,"recipient":r.get::<_,String>(1)?,"topic":r.get::<_,String>(2)?,"active":r.get::<_,bool>(3)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
                json!({"roles":roles,"silences":silences})
            }
            RoleCommand::Pause {
                role,
                reason,
                topic,
                recipient,
            }
            | RoleCommand::Resume {
                role,
                reason,
                topic,
                recipient,
            } => {
                owner()?;
                label(role)?;
                if reason.trim().is_empty() {
                    return Err(invalid("Pause/resume requires reason"));
                }
                let active = matches!(command, RoleCommand::Pause { .. });
                if let Some(topic) = topic {
                    label(topic)?;
                    let recipient = recipient_key(&tx, recipient.as_deref().unwrap_or("*"))?;
                    label(&recipient)?;
                    tx.execute("INSERT INTO ops_silences VALUES(?1,?2,?3,?4,?5,'owner',?6) ON CONFLICT(role,recipient,topic) DO UPDATE SET active=excluded.active,reason=excluded.reason,actor=excluded.actor,updated=excluded.updated",params![role,recipient,topic,active,reader::redact(reason).0,now()])?;
                } else {
                    if recipient.is_some() {
                        return Err(invalid("Recipient requires a topic silence"));
                    }
                    tx.execute("INSERT INTO ops_roles VALUES(?1,?2,?3,'owner',?4) ON CONFLICT(role) DO UPDATE SET paused=excluded.paused,reason=excluded.reason,actor=excluded.actor,updated=excluded.updated",params![role,active,reader::redact(reason).0,now()])?;
                }
                event(
                    &tx,
                    role,
                    "role_policy_changed",
                    &json!({"paused_or_silenced":active,"topic_policy":topic.is_some()}),
                )?;
                json!({"role":role,"paused_or_silenced":active,"persistent":true,"model_woken":false})
            }
        },
        OperationCommand::Policy {
            command: PolicyCommand::Evaluate { action_file },
        } => {
            let mut action: Action = raw_input(action_file)?;
            validate_action(&mut action)?;
            evaluate(p, &tx, &action)?
        }
        OperationCommand::Decision { command } => match command {
            DecisionCommand::Request {
                from_file,
                task_id: task,
            } => {
                let requester = authenticate(p, &tx)?;
                let mut r: DecisionRequest = raw_input(from_file)?;
                if r.schema_version != 1 || r.reason.trim().is_empty() {
                    return Err(invalid("Decision request requires schema and reason"));
                }
                validate_action(&mut r.action)?;
                let task = task_id(&tx, task.as_deref())?;
                let f = fingerprint(p, &r.action)?;
                let existing: Option<String> = tx
                    .query_row(
                        "SELECT id FROM ops_decisions WHERE fingerprint=?1",
                        [&f],
                        |r| r.get(0),
                    )
                    .optional()?;
                let d = existing.unwrap_or_else(|| id("DEC"));
                let mut request_payload = serde_json::to_value(&r)?;
                sanitize(&mut request_payload);
                tx.execute("INSERT OR IGNORE INTO ops_decisions(id,fingerprint,action,policy,state,reason,request,task,created) VALUES(?1,?2,?3,?4,'pending_gateway',?5,?6,?7,?8)",params![d,f,serde_json::to_string(&r.action)?,p.policy_hash(),reader::redact(&r.reason).0,request_payload.to_string(),task,now()])?;
                let inserted = tx.execute(
                    "INSERT OR IGNORE INTO ops_requesters VALUES(?1,?2,?3)",
                    params![d, requester, now()],
                )?;
                if inserted > 0 {
                    event(
                        &tx,
                        &d,
                        "decision_requested",
                        &json!({"requester":requester,"action_fingerprint":f}),
                    )?;
                }
                decision_view(&tx, &d)?
            }
            DecisionCommand::Record {
                id: decision,
                from_file,
            } => {
                owner()?;
                let r: DecisionRecord = input(from_file)?;
                if r.schema_version != 1
                    || r.owner_evidence.trim().is_empty()
                    || r.expires_at <= now()
                {
                    return Err(invalid(
                        "Trusted decision needs owner evidence and future expiry",
                    ));
                }
                let old = decision_view(&tx, decision)?;
                if !["pending_gateway", "waiting_owner", "expired"]
                    .contains(&old["state"].as_str().unwrap_or(""))
                {
                    return Err(Error::new(
                        "REVISION_CONFLICT",
                        "Decision is already terminal",
                        9,
                    ));
                }
                tx.execute("UPDATE ops_decisions SET state=?1,owner_evidence=?2,expires_at=?3,provenance='trusted_local_operator' WHERE id=?4",params![if r.approve{"approved"}else{"denied"},reader::redact(&r.owner_evidence).0,r.expires_at,decision])?;
                event(
                    &tx,
                    decision,
                    "decision_recorded",
                    &json!({"state":if r.approve{"approved"}else{"denied"},"note":reader::redact(&r.note).0,"provenance":"trusted_local_operator"}),
                )?;
                decision_view(&tx, decision)?
            }
            DecisionCommand::Show { id } => decision_view(&tx, id)?,
        },
        OperationCommand::Owner {
            command: OwnerCommand::Queue { limit },
        } => {
            owner()?;
            if *limit == 0 || *limit > 1000 {
                return Err(invalid("Queue limit must be 1..1000"));
            }
            // Suppression is a delivery rule: it precedes ranking/limit and detail projection.
            let has_sessions: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='pctx_sessions')",
                [], |row| row.get(0),
            )?;
            let mut statement = tx.prepare(
                "SELECT id,action FROM ops_decisions WHERE state IN ('pending_gateway','waiting_owner') ORDER BY created,id",
            )?;
            let candidates = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut items = Vec::new();
            for candidate in candidates {
                let (id, payload) = candidate?;
                let action: Action = serde_json::from_str(&payload)?;
                let mut roles = vec!["*".to_string()];
                if let Some(role) = &action.role {
                    roles.push(role.clone());
                }
                // Omitted role metadata must not bypass the actor's known session role.
                if has_sessions {
                    let mut roles_query = tx.prepare(
                        "SELECT DISTINCT role FROM pctx_sessions WHERE role IS NOT NULL AND agent IN (SELECT id FROM agents WHERE id=?1 OR name=?1)",
                    )?;
                    for role in
                        roles_query.query_map([&action.actor], |row| row.get::<_, String>(0))?
                    {
                        roles.push(role?);
                    }
                }
                roles.sort();
                roles.dedup();
                let mut hidden = false;
                for role in &roles {
                    if paused(&tx, role)? {
                        hidden = true;
                        break;
                    }
                    if let Some(topic) = &action.topic
                        && (silenced(&tx, role, &action.actor, topic)?
                            || silenced(&tx, role, "owner", topic)?)
                    {
                        hidden = true;
                        break;
                    }
                }
                if hidden {
                    continue;
                }
                items.push(decision_view(&tx, &id)?);
                if items.len() == *limit {
                    break;
                }
            }
            json!({"decisions":items,"owner_prompt_sent":false})
        }
        OperationCommand::Message { command } => {
            match command {
                MessageCommand::Send {
                    from_file,
                    to_role,
                    to_agent,
                    to_session,
                    task_id: task,
                } => {
                    let sender = authenticate(p, &tx)?;
                    let m: Message = input(from_file)?;
                    enqueue_message_db(
                        p,
                        &tx,
                        &sender,
                        &m,
                        to_role,
                        to_agent,
                        to_session,
                        task.as_deref(),
                    )?
                }
                MessageCommand::Ack {
                    id: message,
                    session: sid,
                } => {
                    let sender = authenticate(p, &tx)?;
                    let s = session(&tx, p, sid)?;
                    if sender != "owner" && sender != s.agent {
                        return Err(denied("Caller does not own recipient session"));
                    }
                    if s.status != "active" {
                        return Err(denied("Recipient session is suspended"));
                    }
                    let (kind, recipient, payload): (String, String, String) = tx
                        .query_row(
                            "SELECT recipient_kind,recipient,payload FROM ops_messages WHERE id=?1",
                            [message],
                            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                        )
                        .optional()?
                        .ok_or_else(|| Error::new("MESSAGE_NOT_FOUND", "Message not found", 6))?;
                    let matches = match kind.as_str() {
                        "session" => recipient == *sid,
                        "agent" => recipient == s.agent,
                        "role" => s.role.as_deref() == Some(&recipient),
                        _ => false,
                    };
                    if !matches {
                        return Err(denied(
                            "Acknowledgement session does not match exact recipient",
                        ));
                    }
                    let m: Message = serde_json::from_str(&payload)?;
                    if m.expires_at.is_some_and(|t| t <= now()) {
                        return Err(Error::new("MESSAGE_EXPIRED", "Message expired", 9));
                    }
                    if paused(&tx, s.role.as_deref().unwrap_or(""))?
                        || (silenced(&tx, s.role.as_deref().unwrap_or("*"), &s.agent, &m.topic)?
                            || silenced(&tx, s.role.as_deref().unwrap_or("*"), sid, &m.topic)?)
                    {
                        return Err(Error::new(
                            "TOPIC_SILENCED",
                            "Delivery blocked by persistent role/topic policy",
                            5,
                        ));
                    }
                    let inserted = tx.execute(
                        "INSERT OR IGNORE INTO ops_message_acks VALUES(?1,?2,?3,?4)",
                        params![message, sid, s.epoch, now()],
                    )?;
                    if inserted > 0 {
                        tx.execute("UPDATE ops_messages SET state='acknowledged' WHERE id=?1 AND state='queued'",[message])?;
                        event(
                            &tx,
                            message,
                            "message_acknowledged",
                            &json!({"session_id":sid,"epoch":s.epoch}),
                        )?;
                    }
                    json!({"message_id":message,"session_id":sid,"context_epoch":s.epoch,"delivery_state":"acknowledged","understanding_proven":false})
                }
                MessageCommand::Resolve {
                    id: message,
                    evidence,
                } => {
                    owner()?;
                    label(evidence)?;
                    let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM ops_decisions WHERE id=?1) OR EXISTS(SELECT 1 FROM checks WHERE id=?1)",[evidence],|r|r.get(0))?;
                    if !exists {
                        return Err(invalid(
                            "Resolution evidence must reference a durable decision or check",
                        ));
                    }
                    let n=tx.execute("UPDATE ops_messages SET state='resolved' WHERE id=?1 AND state<>'resolved'",[message])?;
                    if n > 0 {
                        event(
                            &tx,
                            message,
                            "message_resolved",
                            &json!({"evidence":evidence}),
                        )?;
                    }
                    let exists: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM ops_messages WHERE id=?1)",
                        [message],
                        |r| r.get(0),
                    )?;
                    if !exists {
                        return Err(Error::new("MESSAGE_NOT_FOUND", "Message not found", 6));
                    }
                    json!({"message_id":message,"delivery_state":"resolved","evidence":evidence})
                }
            }
        }
        OperationCommand::Inbox {
            command:
                InboxCommand::Read {
                    session: sid,
                    since_seq,
                    limit,
                },
        } => {
            if *limit == 0 || *limit > 1000 || *since_seq < 0 {
                return Err(invalid("Inbox limit/cursor invalid"));
            }
            let s = session(&tx, p, sid)?;
            if actor() != "owner" {
                let caller = authenticate(p, &tx)?;
                if caller != s.agent {
                    return Err(denied("Caller does not own inbox session"));
                }
            }
            let mut stmt=tx.prepare("SELECT seq,id,recipient_kind,recipient,payload,state FROM ops_messages WHERE seq>?1 AND ((recipient_kind='session' AND recipient=?2) OR (recipient_kind='agent' AND recipient=?3) OR (recipient_kind='role' AND recipient=?4)) ORDER BY seq")?;
            let rows = stmt
                .query_map(params![since_seq, sid, s.agent, s.role], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut visible = vec![];
            let mut cursor = *since_seq;
            let mut suppressed = 0;
            for (seq, id, _kind, _recipient, payload, state) in rows {
                let m: Message = serde_json::from_str(&payload)?;
                cursor = seq;
                if m.expires_at.is_some_and(|t| t <= now())
                    || s.status != "active"
                    || paused(&tx, s.role.as_deref().unwrap_or(""))?
                    || (silenced(&tx, s.role.as_deref().unwrap_or("*"), &s.agent, &m.topic)?
                        || silenced(&tx, s.role.as_deref().unwrap_or("*"), sid, &m.topic)?)
                {
                    suppressed += 1;
                    continue;
                }
                visible.push(
                    json!({"message_seq":seq,"message_id":id,"message":m,"delivery_state":state}),
                );
                if visible.len() >= *limit {
                    break;
                }
            }
            visible.sort_by(|a, b| {
                a["message"]["priority"]
                    .as_u64()
                    .cmp(&b["message"]["priority"].as_u64())
                    .then_with(|| a["message_seq"].as_i64().cmp(&b["message_seq"].as_i64()))
            });
            json!({"session_id":sid,"messages":visible,"next_cursor":cursor,"suppressed_count":suppressed,"delivery_inferred":false,"model_woken":false})
        }
    };
    tx.commit()?;
    Ok(value)
}

// Called before opening the schedule claim/finalization transaction.
pub(crate) fn prepare_scheduled_mailbox(p: &Project) -> Result<()> {
    connect(p)?;
    Ok(())
}

pub(crate) fn enqueue_scheduled_db(
    p: &Project,
    db: &Connection,
    recipient_role: &str,
    message: &Message,
) -> Result<Value> {
    owner()?;
    let mut payload = serde_json::to_value(message)?;
    sanitize(&mut payload);
    let message: Message = serde_json::from_value(payload)?;
    enqueue_message_db(
        p,
        db,
        "owner",
        &message,
        &Some(recipient_role.into()),
        &None,
        &None,
        None,
    )
}

// Both explicit sends and schedule delivery use one bounded, idempotent mailbox contract.
#[allow(clippy::too_many_arguments)]
fn enqueue_message_db(
    p: &Project,
    db: &Connection,
    sender: &str,
    m: &Message,
    to_role: &Option<String>,
    to_agent: &Option<String>,
    to_session: &Option<String>,
    task: Option<&str>,
) -> Result<Value> {
    Ok({
        if m.schema_version != 1
            || m.body.len() > 2048
            || m.body.lines().count() > 5
            || m.body.trim().is_empty()
            || m.priority > 3
            || ![
                "assignment",
                "progress",
                "question",
                "blocker",
                "decision",
                "result",
                "permission_incident",
                "notice",
            ]
            .contains(&m.kind.as_str())
        {
            return Err(invalid("Message schema/type/body/priority invalid"));
        }
        label(&m.topic)?;
        label(&m.idempotency_key)?;
        let recipients = [
            ("role", to_role),
            ("agent", to_agent),
            ("session", to_session),
        ]
        .into_iter()
        .filter_map(|(kind, value)| value.as_ref().map(|v| (kind, v)))
        .collect::<Vec<_>>();
        if recipients.len() != 1 {
            return Err(invalid("Exactly one bounded recipient is required"));
        }
        let (kind, recipient) = recipients[0];
        label(recipient)?;
        let recipient = match kind {
            "agent" => {
                db.query_row(
                    "SELECT id FROM agents WHERE id=?1 OR name=?1",
                    [recipient],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
                .ok_or_else(|| Error::new("AGENT_NOT_FOUND", "Recipient agent not registered", 6))?
            }
            "session" => {
                session(db, p, recipient)?;
                recipient.clone()
            }
            _ => recipient.clone(),
        };
        let task = task_id(db, task)?;
        let payload = serde_json::to_string(&m)?;
        let request = hash(serde_json::to_vec(
            &json!({"payload":m,"recipient_kind":kind,"recipient":recipient,"task":task}),
        )?);
        if let Some((h, response)) = db
            .query_row(
                "SELECT request_hash,response FROM ops_receipts WHERE actor=?1 AND key=?2",
                params![sender, m.idempotency_key],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
        {
            if h != request {
                return Err(Error::new(
                    "IDEMPOTENCY_CONFLICT",
                    "Message key already used with different payload",
                    9,
                ));
            }
            serde_json::from_str(&response)?
        } else {
            type SavedMessage = (i64, String, String, String, Option<String>, String, String);
            let saved:Option<SavedMessage>=db.query_row("SELECT seq,id,recipient_kind,recipient,task,payload,state FROM ops_messages WHERE sender=?1 AND json_extract(payload,'$.idempotency_key')=?2 ORDER BY seq LIMIT 1",params![sender,m.idempotency_key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
            let response = if let Some((
                seq,
                message,
                old_kind,
                old_recipient,
                old_task,
                old_payload,
                state,
            )) = saved
            {
                let old_message: Message = serde_json::from_str(&old_payload)?;
                let previous = hash(serde_json::to_vec(
                    &json!({"payload":old_message,"recipient_kind":old_kind,"recipient":old_recipient,"task":old_task}),
                )?);
                if previous != request {
                    return Err(Error::new(
                        "IDEMPOTENCY_CONFLICT",
                        "Original queued message key has different payload",
                        9,
                    ));
                }
                json!({"message_id":message,"message_seq":seq,"delivery_state":state,"external_transport":"not_attempted","model_woken":false})
            } else {
                if let (Some(task), Some(revision)) = (&task, m.source_revision) {
                    let current: i64 =
                        db.query_row("SELECT revision FROM tasks WHERE id=?1", [task], |r| {
                            r.get(0)
                        })?;
                    if current != revision {
                        return Err(Error::new(
                            "REVISION_CONFLICT",
                            "Message source task revision changed",
                            9,
                        ));
                    }
                }
                let message = id("MSG");
                db.execute("INSERT INTO ops_messages(id,sender,recipient_kind,recipient,task,payload,state,created) VALUES(?1,?2,?3,?4,?5,?6,'queued',?7)",params![message,sender,kind,recipient,task,payload,now()])?;
                let seq = db.last_insert_rowid();
                event(
                    db,
                    &message,
                    "message_queued",
                    &json!({"recipient_kind":kind,"topic_details_omitted":true}),
                )?;
                json!({"message_id":message,"message_seq":seq,"delivery_state":"queued","external_transport":"not_attempted","model_woken":false})
            };
            db.execute(
                "INSERT INTO ops_receipts VALUES(?1,?2,?3,?4)",
                params![sender, m.idempotency_key, request, response.to_string()],
            )?;
            response
        }
    })
}

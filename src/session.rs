//! Explicit context receipts and independent runtime epochs. No source or dialogue ledger.
use crate::{
    domain::{Error, Result, hash, id, now},
    project::Project,
    reader, work,
};
use clap::Subcommand;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use std::collections::BTreeMap;
const SERIALIZER: &str = "minimal-context-v2";
#[derive(Debug, Clone, Subcommand)]
pub enum SessionCommand {
    Attach {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        runtime: String,
        #[arg(long, default_value = "current")]
        workspace: String,
        #[arg(long)]
        native_session: Option<String>,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        account_pool: Option<String>,
        #[arg(long, default_value = "manual-v1")]
        adapter_version: String,
    },
    Suspend {
        #[arg(long)]
        session: String,
        #[arg(long, default_value = "manual")]
        reason: String,
    },
    Reconcile {
        #[arg(long)]
        session: String,
    },
    Boundary {
        #[arg(long)]
        session: String,
        #[arg(long, default_value = "context-boundary")]
        reason: String,
    },
}
#[derive(Debug, Clone, Subcommand)]
pub enum ContextCommand {
    Get {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        session: String,
        #[arg(long,default_value="full",value_parser=["full","delta"])]
        mode: String,
        #[arg(long)]
        since: Option<String>,
        #[arg(long = "scope")]
        scope: Vec<String>,
        #[arg(long, default_value_t = 6000)]
        budget_bytes: usize,
    },
    Ack {
        context: String,
        #[arg(long)]
        session: String,
        #[arg(long)]
        epoch: i64,
        #[arg(long,default_value="explicit-agent",value_parser=["explicit-agent","transport-receipt"])]
        provenance: String,
    },
}
fn invalid(s: &str) -> Error {
    Error::new("INVALID_ARGUMENT", s, 2)
}
fn mismatch(s: &str) -> Error {
    Error::new(
        "BASELINE_MISMATCH",
        format!("{s}; request a new full packet"),
        9,
    )
}
fn connect(p: &Project) -> Result<Connection> {
    p.check_deadline()?;
    let mut db = work::connect(p)?;
    p.check_deadline()?;
    let initialized:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='pctx_session_schema')",[],|r|r.get(0))?;
    if initialized {
        let version: i64 =
            db.query_row("SELECT version FROM pctx_session_schema", [], |r| r.get(0))?;
        if version != 1 {
            return Err(Error::new(
                "DB_SCHEMA_TOO_NEW",
                "Unsupported session schema",
                7,
            ));
        }
        db.execute_batch("CREATE TRIGGER IF NOT EXISTS pctx_session_events_no_update BEFORE UPDATE ON pctx_session_events BEGIN SELECT RAISE(ABORT,'append-only events'); END; CREATE TRIGGER IF NOT EXISTS pctx_session_events_no_delete BEFORE DELETE ON pctx_session_events BEGIN SELECT RAISE(ABORT,'append-only events'); END;")?;
        return Ok(db);
    }
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS pctx_session_schema(version INTEGER NOT NULL);
INSERT INTO pctx_session_schema SELECT 1 WHERE NOT EXISTS(SELECT 1 FROM pctx_session_schema);
CREATE TABLE IF NOT EXISTS pctx_sessions(id TEXT PRIMARY KEY,agent TEXT NOT NULL,runtime TEXT NOT NULL,native_id TEXT,role TEXT,workspace TEXT NOT NULL,account_pool TEXT,adapter TEXT NOT NULL,epoch INTEGER NOT NULL,status TEXT NOT NULL,created INTEGER NOT NULL,updated INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS pctx_context_emissions(id TEXT PRIMARY KEY,session TEXT NOT NULL REFERENCES pctx_sessions(id),epoch INTEGER NOT NULL,task TEXT NOT NULL,policy TEXT NOT NULL,scope TEXT NOT NULL,serializer TEXT NOT NULL,content_hash TEXT NOT NULL,selection TEXT NOT NULL,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS pctx_context_acks(session TEXT NOT NULL,epoch INTEGER NOT NULL,context TEXT NOT NULL REFERENCES pctx_context_emissions(id),provenance TEXT NOT NULL,created INTEGER NOT NULL,PRIMARY KEY(session,epoch,context));
CREATE TABLE IF NOT EXISTS pctx_session_events(seq INTEGER PRIMARY KEY AUTOINCREMENT,session TEXT NOT NULL,kind TEXT NOT NULL,metadata TEXT NOT NULL,created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS pctx_session_capsules(id TEXT PRIMARY KEY,session TEXT NOT NULL,epoch INTEGER NOT NULL,metadata TEXT NOT NULL,created INTEGER NOT NULL);")?;
    let version: i64 = tx.query_row("SELECT version FROM pctx_session_schema", [], |r| r.get(0))?;
    if version != 1 {
        return Err(Error::new(
            "DB_SCHEMA_TOO_NEW",
            "Unsupported session schema",
            7,
        ));
    }
    p.check_deadline()?;
    tx.commit()?;
    db.execute_batch("CREATE TRIGGER IF NOT EXISTS pctx_session_events_no_update BEFORE UPDATE ON pctx_session_events BEGIN SELECT RAISE(ABORT,'append-only events'); END; CREATE TRIGGER IF NOT EXISTS pctx_session_events_no_delete BEFORE DELETE ON pctx_session_events BEGIN SELECT RAISE(ABORT,'append-only events'); END;")?;
    Ok(db)
}
#[derive(Debug)]
struct Session {
    agent: String,
    workspace: String,
    epoch: i64,
    status: String,
}
fn get(db: &Connection, p: &Project, id: &str) -> Result<Session> {
    let s = db
        .query_row(
            "SELECT agent,workspace,epoch,status FROM pctx_sessions WHERE id=?1",
            [id],
            |r| {
                Ok(Session {
                    agent: r.get(0)?,
                    workspace: r.get(1)?,
                    epoch: r.get(2)?,
                    status: r.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| Error::new("SESSION_NOT_FOUND", "Session is not registered", 6))?;
    if s.workspace != p.workspace_id {
        return Err(mismatch("Session belongs to another workspace"));
    }
    if work::authenticated_actor(p, db)?.is_some_and(|actor| actor != s.agent) {
        return Err(Error::new(
            "POLICY_DENIED",
            "Actor does not own this session",
            5,
        ));
    }
    Ok(s)
}
fn event(db: &Connection, session: &str, kind: &str, metadata: &Value) -> Result<()> {
    db.execute(
        "INSERT INTO pctx_session_events(session,kind,metadata,created) VALUES(?1,?2,?3,?4)",
        params![session, kind, metadata.to_string(), now()],
    )?;
    Ok(())
}
fn validate_label(s: &str) -> Result<()> {
    if s.trim().is_empty() || s.len() > 1024 || s.contains('\0') || reader::redact(s).1 {
        return Err(invalid("Invalid session identity field"));
    }
    Ok(())
}
fn snapshot(p: &Project, agent: &str) -> Result<Value> {
    let board = redact_value(work::board(p)?);
    let tasks:Vec<_>=board["tasks"].as_array().into_iter().flatten().filter(|t|t["agent_id"]==agent).map(|t|json!({"task_id":t["task_id"],"state":t["state"],"task_revision":t["task_revision"],"definition_revision":t["definition_revision"],"run":t["run"],"definition_hash":hash(t["definition"].to_string()),"paused":t["state"]=="paused","resume_requires_new_lease":true})).collect();
    let manifest = reader::manifest(p)?;
    Ok(
        json!({"schema_version":1,"project_id":p.project_id,"workspace_id":p.workspace_id,"agent_id":agent,"policy_hash":p.policy_hash(),"manifest":manifest,"event_watermark":board["as_of_seq"],"tasks":tasks,"usage":crate::quota::recovery_state(p)?,"approval_restore":false,"lease_restore":false,"dialogue_included":false}),
    )
}
// Library callers get the same finite budget; an existing outer deadline is
// retained rather than renewed by nested session/context calls.
fn request(p: &Project, operation: impl FnOnce(&Project) -> Result<Value>) -> Result<Value> {
    let mut scoped;
    let p = if p.deadline.is_none() {
        scoped = p.clone();
        scoped.deadline = Some(crate::deadline::Deadline::from_millis(10_000)?);
        &scoped
    } else {
        p
    };
    p.check_deadline()?;
    let result = operation(p);
    // Normalize late storage/serialization failures to the original expiry.
    p.check_deadline()?;
    result
}
pub fn session(p: &Project, c: &SessionCommand) -> Result<Value> {
    request(p, |p| session_inner(p, c))
}
fn session_inner(p: &Project, c: &SessionCommand) -> Result<Value> {
    // Operational snapshot is read before opening our writer transaction.
    let mut db = connect(p)?;
    let capsule = match c {
        SessionCommand::Suspend { session, .. } | SessionCommand::Reconcile { session } => {
            let s = get(&db, p, session)?;
            Some(snapshot(p, &s.agent)?)
        }
        _ => None,
    };
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let value = match c {
        SessionCommand::Attach {
            agent,
            runtime,
            workspace,
            native_session,
            role,
            account_pool,
            adapter_version,
        } => {
            if std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()) != "owner" {
                return Err(Error::new(
                    "POLICY_DENIED",
                    "Registering a session binding requires owner authority",
                    5,
                ));
            }
            for label in [
                Some(agent),
                Some(runtime),
                native_session.as_ref(),
                role.as_ref(),
                account_pool.as_ref(),
                Some(adapter_version),
            ]
            .into_iter()
            .flatten()
            {
                validate_label(label)?;
            }
            if workspace != "current" && workspace != &p.workspace_id {
                return Err(invalid("Attach requires the current workspace"));
            }
            let agent_id: String = tx
                .query_row(
                    "SELECT id FROM agents WHERE id=?1 OR name=?1",
                    [agent],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or_else(|| {
                    Error::new(
                        "AGENT_NOT_FOUND",
                        "Register logical agent before attaching",
                        6,
                    )
                })?;
            let session = id("SESSION");
            tx.execute(
                "INSERT INTO pctx_sessions VALUES(?1,?2,?3,?4,?5,?6,?7,?8,1,'active',?9,?9)",
                params![
                    session,
                    agent_id,
                    runtime,
                    native_session,
                    role,
                    p.workspace_id,
                    account_pool,
                    adapter_version,
                    now()
                ],
            )?;
            event(
                &tx,
                &session,
                "attached",
                &json!({"epoch":1,"agent_id":agent_id}),
            )?;
            json!({"session_id":session,"agent_id":agent_id,"workspace_id":p.workspace_id,"context_epoch":1,"status":"active","full_required":true,"ack_inherited":false})
        }
        SessionCommand::Boundary { session, reason } => {
            validate_label(reason)?;
            let s = get(&tx, p, session)?;
            tx.execute(
                "UPDATE pctx_sessions SET epoch=epoch+1,updated=?1 WHERE id=?2",
                params![now(), session],
            )?;
            event(
                &tx,
                session,
                "boundary",
                &json!({"reason":reader::redact(reason).0,"epoch":s.epoch+1}),
            )?;
            json!({"session_id":session,"context_epoch":s.epoch+1,"status":s.status,"full_required":true})
        }
        SessionCommand::Suspend { session, reason } => {
            validate_label(reason)?;
            let s = get(&tx, p, session)?;
            let mut metadata = capsule.unwrap();
            metadata["context_epoch"] = json!(s.epoch);
            metadata["session_id"] = json!(session);
            metadata["reason"] = json!(reader::redact(reason).0);
            let cap = id("CAP");
            tx.execute(
                "INSERT INTO pctx_session_capsules VALUES(?1,?2,?3,?4,?5)",
                params![cap, session, s.epoch, metadata.to_string(), now()],
            )?;
            tx.execute(
                "UPDATE pctx_sessions SET status='suspended',updated=?1 WHERE id=?2",
                params![now(), session],
            )?;
            event(&tx, session, "suspended", &json!({"capsule_id":cap}))?;
            json!({"session_id":session,"status":"suspended","capsule_id":cap,"capsule":metadata})
        }
        SessionCommand::Reconcile { session } => {
            let s = get(&tx, p, session)?;
            let current = capsule.unwrap();
            let latest:Option<(String,String)>=tx.query_row("SELECT id,metadata FROM pctx_session_capsules WHERE session=?1 ORDER BY rowid DESC LIMIT 1",[session],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            let old = latest
                .as_ref()
                .map(|(_, s)| serde_json::from_str::<Value>(s))
                .transpose()?;
            let paused = current["tasks"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|t| t["paused"] == true);
            let changed = old.as_ref().is_some_and(|o| {
                o["manifest"] != current["manifest"]
                    || o["policy_hash"] != current["policy_hash"]
                    || o["tasks"] != current["tasks"]
            });
            json!({"session_id":session,"context_epoch":s.epoch,"status":s.status,"capsule_id":latest.map(|(id,_)|id),"current":current,"changed_since_capsule":changed,"resume_allowed":s.status=="active"&&!paused,"next_action":if paused{"owner must explicitly resume paused work"}else{"attach a new session, obtain and acknowledge full context, then explicitly claim available work"},"lease_restored":false,"approvals_restored":false})
        }
    };
    p.check_deadline()?;
    tx.commit()?;
    Ok(value)
}
fn redact_value(value: Value) -> Value {
    match value {
        Value::String(s) => json!(reader::redact(&s).0),
        Value::Array(a) => Value::Array(a.into_iter().map(redact_value).collect()),
        Value::Object(o) => Value::Object(
            o.into_iter()
                .map(|(k, v)| {
                    let safe = if [
                        "task_id",
                        "agent_id",
                        "workspace_id",
                        "run_id",
                        "project_id",
                        "session_id",
                    ]
                    .contains(&k.as_str())
                    {
                        v
                    } else {
                        redact_value(v)
                    };
                    (k, safe)
                })
                .collect(),
        ),
        v => v,
    }
}
fn in_scope(path: &str, scope: &[String]) -> bool {
    scope.is_empty()
        || scope.iter().any(|s| {
            path == s
                || path.starts_with(&format!("{s}/"))
                || globset::Glob::new(s).is_ok_and(|g| g.compile_matcher().is_match(path))
        })
}
// Evidence hashes describe the actual masked representation, not merely the
// original file. Text offsets below are delivered UTF-8 offsets; original source
// ranges are copied only when the producer supplies them.
fn representation_metadata(mut metadata: Value, body: &Value) -> Result<Value> {
    metadata["representation"] = body["representation"].clone();
    metadata["delivered_body_hash"] = json!(hash(serde_json::to_vec(body)?));
    metadata["body_hash_format"] = json!("canonical-json-v1");
    if let Some(text) = body["text"].as_str() {
        metadata["delivered_text_hash"] = json!(hash(text.as_bytes()));
        metadata["delivered_text_byte_range"] = json!([0, text.len()]);
    }
    for key in ["byte_range", "line_range", "source_range"] {
        if let Some(range) = body.get(key) {
            metadata[key] = range.clone();
        }
    }
    Ok(metadata)
}
fn selection(
    p: &Project,
    task: &str,
    scope: &[String],
) -> Result<(BTreeMap<String, Value>, BTreeMap<String, Value>)> {
    let task_value = work::execute(
        p,
        &work::WorkCommand::Task {
            command: work::TaskCommand::Show { task: task.into() },
        },
    )?;
    let task_value = redact_value(task_value);
    let mut metadata = BTreeMap::new();
    let mut bodies = BTreeMap::new();
    let task_body = json!({"kind":"task","representation":"metadata","metadata":task_value});
    metadata.insert("task".into(), representation_metadata(json!({"hash":hash(task_value.to_string()),"task_id":task_value["task_id"],"revision":task_value["task_revision"],"definition_revision":task_value["definition_revision"],"kind":"task"}), &task_body)?);
    bodies.insert("task".into(), task_body);
    let inventory = reader::inventory(p, false)?;
    if !inventory.skipped.is_empty() {
        return Err(Error::new(
            "PARTIAL_RESULT",
            "Context inventory is incomplete",
            3,
        ));
    }
    let effective_scope = if scope.is_empty() {
        task_value["definition"]["scope"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        scope.to_vec()
    };
    let docs = crate::documents::load(p, &effective_scope)?;
    for section in ["rules", "decisions", "instructions"] {
        p.check_deadline()?;
        for doc in docs[section].as_array().into_iter().flatten() {
            p.check_deadline()?;
            let path = doc["path"].as_str().unwrap();
            let key = format!("file:{path}");
            let kind = if section == "rules" && doc["required"] == true {
                "required_rule"
            } else if section == "decisions" {
                "decision"
            } else {
                "project_document"
            };
            let mut body = doc.clone();
            body["text"] = body["content"].take();
            body["kind"] = json!(kind);
            body["representation"] = json!("full_span");
            metadata.insert(key.clone(), representation_metadata(json!({"path":path,"hash":doc["file_hash"],"kind":kind,"scope":doc["scope"],"required":doc["required"]}), &body)?);
            bodies.insert(key, body);
        }
    }
    for path in inventory.paths {
        p.check_deadline()?;
        if path.starts_with(".pctx/rules/")
            || path.starts_with(".pctx/decisions/")
            || effective_scope.is_empty()
            || !in_scope(&path, &effective_scope)
        {
            continue;
        }
        let f = reader::read(p, &path)?;
        let key = format!("file:{path}");
        let body = json!({"path":path,"file_hash":f.hash,"kind":"reference","representation":"reference","evidence_status":"observed","freshness":"current"});
        metadata.insert(
            key.clone(),
            representation_metadata(json!({"path":path,"hash":f.hash,"kind":"reference"}), &body)?,
        );
        bodies.insert(key, body);
    }
    Ok((metadata, bodies))
}
pub fn context(p: &Project, c: &ContextCommand) -> Result<Value> {
    request(p, |p| context_inner(p, c))
}
fn context_inner(p: &Project, c: &ContextCommand) -> Result<Value> {
    match c {
        ContextCommand::Ack {
            context,
            session,
            epoch,
            provenance,
        } => {
            if !["explicit-agent", "transport-receipt"].contains(&provenance.as_str()) {
                return Err(invalid("Unknown ack provenance"));
            }
            let mut db = connect(p)?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let s = get(&tx, p, session)?;
            if s.epoch != *epoch {
                return Err(mismatch("Session epoch changed"));
            }
            let emission: Option<(String, i64, String)> = tx
                .query_row(
                    "SELECT session,epoch,policy FROM pctx_context_emissions WHERE id=?1",
                    [context],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            if !emission.is_some_and(|(sid, ep, pol)| {
                sid == *session && ep == *epoch && pol == p.policy_hash()
            }) {
                return Err(mismatch(
                    "Context was not issued to this session/epoch/policy",
                ));
            }
            tx.execute(
                "INSERT OR IGNORE INTO pctx_context_acks VALUES(?1,?2,?3,?4,?5)",
                params![session, epoch, context, provenance, now()],
            )?;
            event(
                &tx,
                session,
                "context_ack",
                &json!({"context_id":context,"epoch":epoch,"provenance":provenance}),
            )?;
            p.check_deadline()?;
            tx.commit()?;
            Ok(
                json!({"context_id":context,"session_id":session,"context_epoch":epoch,"acknowledged":true,"provenance":provenance,"understanding_proven":false}),
            )
        }
        ContextCommand::Get {
            task_id,
            session,
            mode,
            since,
            scope,
            budget_bytes,
        } => {
            let authorization_db = connect(p)?;
            get(&authorization_db, p, session)?;
            drop(authorization_db);
            if !["full", "delta"].contains(&mode.as_str()) {
                return Err(invalid("Unknown context mode"));
            }
            if mode == "full" && since.is_some() {
                return Err(invalid("Full context does not accept --since"));
            }
            if *budget_bytes < 512 {
                return Err(Error::new(
                    "BUDGET_TOO_SMALL",
                    "Minimum context cannot fit",
                    8,
                ));
            }
            let mut scope = scope.clone();
            for s in &scope {
                if s.is_empty()
                    || s.starts_with('/')
                    || s.contains('\\')
                    || s.split('/').any(|x| x == ".." || x == "." || x.is_empty())
                {
                    return Err(invalid("Scope must be a normalized relative path"));
                }
            }
            scope.sort();
            scope.dedup();
            let (metadata, bodies) = selection(p, task_id, &scope).map_err(|e| {
                Error::new(&e.code, format!("Context selection: {}", e.message), e.exit)
            })?;
            let mut db = connect(p)?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let s = get(&tx, p, session)?;
            let selection_json = serde_json::to_string(&metadata)?;
            let policy = p.policy_hash();
            let scope_json = serde_json::to_string(&scope)?;
            let content_hash = hash(serde_json::to_vec(
                &json!({"selection":metadata,"policy":policy,"scope":scope,"serializer":SERIALIZER,"workspace":p.workspace_id}),
            )?);
            let context_id = format!(
                "CTX-{}",
                hash(format!("{session}\0{}\0{content_hash}", s.epoch))
            );
            let mut added = Vec::new();
            let mut changed = Vec::new();
            let mut removed = Vec::new();
            let mut unchanged = 0;
            if mode == "delta" {
                let baseline = since
                    .as_ref()
                    .ok_or_else(|| mismatch("Delta requires --since an acknowledged context"))?;
                let row:Option<(String,i64,String,String,String,String,String)>=tx.query_row("SELECT session,epoch,task,policy,scope,serializer,selection FROM pctx_context_emissions WHERE id=?1",[baseline],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
                let row = row.ok_or_else(|| mismatch("Baseline does not exist"))?;
                let ack:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM pctx_context_acks WHERE session=?1 AND epoch=?2 AND context=?3)",params![session,s.epoch,baseline],|r|r.get(0))?;
                if !ack
                    || row.0 != *session
                    || row.1 != s.epoch
                    || row.2 != *task_id
                    || row.3 != policy
                    || row.4 != scope_json
                    || row.5 != SERIALIZER
                {
                    return Err(mismatch(
                        "Baseline session, epoch, task, permission scope, serializer or acknowledgement differs",
                    ));
                }
                let old: BTreeMap<String, Value> = serde_json::from_str(&row.6)?;
                for (key, value) in &metadata {
                    p.check_deadline()?;
                    match old.get(key) {
                        None => added.push(bodies[key].clone()),
                        Some(v) if v != value => changed.push(bodies[key].clone()),
                        _ => unchanged += 1,
                    }
                }
                for (key, value) in &old {
                    p.check_deadline()?;
                    if !metadata.contains_key(key) {
                        removed.push(json!({"item_id":key,"previous":value,"tombstone":true,"reason":"removed_or_no_longer_visible"}));
                    }
                }
            } else {
                added = bodies.values().cloned().collect();
            }
            let value = json!({"context_id":context_id,"session_id":session,"context_epoch":s.epoch,"task_id":task_id,"mode":mode,"baseline":since,"content_hash":content_hash,"policy_hash":policy,"scope":scope,"serializer":SERIALIZER,"added":added,"changed":changed,"removed":removed,"invalidated":[],"unchanged_count":unchanged,"ack_required":true,"required_minimum_complete":true,"session_status":s.status});
            if crate::render::render(
                &crate::domain::envelope("context", Some(p), value.clone()),
                crate::render::Format::Json,
            )?
            .len()
                > *budget_bytes
            {
                return Err(Error::new(
                    "BUDGET_TOO_SMALL",
                    "Required task/rules/references exceed budget; narrow scope or increase --budget-bytes",
                    8,
                ));
            }
            for item in metadata.values().filter(|m| m.get("path").is_some()) {
                p.check_deadline()?;
                let path = item["path"].as_str().unwrap();
                if reader::read(p, path)?.hash != item["hash"].as_str().unwrap() {
                    return Err(Error::new(
                        "CONCURRENT_MODIFICATION",
                        "Context source changed before emission",
                        4,
                    ));
                }
            }
            let task_revision: (i64, i64) = tx
                .query_row(
                    "SELECT revision,definition_revision FROM tasks WHERE id=?1",
                    [metadata["task"]["task_id"].as_str().unwrap_or("")],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .map_err(|_| Error::new("DB_ERROR", "Context task revision lookup failed", 7))?;
            if Some(task_revision.0) != metadata["task"]["revision"].as_i64()
                || Some(task_revision.1) != metadata["task"]["definition_revision"].as_i64()
            {
                return Err(Error::new(
                    "CONCURRENT_MODIFICATION",
                    "Task changed before context emission",
                    4,
                ));
            }
            tx.execute("INSERT OR IGNORE INTO pctx_context_emissions VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![context_id,session,s.epoch,task_id,policy,scope_json,SERIALIZER,content_hash,selection_json,now()]).map_err(|_|Error::new("DB_ERROR","Context emission insert failed",7))?;
            event(
                &tx,
                session,
                "context_emitted",
                &json!({"context_id":context_id,"mode":mode,"epoch":s.epoch}),
            )?;
            p.check_deadline()?;
            tx.commit()?;
            Ok(value)
        }
    }
}

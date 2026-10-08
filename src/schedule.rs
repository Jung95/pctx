//! Durable schedule intent and occurrence plans. This registry never installs OS jobs.
use crate::{
    domain::{Error, Result, hash, now},
    operations::Action,
    project::Project,
    reader, work,
};
use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone};
use chrono_tz::Tz;
use clap::Subcommand;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
#[derive(Debug, Clone, Subcommand)]
pub enum ScheduleCommand {
    Add {
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        idempotency_key: String,
    },
    Update {
        #[arg(long)]
        namespace: String,
        id: String,
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        expect_revision: i64,
    },
    List {
        #[arg(long)]
        namespace: Option<String>,
    },
    Pause {
        #[arg(long)]
        namespace: String,
        id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        expect_revision: i64,
    },
    Resume {
        #[arg(long)]
        namespace: String,
        id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        expect_revision: i64,
    },
    Remove {
        #[arg(long)]
        namespace: String,
        id: String,
        #[arg(long)]
        expect_revision: i64,
    },
    Reconcile {
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        at: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Cadence {
    Daily { at: String },
    Interval { seconds: i64, anchor: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleDefinition {
    pub schema_version: u32,
    pub namespace: String,
    pub id: String,
    pub timezone: String,
    pub cadence: Cadence,
    pub valid_from: String,
    pub job: String,
    pub bridge: String,
    pub role: String,
    pub recipient: String,
    pub topic: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub context_epoch: Option<i64>,
    #[serde(default)]
    pub action: Option<Action>,
    #[serde(default)]
    pub decision_id: Option<String>,
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default = "misfire")]
    pub misfire: String,
}
fn enabled() -> bool {
    true
}
fn misfire() -> String {
    "coalesce_latest".into()
}
fn invalid(s: &str) -> Error {
    Error::new("INVALID_ARGUMENT", s, 2)
}
fn conflict(s: &str) -> Error {
    Error::new("REVISION_CONFLICT", s, 9)
}
fn owner() -> Result<()> {
    if std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()) != "owner" {
        return Err(Error::new(
            "POLICY_DENIED",
            "Schedule registry changes require owner authority",
            5,
        ));
    }
    Ok(())
}
fn label(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 256 || s.chars().any(char::is_control) || reader::redact(s).1 {
        return Err(invalid("Invalid or sensitive schedule identity field"));
    }
    Ok(())
}
fn instant(s: &str) -> Result<i64> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.timestamp())
        .map_err(|_| invalid("Schedule instants must be RFC3339 with an offset"))
}
fn wall(s: &str) -> Result<NaiveTime> {
    if s.len() != 5 {
        return Err(invalid("Daily time must be HH:MM"));
    }
    NaiveTime::parse_from_str(s, "%H:%M").map_err(|_| invalid("Daily time must be HH:MM"))
}
fn connect(p: &Project) -> Result<Connection> {
    let mut db = work::connect(p)?;
    let initialized: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schedule_schema')",
        [],
        |r| r.get(0),
    )?;
    if initialized {
        let version: i64 = db.query_row("SELECT version FROM schedule_schema", [], |r| r.get(0))?;
        if version != 1 {
            return Err(Error::new(
                "DB_SCHEMA_TOO_NEW",
                "Unsupported schedule schema",
                7,
            ));
        }
        return Ok(db);
    }
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch("CREATE TABLE schedule_schema(version INTEGER NOT NULL);INSERT INTO schedule_schema VALUES(1);
CREATE TABLE schedule_definitions(namespace TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,workspace TEXT NOT NULL,definition TEXT NOT NULL,definition_hash TEXT NOT NULL,enabled INTEGER NOT NULL,pause_source TEXT,removed INTEGER NOT NULL DEFAULT 0,cursor_at INTEGER,last_success INTEGER,next_due INTEGER,created INTEGER NOT NULL,updated INTEGER NOT NULL,PRIMARY KEY(namespace,id));
CREATE TABLE schedule_occurrences(namespace TEXT NOT NULL,schedule TEXT NOT NULL,revision INTEGER NOT NULL,occurrence TEXT NOT NULL,due_at INTEGER NOT NULL,state TEXT NOT NULL,metadata TEXT NOT NULL,planned_at INTEGER NOT NULL,PRIMARY KEY(namespace,schedule,revision,occurrence));
CREATE TABLE schedule_receipts(key TEXT PRIMARY KEY,request_hash TEXT NOT NULL,response TEXT NOT NULL);
CREATE TABLE schedule_events(seq INTEGER PRIMARY KEY AUTOINCREMENT,namespace TEXT NOT NULL,schedule TEXT NOT NULL,kind TEXT NOT NULL,metadata TEXT NOT NULL,created INTEGER NOT NULL);
CREATE TRIGGER schedule_events_no_update BEFORE UPDATE ON schedule_events BEGIN SELECT RAISE(ABORT,'append-only schedule events');END;
CREATE TRIGGER schedule_events_no_delete BEFORE DELETE ON schedule_events BEGIN SELECT RAISE(ABORT,'append-only schedule events');END;")?;
    tx.commit()?;
    Ok(db)
}
fn validate(p: &Project, db: &Connection, d: &ScheduleDefinition) -> Result<()> {
    if d.schema_version != 1 {
        return Err(invalid("Unsupported schedule schema"));
    }
    for s in [&d.namespace, &d.id, &d.role, &d.recipient, &d.topic] {
        label(s)?;
    }
    d.timezone
        .parse::<Tz>()
        .map_err(|_| invalid("Timezone must be a supported IANA name"))?;
    instant(&d.valid_from)?;
    match &d.cadence {
        Cadence::Daily { at } => {
            wall(at)?;
        }
        Cadence::Interval { seconds, anchor } => {
            if !(60..=31536000).contains(seconds) {
                return Err(invalid("Elapsed interval must be 60..31536000 seconds"));
            }
            instant(anchor)?;
        }
    }
    if !["read_query", "queue_digest", "backup", "external_action"].contains(&d.job.as_str())
        || !["manual", "managed"].contains(&d.bridge.as_str())
        || d.misfire != "coalesce_latest"
    {
        return Err(invalid("Unsupported job/bridge/misfire policy"));
    }
    if d.session_id.is_some() != d.context_epoch.is_some() {
        return Err(invalid("Session schedules require exact context_epoch"));
    }
    if let Some(s) = &d.session_id {
        let exists: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='pctx_sessions')",
            [],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(Error::new(
                "SESSION_NOT_FOUND",
                "Session schedule references an unregistered session",
                6,
            ));
        }
        let bound: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM pctx_sessions WHERE id=?1 AND epoch=?2 AND workspace=?3)",
            params![s, d.context_epoch, p.workspace_id],
            |r| r.get(0),
        )?;
        if !bound {
            return Err(Error::new(
                "BASELINE_MISMATCH",
                "Schedule session epoch/workspace changed",
                9,
            ));
        }
    }
    if matches!(d.job.as_str(), "backup" | "external_action")
        && (d.action.is_none() || d.decision_id.is_none())
    {
        return Err(Error::new(
            "OWNER_DECISION_REQUIRED",
            "Non-read schedule intent requires a typed operation action and decision reference",
            5,
        ));
    }
    if let Some(action) = &d.action {
        if d.job == "backup" && action.kind != "backup"
            || d.job == "external_action"
                && ![
                    "network",
                    "external_api",
                    "external_publish",
                    "deploy",
                    "model_call",
                    "helper",
                ]
                .contains(&action.kind.as_str())
        {
            return Err(invalid(
                "Schedule job must match its typed operation action kind",
            ));
        }
        if matches!(d.job.as_str(), "read_query" | "queue_digest") && action.kind != d.job {
            return Err(invalid("Read-only schedule action kind differs from job"));
        }
        if action.argv.len() > 256
            || action.argv.iter().map(String::len).sum::<usize>() > 16384
            || action.amount.is_some_and(|v| v > 0) && action.currency.is_none()
        {
            return Err(invalid(
                "Schedule action requires bounded arguments and a currency for positive cost",
            ));
        }
        if action.schema_version != 1
            || action.role.as_deref() != Some(&d.role)
            || action.topic.as_deref() != Some(&d.topic)
            || action.actor != d.recipient
            || action.scope.is_empty()
            || !action.cost_known
            || action.amount.is_none()
        {
            return Err(invalid(
                "Schedule action must bind recipient, role, topic, scope and known cost",
            ));
        }
        if reader::redact(&serde_json::to_string(action)?).1 {
            return Err(Error::new(
                "POLICY_DENIED",
                "Sensitive schedule action denied",
                5,
            ));
        }
        if action.scope.iter().any(|s| {
            s.is_empty()
                || s.starts_with('/')
                || s.contains('\\')
                || s.split('/').any(|p| p == "..")
        }) {
            return Err(invalid("Schedule action scope must be normalized"));
        }
        if let Some(reason) = grant_reason(p, db, d, now())? {
            return Err(Error::new("OWNER_DECISION_REQUIRED", reason, 5));
        }
    }
    Ok(())
}
fn grant_reason(
    p: &Project,
    db: &Connection,
    d: &ScheduleDefinition,
    at: i64,
) -> Result<Option<String>> {
    if !matches!(d.job.as_str(), "backup" | "external_action") && d.action.is_none() {
        return Ok(None);
    }
    let Some(decision) = &d.decision_id else {
        return Ok(Some("missing_operation_decision".into()));
    };
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='ops_decisions')",
        [],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(Some("operation_decision_unavailable".into()));
    }
    type DecisionRow = (String, String, String, Option<i64>, Option<String>);
    let row: Option<DecisionRow> = db
        .query_row(
            "SELECT action,policy,state,expires_at,provenance FROM ops_decisions WHERE id=?1",
            [decision],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((stored, policy, state, expires, provenance)) = row else {
        return Ok(Some("operation_decision_missing".into()));
    };
    let action: Action = serde_json::from_str(&stored)?;
    if policy != p.policy_hash()
        || state != "approved"
        || !expires.is_some_and(|e| e > at)
        || provenance.as_deref() != Some("trusted_local_operator")
        || serde_json::to_value(&action)? != serde_json::to_value(&d.action)?
    {
        return Ok(Some("operation_decision_stale_or_mismatched".into()));
    }
    Ok(None)
}
fn policy_reasons(
    p: &Project,
    db: &Connection,
    d: &ScheduleDefinition,
    at: i64,
) -> Result<Vec<String>> {
    let mut reasons = Vec::new();
    let ops: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='ops_roles')",
        [],
        |r| r.get(0),
    )?;
    if ops {
        let paused: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM ops_roles WHERE paused=1 AND (role=?1 OR role='*'))",
            [&d.role],
            |r| r.get(0),
        )?;
        if paused {
            reasons.push("role_paused".into());
        }
        let silenced:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM ops_silences WHERE active=1 AND (role=?1 OR role='*') AND (recipient=?2 OR recipient='*') AND topic=?3)",params![d.role,d.recipient,d.topic],|r|r.get(0))?;
        if silenced {
            reasons.push("topic_silenced".into());
        }
    }
    if let Some(s) = &d.session_id {
        let sessions: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='pctx_sessions')",
            [],
            |r| r.get(0),
        )?;
        let valid = if sessions {
            db.query_row("SELECT EXISTS(SELECT 1 FROM pctx_sessions WHERE id=?1 AND epoch=?2 AND workspace=?3 AND status='active')",params![s,d.context_epoch,p.workspace_id],|r|r.get::<_,bool>(0))?
        } else {
            false
        };
        if !valid {
            reasons.push("session_binding_stale_or_suspended".into());
        }
    }
    if let Some(reason) = grant_reason(p, db, d, at)? {
        reasons.push(reason);
    }
    Ok(reasons)
}
fn daily_instant(zone: Tz, date: NaiveDate, time: NaiveTime) -> Result<i64> {
    let mut local = date.and_time(time);
    for _ in 0..=180 {
        match zone.from_local_datetime(&local) {
            LocalResult::Single(dt) => return Ok(dt.timestamp()),
            LocalResult::Ambiguous(a, b) => return Ok(a.min(b).timestamp()),
            LocalResult::None => {
                local += Duration::minutes(1);
                if local.date() != date {
                    break;
                }
            }
        }
    }
    Err(invalid("No valid wall time within the next-valid bound"))
}
/// Returns latest due logical occurrence, next due instant, and policy-visible DST behavior.
fn due(d: &ScheduleDefinition, at: i64) -> Result<(Option<(String, i64)>, i64)> {
    let start = instant(&d.valid_from)?;
    match &d.cadence {
        Cadence::Interval { seconds, anchor } => {
            let anchor = instant(anchor)?;
            let base = anchor.max(start);
            let offset = if base <= anchor {
                0
            } else {
                (base - anchor + seconds - 1) / seconds
            };
            let first = anchor
                .checked_add(
                    offset
                        .checked_mul(*seconds)
                        .ok_or_else(|| invalid("Interval overflow"))?,
                )
                .ok_or_else(|| invalid("Interval overflow"))?;
            if at < first {
                return Ok((None, first));
            }
            let bucket = (at - anchor) / seconds;
            let current = anchor + bucket * seconds;
            let next = current
                .checked_add(*seconds)
                .ok_or_else(|| invalid("Interval overflow"))?;
            Ok((Some((format!("instant:{current}"), current)), next))
        }
        Cadence::Daily { at: time } => {
            let zone = d
                .timezone
                .parse::<Tz>()
                .map_err(|_| invalid("Unknown timezone"))?;
            let now = DateTime::from_timestamp(at, 0)
                .ok_or_else(|| invalid("Invalid evaluation instant"))?
                .with_timezone(&zone);
            let time = wall(time)?;
            let today = now.date_naive();
            let current = daily_instant(zone, today, time)?;
            let date = if current <= at {
                today
            } else {
                today.pred_opt().ok_or_else(|| invalid("Date underflow"))?
            };
            let previous = daily_instant(zone, date, time)?;
            let next_date = if current > at {
                today
            } else {
                today.succ_opt().ok_or_else(|| invalid("Date overflow"))?
            };
            let mut next = daily_instant(zone, next_date, time)?;
            if next < start {
                let start_date = DateTime::from_timestamp(start, 0)
                    .ok_or_else(|| invalid("Invalid start"))?
                    .with_timezone(&zone)
                    .date_naive();
                next = daily_instant(zone, start_date, time)?;
                if next < start {
                    next = daily_instant(
                        zone,
                        start_date
                            .succ_opt()
                            .ok_or_else(|| invalid("Date overflow"))?,
                        time,
                    )?;
                }
            }
            Ok((
                if previous >= start {
                    Some((format!("local-date:{date}"), previous))
                } else {
                    None
                },
                next,
            ))
        }
    }
}
fn definition(path: &PathBuf) -> Result<ScheduleDefinition> {
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 65536 {
        return Err(invalid("Definition must be regular JSON at most 64 KiB"));
    }
    let value: ScheduleDefinition = serde_json::from_slice(&std::fs::read(path)?)?;
    Ok(value)
}
fn event(db: &Connection, d: &ScheduleDefinition, kind: &str, value: Value) -> Result<()> {
    db.execute("INSERT INTO schedule_events(namespace,schedule,kind,metadata,created) VALUES(?1,?2,?3,?4,?5)",params![d.namespace,d.id,kind,value.to_string(),now()])?;
    Ok(())
}
#[derive(Clone)]
struct Stored {
    definition: ScheduleDefinition,
    revision: i64,
    workspace: String,
    enabled: bool,
    pause: Option<String>,
    removed: bool,
    cursor: Option<i64>,
    success: Option<i64>,
    next: Option<i64>,
}
fn get(db: &Connection, namespace: &str, id: &str) -> Result<Stored> {
    db.query_row("SELECT definition,revision,workspace,enabled,pause_source,removed,cursor_at,last_success,next_due FROM schedule_definitions WHERE namespace=?1 AND id=?2",params![namespace,id],|r|Ok((r.get::<_,String>(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).optional()?.ok_or_else(||Error::new("SCHEDULE_NOT_FOUND","No schedule in the exact namespace",6)).and_then(|(definition,revision,workspace,enabled,pause,removed,cursor,success,next)|Ok(Stored{definition:serde_json::from_str(&definition)?,revision,workspace,enabled,pause,removed,cursor,success,next}))
}
fn reconcile(p: &Project, namespace: Option<&str>, at: i64) -> Result<Value> {
    owner()?;
    let mut db = connect(p)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let keys = {
        let mut s=tx.prepare("SELECT namespace,id FROM schedule_definitions WHERE (?1 IS NULL OR namespace=?1) AND removed=0 ORDER BY namespace,id")?;

        s.query_map([namespace], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };
    let mut rows = Vec::new();
    let mut created = 0;
    for (namespace, id) in keys {
        let stored = get(&tx, &namespace, &id)?;
        let d = &stored.definition;
        let (latest, next) = due(d, at)?;
        let mut reasons = policy_reasons(p, &tx, d, at.max(now()))?;
        if stored.cursor.is_some_and(|cursor| at < cursor) {
            reasons.push("clock_regression".into());
        }
        if !stored.enabled || stored.pause.is_some() {
            reasons.push("schedule_explicitly_paused_or_disabled".into());
        }
        if stored.workspace != p.workspace_id {
            reasons.push("workspace_requires_explicit_remap".into());
        }
        let mut occurrence = Value::Null;
        let mut reused = false;
        if let Some((key, when)) = latest {
            let baseline = stored
                .cursor
                .unwrap_or(instant(&d.valid_from)?.saturating_sub(1));
            let coalesced = due(d, when.saturating_sub(1))?
                .0
                .is_some_and(|(_, previous)| previous > baseline);
            let state = if reasons.is_empty() {
                "planned"
            } else {
                "blocked"
            };
            let metadata = json!({"namespace":namespace,"schedule_id":id,"definition_revision":stored.revision,"occurrence":key,"due_at":when,"generated_at":now(),"logical_evaluation_at":at,"state":state,"barriers":reasons,"bridge":d.bridge,"bridge_observed_state":if d.bridge=="manual"{"manual_plan_only"}else{"unknown"},"execution_started":false,"actual_success":false,"coalesced_older_occurrences":coalesced,"missed_count":null,"definition_hash":hash(serde_json::to_vec(d)?)});
            let existing:Option<(String,String)>=tx.query_row("SELECT state,metadata FROM schedule_occurrences WHERE namespace=?1 AND schedule=?2 AND revision=?3 AND occurrence=?4",params![namespace,id,stored.revision,key],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            if let Some((old_state, old_metadata)) = existing {
                reused = true;
                let old: Value = serde_json::from_str(&old_metadata)?;
                if old_state != state || old["barriers"] != metadata["barriers"] {
                    tx.execute("UPDATE schedule_occurrences SET state=?1,metadata=?2,planned_at=?3 WHERE namespace=?4 AND schedule=?5 AND revision=?6 AND occurrence=?7",params![state,metadata.to_string(),at,namespace,id,stored.revision,key])?;
                    event(&tx, d, "occurrence_revalidated", metadata.clone())?;
                    occurrence = metadata;
                } else {
                    occurrence = old;
                }
            } else {
                tx.execute(
                    "INSERT INTO schedule_occurrences VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![
                        namespace,
                        id,
                        stored.revision,
                        key,
                        when,
                        state,
                        metadata.to_string(),
                        at
                    ],
                )?;
                event(&tx, d, "occurrence_planned", metadata.clone())?;
                created += 1;
                occurrence = metadata;
            }
        }
        tx.execute("UPDATE schedule_definitions SET cursor_at=CASE WHEN cursor_at IS NULL OR cursor_at<?1 THEN ?1 ELSE cursor_at END,next_due=?2 WHERE namespace=?3 AND id=?4",params![at,next,namespace,id])?;
        rows.push(json!({"namespace":namespace,"schedule_id":id,"definition_revision":stored.revision,"timezone":d.timezone,"cadence":d.cadence,"enabled":stored.enabled,"pause_source":stored.pause,"role_topic_barriers":reasons,"occurrence":occurrence,"reused":reused,"next_due":next,"last_success":stored.success,"bridge_actual_state":"unknown","registration_performed":false,"execution_started":false,"model_calls":0,"external_messages":0}));
    }
    tx.commit()?;
    Ok(
        json!({"evaluated_at":at,"schedules":rows,"created_occurrences":created,"execution_started":false,"bridge_registration_performed":false,"model_calls":0,"external_messages":0,"missed_policy":"latest_only_no_synthetic_success","dst_policy":{"ambiguous":"earliest_once_per_local_date","nonexistent":"next_valid_minute_within_180_minutes"}}),
    )
}
pub fn execute(p: &Project, c: &ScheduleCommand) -> Result<Value> {
    match c {
        ScheduleCommand::Reconcile { namespace, at } => {
            if let Some(namespace) = namespace {
                label(namespace)?;
            }
            reconcile(
                p,
                namespace.as_deref(),
                at.as_ref()
                    .map(|s| instant(s))
                    .transpose()?
                    .unwrap_or_else(now),
            )
        }
        ScheduleCommand::List { namespace } => {
            if let Some(namespace) = namespace {
                label(namespace)?;
            }
            let db = connect(p)?;
            let mut stmt=db.prepare("SELECT namespace,id FROM schedule_definitions WHERE ?1 IS NULL OR namespace=?1 ORDER BY namespace,id")?;
            let keys = stmt
                .query_map([namespace], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut rows = Vec::new();
            for (namespace, id) in keys {
                let stored = get(&db, &namespace, &id)?;
                let d = &stored.definition;
                rows.push(json!({"namespace":namespace,"schedule_id":id,"definition_revision":stored.revision,"definition":d,"workspace_id":stored.workspace,"enabled":stored.enabled,"pause_source":stored.pause,"removed":stored.removed,"cursor_at":stored.cursor,"last_success":stored.success,"next_due":stored.next,"policy_barriers":policy_reasons(p,&db,d,now())?,"bridge_actual_state":"unknown","registration_performed":false,"execution_started":false}));
            }
            Ok(
                json!({"schedules":rows,"registry_authoritative":true,"bridge_actual_state":"unknown","execution_started":false}),
            )
        }
        ScheduleCommand::Add {
            from_file,
            idempotency_key,
        } => {
            owner()?;
            label(idempotency_key)?;
            let d = definition(from_file)?;
            let mut db = connect(p)?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            validate(p, &tx, &d)?;
            let payload = serde_json::to_string(&d)?;
            let request = hash(format!("{}:{payload}", p.workspace_id));
            if let Some((fingerprint, response)) = tx
                .query_row(
                    "SELECT request_hash,response FROM schedule_receipts WHERE key=?1",
                    [idempotency_key],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?
            {
                if fingerprint != request {
                    return Err(Error::new(
                        "IDEMPOTENCY_CONFLICT",
                        "Schedule idempotency key has another definition",
                        9,
                    ));
                }
                return Ok(serde_json::from_str(&response)?);
            }
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM schedule_definitions WHERE namespace=?1 AND id=?2)",
                params![d.namespace, d.id],
                |r| r.get(0),
            )?;
            if exists {
                return Err(conflict("Schedule already exists in the exact namespace"));
            }
            let (_, next) = due(&d, now())?;
            let pause = if d.enabled {
                None
            } else {
                Some("definition_disabled")
            };
            tx.execute("INSERT INTO schedule_definitions(namespace,id,revision,workspace,definition,definition_hash,enabled,pause_source,removed,cursor_at,last_success,next_due,created,updated) VALUES(?1,?2,1,?3,?4,?5,?6,?7,0,NULL,NULL,?8,?9,?9)",params![d.namespace,d.id,p.workspace_id,payload,hash(&payload),d.enabled,pause,next,now()])?;
            let value = json!({"namespace":d.namespace,"schedule_id":d.id,"definition_revision":1,"definition_hash":hash(&payload),"enabled":d.enabled,"pause_source":pause,"next_due":next,"bridge":d.bridge,"bridge_actual_state":"unknown","registration_performed":false,"execution_started":false});
            event(&tx, &d, "definition_added", value.clone())?;
            tx.execute(
                "INSERT INTO schedule_receipts VALUES(?1,?2,?3)",
                params![idempotency_key, request, value.to_string()],
            )?;
            tx.commit()?;
            Ok(value)
        }
        ScheduleCommand::Update {
            namespace,
            id,
            from_file,
            expect_revision,
        } => {
            owner()?;
            label(namespace)?;
            label(id)?;
            let d = definition(from_file)?;
            if d.namespace != *namespace || d.id != *id {
                return Err(invalid(
                    "Definition must match the explicit namespace and ID",
                ));
            }
            let mut db = connect(p)?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old = get(&tx, namespace, id)?;
            if old.revision != *expect_revision || old.removed {
                return Err(conflict(
                    "Definition revision changed or schedule was removed",
                ));
            }
            validate(p, &tx, &d)?;
            let payload = serde_json::to_string(&d)?;
            let effective_enabled = d.enabled && old.pause.is_none();
            let (_, next) = due(&d, now())?;
            tx.execute("UPDATE schedule_definitions SET revision=revision+1,workspace=?1,definition=?2,definition_hash=?3,enabled=?4,cursor_at=NULL,next_due=?5,updated=?6 WHERE namespace=?7 AND id=?8 AND revision=?9",params![p.workspace_id,payload,hash(&payload),effective_enabled,next,now(),namespace,id,expect_revision])?;
            let value = json!({"namespace":namespace,"schedule_id":id,"definition_revision":old.revision+1,"definition_hash":hash(&payload),"enabled":effective_enabled,"pause_source":old.pause,"bridge_actual_state":"unknown","registration_performed":false,"execution_started":false});
            event(&tx, &d, "definition_updated", value.clone())?;
            tx.commit()?;
            Ok(value)
        }
        ScheduleCommand::Pause {
            namespace,
            id,
            reason,
            expect_revision,
        }
        | ScheduleCommand::Resume {
            namespace,
            id,
            reason,
            expect_revision,
        } => {
            owner()?;
            label(namespace)?;
            label(id)?;
            if reason.trim().is_empty() || reason.len() > 2048 {
                return Err(invalid("Pause/resume needs a bounded reason"));
            }
            let mut db = connect(p)?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old = get(&tx, namespace, id)?;
            if old.revision != *expect_revision || old.removed {
                return Err(conflict(
                    "Definition revision changed or schedule was removed",
                ));
            }
            let pause = matches!(c, ScheduleCommand::Pause { .. });
            if !pause {
                validate(p, &tx, &old.definition)?;
            }
            let source = if pause {
                Some("explicit_owner_pause")
            } else {
                None
            };
            tx.execute("UPDATE schedule_definitions SET revision=revision+1,enabled=?1,pause_source=?2,updated=?3 WHERE namespace=?4 AND id=?5 AND revision=?6",params![!pause,source,now(),namespace,id,expect_revision])?;
            let value = json!({"namespace":namespace,"schedule_id":id,"definition_revision":old.revision+1,"enabled":!pause,"pause_source":source,"reason":reader::redact(reason).0,"role_topic_barriers":policy_reasons(p,&tx,&old.definition,now())?,"bridge_registration_performed":false,"execution_started":false});
            event(
                &tx,
                &old.definition,
                if pause {
                    "schedule_paused"
                } else {
                    "schedule_resumed"
                },
                value.clone(),
            )?;
            tx.commit()?;
            Ok(value)
        }
        ScheduleCommand::Remove {
            namespace,
            id,
            expect_revision,
        } => {
            owner()?;
            label(namespace)?;
            label(id)?;
            let mut db = connect(p)?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old = get(&tx, namespace, id)?;
            if old.revision != *expect_revision || old.removed {
                return Err(conflict(
                    "Definition revision changed or schedule already removed",
                ));
            }
            tx.execute("UPDATE schedule_definitions SET revision=revision+1,enabled=0,removed=1,pause_source='removed_by_owner',updated=?1 WHERE namespace=?2 AND id=?3 AND revision=?4",params![now(),namespace,id,expect_revision])?;
            let value = json!({"namespace":namespace,"schedule_id":id,"definition_revision":old.revision+1,"removed":true,"native_registration_removed":false,"bridge_actual_state":"unknown","execution_started":false});
            event(&tx, &old.definition, "schedule_removed", value.clone())?;
            tx.commit()?;
            Ok(value)
        }
    }
}

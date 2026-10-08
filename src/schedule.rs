//! Durable occurrences, bounded local execution, and explicitly reviewed managed bridges.
use crate::{
    domain::{Error, Result, hash, now},
    operations,
    operations::Action,
    output,
    project::Project,
    project::{atomic_write, private_dir},
    reader, work,
};
use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone};
use chrono_tz::Tz;
use clap::Subcommand;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
    time::{Duration as StdDuration, Instant},
};
#[derive(Debug, Clone, Subcommand)]
pub enum ScheduleCommand {
    /// Read-only exact managed bridge installation proposal.
    Plan {
        #[arg(long)]
        namespace: String,
        id: String,
        #[arg(long, default_value="fixture", value_parser=["fixture","launchd","systemd"])]
        provider: String,
        #[arg(long, default_value = ".pctx/schedule-bridge")]
        staging_root: String,
    },
    /// Install only the reviewed plan. Native OS effects require explicit opt-in.
    Install {
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        expect_hash: String,
        #[arg(long)]
        apply_native: bool,
    },
    Inspect {
        #[arg(long)]
        namespace: String,
        id: String,
        #[arg(long)]
        observe_native: bool,
    },
    /// Resolve only an in-process attempt whose original process is observed absent.
    Recover {
        #[arg(long)]
        namespace: String,
        id: String,
        #[arg(long)]
        revision: i64,
        #[arg(long)]
        occurrence: String,
        #[arg(long)]
        attempt: i64,
        #[arg(long)]
        reason: String,
    },
    Uninstall {
        #[arg(long)]
        namespace: String,
        id: String,
        #[arg(long)]
        expect_hash: String,
        #[arg(long)]
        apply_native: bool,
    },
    /// Execute latest due bounded local jobs; failed finished attempts require explicit retry.
    Tick {
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        retry_failed: bool,
    },
    /// Explicit finite local scheduler lifetime, without model calls.
    RunLoop {
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long, default_value_t = 60)]
        interval_seconds: u64,
        #[arg(long, default_value_t = 1)]
        max_ticks: u32,
        #[arg(long, default_value_t = 300)]
        ttl_seconds: u64,
        #[arg(long)]
        keep_awake: bool,
        #[arg(long)]
        purpose: Option<String>,
    },
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
        if version != 1 && version != 2 {
            return Err(Error::new(
                "DB_SCHEMA_TOO_NEW",
                "Unsupported schedule schema",
                7,
            ));
        }
        if version == 1 {
            upgrade(&mut db)?;
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
    upgrade(&mut db)?;
    Ok(db)
}
fn upgrade(db: &mut Connection) -> Result<()> {
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS schedule_bindings(namespace TEXT NOT NULL,schedule TEXT NOT NULL,revision INTEGER NOT NULL,workspace TEXT NOT NULL,policy TEXT NOT NULL,fingerprint TEXT NOT NULL,profile TEXT NOT NULL,created INTEGER NOT NULL,PRIMARY KEY(namespace,schedule));
CREATE TABLE IF NOT EXISTS schedule_runs(namespace TEXT NOT NULL,schedule TEXT NOT NULL,revision INTEGER NOT NULL,occurrence TEXT NOT NULL,attempt INTEGER NOT NULL,state TEXT NOT NULL,output_ref TEXT,result_hash TEXT,result TEXT NOT NULL,delivery_ref TEXT,error TEXT,started INTEGER NOT NULL,finished INTEGER,PRIMARY KEY(namespace,schedule,revision,occurrence,attempt));
CREATE UNIQUE INDEX IF NOT EXISTS schedule_one_active ON schedule_runs(namespace,schedule) WHERE state='running';
CREATE TABLE IF NOT EXISTS schedule_installations(namespace TEXT NOT NULL,schedule TEXT NOT NULL,workspace TEXT NOT NULL,provider TEXT NOT NULL,plan_hash TEXT NOT NULL,manifest_hash TEXT NOT NULL,state TEXT NOT NULL,metadata TEXT NOT NULL,updated INTEGER NOT NULL,PRIMARY KEY(namespace,schedule));
UPDATE schedule_schema SET version=2;")?;
    tx.commit()?;
    Ok(())
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
        let mut s=tx.prepare("SELECT namespace,id FROM schedule_definitions WHERE (?1 IS NULL OR namespace=?1) AND removed=0 ORDER BY namespace,id LIMIT 1001")?;

        s.query_map([namespace], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };
    if keys.len() > 1000 {
        return Err(Error::new(
            "BUDGET_EXHAUSTED",
            "Tick namespace exceeds 1000 schedule bound; select a narrower namespace",
            10,
        ));
    }
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
                if !["running", "succeeded", "failed", "interrupted_unknown"]
                    .contains(&old_state.as_str())
                    && (old_state != state || old["barriers"] != metadata["barriers"])
                {
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
        ScheduleCommand::Plan {
            namespace,
            id,
            provider,
            staging_root,
        } => plan(p, namespace, id, provider, staging_root),
        ScheduleCommand::Install {
            from_file,
            expect_hash,
            apply_native,
        } => install(p, from_file, expect_hash, *apply_native),
        ScheduleCommand::Inspect {
            namespace,
            id,
            observe_native,
        } => inspect(p, namespace, id, *observe_native),
        ScheduleCommand::Recover {
            namespace,
            id,
            revision,
            occurrence,
            attempt,
            reason,
        } => recover(p, namespace, id, *revision, occurrence, *attempt, reason),
        ScheduleCommand::Uninstall {
            namespace,
            id,
            expect_hash,
            apply_native,
        } => uninstall(p, namespace, id, expect_hash, *apply_native),
        ScheduleCommand::Tick {
            namespace,
            at,
            retry_failed,
        } => tick(
            p,
            namespace.as_deref(),
            at.as_ref()
                .map(|s| instant(s))
                .transpose()?
                .unwrap_or_else(now),
            *retry_failed,
        ),
        ScheduleCommand::RunLoop {
            namespace,
            interval_seconds,
            max_ticks,
            ttl_seconds,
            keep_awake,
            purpose,
        } => run_loop(
            p,
            namespace.as_deref(),
            *interval_seconds,
            *max_ticks,
            *ttl_seconds,
            *keep_awake,
            purpose.as_deref(),
        ),
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

fn current_project_policy(p: &Project) -> Result<()> {
    if fs::symlink_metadata(p.root.join(".pctx/config.toml")).is_ok() {
        let actual = Project::open(&p.root)?;
        if actual.policy_hash() != p.policy_hash()
            || actual.workspace_id != p.workspace_id
            || actual.coordination_id != p.coordination_id
        {
            return Err(Error::new(
                "CONFIG_CHANGED",
                "Live project policy or workspace binding changed; restart with current configuration",
                9,
            ));
        }
    }
    Ok(())
}
fn barriers(p: &Project, db: &Connection, s: &Stored, revision: i64) -> Result<()> {
    owner()?;
    current_project_policy(p)?;
    if s.revision != revision
        || s.removed
        || !s.enabled
        || s.pause.is_some()
        || s.workspace != p.workspace_id
    {
        return Err(Error::new(
            "CONFIG_CHANGED",
            "Schedule revision, pause or workspace changed",
            9,
        ));
    }
    let reasons = policy_reasons(p, db, &s.definition, now())?;
    if !reasons.is_empty() {
        return Err(Error::new("POLICY_DENIED", reasons.join(","), 5));
    }
    Ok(())
}
fn runtime_identity() -> Result<Value> {
    let path = std::env::current_exe()?.canonicalize()?;
    let meta = fs::symlink_metadata(&path)?;
    if !meta.is_file() || meta.len() > 256 * 1024 * 1024 {
        return Err(invalid("Runtime executable identity unavailable"));
    }
    Ok(json!({"executable":path,"hash":hash(fs::read(path)?),"version":env!("CARGO_PKG_VERSION")}))
}
fn relative_root(root: &str) -> Result<()> {
    if !root.starts_with(".pctx/")
        || root.len() > 1024
        || Path::new(root)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(invalid(
            "Bridge staging root must be an explicit relative .pctx subdirectory",
        ));
    }
    Ok(())
}
/// Check each component before any directory creation. Never chmod or follow a user link.
fn stage_path(p: &Project, root: &str, create: bool) -> Result<PathBuf> {
    relative_root(root)?;
    let mut path = p.root.canonicalize()?;
    for component in Path::new(root).components() {
        path.push(component.as_os_str());
        match fs::symlink_metadata(&path) {
            Ok(m) if m.file_type().is_symlink() || !m.is_dir() => {
                return Err(Error::new(
                    "POLICY_DENIED",
                    "Linked or non-directory bridge staging component denied",
                    5,
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && create => {
                fs::create_dir(&path)?;
                private_dir(&path)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(path)
}
fn escaped_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn systemd_quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
    )
}
fn user_id() -> u32 {
    #[cfg(unix)]
    {
        unsafe { libc::getuid() }
    }
    #[cfg(not(unix))]
    {
        0
    }
}
fn native_runtime(provider: &str) -> Result<Value> {
    let path = match provider {
        "launchd" => "/bin/launchctl",
        "systemd" => "/usr/bin/systemctl",
        _ => return Ok(Value::Null),
    };
    if provider == "launchd" && !cfg!(target_os = "macos")
        || provider == "systemd" && !cfg!(target_os = "linux")
    {
        return Err(Error::new(
            "CAPABILITY_UNVERIFIED",
            "Bridge provider does not match this host OS",
            6,
        ));
    }
    let canonical = Path::new(path).canonicalize()?;
    Ok(json!({"executable":canonical,"hash":hash(fs::read(canonical)?)}))
}
fn environment(p: &Project) -> BTreeMap<String, String> {
    let env = BTreeMap::from([
        (
            "PCTX_DATA_DIR".into(),
            p.data_dir.to_string_lossy().into_owned(),
        ),
        ("PCTX_ACTOR".into(), "owner".into()),
    ]);
    #[cfg(target_os = "linux")]
    {
        // Native user systemd requires the explicit local user bus, never credentials.
        if let Ok(v) = std::env::var("XDG_RUNTIME_DIR") {
            if v == format!("/run/user/{}", user_id()) {
                env.insert("XDG_RUNTIME_DIR".into(), v);
            }
        }
    }
    env
}
fn plan(p: &Project, namespace: &str, id: &str, provider: &str, root: &str) -> Result<Value> {
    owner()?;
    current_project_policy(p)?;
    label(namespace)?;
    label(id)?;
    relative_root(root)?;
    stage_path(p, root, false)?;
    if !["fixture", "launchd", "systemd"].contains(&provider) {
        return Err(invalid("Unknown managed bridge provider"));
    }
    let db = connect(p)?;
    let s = get(&db, namespace, id)?;
    if s.removed || s.workspace != p.workspace_id {
        return Err(conflict("Removed or unmapped schedule cannot be installed"));
    }
    let runtime = runtime_identity()?;
    let label = format!(
        "org.pctx.schedule.{}",
        &hash(format!("{}:{namespace}:{id}", p.workspace_id))[..24]
    );
    let argv = vec![
        runtime["executable"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        "--format".into(),
        "json".into(),
        "schedule".into(),
        "tick".into(),
        "--namespace".into(),
        namespace.into(),
    ];
    let env = environment(p);
    let mut files = BTreeMap::new();
    match provider {
        "fixture" => {
            files.insert(format!("{label}.json"),json!({"schema_version":1,"argv":argv,"cwd":p.root,"environment":env,"invocation":"explicit_tick_or_bounded_run_loop","registration":"staged_only"}).to_string());
        }
        "launchd" => {
            let args = argv
                .iter()
                .map(|a| format!("<string>{}</string>", escaped_xml(a)))
                .collect::<String>();
            let vars = env
                .iter()
                .map(|(k, v)| {
                    format!(
                        "<key>{}</key><string>{}</string>",
                        escaped_xml(k),
                        escaped_xml(v)
                    )
                })
                .collect::<String>();
            files.insert(format!("{label}.plist"),format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>Label</key><string>{label}</string><key>ProgramArguments</key><array>{args}</array><key>WorkingDirectory</key><string>{}</string><key>EnvironmentVariables</key><dict>{vars}</dict><key>StartInterval</key><integer>60</integer></dict></plist>\n",escaped_xml(&p.root.to_string_lossy())));
        }
        "systemd" => {
            let args = argv
                .iter()
                .map(|a| systemd_quote(a))
                .collect::<Vec<_>>()
                .join(" ");
            let vars = env
                .iter()
                .map(|(k, v)| format!("Environment={}\n", systemd_quote(&format!("{k}={v}"))))
                .collect::<String>();
            files.insert(format!("{label}.service"),format!("[Unit]\nDescription=PCTX managed local tick\n[Service]\nType=oneshot\nWorkingDirectory={}\n{vars}ExecStart={args}\n",systemd_quote(&p.root.to_string_lossy())));
            files.insert(format!("{label}.timer"),format!("[Unit]\nDescription=PCTX managed local tick timer\n[Timer]\nOnActiveSec=60s\nOnUnitInactiveSec=60s\nUnit={label}.service\n[Install]\nWantedBy=timers.target\n"));
        }
        _ => unreachable!(),
    }
    let stage = p.root.canonicalize()?.join(root);
    let domain = format!("gui/{}", user_id());
    let native_commands = match provider {
        "launchd" => {
            json!({"install":[["bootstrap",domain,stage.join(format!("{label}.plist"))]],"inspect":["print",format!("gui/{}/{label}",user_id())],"remove":["bootout",format!("gui/{}/{label}",user_id())]})
        }
        "systemd" => {
            json!({"install":[["--user","link",stage.join(format!("{label}.service")),stage.join(format!("{label}.timer"))],["--user","start",format!("{label}.timer")]],"inspect":["--user","is-active",format!("{label}.timer")],"remove":["--user","disable","--now",format!("{label}.timer")]})
        }
        _ => Value::Null,
    };
    let mut proposal = json!({"schema_version":1,"namespace":namespace,"schedule_id":id,"revision":s.revision,"definition_hash":hash(serde_json::to_vec(&s.definition)?),"workspace":p.workspace_id,"coordination":p.coordination_id,"policy":p.policy_hash(),"provider":provider,"staging_root":root,"label":label,"runtime":runtime,"native_runtime":native_runtime(provider)?,"native_commands":native_commands,"cwd":p.root,"environment":env,"files":files,"argv":argv,"native_registration_requires_explicit_opt_in":true});
    let digest = hash(serde_json::to_vec(&proposal)?);
    proposal["plan_hash"] = json!(digest);
    Ok(proposal)
}
fn read_plan(path: &Path) -> Result<Value> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 256 * 1024 {
        return Err(invalid("Reviewed plan must be bounded regular JSON"));
    }
    let value: Value = serde_json::from_slice(&fs::read(path)?)?;
    if !value.is_object() || reader::redact(&value.to_string()).1 {
        return Err(invalid("Invalid or sensitive bridge plan"));
    }
    Ok(value)
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| invalid("Malformed managed bridge plan"))
}
fn exact_plan(p: &Project, v: &Value, expected: &str) -> Result<Value> {
    let fresh = plan(
        p,
        string(v, "namespace")?,
        string(v, "schedule_id")?,
        string(v, "provider")?,
        string(v, "staging_root")?,
    )?;
    if fresh != *v || string(&fresh, "plan_hash")? != expected {
        return Err(Error::new(
            "CONFIG_CHANGED",
            "Reviewed bridge definition, runtime, environment or files changed",
            9,
        ));
    }
    Ok(fresh)
}
fn verify_files(p: &Project, v: &Value) -> Result<bool> {
    let root = stage_path(p, string(v, "staging_root")?, false)?;
    for (name, body) in v["files"]
        .as_object()
        .ok_or_else(|| invalid("Missing bridge files"))?
    {
        if Path::new(name).components().count() != 1 {
            return Err(invalid("Invalid managed filename"));
        }
        let path = root.join(name);
        let Ok(meta) = fs::symlink_metadata(&path) else {
            return Ok(false);
        };
        if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > 256 * 1024 {
            return Err(Error::new(
                "POLICY_DENIED",
                "Linked or oversized managed manifest denied",
                5,
            ));
        }
        if fs::read(path)? != string_body(body)?.as_bytes() {
            return Ok(false);
        }
    }
    Ok(true)
}
fn string_body(v: &Value) -> Result<&str> {
    v.as_str()
        .ok_or_else(|| invalid("Managed file body must be text"))
}
fn native_call(p: &Project, v: &Value, args: Vec<String>) -> Result<Value> {
    let installing = args.first().is_some_and(|s| s == "bootstrap")
        || args.iter().any(|s| ["link", "start"].contains(&s.as_str()));
    let executable = string(&v["native_runtime"], "executable")?;
    let mut argv = vec![executable.to_owned()];
    argv.extend(args);
    let binding = output::registered_binding_at(p, &argv, ".")?;
    if binding["executable_hash"] != v["native_runtime"]["hash"] {
        return Err(Error::new(
            "CONFIG_CHANGED",
            "Native bridge executable changed",
            9,
        ));
    }
    let request = output::RunRequest {
        task_id: None,
        session: None,
        retain: "temporary".into(),
        execution_timeout_ms: Some(15000),
        budget_bytes: 16384,
        exit_policy: "child".into(),
        stdin: "closed".into(),
        argv,
    };
    let check = || -> Result<()> {
        exact_plan(p, v, string(v, "plan_hash")?)?;
        if installing {
            let db = connect(p)?;
            let s = get(&db, string(v, "namespace")?, string(v, "schedule_id")?)?;
            barriers(p, &db, &s, s.revision)?;
        }
        if !verify_files(p, v)? {
            return Err(conflict("Owned bridge manifests changed"));
        }
        Ok(())
    };
    output::run_registered_monitored(
        p,
        &request,
        ".",
        &environment(p),
        string(&binding, "fingerprint")?,
        &mut |_| check(),
        &mut || check(),
    )
}
fn succeeded(v: &Value) -> bool {
    v["spawned"] == true
        && v["child_exit_code"] == 0
        && v["termination"] == "exited"
        && v["capture_complete"] == true
        && v["pctx_error"].is_null()
}
fn native_observation(p: &Project, v: &Value) -> Result<Value> {
    let label = string(v, "label")?;
    let args = match string(v, "provider")? {
        "launchd" => vec!["print".into(), format!("gui/{}/{label}", user_id())],
        "systemd" => vec![
            "--user".into(),
            "is-active".into(),
            format!("{label}.timer"),
        ],
        _ => {
            return Ok(
                json!({"registered":false,"state":"staged","observation":"fixture_manifest_only"}),
            );
        }
    };
    let receipt = native_call(p, v, args)?;
    // Exit success for `print` proves registration; is-active proves active timer.
    Ok(
        json!({"registered":succeeded(&receipt),"state":if succeeded(&receipt){"registered"}else{"registration_unknown"},"receipt":receipt}),
    )
}
fn install(p: &Project, path: &Path, expected: &str, apply_native: bool) -> Result<Value> {
    owner()?;
    let v = exact_plan(p, &read_plan(path)?, expected)?;
    let namespace = string(&v, "namespace")?;
    let id = string(&v, "schedule_id")?;
    if apply_native && string(&v, "provider")? == "fixture" {
        return Err(invalid("Fixture bridge never registers with the OS"));
    }
    let root = stage_path(p, string(&v, "staging_root")?, true)?;
    for (name, body) in v["files"].as_object().unwrap() {
        let path = root.join(name);
        if fs::symlink_metadata(&path).is_ok() {
            let meta = fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink()
                || !meta.is_file()
                || fs::read(&path)? != string_body(body)?.as_bytes()
            {
                return Err(conflict(
                    "Existing managed files differ; user changes preserved",
                ));
            }
        } else {
            atomic_write(&path, string_body(body)?.as_bytes(), false)?;
        }
    }
    let mut db = connect(p)?;
    {
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let s = get(&tx, namespace, id)?;
        if s.revision != v["revision"].as_i64().unwrap_or(-1) {
            return Err(conflict("Definition changed during staging"));
        }
        tx.execute("INSERT INTO schedule_bindings VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(namespace,schedule) DO UPDATE SET revision=excluded.revision,workspace=excluded.workspace,policy=excluded.policy,fingerprint=excluded.fingerprint,profile=excluded.profile,created=excluded.created",params![namespace,id,s.revision,p.workspace_id,p.policy_hash(),expected,v.to_string(),now()])?;
        tx.execute("INSERT INTO schedule_installations VALUES(?1,?2,?3,?4,?5,?6,'staged',?7,?8) ON CONFLICT(namespace,schedule) DO UPDATE SET workspace=excluded.workspace,provider=excluded.provider,plan_hash=excluded.plan_hash,manifest_hash=excluded.manifest_hash,state='staged',metadata=excluded.metadata,updated=excluded.updated",params![namespace,id,p.workspace_id,string(&v,"provider")?,expected,hash(v["files"].to_string()),v.to_string(),now()])?;
        event(
            &tx,
            &s.definition,
            "bridge_staged",
            json!({"plan_hash":expected,"provider":v["provider"],"native_effect":false}),
        )?;
        tx.commit()?;
    }
    let mut native_receipts = Vec::new();
    if apply_native {
        let s = get(&db, namespace, id)?;
        barriers(p, &db, &s, s.revision)?;
        db.execute("UPDATE schedule_installations SET state='registration_unknown' WHERE namespace=?1 AND schedule=?2",params![namespace,id])?;
        let label = string(&v, "label")?;
        let commands = match string(&v, "provider")? {
            "launchd" => vec![vec![
                "bootstrap".into(),
                format!("gui/{}", user_id()),
                root.join(format!("{label}.plist"))
                    .to_string_lossy()
                    .into_owned(),
            ]],
            "systemd" => vec![
                vec![
                    "--user".into(),
                    "link".into(),
                    root.join(format!("{label}.service"))
                        .to_string_lossy()
                        .into_owned(),
                    root.join(format!("{label}.timer"))
                        .to_string_lossy()
                        .into_owned(),
                ],
                vec!["--user".into(), "start".into(), format!("{label}.timer")],
            ],
            _ => unreachable!(),
        };
        for command in commands {
            let r = native_call(p, &v, command)?;
            let ok = succeeded(&r);
            native_receipts.push(r);
            if !ok {
                break;
            }
        }
    }
    let mut observed = if apply_native {
        native_observation(p, &v)?
    } else {
        json!({"registered":false,"state":"staged","observation":"manifest_only"})
    };
    if apply_native && !native_receipts.iter().all(succeeded) {
        observed["registered"] = json!(false);
        observed["state"] = json!("registration_unknown");
        observed["ownership"] = json!("native_installation_not_acknowledged");
    }
    db.execute("UPDATE schedule_installations SET state=?1,metadata=?2,updated=?3 WHERE namespace=?4 AND schedule=?5",params![string(&observed,"state")?,json!({"plan":v,"observation":observed,"registration_receipts":native_receipts}).to_string(),now(),namespace,id])?;
    Ok(
        json!({"namespace":namespace,"schedule_id":id,"plan_hash":expected,"installation":observed,"registration_receipts":native_receipts,"model_calls":0,"external_messages":0}),
    )
}
fn installed_plan(db: &Connection, namespace: &str, id: &str) -> Result<(String, Value)> {
    let (state, metadata): (String, String) = db
        .query_row(
            "SELECT state,metadata FROM schedule_installations WHERE namespace=?1 AND schedule=?2",
            params![namespace, id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| {
            Error::new(
                "CAPABILITY_UNVERIFIED",
                "No reviewed local bridge installation",
                6,
            )
        })?;
    let value: Value = serde_json::from_str(&metadata)?;
    Ok((state, value.get("plan").cloned().unwrap_or(value)))
}
fn inspect(p: &Project, namespace: &str, id: &str, observe_native: bool) -> Result<Value> {
    let db = connect(p)?;
    let (state, v) = installed_plan(&db, namespace, id)?;
    if state == "unknown_restored" {
        return Ok(
            json!({"namespace":namespace,"schedule_id":id,"stored_state":state,"owned_files_match":null,"current_binding":false,"registration_observation":"unknown_restored","registration_performed":false,"requires_reapproval":true,"plan_hash":v["plan_hash"]}),
        );
    }
    let present = verify_files(p, &v)?;
    let current = exact_plan(p, &v, string(&v, "plan_hash")?).is_ok();
    let observation = if observe_native && state != "unknown_restored" && present && current {
        Some(native_observation(p, &v)?)
    } else {
        None
    };
    Ok(
        json!({"native_observation":observation,"namespace":namespace,"schedule_id":id,"stored_state":state,"owned_files_match":present,"current_binding":current,"registration_observation":if state=="unknown_restored"{"unknown_restored"}else if state=="registered"{"historical_receipt_requires_fresh_native_inspection"}else{"staged_not_registered"},"registration_performed":false,"plan_hash":v["plan_hash"]}),
    )
}
fn uninstall(
    p: &Project,
    namespace: &str,
    id: &str,
    expected: &str,
    apply_native: bool,
) -> Result<Value> {
    owner()?;
    let db = connect(p)?;
    let (state, v) = installed_plan(&db, namespace, id)?;
    if state == "unknown_restored" {
        return Err(Error::new(
            "CAPABILITY_UNVERIFIED",
            "Restored installation does not carry native removal authority",
            6,
        ));
    }
    if string(&v, "plan_hash")? != expected || !verify_files(p, &v)? {
        return Err(conflict(
            "Removal plan or owned files changed; user modifications retained",
        ));
    }
    if !["staged", "removed"].contains(&state.as_str()) && !apply_native {
        return Err(invalid(
            "Native registration must be removed explicitly before owned manifests",
        ));
    }
    let receipt = if apply_native {
        exact_plan(p, &v, expected)?;
        let label = string(&v, "label")?;
        let args = match string(&v, "provider")? {
            "launchd" => vec!["bootout".into(), format!("gui/{}/{label}", user_id())],
            "systemd" => vec![
                "--user".into(),
                "disable".into(),
                "--now".into(),
                format!("{label}.timer"),
            ],
            _ => return Err(invalid("Fixture has no native registration")),
        };
        let result = native_call(p, &v, args)?;
        if !succeeded(&result) {
            return Ok(
                json!({"removed":false,"state":"native_removal_unconfirmed","receipt":result}),
            );
        }
        Some(result)
    } else {
        None
    };
    let root = stage_path(p, string(&v, "staging_root")?, false)?;
    for (name, body) in v["files"].as_object().unwrap() {
        let path = root.join(name);
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() || fs::read(&path)? != string_body(body)?.as_bytes() {
            return Err(conflict("Managed manifest changed during removal"));
        }
        fs::remove_file(path)?;
    }
    db.execute("UPDATE schedule_installations SET state='removed',updated=?1 WHERE namespace=?2 AND schedule=?3",params![now(),namespace,id])?;
    db.execute(
        "DELETE FROM schedule_bindings WHERE namespace=?1 AND schedule=?2",
        params![namespace, id],
    )?;
    Ok(json!({"removed":true,"receipt":receipt,"only_managed_files":true}))
}

fn execution_binding(p: &Project, db: &Connection, s: &Stored) -> Result<()> {
    if s.definition.bridge == "manual" {
        return Ok(());
    }
    let row:Option<(i64,String,String,String)>=db.query_row("SELECT revision,workspace,policy,profile FROM schedule_bindings WHERE namespace=?1 AND schedule=?2",params![s.definition.namespace,s.definition.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    let Some((revision, workspace, policy, profile)) = row else {
        return Err(Error::new(
            "CAPABILITY_UNVERIFIED",
            "Managed tick requires a reviewed current bridge binding",
            6,
        ));
    };
    let v: Value = serde_json::from_str(&profile)?;
    if revision != s.revision
        || workspace != p.workspace_id
        || policy != p.policy_hash()
        || exact_plan(p, &v, string(&v, "plan_hash")?).is_err()
        || !verify_files(p, &v)?
    {
        return Err(Error::new(
            "CONFIG_CHANGED",
            "Managed bridge binding or manifest changed",
            9,
        ));
    }
    Ok(())
}
fn local_job(p: &Project, d: &ScheduleDefinition, occurrence: &str) -> Result<Value> {
    match d.job.as_str() {
        "read_query" => work::board(p),
        "queue_digest" => operations::execute(
            p,
            &operations::OperationCommand::Owner {
                command: operations::OwnerCommand::Queue { limit: 5 },
            },
        ),
        "backup" => {
            let action = d
                .action
                .as_ref()
                .ok_or_else(|| invalid("Backup action missing"))?;
            // The grant authorizes this exact private backup scope, never arbitrary argv or targets.
            let resource = format!("schedule/{}/{}", d.namespace, d.id);
            if action.resource != resource
                || action.scope != vec![resource.clone()]
                || action.environment != "local"
                || action.amount != Some(0)
                || !action.argv.is_empty()
            {
                return Err(Error::new(
                    "OWNER_DECISION_REQUIRED",
                    "Backup grant must bind exact local schedule resource, zero cost and scope",
                    5,
                ));
            }
            let root = stage_path(p, ".pctx/scheduled-backups", true)?;
            let target = root.join(format!(
                "{}.json",
                hash(format!("{}:{}:{occurrence}", d.namespace, d.id))
            ));
            let v = work::execute(
                p,
                &work::WorkCommand::Control {
                    command: work::ControlCommand::Backup { output: target },
                },
            )?;
            // Portable results expose an opaque owned artifact reference, not private absolute paths.
            Ok(
                json!({"backup_created":true,"artifact_ref":format!("scheduled-backup:{}",hash(format!("{}:{}:{occurrence}",d.namespace,d.id))),"snapshot_metadata":v.get("schema_version")}),
            )
        }
        "external_action" => Err(Error::new(
            "CAPABILITY_UNVERIFIED",
            "External actions require an observed transport/provider; tick does not launch model calls or publish",
            6,
        )),
        _ => Err(invalid("Unsupported schedule execution job")),
    }
}
fn tick(p: &Project, namespace: Option<&str>, at: i64, retry: bool) -> Result<Value> {
    owner()?;
    if at > now().saturating_add(1) {
        return Err(invalid("Tick cannot execute a future logical occurrence"));
    }
    if let Some(n) = namespace {
        label(n)?;
    }
    operations::prepare_scheduled_mailbox(p)?;
    let evaluated = reconcile(p, namespace, at)?;
    let mut results = Vec::new();
    for row in evaluated["schedules"]
        .as_array()
        .ok_or_else(|| invalid("Invalid occurrence plan"))?
    {
        let occurrence = &row["occurrence"];
        if occurrence.is_null() {
            continue;
        }
        let n = string(row, "namespace")?;
        let id = string(row, "schedule_id")?;
        let key = string(occurrence, "occurrence")?;
        let mut db = connect(p)?;
        let (s, attempt) = {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let s = get(&tx, n, id)?;
            let revision = row["definition_revision"].as_i64().unwrap_or(-1);
            let gate = barriers(p, &tx, &s, revision).and_then(|_| execution_binding(p, &tx, &s));
            if let Err(e) = gate {
                results.push(json!({"namespace":n,"schedule_id":id,"occurrence":key,"state":"blocked","error":e.code,"execution_started":false}));
                continue;
            }
            let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM schedule_runs WHERE namespace=?1 AND schedule=?2 AND state IN ('running','interrupted_unknown'))",params![n,id],|r|r.get(0))?;
            if active {
                results.push(json!({"namespace":n,"schedule_id":id,"state":"execution_owner_unknown_or_busy","execution_started":false}));
                continue;
            }
            let previous:Option<(i64,String)>=tx.query_row("SELECT attempt,state FROM schedule_runs WHERE namespace=?1 AND schedule=?2 AND revision=?3 AND occurrence=?4 ORDER BY attempt DESC LIMIT 1",params![n,id,s.revision,key],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            if previous
                .as_ref()
                .is_some_and(|(_, state)| state == "succeeded" || !retry)
            {
                results.push(json!({"namespace":n,"schedule_id":id,"occurrence":key,"state":"already_recorded","execution_started":false}));
                continue;
            }
            let attempt = previous.map_or(1, |(a, _)| a + 1);
            tx.execute("INSERT INTO schedule_runs(namespace,schedule,revision,occurrence,attempt,state,result,started) VALUES(?1,?2,?3,?4,?5,'running',?6,?7)",params![n,id,s.revision,key,attempt,json!({"owner_pid":std::process::id(),"workspace":p.workspace_id,"execution_kind":"in_process"}).to_string(),now()])?;
            tx.execute("UPDATE schedule_occurrences SET state='running' WHERE namespace=?1 AND schedule=?2 AND revision=?3 AND occurrence=?4",params![n,id,s.revision,key])?;
            event(
                &tx,
                &s.definition,
                "execution_claimed",
                json!({"occurrence":key,"attempt":attempt,"revision":s.revision}),
            )?;
            tx.commit()?;
            (s, attempt)
        };
        // Claim is durable and committed before opening application modules/backup connections.
        // One final admission immediately before the bounded in-process action.
        let admitted = get(&db, n, id).and_then(|current| {
            barriers(p, &db, &current, s.revision).and_then(|_| execution_binding(p, &db, &current))
        });
        let execution_started = admitted.is_ok();
        let outcome = admitted.and_then(|_| local_job(p, &s.definition, key));
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = get(&tx, n, id)?;
        let current_gate = barriers(p, &tx, &current, s.revision)
            .and_then(|_| execution_binding(p, &tx, &current));
        let (state, result, error) = match outcome {
            Ok(v) => match current_gate {
                Ok(()) => ("succeeded", v, None),
                Err(e) => (
                    "failed",
                    json!({"effect_observed":true,"notification_suppressed":true}),
                    Some(e.code),
                ),
            },
            Err(e) => ("failed", json!({"effect_success":false}), Some(e.code)),
        };
        let masked = reader::redact(&result.to_string()).0;
        let full_result: Value = serde_json::from_str(&masked)?;
        let digest = hash(&masked);
        let result = if s.definition.job == "read_query" {
            let tasks=full_result["tasks"].as_array().map(|rows|rows.iter().take(5).map(|r|json!({"task_id":r["task_id"],"state":r["state"],"revision":r["task_revision"],"completion_validity":r["completion_validity"]})).collect::<Vec<_>>()).unwrap_or_default();
            json!({"tasks":tasks,"observed_task_count":full_result["tasks"].as_array().map_or(0,Vec::len),"done_count":full_result["done_count"],"valid_done_count":full_result["valid_done_count"],"source_hash":digest,"query_ref":"pctx board","scope":"coordination_board","compact_result":true})
        } else {
            full_result
        };
        let previous_hash:Option<String>=tx.query_row("SELECT result_hash FROM schedule_runs WHERE namespace=?1 AND schedule=?2 AND state='succeeded' ORDER BY finished DESC,attempt DESC LIMIT 1",params![n,id],|r|r.get(0)).optional()?.flatten();
        let changed = previous_hash.as_deref() != Some(&digest);
        let noteworthy = match s.definition.job.as_str() {
            "read_query" => result["tasks"].as_array().is_some_and(|v| !v.is_empty()),
            "queue_digest" => result["decisions"]
                .as_array()
                .is_some_and(|v| !v.is_empty()),
            "backup" => true,
            _ => false,
        };
        let summary = match s.definition.job.as_str() {
            "read_query" => format!(
                "Tasks: {}; done: {}; current done: {}.",
                result["observed_task_count"], result["done_count"], result["valid_done_count"]
            ),
            "queue_digest" => format!(
                "Owner decisions requiring review: {}; confirmed minutes: unknown.",
                result["decisions"].as_array().map_or(0, Vec::len)
            ),
            _ => "Consistent local backup artifact created.".into(),
        };
        let delivery = if state == "succeeded" && changed && noteworthy {
            let message = operations::Message {
                schema_version: 1,
                kind: "notice".into(),
                topic: s.definition.topic.clone(),
                body: format!(
                    "Schedule {}: {} completed.\n{}\nGenerated at {}.\nEvidence: schedule-result:{}",
                    s.definition.id,
                    s.definition.job,
                    summary,
                    now(),
                    digest
                ),
                priority: 1,
                correlation_id: Some(format!("schedule:{}:{}", n, id)),
                evidence_refs: vec![format!("schedule-result:{digest}")],
                expires_at: None,
                source_revision: Some(s.revision),
                idempotency_key: format!(
                    "schedule:{}",
                    hash(format!("{n}:{id}:{}:{key}:{digest}", s.revision))
                ),
            };
            // Same writer transaction as final admission and result: pause cannot race queue insertion.
            Some(operations::enqueue_scheduled_db(
                p,
                &tx,
                &s.definition.recipient,
                &message,
            )?)
        } else {
            None
        };
        tx.execute("UPDATE schedule_runs SET state=?1,result_hash=?2,result=?3,delivery_ref=?4,error=?5,finished=?6 WHERE namespace=?7 AND schedule=?8 AND revision=?9 AND occurrence=?10 AND attempt=?11 AND state='running'",params![state,digest,result.to_string(),delivery.as_ref().map(Value::to_string),error,now(),n,id,s.revision,key,attempt])?;
        let mut final_metadata = occurrence.clone();
        final_metadata["state"] = json!(state);
        final_metadata["actual_success"] = json!(state == "succeeded");
        final_metadata["execution_started"] = json!(execution_started);
        final_metadata["result_hash"] = json!(digest);
        final_metadata["generated_at"] = json!(now());
        tx.execute("UPDATE schedule_occurrences SET state=?1,metadata=?2 WHERE namespace=?3 AND schedule=?4 AND revision=?5 AND occurrence=?6",params![state,final_metadata.to_string(),n,id,s.revision,key])?;
        if state == "succeeded" {
            tx.execute("UPDATE schedule_definitions SET last_success=?1 WHERE namespace=?2 AND id=?3 AND revision=?4",params![now(),n,id,s.revision])?;
        }
        let receipt = json!({"namespace":n,"schedule_id":id,"revision":s.revision,"occurrence":key,"attempt":attempt,"state":state,"execution_started":execution_started,"actual_success":state=="succeeded","result_hash":digest,"result":result,"error":error,"changed":changed,"delivery":delivery,"external_messages":0,"model_calls":0});
        event(&tx, &s.definition, "execution_finished", receipt.clone())?;
        tx.commit()?;
        results.push(receipt);
    }
    Ok(
        json!({"evaluated_at":at,"runs":results,"model_calls":0,"external_messages":0,"delivery_semantics":"local_queued_only"}),
    )
}

struct Awake {
    #[cfg(target_os = "macos")]
    assertion: u32,
}
impl Awake {
    fn release(mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        unsafe {
            if self.assertion != 0 {
                let result = IOPMAssertionRelease(self.assertion);
                if result != 0 {
                    return Err(Error::new(
                        "CAPABILITY_UNVERIFIED",
                        "Keep-awake release was not acknowledged by the host",
                        6,
                    ));
                }
                self.assertion = 0;
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = &mut self;
        }
        Ok(())
    }
    fn acquire(purpose: &str, timeout_seconds: f64) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            use std::ffi::CString;
            let kind = CString::new("PreventUserIdleSystemSleep")
                .map_err(|_| invalid("Invalid assertion kind"))?;
            let name = CString::new(purpose).map_err(|_| invalid("Invalid keep-awake purpose"))?;
            // Explicit job-owned assertion, released by RAII and by OS when this process exits.
            unsafe {
                let k = CFStringCreateWithCString(std::ptr::null(), kind.as_ptr(), 0x08000100);
                let n = CFStringCreateWithCString(std::ptr::null(), name.as_ptr(), 0x08000100);
                if k.is_null() || n.is_null() {
                    if !k.is_null() {
                        CFRelease(k);
                    }
                    if !n.is_null() {
                        CFRelease(n);
                    }
                    return Err(Error::new(
                        "CAPABILITY_UNVERIFIED",
                        "Native keep-awake string allocation failed",
                        6,
                    ));
                }
                let mut assertion = 0;
                let status = IOPMAssertionCreateWithDescription(
                    k,
                    n,
                    std::ptr::null(),
                    n,
                    std::ptr::null(),
                    timeout_seconds,
                    std::ptr::null(),
                    &mut assertion,
                );
                CFRelease(k);
                CFRelease(n);
                if status != 0 {
                    return Err(Error::new(
                        "CAPABILITY_UNVERIFIED",
                        "Host denied keep-awake assertion",
                        6,
                    ));
                }
                Ok(Self { assertion })
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (purpose, timeout_seconds);
            Err(Error::new(
                "CAPABILITY_UNVERIFIED",
                "Native job-owned keep-awake capability is not verified on this platform",
                6,
            ))
        }
    }
}
impl Drop for Awake {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        unsafe {
            if self.assertion != 0 {
                IOPMAssertionRelease(self.assertion);
            }
        }
    }
}
#[cfg(target_os = "macos")]
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithDescription(
        kind: *const std::ffi::c_void,
        name: *const std::ffi::c_void,
        details: *const std::ffi::c_void,
        reason: *const std::ffi::c_void,
        bundle: *const std::ffi::c_void,
        timeout: f64,
        action: *const std::ffi::c_void,
        id: *mut u32,
    ) -> i32;
    fn IOPMAssertionRelease(id: u32) -> i32;
}
#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(
        allocator: *const std::ffi::c_void,
        string: *const std::ffi::c_char,
        encoding: u32,
    ) -> *const std::ffi::c_void;
    fn CFRelease(value: *const std::ffi::c_void);
}
fn release_awake(awake: Option<Awake>) -> Result<()> {
    if let Some(a) = awake {
        a.release()?;
    }
    Ok(())
}
fn run_loop(
    p: &Project,
    namespace: Option<&str>,
    interval: u64,
    max_ticks: u32,
    ttl: u64,
    keep_awake: bool,
    purpose: Option<&str>,
) -> Result<Value> {
    owner()?;
    if !(1..=3600).contains(&interval)
        || !(1..=10000).contains(&max_ticks)
        || !(1..=86400).contains(&ttl)
    {
        return Err(invalid(
            "Managed loop requires bounded interval, tick count and TTL",
        ));
    }
    let purpose = purpose.unwrap_or("");
    if keep_awake && (purpose.trim().is_empty() || purpose.len() > 256 || reader::redact(purpose).1)
    {
        return Err(invalid(
            "Keep-awake requires a bounded non-sensitive purpose",
        ));
    }
    let start = Instant::now();
    let mut awake = None;
    let mut runs = Vec::new();
    let db = connect(p)?;
    event(
        &db,
        &ScheduleDefinition {
            schema_version: 1,
            namespace: namespace.unwrap_or("*").into(),
            id: "managed-loop".into(),
            timezone: "UTC".into(),
            cadence: Cadence::Interval {
                seconds: 60,
                anchor: "1970-01-01T00:00:00Z".into(),
            },
            valid_from: "1970-01-01T00:00:00Z".into(),
            job: "read_query".into(),
            bridge: "manual".into(),
            role: "owner".into(),
            recipient: "owner".into(),
            topic: "managed-loop".into(),
            session_id: None,
            context_epoch: None,
            action: None,
            decision_id: None,
            enabled: true,
            misfire: misfire(),
        },
        "runtime_started",
        json!({"pid":std::process::id(),"ttl_seconds":ttl,"max_ticks":max_ticks,"keep_awake_requested":keep_awake,"purpose":purpose}),
    )?;
    for _ in 0..max_ticks {
        if start.elapsed() >= StdDuration::from_secs(ttl) {
            break;
        }
        // No assertion survives explicit pause/silence: reevaluate all current bound definitions.
        let mut statement=db.prepare("SELECT namespace,id FROM schedule_definitions WHERE removed=0 AND (?1 IS NULL OR namespace=?1)")?;
        let keys = statement
            .query_map([namespace], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(statement);
        let allowed = keys
            .iter()
            .any(|(n, id)| get(&db, n, id).is_ok_and(|s| barriers(p, &db, &s, s.revision).is_ok()));
        if !allowed {
            release_awake(awake.take())?;
            break;
        }
        if keep_awake && awake.is_none() {
            awake = Some(Awake::acquire(
                purpose,
                (ttl as f64 - start.elapsed().as_secs_f64()).max(0.001),
            )?);
        }
        runs.push(tick(p, namespace, now(), false)?);
        if runs.len() >= max_ticks as usize {
            break;
        }
        // Bounded one-second slices recheck pause instead of holding wake through a long sleep.
        for _ in 0..interval {
            if start.elapsed() >= StdDuration::from_secs(ttl) {
                break;
            }
            std::thread::sleep(StdDuration::from_secs(1));
            if !keys.iter().any(|(n, id)| {
                get(&db, n, id).is_ok_and(|s| barriers(p, &db, &s, s.revision).is_ok())
            }) {
                release_awake(awake.take())?;
                return Ok(
                    json!({"ticks":runs,"stopped":"paused_or_silenced","assertion_released":true,"model_calls":0}),
                );
            }
        }
    }
    release_awake(awake)?;
    Ok(
        json!({"ticks":runs,"ttl_seconds":ttl,"keep_awake_requested":keep_awake,"assertion_released":true,"runtime_finished":true,"model_calls":0,"external_messages":0}),
    )
}

fn recover(
    p: &Project,
    namespace: &str,
    id: &str,
    revision: i64,
    occurrence: &str,
    attempt: i64,
    reason: &str,
) -> Result<Value> {
    owner()?;
    label(namespace)?;
    label(id)?;
    if reason.trim().is_empty() || reason.len() > 2048 {
        return Err(invalid("Recovery requires a bounded reason"));
    }
    let mut db = connect(p)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let s = get(&tx, namespace, id)?;
    let (state,result):(String,String)=tx.query_row("SELECT state,result FROM schedule_runs WHERE namespace=?1 AND schedule=?2 AND revision=?3 AND occurrence=?4 AND attempt=?5",params![namespace,id,revision,occurrence,attempt],|r|Ok((r.get(0)?,r.get(1)?)))?;
    let metadata: Value = serde_json::from_str(&result)?;
    if !["running", "interrupted_unknown"].contains(&state.as_str())
        || metadata["workspace"] != p.workspace_id
        || metadata["execution_kind"] != "in_process"
    {
        return Err(Error::new(
            "RESOURCE_OWNER_UNKNOWN",
            "Attempt lacks current-workspace in-process ownership proof",
            10,
        ));
    }
    let pid = metadata["owner_pid"]
        .as_u64()
        .filter(|p| *p > 0 && *p <= i32::MAX as u64)
        .ok_or_else(|| invalid("Invalid run owner PID"))?;
    #[cfg(unix)]
    {
        let rc = unsafe { libc::kill(pid as i32, 0) };
        if rc == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            return Err(Error::new(
                "RESOURCE_OWNER_UNKNOWN",
                "Run owner may still be alive; no TTL takeover",
                10,
            ));
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        return Err(Error::new(
            "CAPABILITY_UNVERIFIED",
            "Process absence observation is not supported on this platform",
            6,
        ));
    }
    #[cfg(unix)]
    {
        tx.execute("UPDATE schedule_runs SET state='failed',error='OWNER_PROCESS_OBSERVED_ABSENT',finished=?1 WHERE namespace=?2 AND schedule=?3 AND revision=?4 AND occurrence=?5 AND attempt=?6",params![now(),namespace,id,revision,occurrence,attempt])?;
        tx.execute("UPDATE schedule_occurrences SET state='failed' WHERE namespace=?1 AND schedule=?2 AND revision=?3 AND occurrence=?4",params![namespace,id,revision,occurrence])?;
        let value = json!({"namespace":namespace,"schedule_id":id,"revision":revision,"occurrence":occurrence,"attempt":attempt,"state":"failed","previous_success":"unknown","owner_process_observed_absent":true,"reason":reader::redact(reason).0,"retry_requires_explicit_flag":true});
        event(
            &tx,
            &s.definition,
            "execution_owner_recovered",
            value.clone(),
        )?;
        tx.commit()?;
        Ok(value)
    }
}

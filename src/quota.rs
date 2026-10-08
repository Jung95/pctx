//! Explicit provider observations, separate units, and soft local reservations.
use crate::{
    domain::{Error, Result, hash, id, now},
    project::Project,
    reader, work,
};
use clap::Subcommand;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
const VERSION: i64 = 1;
#[derive(Debug, Clone, Subcommand)]
pub enum QuotaCommand {
    Ingest {
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        idempotency_key: String,
    },
    Report {
        #[arg(long)]
        pool: Option<String>,
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long,default_value="pool",value_parser=["pool","task","session","role","model"])]
        group_by: String,
        #[arg(long, default_value = "7d")]
        window: String,
        #[arg(long)]
        include_coordination: bool,
        #[arg(long, default_value_t = 900)]
        max_age_seconds: i64,
    },
    Reconcile {
        #[arg(long)]
        pool: String,
        #[arg(long, default_value_t = 900)]
        max_age_seconds: i64,
    },
    Plan {
        #[arg(long)]
        pool: String,
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long, default_value_t = 900)]
        max_age_seconds: i64,
    },
    Reserve {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        pool: String,
        #[arg(long)]
        unit: String,
        #[arg(long)]
        amount: f64,
        #[arg(long)]
        limit: f64,
        #[arg(long, default_value = "coding-small")]
        policy: String,
        #[arg(long)]
        idempotency_key: String,
    },
    Release {
        reservation: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub observation_id: String,
    pub pool_id: String,
    pub provider: String,
    pub model: String,
    pub metric: String,
    pub unit: String,
    pub source: String,
    pub collector: String,
    pub source_revision: String,
    pub observed_at: String,
    pub window_id: String,
    pub window_start: String,
    pub window_end: String,
    pub reset_at: Option<String>,
    pub timezone: Option<String>,
    pub status: String,
    pub amount: Option<f64>,
    pub kind: String,
    pub request_id: Option<String>,
    pub session_id: Option<String>,
    pub context_epoch: Option<i64>,
    pub counter_epoch: Option<String>,
    pub task_id: Option<String>,
    pub role: Option<String>,
    #[serde(default)]
    pub pricing_table_version: Option<String>,
    #[serde(default)]
    pub counter_origin_zero: bool,
    #[serde(default = "workload")]
    pub workload: String,
}
fn workload() -> String {
    "execution".into()
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageBatch {
    pub schema_version: u32,
    pub observations: Vec<Observation>,
}
fn invalid(s: &str) -> Error {
    Error::new("INVALID_ARGUMENT", s, 2)
}
fn conflict(s: &str) -> Error {
    Error::new("IDEMPOTENCY_CONFLICT", s, 9)
}
fn owner() -> Result<()> {
    if std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()) != "owner" {
        return Err(Error::new(
            "POLICY_DENIED",
            "Usage imports and reservations require owner authority",
            5,
        ));
    }
    Ok(())
}
fn label(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 256 || s.contains('\0') || reader::redact(s).1 {
        return Err(invalid("Invalid or sensitive usage identity field"));
    }
    Ok(())
}
fn stamp(s: &str) -> Result<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.timestamp())
        .map_err(|_| invalid("Usage times must be absolute RFC3339 with an offset"))
}
fn request_phase<T>(p: &Project, call: impl FnOnce() -> Result<T>) -> Result<T> {
    p.check_deadline()?;
    let result = call();
    p.check_deadline()?;
    result
}
fn collect_rows<T>(
    p: &Project,
    db: &Connection,
    mut rows: impl Iterator<Item = rusqlite::Result<T>>,
) -> Result<Vec<T>> {
    let mut values = Vec::new();
    loop {
        p.configure_sqlite(db)?;
        let row = rows.next();
        p.check_deadline()?;
        match row {
            Some(row) => values.push(row.map_err(|e| p.map_sqlite_error(e))?),
            None => return Ok(values),
        }
    }
}
fn connect(p: &Project) -> Result<Connection> {
    request_phase(p, || connect_inner(p))
}
fn connect_inner(p: &Project) -> Result<Connection> {
    let mut db = work::connect(p)?;
    let initialized: bool = p.sqlite_call(&db, || {
        db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='quota_schema')",
            [],
            |r| r.get(0),
        )
    })?;
    if initialized {
        let version: i64 = p.sqlite_call(&db, || {
            db.query_row("SELECT version FROM quota_schema", [], |r| r.get(0))
        })?;
        if version != VERSION {
            return Err(Error::new(
                "DB_SCHEMA_TOO_NEW",
                "Unsupported quota schema",
                7,
            ));
        }
        p.sqlite_call(&db, || db.execute_batch("CREATE TRIGGER IF NOT EXISTS quota_events_no_update BEFORE UPDATE ON quota_events BEGIN SELECT RAISE(ABORT,'append-only events'); END; CREATE TRIGGER IF NOT EXISTS quota_events_no_delete BEFORE DELETE ON quota_events BEGIN SELECT RAISE(ABORT,'append-only events'); END;"))?;
        return Ok(db);
    }
    p.configure_sqlite(&db)?;
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| p.map_sqlite_error(e))?;
    p.check_deadline()?;
    p.sqlite_call(&tx, || tx.execute_batch("CREATE TABLE quota_schema(version INTEGER NOT NULL);INSERT INTO quota_schema VALUES(1);
CREATE TABLE quota_observations(observation_id TEXT PRIMARY KEY,pool TEXT NOT NULL,provider TEXT NOT NULL,model TEXT NOT NULL,metric TEXT NOT NULL,unit TEXT NOT NULL,source TEXT NOT NULL,session TEXT,context_epoch INTEGER,counter_epoch TEXT,observed INTEGER NOT NULL,window_start INTEGER NOT NULL,window_end INTEGER NOT NULL,payload TEXT NOT NULL,payload_hash TEXT NOT NULL);
CREATE INDEX quota_lookup ON quota_observations(pool,metric,observed);
CREATE TABLE quota_receipts(key TEXT PRIMARY KEY,request_hash TEXT NOT NULL,response TEXT NOT NULL);
CREATE TABLE quota_reservations(id TEXT PRIMARY KEY,task TEXT NOT NULL,pool TEXT NOT NULL,unit TEXT NOT NULL,amount REAL NOT NULL,policy TEXT NOT NULL,policy_hash TEXT NOT NULL,status TEXT NOT NULL,created INTEGER NOT NULL);
CREATE TABLE quota_events(seq INTEGER PRIMARY KEY AUTOINCREMENT,entity TEXT NOT NULL,kind TEXT NOT NULL,metadata TEXT NOT NULL,created INTEGER NOT NULL);"))?;
    p.check_deadline()?;
    tx.commit().map_err(|e| p.map_sqlite_error(e))?;
    p.check_deadline()?;
    p.sqlite_call(&db, || db.execute_batch("CREATE TRIGGER IF NOT EXISTS quota_events_no_update BEFORE UPDATE ON quota_events BEGIN SELECT RAISE(ABORT,'append-only events'); END; CREATE TRIGGER IF NOT EXISTS quota_events_no_delete BEFORE DELETE ON quota_events BEGIN SELECT RAISE(ABORT,'append-only events'); END;"))?;
    Ok(db)
}
fn canonical_task(db: &Connection, name: &str) -> Result<String> {
    let number = name
        .strip_prefix("T-")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(-1);
    db.query_row(
        "SELECT id FROM tasks WHERE id=?1 OR number=?2",
        params![name, number],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| Error::new("TASK_NOT_FOUND", "Task target is not registered", 6))
}
fn validate(db: &Connection, p: &Project, o: &Observation) -> Result<()> {
    for s in [
        &o.observation_id,
        &o.pool_id,
        &o.provider,
        &o.model,
        &o.metric,
        &o.unit,
        &o.source,
        &o.collector,
        &o.source_revision,
        &o.window_id,
    ] {
        label(s)?;
    }
    for s in [
        o.request_id.as_ref(),
        o.counter_epoch.as_ref(),
        o.role.as_ref(),
        o.pricing_table_version.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        label(s)?;
    }
    if ![
        "input_tokens",
        "output_tokens",
        "cache_read_tokens",
        "cache_write_tokens",
        "subscription_quota",
        "context_window",
        "api_cost",
        "vendor_quota",
    ]
    .contains(&o.metric.as_str())
        || !["actual", "estimate", "unknown", "blocked"].contains(&o.status.as_str())
        || !["request", "cumulative", "quota"].contains(&o.kind.as_str())
        || ![
            "provider_request",
            "provider_quota",
            "statusline",
            "otel",
            "manual",
        ]
        .contains(&o.source.as_str())
        || !["execution", "coordination", "retrieval", "recovery"].contains(&o.workload.as_str())
    {
        return Err(invalid(
            "Unsupported usage metric/status/kind/source/workload",
        ));
    }
    if o.amount.is_some_and(|v| !v.is_finite() || v < 0.0)
        || matches!(o.status.as_str(), "unknown" | "blocked") && o.amount.is_some()
        || matches!(o.status.as_str(), "actual" | "estimate") && o.amount.is_none()
    {
        return Err(invalid("Usage amount/status mismatch"));
    }
    if o.metric.ends_with("tokens") && o.unit != "tokens"
        || matches!(o.metric.as_str(), "subscription_quota" | "context_window")
            && o.unit != "percentage"
        || o.metric == "api_cost" && !["USD", "EUR", "GBP", "JPY"].contains(&o.unit.as_str())
    {
        return Err(invalid("Usage metric requires its explicit unit"));
    }
    if o.metric == "api_cost" && o.status == "estimate" && o.pricing_table_version.is_none() {
        return Err(invalid(
            "API cost estimates require pricing_table_version and explicit currency unit",
        ));
    }
    if o.unit == "tokens"
        && o.amount
            .is_some_and(|v| v.fract() != 0.0 || v > 9007199254740991.0)
    {
        return Err(invalid(
            "Token counter must be an exact nonnegative integer <=2^53-1",
        ));
    }
    if o.unit == "percentage" && o.amount.is_some_and(|v| v > 100.0) {
        return Err(invalid("Percentage must be 0..100"));
    }
    let observed = stamp(&o.observed_at)?;
    let first = stamp(&o.window_start)?;
    let last = stamp(&o.window_end)?;
    if first >= last || observed < first || observed >= last || observed > now() + 300 {
        return Err(invalid(
            "Observation must be within its absolute window and not in the future",
        ));
    }
    if let Some(reset) = &o.reset_at
        && stamp(reset)? < observed
    {
        return Err(invalid("Reset precedes observation"));
    }
    if let Some(zone) = &o.timezone {
        zone.parse::<chrono_tz::Tz>()
            .map_err(|_| invalid("Unknown IANA timezone"))?;
    }
    if o.kind == "quota" {
        if o.source != "provider_quota" && o.source != "manual" {
            return Err(invalid(
                "Quota needs provider quota or explicitly manual source",
            ));
        }
        if o.kind == "quota" && o.metric.ends_with("tokens") {
            return Err(invalid("Tokens cannot be treated as subscription quota"));
        }
    } else {
        if o.task_id.is_none() || o.session_id.is_none() || o.context_epoch.is_none() {
            return Err(invalid(
                "Usage needs task, session and context epoch attribution",
            ));
        }
        if o.kind == "request" && o.request_id.is_none()
            || o.kind == "cumulative" && o.counter_epoch.is_none()
        {
            return Err(invalid(
                "Usage requires request ID or cumulative counter epoch",
            ));
        }
    }
    if let Some(task) = &o.task_id {
        let exists: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)",
            [task],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(Error::new(
                "TASK_NOT_FOUND",
                "Usage task ID is not registered",
                6,
            ));
        }
    }
    if let Some(s) = &o.session_id {
        let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='pctx_sessions' AND type='table')",[],|r|r.get(0))?;
        if !exists {
            return Err(Error::new(
                "SESSION_NOT_FOUND",
                "Usage session not registered",
                6,
            ));
        }
        let info: Option<(i64, String, Option<String>)> = db
            .query_row(
                "SELECT epoch,workspace,account_pool FROM pctx_sessions WHERE id=?1",
                [s],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (epoch, workspace, pool) =
            info.ok_or_else(|| Error::new("SESSION_NOT_FOUND", "Usage session not registered", 6))?;
        if o.context_epoch != Some(epoch)
            || workspace != p.workspace_id
            || pool.as_ref().is_some_and(|pool| pool != &o.pool_id)
        {
            return Err(Error::new(
                "BASELINE_MISMATCH",
                "Usage session epoch/workspace/account pool does not match",
                9,
            ));
        }
    }
    Ok(())
}
fn event(db: &Connection, id: &str, kind: &str, value: Value) -> Result<()> {
    db.execute(
        "INSERT INTO quota_events(entity,kind,metadata,created) VALUES(?1,?2,?3,?4)",
        params![id, kind, value.to_string(), now()],
    )?;
    Ok(())
}
fn receipt(db: &Connection, key: &str, request: &str) -> Result<Option<Value>> {
    if let Some((old, response)) = db
        .query_row(
            "SELECT request_hash,response FROM quota_receipts WHERE key=?1",
            [key],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?
    {
        if old != request {
            return Err(conflict("Idempotency key already has a different request"));
        }
        return Ok(Some(serde_json::from_str(&response)?));
    }
    Ok(None)
}
fn ingest(p: &Project, path: &PathBuf, key: &str) -> Result<Value> {
    owner()?;
    label(key)?;
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 1048576 {
        return Err(invalid("Usage import must be regular JSON at most one MiB"));
    }
    let batch: UsageBatch = serde_json::from_slice(&std::fs::read(path)?)?;
    if batch.schema_version != 1
        || batch.observations.is_empty()
        || batch.observations.len() > 10000
    {
        return Err(invalid(
            "Usage batch requires schema 1 and 1..10000 observations",
        ));
    }
    let request = hash(serde_json::to_vec(&batch)?);
    let mut db = connect(p)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(old) = receipt(&tx, key, &request)? {
        return Ok(old);
    }
    let mut inserted = 0;
    let mut duplicate = 0;
    for raw in &batch.observations {
        let mut normalized = raw.clone();
        if let Some(task) = &raw.task_id {
            normalized.task_id = Some(canonical_task(&tx, task)?);
        }
        let o = &normalized;
        validate(&tx, p, o)?;
        if o.kind == "request" && o.status == "actual" {
            let previous:Option<String>=tx.query_row("SELECT payload FROM quota_observations WHERE pool=?1 AND provider=?2 AND metric=?3 AND json_extract(payload,'$.request_id')=?4 ORDER BY observed DESC LIMIT 1",params![o.pool_id,o.provider,o.metric,o.request_id],|r|r.get(0)).optional()?;
            if let Some(previous) = previous {
                let old: Observation = serde_json::from_str(&previous)?;
                if old.amount.zip(o.amount).is_some_and(|(a, b)| a != b)
                    || old.model != o.model
                    || old.unit != o.unit
                    || old.task_id != o.task_id
                    || old.session_id != o.session_id
                    || old.context_epoch != o.context_epoch
                {
                    return Err(conflict(
                        "Provider request identity has conflicting attribution or amount",
                    ));
                }
            }
        }
        let payload = serde_json::to_string(o)?;
        let fingerprint = hash(&payload);
        if let Some(existing) = tx
            .query_row(
                "SELECT payload_hash FROM quota_observations WHERE observation_id=?1",
                [&o.observation_id],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            if existing != fingerprint {
                return Err(conflict("Observation ID has changed content"));
            }
            duplicate += 1;
            continue;
        }
        if o.kind == "cumulative" && o.status == "actual" {
            let previous:Option<String>=tx.query_row("SELECT payload FROM quota_observations WHERE pool=?1 AND provider=?2 AND model=?3 AND metric=?4 AND source=?5 AND session=?6 AND context_epoch=?7 AND counter_epoch=?8 ORDER BY observed DESC,rowid DESC LIMIT 1",params![o.pool_id,o.provider,o.model,o.metric,o.source,o.session_id,o.context_epoch,o.counter_epoch],|r|r.get(0)).optional()?;
            if let Some(previous) = previous {
                let prev: Observation = serde_json::from_str(&previous)?;
                if stamp(&o.observed_at)? < stamp(&prev.observed_at)?
                    || o.amount
                        .zip(prev.amount)
                        .is_some_and(|(new, old)| new < old)
                {
                    return Err(Error::new(
                        "USAGE_OBSERVATION_STALE",
                        "Cumulative counter regressed; use a new counter epoch for reset",
                        9,
                    ));
                }
            }
        }
        tx.execute("INSERT INTO quota_observations VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",params![o.observation_id,o.pool_id,o.provider,o.model,o.metric,o.unit,o.source,o.session_id,o.context_epoch,o.counter_epoch,stamp(&o.observed_at)?,stamp(&o.window_start)?,stamp(&o.window_end)?,payload,fingerprint])?;
        event(
            &tx,
            &o.observation_id,
            "usage_observed",
            json!({"pool_id":o.pool_id,"metric":o.metric,"source":o.source,"status":o.status}),
        )?;
        inserted += 1;
    }
    let value = json!({"inserted":inserted,"duplicates":duplicate,"observation_count":batch.observations.len(),"transport":"explicit_json_import","source_authentication":"not_asserted","usage_estimated_from_bytes":false});
    tx.execute(
        "INSERT INTO quota_receipts VALUES(?1,?2,?3)",
        params![key, request, value.to_string()],
    )?;
    tx.commit()?;
    Ok(value)
}
fn observations(p: &Project, db: &Connection, pool: Option<&str>) -> Result<Vec<Observation>> {
    // Restore preserves history, but old capacity observations cannot authorize work.
    p.check_deadline()?;
    let cutoff: i64 = p.sqlite_call(db, || db.query_row("SELECT coalesce(max(json_extract(metadata,'$.historical_observation_rowid')),0) FROM quota_events WHERE kind='control_restored'",[],|r|r.get(0)))?;
    let mut statement = p.sqlite_call(db, || db.prepare("SELECT rowid,payload FROM quota_observations WHERE ?1 IS NULL OR pool=?1 ORDER BY observed,rowid"))?;
    let rows = statement
        .query_map([pool], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| p.map_sqlite_error(e))?;
    let mut observations = Vec::new();
    let mut rows = rows;
    loop {
        p.configure_sqlite(db)?;
        let row = rows.next();
        p.check_deadline()?;
        let Some(row) = row else {
            break;
        };
        let (rowid, payload) = row.map_err(|e| p.map_sqlite_error(e))?;
        let mut o: Observation = serde_json::from_str(&payload)?;
        p.check_deadline()?;
        if rowid <= cutoff && o.kind == "quota" {
            o.status = "unknown".into();
        }
        observations.push(o);
    }
    p.check_deadline()?;
    Ok(observations)
}

fn priority(source: &str) -> usize {
    match source {
        "provider_request" | "provider_quota" => 0,
        "statusline" => 1,
        "otel" => 2,
        _ => 3,
    }
}
fn quota_status(
    p: &Project,
    observations: &[Observation],
    pool: &str,
    max_age: i64,
) -> Result<Value> {
    p.check_deadline()?;
    if max_age <= 0 || max_age > 86400 {
        return Err(invalid("Quota max age must be 1..86400 seconds"));
    }
    let mut windows: BTreeMap<String, &Observation> = BTreeMap::new();
    for o in observations {
        p.check_deadline()?;
        if o.pool_id != pool || o.kind != "quota" || o.metric != "subscription_quota" {
            continue;
        }
        let key = format!("{}:{}", o.provider, o.window_id);
        if windows.get(&key).is_none_or(|old| {
            priority(&o.source) < priority(&old.source)
                || priority(&o.source) == priority(&old.source)
                    && stamp(&o.observed_at).unwrap_or(0) >= stamp(&old.observed_at).unwrap_or(0)
        }) {
            windows.insert(key, o);
        }
    }
    let mut state = "unknown";
    let mut rows = Vec::new();
    let mut known = false;
    let mut unknown = false;
    for o in windows.values() {
        p.check_deadline()?;
        let stale = now() - stamp(&o.observed_at)? > max_age
            || now() >= stamp(&o.window_end)?
            || o.reset_at
                .as_ref()
                .is_some_and(|reset| stamp(reset).is_ok_and(|t| now() >= t));
        let current = !stale && o.source == "provider_quota";
        let status = if !current || o.status == "unknown" || o.status == "estimate" {
            unknown = true;
            "unknown"
        } else if o.status == "blocked" {
            known = true;
            "blocked"
        } else if let Some(amount) = o.amount {
            known = true;
            if amount > 90.0 {
                "drain"
            } else if amount > 80.0 {
                "conserve"
            } else {
                "normal"
            }
        } else {
            unknown = true;
            "unknown"
        };
        if status == "blocked"
            || status == "drain" && state != "blocked"
            || status == "conserve" && !["blocked", "drain"].contains(&state)
            || status == "normal" && state == "unknown"
        {
            state = status;
        }
        rows.push(json!({"window_id":o.window_id,"provider":o.provider,"observed_at":o.observed_at,"used_percentage":if current&&o.status=="actual"{o.amount}else{None},"reset_at":o.reset_at,"timezone":o.timezone,"status":status,"source":o.source,"collector":o.collector,"source_revision":o.source_revision,"observation_id":o.observation_id,"freshness":if stale{"stale"}else{"current"}}));
    }
    if !known || unknown && !["blocked", "drain"].contains(&state) {
        state = "unknown";
    }
    Ok(
        json!({"pool_id":pool,"state":state,"windows":rows,"thresholds":{"conserve_above_percentage":80,"drain_above_percentage":90,"max_age_seconds":max_age},"paid_work_allowed":matches!(state,"normal"|"conserve"),"helper_start_allowed":state=="normal","reset_requires_reobservation":true,"subscription_derived_from_tokens":false,"usage_coverage":"provider_window_observations_only"}),
    )
}
fn duration(window: &str) -> Result<i64> {
    let unit = window
        .chars()
        .last()
        .ok_or_else(|| invalid("Window missing"))?;
    if !matches!(unit, 'h' | 'd') {
        return Err(invalid("Window must use h or d"));
    }
    let number = window[..window.len() - 1]
        .parse::<i64>()
        .map_err(|_| invalid("Window must be 1h..8760h or 1d..365d"))?;
    let seconds = number
        .checked_mul(if unit == 'h' { 3600 } else { 86400 })
        .ok_or_else(|| invalid("Unsupported window"))?;
    if !(3600..=365 * 86400).contains(&seconds) {
        return Err(invalid("Report window must be 1h..365d"));
    }
    Ok(seconds)
}
#[derive(Default)]
struct Total {
    actual: Option<f64>,
    estimated: Option<f64>,
    manual: Option<f64>,
    observations: usize,
    unknown: bool,
    sources: BTreeSet<String>,
    references: BTreeSet<String>,
}
fn lane(o: &Observation) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        o.pool_id,
        o.provider,
        o.model,
        o.metric,
        o.unit,
        o.session_id.as_deref().unwrap_or("pool"),
        o.context_epoch.unwrap_or(0),
        o.window_id
    )
}
// Parameters mirror independent CLI filters; none imply unit conversion.
#[allow(clippy::too_many_arguments)]
fn report(
    p: &Project,
    pool: Option<&str>,
    task_filter: Option<&str>,
    session_filter: Option<&str>,
    group_by: &str,
    window: &str,
    include_coordination: bool,
    max_age: i64,
) -> Result<Value> {
    p.check_deadline()?;
    if let Some(pool) = pool {
        label(pool)?;
    }
    if !["pool", "task", "session", "role", "model"].contains(&group_by) {
        return Err(invalid("Unknown grouping"));
    }
    let cutoff = now() - duration(window)?;
    let db = connect(p)?;
    let task_filter = task_filter
        .map(|task| {
            p.configure_sqlite(&db)?;
            request_phase(p, || canonical_task(&db, task))
        })
        .transpose()?;
    let all = observations(p, &db, pool)?;
    // One actual source per model/session/metric/window prevents statusline/OTel/request overlap.
    let mut preferred: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for o in &all {
        p.check_deadline()?;
        if o.kind == "quota" || o.status != "actual" || o.source == "manual" {
            continue;
        }
        let key = lane(o);
        let value = (priority(&o.source), o.source.clone());
        if preferred.get(&key).is_none_or(|old| value < *old) {
            preferred.insert(key, value);
        }
    }
    let mut totals: BTreeMap<String, Total> = BTreeMap::new();
    let mut previous: BTreeMap<String, (f64, i64)> = BTreeMap::new();
    let mut requests = BTreeSet::new();
    let mut ignored = 0;
    for o in &all {
        p.check_deadline()?;
        if o.kind == "quota" {
            continue;
        }
        let actual = o.status == "actual" && o.source != "manual";
        if actual
            && preferred
                .get(&lane(o))
                .is_some_and(|(_, source)| source != &o.source)
        {
            ignored += 1;
            continue;
        }
        let observed = stamp(&o.observed_at)?;
        let mut amount = o.amount;
        let mut coverage_unknown = o.status == "unknown";
        if o.kind == "cumulative" && actual {
            let series = format!(
                "{}\0{}\0{}",
                lane(o),
                o.source,
                o.counter_epoch.as_deref().unwrap_or("")
            );
            let current = o.amount.ok_or_else(|| invalid("Counter amount missing"))?;
            amount = if let Some((old, old_time)) = previous.insert(series, (current, observed)) {
                if observed >= old_time && current >= old {
                    Some(current - old)
                } else {
                    coverage_unknown = true;
                    None
                }
            } else if o.counter_origin_zero {
                Some(current)
            } else {
                coverage_unknown = true;
                None
            };
        }
        if o.kind == "request" {
            let key = format!(
                "{}\0{}\0{}\0{}\0{}",
                o.pool_id,
                o.provider,
                o.request_id.as_deref().unwrap_or(""),
                o.metric,
                if actual { "actual" } else { &o.source }
            );
            if !requests.insert(key) {
                ignored += 1;
                continue;
            }
        }
        if observed < cutoff
            || task_filter
                .as_deref()
                .is_some_and(|t| o.task_id.as_deref() != Some(t))
            || session_filter.is_some_and(|s| o.session_id.as_deref() != Some(s))
            || !include_coordination && o.workload == "coordination"
        {
            continue;
        }
        let group = match group_by {
            "task" => o.task_id.as_deref().unwrap_or("unattributed"),
            "session" => o.session_id.as_deref().unwrap_or("unattributed"),
            "role" => o.role.as_deref().unwrap_or("unattributed"),
            "model" => &o.model,
            _ => &o.pool_id,
        };
        let key = format!("{group}\0{}\0{}", o.metric, o.unit);
        let t = totals.entry(key).or_default();
        t.observations += 1;
        t.unknown |= coverage_unknown;
        t.sources.insert(o.source.clone());
        t.references.insert(o.observation_id.clone());
        if let Some(value) = amount {
            if o.source == "manual" {
                *t.manual.get_or_insert(0.0) += value;
            } else if o.status == "estimate" {
                *t.estimated.get_or_insert(0.0) += value;
            } else if actual {
                *t.actual.get_or_insert(0.0) += value;
            }
        }
    }
    let groups:Vec<_>=totals.into_iter().map(|(key,t)|{p.check_deadline()?;let keys:Vec<_>=key.split('\0').collect();Ok(json!({"group":keys[0],"metric":keys[1],"unit":keys[2],"actual_amount":t.actual,"estimated_amount":t.estimated,"manual_reported_amount":t.manual,"status":if t.unknown||t.actual.is_none(){"unknown_or_partial"}else{"observed"},"coverage":if t.unknown{"partial"}else{"observed_intervals_only"},"sources":t.sources,"observation_refs":t.references,"observations":t.observations}))}).collect::<Result<Vec<_>>>()?;
    let mut pools: BTreeSet<String> = all
        .iter()
        .map(|o| {
            p.check_deadline()?;
            Ok(o.pool_id.clone())
        })
        .collect::<Result<_>>()?;
    if let Some(pool) = pool {
        pools.insert(pool.into());
    }
    let statuses = pools
        .iter()
        .map(|pool| quota_status(p, &all, pool, max_age))
        .collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"group_by":group_by,"window":window,"include_coordination":include_coordination,"groups":groups,"quota":statuses,"other_windows":all.iter().filter(|o|o.kind=="quota"&&o.metric!="subscription_quota").collect::<Vec<_>>(),"collector_status":if all.is_empty(){"unknown"}else{"explicit_imported_observations"},"actual_usage_available":groups.iter().any(|g|!g["actual_amount"].is_null()),"ignored_duplicate_source_observations":ignored,"source_precedence":["provider_request","statusline","otel","manual_separate"],"local_cache_tokens_included":false,"subscription_token_conversion":false,"billing_amount_asserted":false}),
    )
}
fn barriers(p: &Project, db: &Connection, task: Option<&str>) -> Result<Vec<String>> {
    p.check_deadline()?;
    let mut reasons = Vec::new();
    if let Some(task) = task {
        let (state, agent): (String, Option<String>) = p
            .sqlite_call(db, || {
                db.query_row("SELECT state,agent FROM tasks WHERE id=?1", [task], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .optional()
            })?
            .ok_or_else(|| Error::new("TASK_NOT_FOUND", "Task not found", 6))?;
        if state == "paused" {
            reasons.push("task_paused".into());
        }
        if ["done", "cancelled", "blocked", "in_review"].contains(&state.as_str()) {
            reasons.push(format!("task_{state}"));
        }
        let ops:bool=p.sqlite_call(db,||db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='ops_roles' AND type='table')",[],|r|r.get(0)))?;
        let sessions:bool=p.sqlite_call(db,||db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='pctx_sessions' AND type='table')",[],|r|r.get(0)))?;
        if ops && sessions {
            let paused:bool=p.sqlite_call(db,||db.query_row("SELECT EXISTS(SELECT 1 FROM ops_roles WHERE paused=1 AND (role='*' OR role IN (SELECT role FROM pctx_sessions WHERE agent=?1)))",[&agent],|r|r.get(0)))?;
            if paused {
                reasons.push("role_paused".into());
            }
            let silenced:bool=p.sqlite_call(db,||db.query_row("SELECT EXISTS(SELECT 1 FROM ops_silences WHERE active=1 AND (role='*' OR role IN (SELECT role FROM pctx_sessions WHERE agent=?1)) AND (recipient='*' OR recipient=?1 OR recipient IN (SELECT id FROM pctx_sessions WHERE agent=?1)))",[&agent],|r|r.get(0)))?;
            if silenced {
                reasons.push("topic_silenced_requires_explicit_policy_evaluation".into());
            }
        }
    }
    p.check_deadline()?;
    Ok(reasons)
}
fn plan(p: &Project, pool: &str, task: Option<&str>, max_age: i64) -> Result<Value> {
    p.check_deadline()?;
    label(pool)?;
    let db = connect(p)?;
    let observations = observations(p, &db, Some(pool))?;
    let status = quota_status(p, &observations, pool, max_age)?;
    let task = task
        .map(|task| {
            p.configure_sqlite(&db)?;
            request_phase(p, || canonical_task(&db, task))
        })
        .transpose()?;
    let reasons = barriers(p, &db, task.as_deref())?;
    let active: Vec<Value> = {
        let mut s=p.sqlite_call(&db,|| db.prepare("SELECT id,task,unit,amount,policy,status FROM quota_reservations WHERE pool=?1 AND status='active' ORDER BY id"))?;

        collect_rows(p, &db, s.query_map([pool],|r|Ok(json!({"reservation_id":r.get::<_,String>(0)?,"task_id":r.get::<_,String>(1)?,"unit":r.get::<_,String>(2)?,"amount":r.get::<_,f64>(3)?,"policy":r.get::<_,String>(4)?,"status":r.get::<_,String>(5)?})))?)?
    };
    let state = status["state"].as_str().unwrap_or("unknown");
    let next = if !reasons.is_empty() {
        "Preserve owner pause/silence; request explicit policy evaluation."
    } else {
        match state {
            "normal" => {
                "Obtain full context for a fresh session and explicitly reserve local budget before new work."
            }
            "conserve" => {
                "Reduce repeated context and notifications; finish a small explicitly authorized action."
            }
            "drain" => {
                "Do not start helpers or large work; finish at a safe boundary and persist a capsule."
            }
            "blocked" => {
                "Stop model retries; keep local records and reobserve provider availability."
            }
            _ => {
                "Usage/quota is unknown; collect a fresh provider observation before paid work or helpers."
            }
        }
    };
    Ok(
        json!({"pool_id":pool,"task_id":task,"quota":status,"barriers":reasons,"new_work_allowed":reasons.is_empty()&&status["paid_work_allowed"]==true,"next_action":next,"reservations":active,"model_started":false,"paid_calls":0,"pause_modified":false,"silence_modified":false,"context_epoch_inherited":false,"approval_or_lease_restored":false,"reset_is_authorization":false}),
    )
}
// Explicit reservation fields preserve the exact owner-authorized contract.
#[allow(clippy::too_many_arguments)]
fn reserve(
    p: &Project,
    task: &str,
    pool: &str,
    unit: &str,
    amount: f64,
    limit: f64,
    policy: &str,
    key: &str,
) -> Result<Value> {
    owner()?;
    for s in [pool, unit, policy, key] {
        label(s)?;
    }
    if !["tokens", "USD", "EUR", "GBP", "JPY", "percentage"].contains(&unit)
        || !amount.is_finite()
        || !limit.is_finite()
        || amount <= 0.0
        || limit <= 0.0
        || amount > limit
    {
        return Err(invalid(
            "Reservation requires a positive amount and explicit same-unit local limit",
        ));
    }
    let mut db = connect(p)?;
    let task = canonical_task(&db, task)?;
    let request = hash(serde_json::to_vec(
        &json!({"task":task,"pool":pool,"unit":unit,"amount":amount,"limit":limit,"policy":policy,"policy_hash":p.policy_hash()}),
    )?);
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(mut old) = receipt(&tx, key, &request)? {
        let current: Option<String> = tx
            .query_row(
                "SELECT status FROM quota_reservations WHERE id=?1",
                [old["reservation_id"].as_str().unwrap_or("")],
                |r| r.get(0),
            )
            .optional()?;
        old["state"] = json!(current.unwrap_or_else(|| "invalidated".into()));
        old["replayed"] = json!(true);
        return Ok(old);
    }
    let reasons = barriers(p, &tx, Some(&task))?;
    if !reasons.is_empty() {
        return Err(Error::new(
            if reasons.iter().any(|r| r.contains("silenced")) {
                "TOPIC_SILENCED"
            } else {
                "ROLE_PAUSED"
            },
            "Persistent task/role/topic policy blocks the reservation",
            5,
        ));
    }
    let status = quota_status(p, &observations(p, &tx, Some(pool))?, pool, 900)?;
    if !matches!(status["state"].as_str(), Some("normal" | "conserve")) {
        return Err(Error::new(
            "BUDGET_EXHAUSTED",
            "Pool availability is blocked, draining or unknown",
            10,
        ));
    }
    let reserved:f64=tx.query_row("SELECT coalesce(sum(amount),0.0) FROM quota_reservations WHERE pool=?1 AND unit=?2 AND status='active'",params![pool,unit],|r|r.get(0))?;
    if reserved + amount > limit {
        return Err(Error::new(
            "BUDGET_EXHAUSTED",
            "Soft local reservations exceed the explicit same-unit limit",
            10,
        ));
    }
    let reservation = id("BUDGET");
    tx.execute(
        "INSERT INTO quota_reservations VALUES(?1,?2,?3,?4,?5,?6,?7,'active',?8)",
        params![
            reservation,
            task,
            pool,
            unit,
            amount,
            policy,
            p.policy_hash(),
            now()
        ],
    )?;
    let value = json!({"reservation_id":reservation,"task_id":task,"pool_id":pool,"unit":unit,"amount":amount,"local_limit":limit,"local_total_reserved":reserved+amount,"state":"active","provider_guaranteed":false,"soft_local_constraint":true,"model_started":false});
    event(&tx, &reservation, "budget_reserved", value.clone())?;
    tx.execute(
        "INSERT INTO quota_receipts VALUES(?1,?2,?3)",
        params![key, request, value.to_string()],
    )?;
    tx.commit()?;
    Ok(value)
}
pub fn recovery_state(p: &Project) -> Result<Value> {
    request_phase(p, || recovery_state_inner(p))
}
fn recovery_state_inner(p: &Project) -> Result<Value> {
    let db = connect(p)?;
    let all = observations(p, &db, None)?;
    let pools: BTreeSet<_> = all
        .iter()
        .map(|o| {
            p.check_deadline()?;
            Ok(o.pool_id.clone())
        })
        .collect::<Result<_>>()?;
    let quota = pools
        .iter()
        .map(|pool| quota_status(p, &all, pool, 900))
        .collect::<Result<Vec<_>>>()?;
    let refs:Vec<_>=all.iter().map(|o|json!({"observation_id":o.observation_id,"pool_id":o.pool_id,"source":o.source,"source_revision":o.source_revision,"metric":o.metric,"unit":o.unit,"model":o.model,"task_id":o.task_id,"role":o.role,"window_id":o.window_id,"window_start":o.window_start,"window_end":o.window_end,"reset_at":o.reset_at,"observed_at":o.observed_at,"status":o.status,"session_id":o.session_id,"context_epoch":o.context_epoch,"counter_epoch":o.counter_epoch})).collect();
    Ok(
        json!({"quota_schema_version":VERSION,"observations":refs,"quota":quota,"counter_epochs_inherited":false,"reservations_require_revalidation":true,"missing_collector":"unknown","restore_may_resume_paid_work":false}),
    )
}
pub fn execute(p: &Project, c: &QuotaCommand) -> Result<Value> {
    if matches!(
        c,
        QuotaCommand::Report { .. } | QuotaCommand::Plan { .. } | QuotaCommand::Reconcile { .. }
    ) {
        let mut scope = p.clone();
        if scope.deadline.is_none() {
            scope.deadline = Some(crate::deadline::Deadline::from_millis(10_000)?);
        }
        request_phase(&scope, || execute_inner(&scope, c))
    } else {
        execute_inner(p, c)
    }
}
fn execute_inner(p: &Project, c: &QuotaCommand) -> Result<Value> {
    match c {
        QuotaCommand::Ingest {
            from_file,
            idempotency_key,
        } => ingest(p, from_file, idempotency_key),
        QuotaCommand::Report {
            pool,
            task_id,
            session,
            group_by,
            window,
            include_coordination,
            max_age_seconds,
        } => report(
            p,
            pool.as_deref(),
            task_id.as_deref(),
            session.as_deref(),
            group_by,
            window,
            *include_coordination,
            *max_age_seconds,
        ),
        QuotaCommand::Reconcile {
            pool,
            max_age_seconds,
        } => plan(p, pool, None, *max_age_seconds),
        QuotaCommand::Plan {
            pool,
            task_id,
            max_age_seconds,
        } => plan(p, pool, task_id.as_deref(), *max_age_seconds),
        QuotaCommand::Reserve {
            task_id,
            pool,
            unit,
            amount,
            limit,
            policy,
            idempotency_key,
        } => reserve(
            p,
            task_id,
            pool,
            unit,
            *amount,
            *limit,
            policy,
            idempotency_key,
        ),
        QuotaCommand::Release { reservation } => {
            owner()?;
            let mut db = connect(p)?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM quota_reservations WHERE id=?1)",
                [reservation],
                |r| r.get(0),
            )?;
            if !exists {
                return Err(Error::new(
                    "RESERVATION_NOT_FOUND",
                    "Reservation does not exist",
                    6,
                ));
            }
            let changed = tx.execute(
                "UPDATE quota_reservations SET status='released' WHERE id=?1 AND status='active'",
                [reservation],
            )?;
            if changed > 0 {
                event(
                    &tx,
                    reservation,
                    "budget_released",
                    json!({"reservation_id":reservation}),
                )?;
            }
            let state: String = tx.query_row(
                "SELECT status FROM quota_reservations WHERE id=?1",
                [reservation],
                |r| r.get(0),
            )?;
            tx.commit()?;
            Ok(json!({"reservation_id":reservation,"state":state,"model_started":false}))
        }
    }
}

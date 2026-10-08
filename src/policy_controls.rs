//! Owner-bound persistent restrictions and narrowly scoped reporting exceptions.
//! The caller owns authentication, a single control-DB transaction and delivery.
//! The local principal label is a trusted transport label, not OS authentication.
use crate::{
    domain::{Error, Result, hash, id, now},
    project::Project,
    reader,
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAX_POLICIES: i64 = 4096;
fn invalid() -> Error {
    Error::new("INVALID_ARGUMENT", "Invalid reporting policy input", 2)
}
fn denied() -> Error {
    Error::new("POLICY_DENIED", "Original trusted policy owner required", 5)
}
fn conflict() -> Error {
    Error::new(
        "POLICY_CONFLICT",
        "Restriction authority requires explicit resolution",
        5,
    )
}
fn stale() -> Error {
    Error::new("REVISION_CONFLICT", "Policy revision changed", 9)
}
fn unavailable() -> Error {
    Error::new(
        "POLICY_UNAVAILABLE",
        "Policy metadata unavailable or exceeds bounds",
        7,
    )
}
fn label(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > 256
        || value.chars().any(char::is_control)
        || reader::redact(value).1
    {
        return Err(invalid());
    }
    Ok(())
}
fn evidence(value: &str) -> Result<String> {
    if value.trim().is_empty() || value.len() > 4096 {
        return Err(invalid());
    }
    Ok(hash(value.as_bytes()))
}
fn parsed<T: for<'a> Deserialize<'a>>(input: &Value) -> Result<T> {
    if serde_json::to_vec(input)?.len() > 65536 {
        return Err(invalid());
    }
    Ok(serde_json::from_value(input.clone())?)
}
fn env(name: &str, default: &str) -> Result<String> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(default.into()),
        Err(_) => Err(denied()),
    }
}
/// Trusted local operator provenance only. Agent run credentials never become
/// owner authority merely by selecting the owner actor label.
pub fn trusted_principal() -> Result<String> {
    if env("PCTX_ACTOR", "owner")? != "owner"
        || !env("PCTX_RUN_ID", "")?.is_empty()
        || !env("PCTX_RUN_CAPABILITY", "")?.is_empty()
    {
        return Err(denied());
    }
    let value = env("PCTX_OWNER_PRINCIPAL", "owner")?;
    label(&value)?;
    Ok(value)
}
pub(crate) fn install_triggers(db: &Connection) -> Result<()> {
    db.execute_batch(r#"CREATE TRIGGER IF NOT EXISTS ops_restrictions_identity BEFORE UPDATE ON ops_restrictions WHEN NEW.id!=OLD.id OR NEW.role!=OLD.role OR NEW.recipient!=OLD.recipient OR NEW.topic!=OLD.topic OR (OLD.original_owner IS NOT NULL AND (NEW.original_owner IS NULL OR NEW.original_owner!=OLD.original_owner)) OR NEW.revision!=OLD.revision+1 BEGIN SELECT RAISE(ABORT,'immutable restriction identity'); END;
CREATE TRIGGER IF NOT EXISTS ops_restrictions_no_delete BEFORE DELETE ON ops_restrictions BEGIN SELECT RAISE(ABORT,'persistent restriction'); END;
CREATE TRIGGER IF NOT EXISTS ops_reporting_exceptions_identity BEFORE UPDATE ON ops_reporting_exceptions WHEN NEW.id!=OLD.id OR NEW.original_owner!=OLD.original_owner OR NEW.revision!=OLD.revision+1 BEGIN SELECT RAISE(ABORT,'immutable exception identity'); END;
CREATE TRIGGER IF NOT EXISTS ops_reporting_exceptions_no_delete BEFORE DELETE ON ops_reporting_exceptions BEGIN SELECT RAISE(ABORT,'persistent reporting exception'); END;"#)?;
    Ok(())
}
fn table(db: &Connection, name: &str) -> Result<bool> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )?)
}
fn bounded(db: &Connection, name: &str) -> Result<()> {
    // Names are internal constants, never caller data.
    let n: i64 = db.query_row(
        &format!("SELECT COUNT(*) FROM (SELECT 1 FROM {name} LIMIT 4097)"),
        [],
        |r| r.get(0),
    )?;
    if n > MAX_POLICIES {
        return Err(unavailable());
    }
    Ok(())
}
fn event(p: &Project, db: &Connection, role: &str, principal: &str, data: &Value) -> Result<()> {
    p.check_deadline()?;
    db.execute("INSERT INTO ops_events(entity,kind,actor,payload,created) VALUES(?1,'role_policy_changed',?2,?3,?4)",params![role,principal,data.to_string(),now()])?;
    p.check_deadline()
}
/// Must run inside the caller's mutation transaction after base operations
/// schema initialization. Legacy rows have unknown owners, never guessed ones.
pub fn ensure_schema(p: &Project, db: &Connection) -> Result<()> {
    p.check_deadline()?;
    if db.is_autocommit() {
        return Err(unavailable());
    }
    for name in ["ops_roles", "ops_silences", "ops_events"] {
        if !table(db, name)? {
            return Err(unavailable());
        }
    }
    if table(db, "ops_policy_schema")? {
        let versions: Vec<i64> = db
            .prepare("SELECT version FROM ops_policy_schema LIMIT 2")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if versions != [1] {
            return Err(unavailable());
        }
        for name in ["ops_restrictions", "ops_reporting_exceptions"] {
            if !table(db, name)? {
                return Err(unavailable());
            }
        }
    } else {
        if table(db, "ops_restrictions")? || table(db, "ops_reporting_exceptions")? {
            return Err(unavailable());
        }
        db.execute_batch("CREATE TABLE ops_policy_schema(version INTEGER PRIMARY KEY CHECK(version=1));
INSERT INTO ops_policy_schema(version) VALUES(1);
CREATE TABLE ops_restrictions(id TEXT PRIMARY KEY,role TEXT NOT NULL,recipient TEXT NOT NULL,topic TEXT NOT NULL,original_owner TEXT,revision INTEGER NOT NULL,active INTEGER NOT NULL,evidence_hash TEXT NOT NULL,provenance TEXT NOT NULL,UNIQUE(role,recipient,topic));
CREATE TABLE ops_reporting_exceptions(id TEXT PRIMARY KEY,original_owner TEXT NOT NULL,revision INTEGER NOT NULL,active INTEGER NOT NULL,role TEXT NOT NULL,recipient TEXT NOT NULL,topic TEXT NOT NULL,category TEXT NOT NULL,policy_json TEXT NOT NULL,evidence_hash TEXT NOT NULL,reason_hash TEXT NOT NULL,created INTEGER NOT NULL);
")?;
    }
    install_triggers(db)?;
    for name in [
        "ops_roles",
        "ops_silences",
        "ops_restrictions",
        "ops_reporting_exceptions",
    ] {
        bounded(db, name)?;
        p.check_deadline()?;
    }
    let mut rows = Vec::new();
    {
        let mut query=db.prepare("SELECT CASE WHEN length(CAST(role AS BLOB))<=256 THEN role END,'*','',paused FROM ops_roles UNION ALL SELECT CASE WHEN length(CAST(role AS BLOB))<=256 THEN role END,CASE WHEN length(CAST(recipient AS BLOB))<=256 THEN recipient END,CASE WHEN length(CAST(topic AS BLOB))<=256 THEN topic END,active FROM ops_silences LIMIT 4097")?;
        for row in query.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, bool>(3)?,
            ))
        })? {
            p.check_deadline()?;
            rows.push(row?);
        }
    }
    if rows.len() > MAX_POLICIES as usize {
        return Err(unavailable());
    }
    for (role, recipient, topic, active) in rows {
        label(&role)?;
        label(&recipient)?;
        if !topic.is_empty() {
            label(&topic)?;
        }
        db.execute("INSERT OR IGNORE INTO ops_restrictions(id,role,recipient,topic,original_owner,revision,active,evidence_hash,provenance) VALUES(?1,?2,?3,?4,NULL,1,?5,'','legacy_unknown')",params![id("restriction"),role,recipient,topic,active])?;
        p.check_deadline()?;
    }
    bounded(db, "ops_restrictions")?;
    p.check_deadline()
}
#[derive(Clone)]
struct Restriction {
    id: String,
    role: String,
    recipient: String,
    topic: String,
    owner: Option<String>,
    revision: i64,
    active: bool,
}
fn restriction(db: &Connection, identifier: &str) -> Result<Restriction> {
    label(identifier)?;
    db.query_row("SELECT id,role,recipient,topic,original_owner,revision,active FROM ops_restrictions WHERE id=?1",[identifier],row_restriction).optional()?.ok_or_else(conflict)
}
fn row_restriction(r: &rusqlite::Row<'_>) -> rusqlite::Result<Restriction> {
    Ok(Restriction {
        id: r.get(0)?,
        role: r.get(1)?,
        recipient: r.get(2)?,
        topic: r.get(3)?,
        owner: r.get(4)?,
        revision: r.get(5)?,
        active: r.get(6)?,
    })
}
fn canonical_recipient(p: &Project, db: &Connection, value: &str) -> Result<String> {
    label(value)?;
    p.check_deadline()?;
    if !table(db, "agents")? {
        return Ok(value.into());
    }
    // Canonical IDs win over another agent's display alias.
    let found = db
        .query_row("SELECT id FROM agents WHERE id=?1", [value], |r| {
            r.get::<_, String>(0)
        })
        .optional()?;
    let result = match found {
        Some(id) => id,
        None => db
            .query_row("SELECT id FROM agents WHERE name=?1", [value], |r| {
                r.get::<_, String>(0)
            })
            .optional()?
            .unwrap_or_else(|| value.into()),
    };
    label(&result)?;
    p.check_deadline()?;
    Ok(result)
}
fn consistent(db: &Connection, r: &Restriction) -> Result<()> {
    let underlying: Option<bool> = if r.topic.is_empty() {
        db.query_row(
            "SELECT paused FROM ops_roles WHERE role=?1",
            [&r.role],
            |row| row.get(0),
        )
        .optional()?
    } else {
        db.query_row(
            "SELECT active FROM ops_silences WHERE role=?1 AND recipient=?2 AND topic=?3",
            params![r.role, r.recipient, r.topic],
            |row| row.get(0),
        )
        .optional()?
    };
    if underlying != Some(r.active) {
        return Err(conflict());
    }
    Ok(())
}
fn write_state(
    p: &Project,
    db: &Connection,
    r: &Restriction,
    active: bool,
    principal: &str,
    reason: &str,
) -> Result<()> {
    p.check_deadline()?;
    let reason = reader::redact(reason).0;
    if r.topic.is_empty() {
        db.execute("INSERT INTO ops_roles(role,paused,reason,actor,updated) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(role) DO UPDATE SET paused=excluded.paused,reason=excluded.reason,actor=excluded.actor,updated=excluded.updated",params![r.role,active,reason,principal,now()])?;
    } else {
        db.execute("INSERT INTO ops_silences(role,recipient,topic,active,reason,actor,updated) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(role,recipient,topic) DO UPDATE SET active=excluded.active,reason=excluded.reason,actor=excluded.actor,updated=excluded.updated",params![r.role,r.recipient,r.topic,active,reason,principal,now()])?;
    }
    p.check_deadline()
}
fn receipt(r: &Restriction) -> Value {
    json!({"restriction_id":r.id,"revision":r.revision,"active":r.active})
}
/// Existing lexical restriction grammar, without principal or recipient-state resolution.
pub fn validate_restriction_arguments(
    role: &str,
    recipient: Option<&str>,
    topic: Option<&str>,
    reason: &str,
) -> Result<()> {
    label(role)?;
    evidence(reason)?;
    if topic.is_none() && recipient.is_some() {
        return Err(invalid());
    }
    // The existing canonical-recipient parser accepts "*" as an ordinary label.
    // Its database-dependent alias resolution remains in change_restriction.
    label(recipient.unwrap_or("*"))?;
    if let Some(topic) = topic {
        label(topic)?;
    }
    Ok(())
}

/// Creates a new owner binding or changes the original owner's existing one.
/// A legacy owner must first be explicitly attested, even for an inactive row.
pub fn change_restriction(
    p: &Project,
    db: &Connection,
    role: &str,
    recipient: Option<&str>,
    topic: Option<&str>,
    active: bool,
    reason: &str,
) -> Result<Value> {
    p.check_deadline()?;
    let principal = trusted_principal()?;
    validate_restriction_arguments(role, recipient, topic, reason)?;
    let reason_hash = evidence(reason)?;
    let recipient = canonical_recipient(p, db, recipient.unwrap_or("*"))?;
    let topic = topic.unwrap_or("");
    if !topic.is_empty() {
        label(topic)?;
    } else if topic.is_empty() && recipient != "*" {
        return Err(invalid());
    }
    ensure_schema(p, db)?;
    let found=db.query_row("SELECT id,role,recipient,topic,original_owner,revision,active FROM ops_restrictions WHERE role=?1 AND recipient=?2 AND topic=?3",params![role,recipient,topic],row_restriction).optional()?;
    if found.is_none() && !active {
        p.check_deadline()?;
        return Ok(json!({"restriction_id":null,"revision":0,"active":false,"unchanged":true}));
    }
    let mut r = if let Some(mut r) = found {
        if r.owner.as_deref() != Some(&principal) {
            return Err(denied());
        }
        consistent(db, &r)?;
        let count=db.execute("UPDATE ops_restrictions SET active=?1,revision=revision+1,evidence_hash=?2,provenance='trusted_local_operator' WHERE id=?3 AND revision=?4",params![active,reason_hash,r.id,r.revision])?;
        if count != 1 {
            return Err(stale());
        }
        r.revision += 1;
        r.active = active;
        r
    } else {
        let count: i64 = db.query_row("SELECT COUNT(*) FROM ops_restrictions", [], |row| {
            row.get(0)
        })?;
        if count >= MAX_POLICIES {
            return Err(unavailable());
        }
        let r = Restriction {
            id: id("restriction"),
            role: role.into(),
            recipient,
            topic: topic.into(),
            owner: Some(principal.clone()),
            revision: 1,
            active,
        };
        db.execute("INSERT INTO ops_restrictions(id,role,recipient,topic,original_owner,revision,active,evidence_hash,provenance) VALUES(?1,?2,?3,?4,?5,1,?6,?7,'trusted_local_operator')",params![r.id,r.role,r.recipient,r.topic,principal,active,reason_hash])?;
        r
    };
    write_state(p, db, &r, active, &principal, reason)?;
    r.active = active;
    event(
        p,
        db,
        role,
        &principal,
        &json!({"restriction_id":r.id,"revision":r.revision,"active":active,"evidence_hash":reason_hash}),
    )?;
    Ok(receipt(&r))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Attestation {
    schema_version: u32,
    id: String,
    expected_revision: i64,
    owner_evidence: String,
    original_owner: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionInput {
    schema_version: u32,
    id: String,
    expected_revision: i64,
    owner_evidence: String,
}
pub fn attest_owner(p: &Project, db: &Connection, input: &Value) -> Result<Value> {
    p.check_deadline()?;
    let principal = trusted_principal()?;
    let request: Attestation = parsed(input)?;
    if request.schema_version != 1 || request.original_owner != principal {
        return Err(denied());
    }
    let evidence_hash = evidence(&request.owner_evidence)?;
    ensure_schema(p, db)?;
    let mut r = restriction(db, &request.id)?;
    consistent(db, &r)?;
    if r.revision != request.expected_revision {
        return Err(stale());
    }
    if r.owner.is_some() {
        return Err(denied());
    }
    if db.execute("UPDATE ops_restrictions SET original_owner=?1,revision=revision+1,evidence_hash=?2,provenance='trusted_local_owner_attestation' WHERE id=?3 AND revision=?4 AND original_owner IS NULL",params![principal,evidence_hash,r.id,r.revision])?!=1{return Err(stale());}
    r.revision += 1;
    event(
        p,
        db,
        &r.role,
        &principal,
        &json!({"restriction_id":r.id,"revision":r.revision,"attested":true,"evidence_hash":evidence_hash}),
    )?;
    Ok(receipt(&r))
}
pub fn release(p: &Project, db: &Connection, input: &Value) -> Result<Value> {
    p.check_deadline()?;
    let principal = trusted_principal()?;
    let request: RevisionInput = parsed(input)?;
    if request.schema_version != 1 {
        return Err(invalid());
    }
    let evidence_hash = evidence(&request.owner_evidence)?;
    ensure_schema(p, db)?;
    let mut r = restriction(db, &request.id)?;
    consistent(db, &r)?;
    if r.owner.as_deref() != Some(&principal) {
        return Err(denied());
    }
    if r.revision != request.expected_revision {
        return Err(stale());
    }
    if db.execute("UPDATE ops_restrictions SET active=0,revision=revision+1,evidence_hash=?1 WHERE id=?2 AND revision=?3",params![evidence_hash,r.id,r.revision])?!=1{return Err(stale());}
    r.revision += 1;
    r.active = false;
    write_state(
        p,
        db,
        &r,
        false,
        &principal,
        "Original owner released restriction",
    )?;
    event(
        p,
        db,
        &r.role,
        &principal,
        &json!({"restriction_id":r.id,"revision":r.revision,"active":false,"evidence_hash":evidence_hash}),
    )?;
    Ok(receipt(&r))
}
/// Owner-only local management metadata; reasons and input evidence are hashes.
pub fn list_restrictions(p: &Project, db: &Connection) -> Result<Value> {
    trusted_principal()?;
    p.check_deadline()?;
    if !table(db, "ops_policy_schema")? || !table(db, "ops_restrictions")? {
        return Err(unavailable());
    }
    let versions: Vec<i64> = db
        .prepare("SELECT version FROM ops_policy_schema LIMIT 2")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if versions != [1] {
        return Err(unavailable());
    }
    let mut query = db.prepare("SELECT CASE WHEN length(CAST(id AS BLOB))<=256 THEN id END,CASE WHEN length(CAST(role AS BLOB))<=256 THEN role END,CASE WHEN length(CAST(recipient AS BLOB))<=256 THEN recipient END,CASE WHEN length(CAST(topic AS BLOB))<=256 THEN topic END,CASE WHEN original_owner IS NULL OR length(CAST(original_owner AS BLOB))<=256 THEN original_owner END,revision,active,CASE WHEN length(CAST(provenance AS BLOB))<=256 THEN provenance END,CASE WHEN length(CAST(evidence_hash AS BLOB))<=64 THEN evidence_hash END FROM ops_restrictions ORDER BY id LIMIT 4097")?;
    let mut rows = Vec::new();
    for row in query.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, bool>(6)?,
            r.get::<_, String>(7)?,
            r.get::<_, String>(8)?,
        ))
    })? {
        p.check_deadline()?;
        let (
            id,
            role,
            recipient,
            topic,
            original_owner,
            revision,
            active,
            provenance,
            evidence_hash,
        ) = row?;
        label(&id)?;
        label(&role)?;
        label(&recipient)?;
        if !topic.is_empty() {
            label(&topic)?;
        }
        if let Some(owner) = &original_owner {
            label(owner)?;
        }
        if provenance.len() > 256 || evidence_hash.len() > 64 || revision < 1 {
            return Err(unavailable());
        }
        rows.push(json!({"id":id,"role":role,"recipient":recipient,"topic":topic,"original_owner":original_owner,"revision":revision,"active":active,"provenance":provenance,"evidence_hash":evidence_hash}));
    }
    if rows.len() > MAX_POLICIES as usize {
        return Err(unavailable());
    }
    p.check_deadline()?;
    Ok(json!({"schema_version":1,"restrictions":rows}))
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Category {
    ApprovalPrompt,
    PermissionIncident,
    Deadline,
    Security,
}
impl Category {
    fn key(&self) -> &'static str {
        match self {
            Self::ApprovalPrompt => "approval_prompt",
            Self::PermissionIncident => "permission_incident",
            Self::Deadline => "deadline",
            Self::Security => "security",
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Precedence {
    Exception,
    Restriction,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RestrictionRef {
    id: String,
    revision: i64,
    #[serde(default)]
    precedence: Option<Precedence>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExceptionPolicy {
    schema_version: u32,
    restriction_refs: Vec<RestrictionRef>,
    role: String,
    recipient: String,
    topic: String,
    source_scope: Vec<String>,
    payload_hash: String,
    category: Category,
    not_before: i64,
    expires_at: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionInput {
    schema_version: u32,
    restriction_refs: Vec<RestrictionRef>,
    role: String,
    recipient: String,
    topic: String,
    source_scope: Vec<String>,
    payload_hash: String,
    category: Category,
    not_before: i64,
    expires_at: i64,
    reason: String,
    owner_evidence: String,
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn relative(path: &str) -> Result<()> {
    if path.is_empty()
        || path.len() > 4096
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path.chars().any(char::is_control)
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(invalid());
    }
    Ok(())
}
fn scopes(values: &[String]) -> Result<GlobSet> {
    if values.is_empty() || values.len() > 32 {
        return Err(invalid());
    }
    let mut builder = GlobSetBuilder::new();
    for value in values {
        if value.len() > 256 || reader::redact(value).1 {
            return Err(invalid());
        }
        relative(value)?;
        builder.add(Glob::new(value).map_err(|_| invalid())?);
    }
    builder.build().map_err(|_| invalid())
}
fn validate_policy(policy: &ExceptionPolicy) -> Result<()> {
    if policy.schema_version != 1
        || policy.restriction_refs.is_empty()
        || policy.restriction_refs.len() > 64
        || policy.expires_at <= policy.not_before
        || policy
            .expires_at
            .checked_sub(policy.not_before)
            .is_none_or(|n| n > 30 * 24 * 60 * 60)
    {
        return Err(invalid());
    }
    if !valid_hash(&policy.payload_hash) {
        return Err(invalid());
    }
    label(&policy.role)?;
    label(&policy.recipient)?;
    label(&policy.topic)?;
    scopes(&policy.source_scope)?;
    let mut seen = std::collections::BTreeSet::new();
    for r in &policy.restriction_refs {
        label(&r.id)?;
        if r.revision < 1 || !seen.insert(&r.id) {
            return Err(invalid());
        }
    }
    Ok(())
}
pub fn record_exception(p: &Project, db: &Connection, input: &Value) -> Result<Value> {
    p.check_deadline()?;
    let principal = trusted_principal()?;
    let request: ExceptionInput = parsed(input)?;
    let evidence_hash = evidence(&request.owner_evidence)?;
    let reason_hash = evidence(&request.reason)?;
    let policy = ExceptionPolicy {
        schema_version: request.schema_version,
        restriction_refs: request.restriction_refs,
        role: request.role,
        recipient: canonical_recipient(p, db, &request.recipient)?,
        topic: request.topic,
        source_scope: request.source_scope,
        payload_hash: request.payload_hash,
        category: request.category,
        not_before: request.not_before,
        expires_at: request.expires_at,
    };
    validate_policy(&policy)?;
    let timestamp = now();
    if policy.expires_at <= timestamp
        || policy
            .expires_at
            .checked_sub(timestamp)
            .is_none_or(|n| n > 30 * 24 * 60 * 60)
    {
        return Err(invalid());
    }
    ensure_schema(p, db)?;
    for reference in &policy.restriction_refs {
        p.check_deadline()?;
        let r = restriction(db, &reference.id)?;
        consistent(db, &r)?;
        if r.owner.as_deref() != Some(&principal) {
            return Err(denied());
        }
        if !r.active || r.revision != reference.revision {
            return Err(stale());
        }
        if !(r.role == "*" || r.role == policy.role)
            || !(r.recipient == "*" || r.recipient == policy.recipient)
        {
            return Err(conflict());
        }
    }
    let count: i64 = db.query_row("SELECT COUNT(*) FROM ops_reporting_exceptions", [], |r| {
        r.get(0)
    })?;
    if count >= MAX_POLICIES {
        return Err(unavailable());
    }
    let identifier = id("report-exception");
    db.execute("INSERT INTO ops_reporting_exceptions(id,original_owner,revision,active,role,recipient,topic,category,policy_json,evidence_hash,reason_hash,created) VALUES(?1,?2,1,1,?3,?4,?5,?6,?7,?8,?9,?10)",params![identifier,principal,policy.role,policy.recipient,policy.topic,policy.category.key(),serde_json::to_string(&policy)?,evidence_hash,reason_hash,timestamp])?;
    event(
        p,
        db,
        &policy.role,
        &principal,
        &json!({"exception_id":identifier,"revision":1,"active":true,"evidence_hash":evidence_hash,"reason_hash":reason_hash}),
    )?;
    Ok(json!({"exception_id":identifier,"revision":1,"active":true}))
}
pub fn revoke_exception(p: &Project, db: &Connection, input: &Value) -> Result<Value> {
    p.check_deadline()?;
    let principal = trusted_principal()?;
    let request: RevisionInput = parsed(input)?;
    if request.schema_version != 1 {
        return Err(invalid());
    }
    label(&request.id)?;
    let evidence_hash = evidence(&request.owner_evidence)?;
    ensure_schema(p, db)?;
    let (owner, revision, role): (String, i64, String) = db
        .query_row(
            "SELECT original_owner,revision,role FROM ops_reporting_exceptions WHERE id=?1",
            [&request.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(conflict)?;
    if owner != principal {
        return Err(denied());
    }
    if revision != request.expected_revision {
        return Err(stale());
    }
    if db.execute("UPDATE ops_reporting_exceptions SET active=0,revision=revision+1,evidence_hash=?1 WHERE id=?2 AND revision=?3",params![evidence_hash,request.id,revision])?!=1{return Err(stale());}
    event(
        p,
        db,
        &role,
        &principal,
        &json!({"exception_id":request.id,"revision":revision+1,"active":false,"evidence_hash":evidence_hash}),
    )?;
    Ok(json!({"exception_id":request.id,"revision":revision+1,"active":false}))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportInput {
    schema_version: u32,
    role: String,
    recipient: String,
    topic: String,
    category: Category,
    source_paths: Vec<String>,
    payload_hash: String,
}
pub(crate) struct ReportAdmission {
    pub(crate) data: Value,
    pub(crate) valid_until: Option<i64>,
    pub(crate) source_hash: Option<String>,
}
fn decision(p: &Project, state: &str, code: &str, facts: &Value) -> Result<ReportAdmission> {
    p.check_deadline()?;
    if p.current_policy_hash()?
        .as_ref()
        .is_some_and(|current| current != &p.policy_hash())
    {
        return Err(unavailable());
    }
    Ok(ReportAdmission {
        data: json!({"state":state,"reason_code":code,"fingerprint":hash(serde_json::to_vec(&json!({"state":state,"reason_code":code,"facts":facts}))?)}),
        valid_until: None,
        source_hash: None,
    })
}
/// Read-only reporting admission, never execution authority or policy release.
/// Every conflicting restriction must have a current explicit higher-priority
/// exception covering every declared source. A pause remains active for claims,
/// wakes and schedules; this method can only authorize a reporting delivery.
/// Source content is never inspected.
pub fn evaluate_report(p: &Project, db: &Connection, input: &Value) -> Result<Value> {
    Ok(report_admission(p, db, input)?.data)
}
pub(crate) fn report_admission(
    p: &Project,
    db: &Connection,
    input: &Value,
) -> Result<ReportAdmission> {
    p.check_deadline()?;
    let request: ReportInput = parsed(input)?;
    if request.schema_version != 1
        || request.source_paths.is_empty()
        || request.source_paths.len() > 64
        || !valid_hash(&request.payload_hash)
    {
        return Err(invalid());
    }
    label(&request.role)?;
    label(&request.topic)?;
    let classifier = crate::source_delivery::SourceClassifier::new(p)?;
    // Reader excludes are checked before reading managed declaration metadata.
    for path in &request.source_paths {
        relative(path)?;
        if reader::policy_allows(p, path).is_err() {
            p.check_deadline()?;
            return decision(
                p,
                "held",
                "SOURCE_POLICY_DENIED",
                &json!({"request":hash(serde_json::to_vec(input)?)}),
            );
        }
    }
    let (mut topics, unknown, source_hash) =
        report_source_classes(p, &classifier, &request.source_paths)?;
    topics.insert(request.topic.clone());
    let mut admission =
        evaluate_classified_report(p, db, input, &request, &topics, unknown, &source_hash)?;
    classifier.revalidate()?;
    if report_source_classes(p, &classifier, &request.source_paths)?.2 != source_hash {
        return Err(Error::new(
            "CONCURRENT_MODIFICATION",
            "Reporting source classification changed",
            4,
        ));
    }
    classifier.revalidate()?;
    if admission.valid_until.is_some_and(|end| end <= now()) {
        return decision(
            p,
            "held",
            "POLICY_CONFLICT",
            &json!({"request":hash(serde_json::to_vec(input)?),"source_classification_hash":source_hash}),
        );
    }
    p.check_deadline()?;
    admission.source_hash = Some(source_hash);
    Ok(admission)
}
pub(crate) fn report_source_binding(p: &Project, paths: &[String]) -> Result<String> {
    let classifier = crate::source_delivery::SourceClassifier::new(p)?;
    let result = report_source_classes(p, &classifier, paths)?.2;
    classifier.revalidate()?;
    Ok(result)
}
fn report_source_classes(
    p: &Project,
    classifier: &crate::source_delivery::SourceClassifier<'_>,
    paths: &[String],
) -> Result<(std::collections::BTreeSet<String>, bool, String)> {
    let mut topics = std::collections::BTreeSet::new();
    let mut unknown = false;
    let mut entries = Vec::new();
    for path in paths {
        p.check_deadline()?;
        let declared = crate::documents::source_topics(p, path).map_err(|error| {
            if error.code == "TIMEOUT" {
                error
            } else {
                unavailable()
            }
        })?;
        let labels = classifier.labels(path, declared.as_deref())?;
        match &labels {
            None => unknown = true,
            Some(labels) => topics.extend(labels.iter().cloned()),
        }
        entries.push(json!({"path_hash":hash(path.as_bytes()),"labels":labels}));
    }
    p.check_deadline()?;
    Ok((topics, unknown, hash(serde_json::to_vec(&entries)?)))
}
#[allow(clippy::too_many_arguments)]
fn evaluate_classified_report(
    p: &Project,
    db: &Connection,
    input: &Value,
    request: &ReportInput,
    topics: &std::collections::BTreeSet<String>,
    unknown: bool,
    source_hash: &str,
) -> Result<ReportAdmission> {
    let recipient = canonical_recipient(p, db, &request.recipient)?;
    for path in &request.source_paths {
        relative(path)?;
    }
    reader::validate_root(p)?;
    if p.current_policy_hash()?
        .as_ref()
        .is_some_and(|current| current != &p.policy_hash())
    {
        return Err(unavailable());
    }
    let request_hash = hash(serde_json::to_vec(input)?);
    for path in &request.source_paths {
        if reader::policy_allows(p, path).is_err() {
            p.check_deadline()?;
            return decision(
                p,
                "held",
                "SOURCE_POLICY_DENIED",
                &json!({"request":request_hash}),
            );
        }
    }
    if !table(db, "ops_roles")? || !table(db, "ops_silences")? {
        return Err(unavailable());
    }
    if table(db, "ops_policy_schema")? {
        let versions: Vec<i64> = db
            .prepare("SELECT version FROM ops_policy_schema LIMIT 2")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if versions != [1]
            || !table(db, "ops_restrictions")?
            || !table(db, "ops_reporting_exceptions")?
        {
            return Err(unavailable());
        }
    }
    let watermark: i64 = if table(db, "ops_events")? {
        db.query_row("SELECT COALESCE(MAX(seq),0) FROM ops_events WHERE kind='role_policy_changed' AND (entity=?1 OR entity='*')",[&request.role],|r|r.get(0))?
    } else {
        0
    };
    let base = json!({"request":request_hash,"policy":p.policy_hash(),"workspace":hash(p.workspace_id.as_bytes()),"revision":watermark,"source_classification_hash":source_hash});
    if unknown && db.query_row("SELECT EXISTS(SELECT 1 FROM ops_silences WHERE active=1 AND (role=?1 OR role='*') AND (recipient=?2 OR recipient='*'))", params![request.role, recipient], |r| r.get::<_,bool>(0))? {
        return decision(p, "held", "SOURCE_TOPIC_REQUIRED", &base);
    }
    let topic_json = serde_json::to_string(topics)?;
    let count:i64=db.query_row("SELECT COUNT(*) FROM (SELECT 1 FROM ops_silences WHERE active=1 AND (role=?1 OR role='*') AND (recipient=?2 OR recipient='*') AND topic IN (SELECT value FROM json_each(?3)) UNION ALL SELECT 1 FROM ops_roles WHERE paused=1 AND (role=?1 OR role='*') LIMIT 4097)",params![request.role,recipient,topic_json],|r|r.get(0))?;
    if count > MAX_POLICIES {
        return Err(unavailable());
    }
    if count == 0 {
        return decision(p, "allowed", "NO_REPORTING_CONFLICT", &base);
    }
    if !table(db, "ops_policy_schema")?
        || !table(db, "ops_restrictions")?
        || !table(db, "ops_reporting_exceptions")?
    {
        return decision(p, "held", "POLICY_CONFLICT", &base);
    }
    let versions: Vec<i64> = db
        .prepare("SELECT version FROM ops_policy_schema LIMIT 2")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if versions != [1] {
        return Err(unavailable());
    }
    let mut conflicts = Vec::new();
    let mut query=db.prepare("SELECT CASE WHEN length(CAST(m.id AS BLOB))<=256 THEN m.id END,m.role,m.recipient,m.topic,CASE WHEN length(CAST(m.original_owner AS BLOB))<=256 THEN m.original_owner END,m.revision,m.active FROM (SELECT role,recipient,topic FROM ops_silences WHERE active=1 AND (role=?1 OR role='*') AND (recipient=?2 OR recipient='*') AND topic IN (SELECT value FROM json_each(?3)) UNION ALL SELECT role,'*','' FROM ops_roles WHERE paused=1 AND (role=?1 OR role='*')) s LEFT JOIN ops_restrictions m ON m.role=s.role AND m.recipient=s.recipient AND m.topic=s.topic LIMIT 4097")?;
    let rows = query.query_map(params![request.role, recipient, topic_json], |row| {
        if row.get::<_, Option<String>>(0)?.is_none() {
            Ok(None)
        } else {
            Ok(Some(row_restriction(row)?))
        }
    })?;
    for row in rows {
        p.check_deadline()?;
        match row? {
            Some(r) if r.active && r.owner.is_some() => conflicts.push(r),
            _ => return decision(p, "held", "POLICY_CONFLICT", &base),
        }
    }
    let mut candidates=db.prepare("SELECT CASE WHEN length(CAST(id AS BLOB))<=256 THEN id END,CASE WHEN length(CAST(original_owner AS BLOB))<=256 THEN original_owner END,revision,CASE WHEN length(CAST(policy_json AS BLOB))<=65536 THEN policy_json END FROM ops_reporting_exceptions WHERE active=1 AND role=?1 AND recipient=?2 AND topic=?3 AND category=?4 LIMIT 4097")?;
    let rows = candidates.query_map(
        params![
            request.role,
            recipient,
            request.topic,
            request.category.key()
        ],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        },
    )?;
    let timestamp = now();
    let mut coverage = std::collections::BTreeSet::new();
    let mut applied = Vec::new();
    let mut observed = 0;
    for row in rows {
        p.check_deadline()?;
        observed += 1;
        if observed > MAX_POLICIES {
            return Err(unavailable());
        }
        let (identifier, owner, revision, body) = row?;
        if body.len() > 65536 {
            return Err(unavailable());
        }
        let policy: ExceptionPolicy = serde_json::from_str(&body).map_err(|_| unavailable())?;
        validate_policy(&policy).map_err(|_| unavailable())?;
        if policy.role != request.role
            || policy.recipient != recipient
            || policy.topic != request.topic
            || policy.category.key() != request.category.key()
        {
            return Err(unavailable());
        }
        if policy.payload_hash != request.payload_hash {
            continue;
        }
        if timestamp < policy.not_before || timestamp >= policy.expires_at {
            continue;
        }
        let scope = scopes(&policy.source_scope)?;
        if !request.source_paths.iter().all(|path| scope.is_match(path)) {
            continue;
        }
        // One stale reference invalidates the entire exception; an old approval
        // cannot be reused after any referenced restriction changes or releases.
        let mut current = true;
        for reference in &policy.restriction_refs {
            p.check_deadline()?;
            let r = restriction(db, &reference.id)?;
            if !r.active || r.revision != reference.revision || r.owner.as_deref() != Some(&owner) {
                current = false;
                break;
            }
            consistent(db, &r)?;
        }
        if !current {
            continue;
        }
        for reference in &policy.restriction_refs {
            if conflicts
                .iter()
                .any(|r| r.id == reference.id && r.revision == reference.revision)
            {
                if !matches!(reference.precedence, Some(Precedence::Exception)) {
                    return decision(p, "held", "POLICY_CONFLICT", &base);
                }
                coverage.insert(reference.id.clone());
            }
        }
        applied.push(json!({"id":identifier,"revision":revision,"expires_at":policy.expires_at}));
    }
    // Recheck wall-clock validity after all bounded database/scope work.
    // A receipt cannot preserve authority across an expiry during evaluation.
    let completed_at = now();
    if applied.iter().any(|entry| {
        entry["expires_at"]
            .as_i64()
            .is_none_or(|end| end <= completed_at)
    }) {
        return decision(p, "held", "POLICY_CONFLICT", &base);
    }
    if conflicts.iter().any(|r| !coverage.contains(&r.id)) {
        return decision(p, "held", "POLICY_CONFLICT", &base);
    }
    let mut result = decision(
        p,
        "allowed",
        "EXPLICIT_REPORTING_EXCEPTION",
        &json!({"base":base,"exceptions":applied,"restrictions":conflicts.iter().map(|r|json!({"id":r.id,"revision":r.revision})).collect::<Vec<_>>()}),
    )?;
    result.valid_until = applied
        .iter()
        .filter_map(|entry| entry["expires_at"].as_i64())
        .min();
    Ok(result)
}

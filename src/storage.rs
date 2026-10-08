use crate::{
    deadline::Deadline,
    domain::{Error, FileEntry, Result, hash, id, now},
    project::Project,
    reader,
};
use fs2::FileExt;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};

fn bounded_project(p: &Project, milliseconds: u64) -> Result<Project> {
    let mut p = p.clone();
    if p.deadline.is_none() {
        p.deadline = Some(Deadline::from_millis(milliseconds)?);
    }
    p.check_deadline()?;
    Ok(p)
}
fn sqlite_budget(p: &Project, db: &rusqlite::Connection) -> Result<()> {
    let wait = p.remaining(Duration::from_secs(5))?;
    // SQLite truncates timeout to milliseconds. Round up by less than 1ms so
    // request exhaustion is classified as TIMEOUT rather than early INDEX_BUSY.
    db.busy_timeout(wait.saturating_add(Duration::from_nanos(999_999)))?;
    Ok(())
}
fn sql_error(p: &Project, e: rusqlite::Error) -> Error {
    p.check_deadline().err().unwrap_or_else(|| e.into())
}
fn connect(p: &Project) -> Result<rusqlite::Connection> {
    p.check_deadline()?;
    let db = p
        .connect(false)
        .map_err(|e| p.check_deadline().err().unwrap_or(e))?;
    sqlite_budget(p, &db)?;
    p.check_deadline()?;
    Ok(db)
}

fn writer(p: &Project) -> Result<fs::File> {
    p.check_deadline()?;
    let f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(p.workspace_dir.join("writer.lock"))?;
    let start = Instant::now();
    loop {
        p.check_deadline()?;
        if f.try_lock_exclusive().is_ok() {
            p.check_deadline()?;
            return Ok(f);
        }
        if p.deadline.is_none() && start.elapsed() > Duration::from_secs(5) {
            return Err(Error::new("INDEX_BUSY", "Workspace writer lock busy", 7));
        }
        std::thread::sleep(p.remaining(Duration::from_millis(10))?);
    }
}
fn migrate(p: &Project, db: &mut rusqlite::Connection) -> Result<()> {
    p.check_deadline()?;
    sqlite_budget(p, db)?;
    // Acquire writer admission before schema reads. A deferred read-to-write
    // upgrade can fail immediately without consuming SQLite's busy budget.
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| sql_error(p, e))?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS workspace_meta(project_id TEXT NOT NULL,workspace_id TEXT NOT NULL,active_generation_id TEXT);
CREATE TABLE IF NOT EXISTS generations(id TEXT PRIMARY KEY,parent_id TEXT,state TEXT NOT NULL,started_at INTEGER,completed_at INTEGER,policy_hash TEXT,parser_set_hash TEXT);
CREATE TABLE IF NOT EXISTS file_versions(id TEXT PRIMARY KEY,path TEXT NOT NULL,content_hash TEXT NOT NULL,parser_hash TEXT NOT NULL,metadata TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS generation_files(generation_id TEXT REFERENCES generations(id),path TEXT NOT NULL,file_version_id TEXT REFERENCES file_versions(id),PRIMARY KEY(generation_id,path));
CREATE INDEX IF NOT EXISTS file_path ON file_versions(path,content_hash);
CREATE TABLE IF NOT EXISTS checkpoints(id TEXT PRIMARY KEY,name TEXT UNIQUE,manifest TEXT NOT NULL,manifest_hash TEXT NOT NULL,created_at INTEGER,pinned INTEGER NOT NULL DEFAULT 0);
PRAGMA user_version=1;").map_err(|e| sql_error(p, e))?;
    p.check_deadline()?;
    sqlite_budget(p, &tx)?;
    tx.commit().map_err(|e| sql_error(p, e))?;
    Ok(())
}
pub const PARSER_SET: &str = "tree-sitter-0.25/python-0.25/js-0.25/ts-0.23/md-heading-v1";
pub fn update(p: &Project) -> Result<Value> {
    let bounded = bounded_project(p, 120000)?;
    let p = &bounded;
    let _lock = writer(p)?;
    let mut db = connect(p)?;
    migrate(p, &mut db)?;
    let meta: Option<(String, String, Option<String>)> = db
        .query_row(
            "SELECT project_id,workspace_id,active_generation_id FROM workspace_meta",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| sql_error(p, e))?;
    if let Some((project, ws, _)) = &meta
        && (project != &p.project_id || ws != &p.workspace_id)
    {
        return Err(Error::new(
            "WORKSPACE_MISMATCH",
            "Index belongs to another workspace",
            9,
        ));
    }
    let inventory = reader::inventory(p, false)?;
    p.check_deadline()?;
    if inventory.skipped.iter().any(|s| s["reason"] == "TIMEOUT") {
        return Err(Error::new("TIMEOUT", "Inventory deadline expired", 7));
    }
    let mut skipped = inventory.skipped;
    let mut entries = Vec::new();
    let mut reused = 0;
    let parser_hash = hash(format!(
        "{PARSER_SET}:{}:{}",
        p.policy_hash(),
        serde_json::to_string(&p.config.index).unwrap_or_default()
    ));
    for path in inventory.paths {
        p.check_deadline()?;
        let file = match reader::read(p, &path) {
            Ok(f) => f,
            Err(e) if e.code == "TIMEOUT" => return Err(e),
            Err(e) => {
                skipped.push(json!({"path":path,"reason":e.code}));
                continue;
            }
        };
        p.check_deadline()?;
        let key = hash(format!("{}:{}:{}", path, file.hash, parser_hash));
        sqlite_budget(p, &db)?;
        let cached: Option<String> = db
            .query_row(
                "SELECT metadata FROM file_versions WHERE id=?1",
                [&key],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| sql_error(p, e))?;
        let entry = if let Some(c) = cached {
            reused += 1;
            serde_json::from_str::<FileEntry>(&c)?
        } else {
            crate::search::analyze_with_deadline(&path, &file.hash, &file.text, p.deadline)?
        };
        // Names and paths are metadata too: exclude any item changed by masking.
        if reader::redact(&entry.path).1 {
            skipped.push(json!({"reason":"sensitive_metadata"}));
            continue;
        }
        let mut entry = entry;
        entry
            .symbols
            .retain(|s| !reader::redact(&s.name).1 && !reader::redact(&s.qualified_name).1);
        entries.push((key, entry));
    }
    p.check_deadline()?;
    let generation = id("GEN");
    let parent = meta.and_then(|(_, _, g)| g);
    sqlite_budget(p, &db)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| sql_error(p, e))?;
    tx.execute(
        "INSERT INTO generations VALUES(?1,?2,'building',?3,NULL,?4,?5)",
        params![generation, parent, now(), p.policy_hash(), parser_hash],
    )
    .map_err(|e| sql_error(p, e))?;
    for (key, e) in &entries {
        p.check_deadline()?;
        tx.execute(
            "INSERT OR IGNORE INTO file_versions VALUES(?1,?2,?3,?4,?5)",
            params![
                key,
                e.path,
                e.file_hash,
                parser_hash,
                serde_json::to_string(e)?
            ],
        )
        .map_err(|e| sql_error(p, e))?;
        tx.execute(
            "INSERT INTO generation_files VALUES(?1,?2,?3)",
            params![generation, e.path, key],
        )
        .map_err(|e| sql_error(p, e))?;
    }
    tx.execute(
        "UPDATE generations SET state='ready',completed_at=?1 WHERE id=?2",
        params![now(), generation],
    )
    .map_err(|e| sql_error(p, e))?;
    tx.execute("DELETE FROM workspace_meta", [])
        .map_err(|e| sql_error(p, e))?;
    tx.execute(
        "INSERT INTO workspace_meta VALUES(?1,?2,?3)",
        params![p.project_id, p.workspace_id, generation],
    )
    .map_err(|e| sql_error(p, e))?;
    p.check_deadline()?;
    sqlite_budget(p, &tx)?;
    tx.commit().map_err(|e| sql_error(p, e))?;
    Ok(
        json!({"generation_id":generation,"files":entries.len(),"reused_files":reused,"skipped":skipped,"coverage":if skipped.is_empty(){"complete"}else{"partial"}}),
    )
}
pub fn snapshot(p: &Project) -> Result<(Option<String>, Vec<FileEntry>)> {
    snapshot_rows(p, true)
}

/// Internal search input only: rows are current-policy eligible metadata, not
/// physically authorized sources. Search must authorize every actual match.
pub(crate) fn metadata_search_snapshot(p: &Project) -> Result<(Option<String>, Vec<FileEntry>)> {
    snapshot_rows(p, false)
}

/// Internal body-search input only. Metadata cannot eliminate a body candidate:
/// search must physically authorize and verified-read every policy-eligible row
/// in the requested scope/language before evaluating text/all expressions.
/// The generic snapshot remains physically authorized for other consumers.
pub(crate) fn body_search_snapshot(p: &Project) -> Result<(Option<String>, Vec<FileEntry>)> {
    snapshot_rows(p, false)
}

fn snapshot_rows(p: &Project, physical: bool) -> Result<(Option<String>, Vec<FileEntry>)> {
    let bounded = bounded_project(p, 10000)?;
    let p = &bounded;
    p.check_deadline()?;
    reader::validate_policy(p)?;
    reader::validate_root(p)?;
    let _lock = reader_lock(p)?;
    if !p.index_db().exists() {
        return Err(Error::new(
            "NOT_INITIALIZED",
            "Index missing; run pctx index update",
            6,
        ));
    }
    let db = connect(p)?;
    sqlite_budget(p, &db)?;
    let tx = db.unchecked_transaction().map_err(|e| sql_error(p, e))?;
    let g:Option<String>=tx.query_row("SELECT active_generation_id FROM workspace_meta WHERE project_id=?1 AND workspace_id=?2",params![p.project_id,p.workspace_id],|r|r.get(0)).optional().map_err(|e| sql_error(p, e))?.flatten();
    let g = g.ok_or_else(|| Error::new("NOT_INITIALIZED", "No ready index generation", 6))?;
    let mut stmt=tx.prepare("SELECT f.metadata FROM generation_files g JOIN file_versions f ON f.id=g.file_version_id JOIN generations x ON x.id=g.generation_id WHERE g.generation_id=?1 AND x.state='ready' ORDER BY g.path").map_err(|e| sql_error(p, e))?;
    let rows = stmt
        .query_map([&g], |r| r.get::<_, String>(0))
        .map_err(|e| sql_error(p, e))?;
    let mut entries = Vec::new();
    for row in rows {
        p.check_deadline()?;
        let f: FileEntry = serde_json::from_str(&row.map_err(|e| sql_error(p, e))?)?;
        let access = if physical {
            reader::authorize(p, &f.path).map(|_| ())
        } else {
            reader::policy_allows(p, &f.path)
        };
        match access {
            Ok(()) => entries.push(f),
            Err(e)
                if matches!(
                    e.code.as_str(),
                    "TIMEOUT" | "INVALID_CONFIG" | "POLICY_UNAVAILABLE"
                ) =>
            {
                return Err(e);
            }
            Err(e)
                if physical || matches!(e.code.as_str(), "POLICY_DENIED" | "PATH_OUTSIDE_ROOT") => {
            }
            Err(e) => return Err(e),
        }
    }
    reader::validate_root(p)?;
    p.check_deadline()?;
    Ok((Some(g), entries))
}
pub fn gc(p: &Project, apply: bool) -> Result<Value> {
    let bounded = bounded_project(p, 10000)?;
    let p = &bounded;
    let _lock = writer(p)?;
    let mut db = connect(p)?;
    let mut stmt=db.prepare("SELECT id FROM generations WHERE id NOT IN (SELECT id FROM generations WHERE state='ready' ORDER BY completed_at DESC,rowid DESC LIMIT 2)").map_err(|e| sql_error(p, e))?;
    let ids = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| sql_error(p, e))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(stmt);
    if apply {
        sqlite_budget(p, &db)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| sql_error(p, e))?;
        for g in &ids {
            p.check_deadline()?;
            tx.execute("DELETE FROM generation_files WHERE generation_id=?1", [g])
                .map_err(|e| sql_error(p, e))?;
            tx.execute("DELETE FROM generations WHERE id=?1", [g])
                .map_err(|e| sql_error(p, e))?;
        }
        tx.execute("DELETE FROM file_versions WHERE id NOT IN (SELECT file_version_id FROM generation_files)",[]).map_err(|e| sql_error(p, e))?;
        p.check_deadline()?;
        sqlite_budget(p, &tx)?;
        tx.commit().map_err(|e| sql_error(p, e))?;
    }
    Ok(
        json!({"dry_run":!apply,"generations":ids,"control_preserved":true,"checkpoints_preserved":true}),
    )
}
pub fn checkpoint(p: &Project, name: Option<&str>, pin: bool) -> Result<Value> {
    let bounded = bounded_project(p, 120000)?;
    let p = &bounded;
    checkpoint_scoped(p, name, pin, &[])
}
fn checkpoint_arguments(name: Option<&str>, scopes: &[String]) -> Result<globset::GlobSet> {
    if name.is_some_and(|n| n.is_empty() || n.len() > 256 || reader::redact(n).1) {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Invalid or sensitive checkpoint name",
            2,
        ));
    }
    scope_matcher(scopes)
}

/// Pure checkpoint argument admission, without project or writer access.
pub fn validate_checkpoint_request(name: Option<&str>, scopes: &[String]) -> Result<()> {
    checkpoint_arguments(name, scopes).map(|_| ())
}

pub fn checkpoint_scoped(
    p: &Project,
    name: Option<&str>,
    pin: bool,
    scopes: &[String],
) -> Result<Value> {
    let bounded = bounded_project(p, 120_000)?;
    let p = &bounded;
    let matcher = checkpoint_arguments(name, scopes)?;
    let _lock = writer(p)?;
    let mut map = reader::manifest(p)?;
    p.check_deadline()?;
    map.retain(|path, _| in_scope(path, scopes, &matcher));
    let payload = json!({"schema_version":1,"id":id("CP"),"name":name,"project_id":p.project_id,"workspace_id":p.workspace_id,"created_at":now(),"policy_hash":p.policy_hash(),"files":map,"scope":if scopes.is_empty(){vec![".".to_string()]}else{scopes.to_vec()},"config_hash":hash(serde_json::to_vec(&p.config)?),"parser_hash":hash(PARSER_SET)});
    let mut db = connect(p)?;
    migrate(p, &mut db)?;
    if let Some(name) = name
        && db
            .query_row("SELECT 1 FROM checkpoints WHERE name=?1", [name], |r| {
                r.get::<_, i64>(0)
            })
            .optional()
            .map_err(|e| sql_error(p, e))?
            .is_some()
    {
        return Err(Error::new(
            "REVISION_CONFLICT",
            "Checkpoint name already exists",
            9,
        ));
    }
    let bytes = serde_json::to_vec(&payload)?;
    sqlite_budget(p, &db)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| sql_error(p, e))?;
    tx.execute(
        "INSERT INTO checkpoints VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            payload["id"].as_str(),
            name,
            String::from_utf8_lossy(&bytes),
            hash(&bytes),
            now(),
            pin
        ],
    )
    .map_err(|e| sql_error(p, e))?;
    p.check_deadline()?;
    sqlite_budget(p, &tx)?;
    tx.commit().map_err(|e| sql_error(p, e))?;
    Ok(payload)
}
pub fn checkpoint_get(p: &Project, key: &str) -> Result<Value> {
    let bounded = bounded_project(p, 10000)?;
    let p = &bounded;
    let _lock = reader_lock(p)?;
    let db = connect(p)?;
    let s: Option<(String, String)> = db
        .query_row(
            "SELECT manifest,manifest_hash FROM checkpoints WHERE id=?1 OR name=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| sql_error(p, e))?;
    {
        let (body, digest) =
            s.ok_or_else(|| Error::new("CHECKPOINT_NOT_FOUND", "Checkpoint not found", 6))?;
        if hash(body.as_bytes()) != digest {
            return Err(Error::new(
                "INVALID_ARCHIVE",
                "Checkpoint manifest checksum mismatch",
                7,
            ));
        }
        let mut value: Value = serde_json::from_str(&body)?;
        filter_checkpoint(p, &mut value)?;
        Ok(value)
    }
}
pub fn checkpoint_list(p: &Project) -> Result<Value> {
    let bounded = bounded_project(p, 10000)?;
    let p = &bounded;
    let _lock = reader_lock(p)?;
    let db = connect(p)?;
    let mut stmt = db
        .prepare("SELECT manifest,manifest_hash FROM checkpoints ORDER BY created_at,id")
        .map_err(|e| sql_error(p, e))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| sql_error(p, e))?;
    let mut items = Vec::new();
    for r in rows {
        p.check_deadline()?;
        let (body, digest) = r.map_err(|e| sql_error(p, e))?;
        if hash(body.as_bytes()) != digest {
            return Err(Error::new(
                "INVALID_ARCHIVE",
                "Checkpoint manifest checksum mismatch",
                7,
            ));
        }
        let mut value = serde_json::from_str::<Value>(&body)?;
        filter_checkpoint(p, &mut value)?;
        items.push(value);
    }
    p.check_deadline()?;
    Ok(json!({"items":items}))
}
pub fn changes(p: &Project, key: &str) -> Result<Value> {
    let bounded = bounded_project(p, 10000)?;
    let p = &bounded;
    let cp = checkpoint_get(p, key)?;
    if cp["project_id"] != p.project_id || cp["workspace_id"] != p.workspace_id {
        return Err(Error::new(
            "WORKSPACE_MISMATCH",
            "Checkpoint belongs to another workspace",
            9,
        ));
    }
    let old: BTreeMap<String, String> = serde_json::from_value(cp["files"].clone())?;
    let scopes: Vec<String> = serde_json::from_value(cp["scope"].clone())?;
    let matcher = scope_matcher(&scopes)?;
    let mut current = reader::manifest(p)?;
    p.check_deadline()?;
    current.retain(|path, _| in_scope(path, &scopes, &matcher));
    let mut items = Vec::new();
    for (path, h) in &current {
        p.check_deadline()?;
        let change = match old.get(path) {
            None => "added",
            Some(x) if x != h => "modified",
            _ => "unchanged",
        };
        items.push(json!({"path":path,"change":change,"file_hash":h}));
    }
    for path in old.keys() {
        p.check_deadline()?;
        if !current.contains_key(path) {
            let access = reader::authorize(p, path);
            if let Err(e) = &access
                && matches!(
                    e.code.as_str(),
                    "TIMEOUT" | "INVALID_CONFIG" | "POLICY_UNAVAILABLE"
                )
            {
                return Err(e.clone());
            }
            if access.err().is_none_or(|e| e.code == "IO_ERROR") {
                items.push(json!({"path":path,"change":"deleted"}));
            }
        }
    }
    let mut renames = Vec::new();
    for (removed, digest) in &old {
        p.check_deadline()?;
        if !current.contains_key(removed) {
            let old_count = old.values().filter(|h| *h == digest).count();
            let new_paths = current
                .iter()
                .filter(|(path, h)| *h == digest && !old.contains_key(*path))
                .map(|(path, _)| path)
                .collect::<Vec<_>>();
            let new_count = current.values().filter(|h| *h == digest).count();
            if old_count == 1 && new_count == 1 && new_paths.len() == 1 {
                renames.push(json!({"from":removed,"to":new_paths[0],"kind":"unique_hash_hint"}));
            }
        }
    }
    items.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    p.check_deadline()?;
    Ok(
        json!({"checkpoint_id":cp["id"],"items":items,"rename_hints":renames,"comparison_scope":scopes,"policy_changed":cp["policy_hash"]!=p.policy_hash(),"historical_source_available":false}),
    )
}
/// Pure name grammar shared by CLI preflight and handoff producers.
pub fn validate_handoff_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::new("INVALID_ARGUMENT", "Invalid handoff name", 2));
    }
    Ok(())
}
pub fn handoff_create(p: &Project, name: &str, source: &Path, replace: bool) -> Result<Value> {
    validate_handoff_name(name)?;
    let bounded = bounded_project(p, 10000)?;
    let p = &bounded;
    let bytes = crate::input::explicit_file_bytes(source, p.deadline.unwrap())?;
    if bytes.len() > 1048576 {
        return Err(Error::new("FILE_TOO_LARGE", "Handoff input too large", 2));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| Error::new("UNSUPPORTED_ENCODING", "Handoff must be UTF-8", 2))?;
    let (text, redacted) = reader::redact(text);
    p.check_deadline()?;
    let cp = checkpoint(p, None, false)?;
    let meta = json!({"schema_version":1,"id":id("HO"),"name":name,"project_id":p.project_id,"checkpoint_id":cp["id"],"source":"user_provided","redacted":redacted});
    let content = format!("---\n{}\n---\n{}", serde_json::to_string(&meta)?, text);
    p.check_deadline()?;
    crate::project::write_handoff(p, name, content.as_bytes(), replace)?;
    Ok(
        json!({"path":format!(".pctx/handoffs/{name}.md"),"checkpoint_id":cp["id"],"evidence_origin":"user_provided"}),
    )
}
pub fn handoff_show(p: &Project, name: &str, validate: bool) -> Result<Value> {
    validate_handoff_name(name)?;
    let bounded = bounded_project(p, 10000)?;
    let p = &bounded;
    let file = reader::read(p, &format!(".pctx/handoffs/{name}.md"))?;
    let parts: Vec<&str> = file.text.splitn(3, "---").collect();
    let meta: Value = serde_json::from_str(parts.get(1).unwrap_or(&"").trim())?;
    let validation = if validate {
        Some(changes(p, meta["checkpoint_id"].as_str().unwrap_or(""))?)
    } else {
        None
    };
    Ok(
        json!({"metadata":meta,"content":reader::redact(&file.text).0,"evidence_origin":"user_provided","validation":validation}),
    )
}

fn filter_checkpoint(p: &Project, cp: &mut Value) -> Result<()> {
    p.check_deadline()?;
    reader::validate_policy(p)?;
    if let Some(files) = cp["files"].as_object_mut() {
        let mut denied = Vec::new();
        for path in files.keys() {
            p.check_deadline()?;
            match reader::policy_allows(p, path) {
                Ok(()) => {}
                Err(e) if matches!(e.code.as_str(), "POLICY_DENIED" | "PATH_OUTSIDE_ROOT") => {
                    denied.push(path.clone())
                }
                Err(e) => return Err(e),
            }
        }
        for path in denied {
            files.remove(&path);
        }
    }
    cp["policy_filtered"] = json!(true);
    Ok(())
}

fn reader_lock(p: &Project) -> Result<fs::File> {
    p.check_deadline()?;
    let f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(p.workspace_dir.join("writer.lock"))?;
    let start = Instant::now();
    loop {
        p.check_deadline()?;
        if f.try_lock_shared().is_ok() {
            p.check_deadline()?;
            return Ok(f);
        }
        if p.deadline.is_none() && start.elapsed() > Duration::from_secs(5) {
            return Err(Error::new("INDEX_BUSY", "Index publication in progress", 7));
        }
        std::thread::sleep(p.remaining(Duration::from_millis(10))?);
    }
}
/// Build independently and publish only after validation. Recovery never deletes
/// the previous database: unreadable metadata remains in the quarantine artifact.
pub fn rebuild(p: &Project) -> Result<Value> {
    let bounded = bounded_project(p, 120000)?;
    let p = &bounded;
    let _lock = writer(p)?;
    let stage = p.workspace_dir.join(format!(".{}", id("rebuild")));
    crate::project::private_dir(&stage)?;
    let mut staged = p.clone();
    staged.workspace_dir = stage.clone();
    let result = (|| -> Result<Value> {
        let old = p.index_db();
        let mut checkpoints = Vec::new();
        let mut recovery_warning = None;
        if old.exists() {
            let capture =
                (|| -> Result<()> {
                    p.check_deadline()?;
                    let db = rusqlite::Connection::open_with_flags(
                        &old,
                        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                    )?;
                    if let Some(deadline) = p.deadline {
                        db.progress_handler(1000, Some(move || deadline.check().is_err()))?;
                    }
                    db.busy_timeout(
                        p.remaining(Duration::from_secs(5))?
                            .saturating_add(Duration::from_nanos(999_999)),
                    )?;
                    let version: i64 = db
                        .pragma_query_value(None, "user_version", |r| r.get(0))
                        .map_err(|e| sql_error(p, e))?;
                    if version > 1 {
                        return Err(Error::new(
                            "MIGRATION_REQUIRED",
                            "Index schema is newer than this binary",
                            7,
                        ));
                    }
                    let check: String = db
                        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
                        .map_err(|e| sql_error(p, e))?;
                    if check != "ok" {
                        return Err(Error::new(
                            "INDEX_CORRUPT",
                            "Existing index failed integrity check",
                            7,
                        ));
                    }
                    let meta: Option<(String, String)> = db
                        .query_row(
                            "SELECT project_id,workspace_id FROM workspace_meta",
                            [],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .optional()
                        .map_err(|e| sql_error(p, e))?;
                    if meta.is_some_and(|(project, workspace)| {
                        project != p.project_id || workspace != p.workspace_id
                    }) {
                        return Err(Error::new(
                            "WORKSPACE_MISMATCH",
                            "Existing index belongs to another workspace",
                            9,
                        ));
                    }
                    let mut stmt = db.prepare(
                    "SELECT id,name,manifest,manifest_hash,created_at,pinned FROM checkpoints",
                ).map_err(|e| sql_error(p, e))?;
                    let rows = stmt
                        .query_map([], |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, Option<String>>(1)?,
                                r.get::<_, String>(2)?,
                                r.get::<_, String>(3)?,
                                r.get::<_, i64>(4)?,
                                r.get::<_, i64>(5)?,
                            ))
                        })
                        .map_err(|e| sql_error(p, e))?;
                    for row in rows {
                        p.check_deadline()?;
                        let cp = row.map_err(|e| sql_error(p, e))?;
                        if hash(cp.2.as_bytes()) != cp.3 {
                            return Err(Error::new(
                                "INDEX_CORRUPT",
                                "Checkpoint checksum mismatch",
                                7,
                            ));
                        }
                        checkpoints.push(cp);
                    }
                    Ok(())
                })();
            if let Err(e) = capture {
                p.check_deadline()?;
                if ["TIMEOUT", "MIGRATION_REQUIRED", "WORKSPACE_MISMATCH"]
                    .contains(&e.code.as_str())
                {
                    return Err(e);
                }
                checkpoints.clear();
                recovery_warning = Some(
                    "Unreadable historical metadata remains in the quarantined database; checkpoints were not reconstructed from source",
                );
            }
        }
        let mut data = update(&staged)?;
        let db = connect(&staged)?;
        for (cp, name, manifest, digest, created, pinned) in &checkpoints {
            p.check_deadline()?;
            db.execute(
                "INSERT INTO checkpoints VALUES(?1,?2,?3,?4,?5,?6)",
                params![cp, name, manifest, digest, created, pinned],
            )
            .map_err(|e| sql_error(p, e))?;
        }
        let check: String = db
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(|e| sql_error(p, e))?;
        if check != "ok" {
            return Err(Error::new(
                "INDEX_CORRUPT",
                "Staged index failed validation",
                7,
            ));
        }
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .map_err(|e| sql_error(p, e))?;
        drop(db);
        let (_, entries) = snapshot(&staged)?;
        for entry in entries {
            p.check_deadline()?;
            if reader::read(p, &entry.path)?.hash != entry.file_hash {
                return Err(Error::new(
                    "STALE_RESULT",
                    "Source changed during rebuild; previous index retained",
                    4,
                ));
            }
        }
        // snapshot may have enabled WAL again; consolidate before moving the file.
        let db = connect(&staged)?;
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .map_err(|e| sql_error(p, e))?;
        drop(db);
        p.check_deadline()?;
        let quarantine = p
            .workspace_dir
            .join(format!("index-quarantine-{}.sqlite3", id("old")));
        if old.exists() {
            if recovery_warning.is_none() {
                let db = connect(p)?;
                db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
                    .map_err(|e| sql_error(p, e))?;
                drop(db);
            }
            // Preserve recovery evidence while the active pathname remains present.
            // Publication is one atomic replacement, not two dependent renames.
            fs::hard_link(&old, &quarantine)?;
            for suffix in ["-wal", "-shm"] {
                let side = std::path::PathBuf::from(format!("{}{suffix}", old.display()));
                if side.exists() {
                    fs::copy(
                        &side,
                        std::path::PathBuf::from(format!("{}{suffix}", quarantine.display())),
                    )?;
                }
            }
            fs::File::open(&p.workspace_dir)?.sync_all()?;
            if recovery_warning.is_some() {
                for suffix in ["-wal", "-shm"] {
                    let side = std::path::PathBuf::from(format!("{}{suffix}", old.display()));
                    if side.exists() {
                        fs::remove_file(side)?;
                    }
                }
            }
        }
        p.check_deadline()?;
        fs::rename(staged.index_db(), &old)?;
        fs::File::open(&p.workspace_dir)?.sync_all()?;
        data["rebuilt"] = json!(true);
        data["checkpoints_preserved"] = json!(checkpoints.len());
        data["previous_database"] = json!(quarantine.exists().then_some(quarantine));
        data["recovery_warning"] = json!(recovery_warning);
        Ok(data)
    })();
    let _ = fs::remove_dir_all(stage);
    result
}
pub fn checkpoint_delete(p: &Project, key: &str) -> Result<Value> {
    let bounded = bounded_project(p, 10000)?;
    let p = &bounded;
    let _lock = writer(p)?;
    let mut db = connect(p)?;
    sqlite_budget(p, &db)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| sql_error(p, e))?;
    let n = tx
        .execute("DELETE FROM checkpoints WHERE id=?1 OR name=?1", [key])
        .map_err(|e| sql_error(p, e))?;
    if n == 0 {
        return Err(Error::new(
            "CHECKPOINT_NOT_FOUND",
            "Checkpoint not found",
            6,
        ));
    }
    p.check_deadline()?;
    sqlite_budget(p, &tx)?;
    tx.commit().map_err(|e| sql_error(p, e))?;
    Ok(json!({"deleted":true}))
}

fn scope_matcher(scopes: &[String]) -> Result<globset::GlobSet> {
    let mut b = globset::GlobSetBuilder::new();
    for scope in scopes {
        if scope == "." {
            continue;
        }
        if scope.starts_with('/')
            || scope.contains('\\')
            || scope.split('/').any(|part| part == "..")
        {
            return Err(Error::new(
                "INVALID_ARGUMENT",
                "Scope must be project relative",
                2,
            ));
        }
        b.add(
            globset::Glob::new(scope)
                .map_err(|_| Error::new("INVALID_ARGUMENT", "Invalid scope glob", 2))?,
        );
    }
    b.build()
        .map_err(|_| Error::new("INVALID_ARGUMENT", "Invalid scope", 2))
}
fn in_scope(path: &str, scopes: &[String], matcher: &globset::GlobSet) -> bool {
    scopes.is_empty()
        || scopes.iter().any(|s| {
            s == "." || path == s || path.starts_with(&format!("{}/", s.trim_end_matches('/')))
        })
        || matcher.is_match(path)
}

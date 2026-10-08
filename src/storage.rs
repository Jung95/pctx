use crate::{
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
fn writer(p: &Project) -> Result<fs::File> {
    let f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(p.workspace_dir.join("writer.lock"))?;
    let start = Instant::now();
    loop {
        if f.try_lock_exclusive().is_ok() {
            return Ok(f);
        }
        if start.elapsed() > Duration::from_secs(5) {
            return Err(Error::new("INDEX_BUSY", "Workspace writer lock busy", 7));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn migrate(db: &mut rusqlite::Connection) -> Result<()> {
    let tx = db.transaction()?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS workspace_meta(project_id TEXT NOT NULL,workspace_id TEXT NOT NULL,active_generation_id TEXT);
CREATE TABLE IF NOT EXISTS generations(id TEXT PRIMARY KEY,parent_id TEXT,state TEXT NOT NULL,started_at INTEGER,completed_at INTEGER,policy_hash TEXT,parser_set_hash TEXT);
CREATE TABLE IF NOT EXISTS file_versions(id TEXT PRIMARY KEY,path TEXT NOT NULL,content_hash TEXT NOT NULL,parser_hash TEXT NOT NULL,metadata TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS generation_files(generation_id TEXT REFERENCES generations(id),path TEXT NOT NULL,file_version_id TEXT REFERENCES file_versions(id),PRIMARY KEY(generation_id,path));
CREATE INDEX IF NOT EXISTS file_path ON file_versions(path,content_hash);
CREATE TABLE IF NOT EXISTS checkpoints(id TEXT PRIMARY KEY,name TEXT UNIQUE,manifest TEXT NOT NULL,manifest_hash TEXT NOT NULL,created_at INTEGER,pinned INTEGER NOT NULL DEFAULT 0);
PRAGMA user_version=1;")?;
    tx.commit()?;
    Ok(())
}
pub const PARSER_SET: &str = "tree-sitter-0.25/python-0.25/js-0.25/ts-0.23/md-heading-v1";
pub fn update(p: &Project) -> Result<Value> {
    let _lock = writer(p)?;
    let mut db = p.connect(false)?;
    migrate(&mut db)?;
    let meta: Option<(String, String, Option<String>)> = db
        .query_row(
            "SELECT project_id,workspace_id,active_generation_id FROM workspace_meta",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
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
    let mut skipped = inventory.skipped;
    let mut entries = Vec::new();
    let mut reused = 0;
    let parser_hash = hash(format!(
        "{PARSER_SET}:{}:{}",
        p.policy_hash(),
        serde_json::to_string(&p.config.index).unwrap_or_default()
    ));
    for path in inventory.paths {
        let file = match reader::read(p, &path) {
            Ok(f) => f,
            Err(e) => {
                skipped.push(json!({"path":path,"reason":e.code}));
                continue;
            }
        };
        let key = hash(format!("{}:{}:{}", path, file.hash, parser_hash));
        let cached: Option<String> = db
            .query_row(
                "SELECT metadata FROM file_versions WHERE id=?1",
                [&key],
                |r| r.get(0),
            )
            .optional()?;
        let entry = if let Some(c) = cached {
            reused += 1;
            serde_json::from_str::<FileEntry>(&c)?
        } else {
            crate::search::analyze(&path, &file.hash, &file.text)?
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
    let generation = id("GEN");
    let parent = meta.and_then(|(_, _, g)| g);
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO generations VALUES(?1,?2,'building',?3,NULL,?4,?5)",
        params![generation, parent, now(), p.policy_hash(), parser_hash],
    )?;
    for (key, e) in &entries {
        tx.execute(
            "INSERT OR IGNORE INTO file_versions VALUES(?1,?2,?3,?4,?5)",
            params![
                key,
                e.path,
                e.file_hash,
                parser_hash,
                serde_json::to_string(e)?
            ],
        )?;
        tx.execute(
            "INSERT INTO generation_files VALUES(?1,?2,?3)",
            params![generation, e.path, key],
        )?;
    }
    tx.execute(
        "UPDATE generations SET state='ready',completed_at=?1 WHERE id=?2",
        params![now(), generation],
    )?;
    tx.execute("DELETE FROM workspace_meta", [])?;
    tx.execute(
        "INSERT INTO workspace_meta VALUES(?1,?2,?3)",
        params![p.project_id, p.workspace_id, generation],
    )?;
    tx.commit()?;
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

fn snapshot_rows(p: &Project, physical: bool) -> Result<(Option<String>, Vec<FileEntry>)> {
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
    let db = p.connect(false)?;
    let tx = db.unchecked_transaction()?;
    let g:Option<String>=tx.query_row("SELECT active_generation_id FROM workspace_meta WHERE project_id=?1 AND workspace_id=?2",params![p.project_id,p.workspace_id],|r|r.get(0)).optional()?.flatten();
    let g = g.ok_or_else(|| Error::new("NOT_INITIALIZED", "No ready index generation", 6))?;
    let mut stmt=tx.prepare("SELECT f.metadata FROM generation_files g JOIN file_versions f ON f.id=g.file_version_id JOIN generations x ON x.id=g.generation_id WHERE g.generation_id=?1 AND x.state='ready' ORDER BY g.path")?;
    let rows = stmt.query_map([&g], |r| r.get::<_, String>(0))?;
    let mut entries = Vec::new();
    for row in rows {
        let f: FileEntry = serde_json::from_str(&row?)?;
        let access = if physical {
            reader::authorize(p, &f.path).map(|_| ())
        } else {
            reader::policy_allows(p, &f.path)
        };
        match access {
            Ok(()) => entries.push(f),
            Err(e) if matches!(e.code.as_str(), "INVALID_CONFIG" | "POLICY_UNAVAILABLE") => {
                return Err(e);
            }
            Err(e)
                if physical || matches!(e.code.as_str(), "POLICY_DENIED" | "PATH_OUTSIDE_ROOT") => {
            }
            Err(e) => return Err(e),
        }
    }
    reader::validate_root(p)?;
    Ok((Some(g), entries))
}
pub fn gc(p: &Project, apply: bool) -> Result<Value> {
    let _lock = writer(p)?;
    let mut db = p.connect(false)?;
    let mut stmt=db.prepare("SELECT id FROM generations WHERE id NOT IN (SELECT id FROM generations WHERE state='ready' ORDER BY completed_at DESC,rowid DESC LIMIT 2)")?;
    let ids = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(stmt);
    if apply {
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for g in &ids {
            tx.execute("DELETE FROM generation_files WHERE generation_id=?1", [g])?;
            tx.execute("DELETE FROM generations WHERE id=?1", [g])?;
        }
        tx.execute("DELETE FROM file_versions WHERE id NOT IN (SELECT file_version_id FROM generation_files)",[])?;
        tx.commit()?;
    }
    Ok(
        json!({"dry_run":!apply,"generations":ids,"control_preserved":true,"checkpoints_preserved":true}),
    )
}
pub fn checkpoint(p: &Project, name: Option<&str>, pin: bool) -> Result<Value> {
    checkpoint_scoped(p, name, pin, &[])
}
pub fn checkpoint_scoped(
    p: &Project,
    name: Option<&str>,
    pin: bool,
    scopes: &[String],
) -> Result<Value> {
    let _lock = writer(p)?;
    if name.is_some_and(|n| n.is_empty() || n.len() > 256 || reader::redact(n).1) {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Invalid or sensitive checkpoint name",
            2,
        ));
    }
    let mut map = reader::manifest(p)?;
    let matcher = scope_matcher(scopes)?;
    map.retain(|path, _| in_scope(path, scopes, &matcher));
    let payload = json!({"schema_version":1,"id":id("CP"),"name":name,"project_id":p.project_id,"workspace_id":p.workspace_id,"created_at":now(),"policy_hash":p.policy_hash(),"files":map,"scope":if scopes.is_empty(){vec![".".to_string()]}else{scopes.to_vec()},"config_hash":hash(serde_json::to_vec(&p.config)?),"parser_hash":hash(PARSER_SET)});
    let mut db = p.connect(false)?;
    migrate(&mut db)?;
    if let Some(name) = name
        && db
            .query_row("SELECT 1 FROM checkpoints WHERE name=?1", [name], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
            .is_some()
    {
        return Err(Error::new(
            "REVISION_CONFLICT",
            "Checkpoint name already exists",
            9,
        ));
    }
    let bytes = serde_json::to_vec(&payload)?;
    db.execute(
        "INSERT INTO checkpoints VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            payload["id"].as_str(),
            name,
            String::from_utf8_lossy(&bytes),
            hash(&bytes),
            now(),
            pin
        ],
    )?;
    Ok(payload)
}
pub fn checkpoint_get(p: &Project, key: &str) -> Result<Value> {
    let _lock = reader_lock(p)?;
    let db = p.connect(false)?;
    let s: Option<(String, String)> = db
        .query_row(
            "SELECT manifest,manifest_hash FROM checkpoints WHERE id=?1 OR name=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
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
        filter_checkpoint(p, &mut value);
        Ok(value)
    }
}
pub fn checkpoint_list(p: &Project) -> Result<Value> {
    let _lock = reader_lock(p)?;
    let db = p.connect(false)?;
    let mut stmt =
        db.prepare("SELECT manifest,manifest_hash FROM checkpoints ORDER BY created_at,id")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut items = Vec::new();
    for r in rows {
        let (body, digest) = r?;
        if hash(body.as_bytes()) != digest {
            return Err(Error::new(
                "INVALID_ARCHIVE",
                "Checkpoint manifest checksum mismatch",
                7,
            ));
        }
        let mut value = serde_json::from_str::<Value>(&body)?;
        filter_checkpoint(p, &mut value);
        items.push(value);
    }
    Ok(json!({"items":items}))
}
pub fn changes(p: &Project, key: &str) -> Result<Value> {
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
    current.retain(|path, _| in_scope(path, &scopes, &matcher));
    let mut items = Vec::new();
    for (path, h) in &current {
        let change = match old.get(path) {
            None => "added",
            Some(x) if x != h => "modified",
            _ => "unchanged",
        };
        items.push(json!({"path":path,"change":change,"file_hash":h}));
    }
    for path in old.keys() {
        if !current.contains_key(path)
            && reader::authorize(p, path)
                .err()
                .is_none_or(|e| e.code == "IO_ERROR")
        {
            items.push(json!({"path":path,"change":"deleted"}));
        }
    }
    let mut renames = Vec::new();
    for (removed, digest) in &old {
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
    Ok(
        json!({"checkpoint_id":cp["id"],"items":items,"rename_hints":renames,"comparison_scope":scopes,"policy_changed":cp["policy_hash"]!=p.policy_hash(),"historical_source_available":false}),
    )
}
pub fn handoff_create(p: &Project, name: &str, source: &Path, replace: bool) -> Result<Value> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::new("INVALID_ARGUMENT", "Invalid handoff name", 2));
    }
    let bytes = fs::read(source)?;
    if bytes.len() > 1048576 {
        return Err(Error::new("FILE_TOO_LARGE", "Handoff input too large", 2));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| Error::new("UNSUPPORTED_ENCODING", "Handoff must be UTF-8", 2))?;
    let (text, redacted) = reader::redact(text);
    let cp = checkpoint(p, None, false)?;
    let meta = json!({"schema_version":1,"id":id("HO"),"name":name,"project_id":p.project_id,"checkpoint_id":cp["id"],"source":"user_provided","redacted":redacted});
    let content = format!("---\n{}\n---\n{}", serde_json::to_string(&meta)?, text);
    crate::project::write_handoff(p, name, content.as_bytes(), replace)?;
    Ok(
        json!({"path":format!(".pctx/handoffs/{name}.md"),"checkpoint_id":cp["id"],"evidence_origin":"user_provided"}),
    )
}
pub fn handoff_show(p: &Project, name: &str, validate: bool) -> Result<Value> {
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::new("INVALID_ARGUMENT", "Invalid handoff name", 2));
    }
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

fn filter_checkpoint(p: &Project, cp: &mut Value) {
    if let Some(files) = cp["files"].as_object_mut() {
        files.retain(|path, _| crate::reader::policy_allows(p, path).is_ok());
    }
    cp["policy_filtered"] = json!(true);
}

fn reader_lock(p: &Project) -> Result<fs::File> {
    let f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(p.workspace_dir.join("writer.lock"))?;
    let start = Instant::now();
    loop {
        if f.try_lock_shared().is_ok() {
            return Ok(f);
        }
        if start.elapsed() > Duration::from_secs(5) {
            return Err(Error::new("INDEX_BUSY", "Index publication in progress", 7));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
/// Build independently and publish only after validation. Recovery never deletes
/// the previous database: unreadable metadata remains in the quarantine artifact.
pub fn rebuild(p: &Project) -> Result<Value> {
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
            let capture = (|| -> Result<()> {
                let db = rusqlite::Connection::open_with_flags(
                    &old,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )?;
                let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
                if version > 1 {
                    return Err(Error::new(
                        "MIGRATION_REQUIRED",
                        "Index schema is newer than this binary",
                        7,
                    ));
                }
                let check: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
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
                    .optional()?;
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
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, i64>(5)?,
                    ))
                })?;
                for row in rows {
                    let cp = row?;
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
                if ["MIGRATION_REQUIRED", "WORKSPACE_MISMATCH"].contains(&e.code.as_str()) {
                    return Err(e);
                }
                checkpoints.clear();
                recovery_warning = Some(
                    "Unreadable historical metadata remains in the quarantined database; checkpoints were not reconstructed from source",
                );
            }
        }
        let mut data = update(&staged)?;
        let db = staged.connect(false)?;
        for (cp, name, manifest, digest, created, pinned) in &checkpoints {
            db.execute(
                "INSERT INTO checkpoints VALUES(?1,?2,?3,?4,?5,?6)",
                params![cp, name, manifest, digest, created, pinned],
            )?;
        }
        let check: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(Error::new(
                "INDEX_CORRUPT",
                "Staged index failed validation",
                7,
            ));
        }
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")?;
        drop(db);
        let (_, entries) = snapshot(&staged)?;
        for entry in entries {
            if reader::read(p, &entry.path)?.hash != entry.file_hash {
                return Err(Error::new(
                    "STALE_RESULT",
                    "Source changed during rebuild; previous index retained",
                    4,
                ));
            }
        }
        // snapshot may have enabled WAL again; consolidate before moving the file.
        let db = staged.connect(false)?;
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")?;
        drop(db);
        let quarantine = p
            .workspace_dir
            .join(format!("index-quarantine-{}.sqlite3", id("old")));
        if old.exists() {
            if recovery_warning.is_none() {
                let db = p.connect(false)?;
                db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")?;
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
    let _lock = writer(p)?;
    let db = p.connect(false)?;
    let n = db.execute("DELETE FROM checkpoints WHERE id=?1 OR name=?1", [key])?;
    if n == 0 {
        return Err(Error::new(
            "CHECKPOINT_NOT_FOUND",
            "Checkpoint not found",
            6,
        ));
    }
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

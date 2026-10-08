//! Project-managed rules and decisions are data with source provenance, never executable instructions.
use crate::{
    domain::{Error, Result},
    project::Project,
    reader,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    schema_version: u32,
    #[serde(default)]
    topics: Option<Vec<String>>,
    id: String,
    #[serde(default = "all")]
    scope: Vec<String>,
    #[serde(default = "yes")]
    required: bool,
    #[serde(default)]
    conflicts_with: Vec<String>,
    #[serde(default)]
    properties: BTreeMap<String, Value>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    #[serde(default = "version")]
    schema_version: u32,
    #[serde(default)]
    topics: Option<Vec<String>>,
    id: String,
    status: String,
    date: String,
    #[serde(default, deserialize_with = "supersedes")]
    supersedes: Vec<String>,
    #[serde(default = "all")]
    scope: Vec<String>,
    #[serde(default, deserialize_with = "supersedes")]
    reinstates: Vec<String>,
}
#[derive(Clone)]
struct ObservedDecision {
    id: String,
    path: String,
    hash: String,
    status: String,
    supersedes: Vec<String>,
    reinstates: Vec<String>,
}
#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Retirement {
    version: u32,
    project: String,
    workspace: String,
    subject: String,
    replacement: String,
    replacement_path: String,
    replacement_hash: String,
    status: String,
    policy: String,
}
fn retirement_entity(p: &Project, subject: &str) -> String {
    format!(
        "DOC-{}",
        crate::domain::hash(
            serde_json::to_vec(&json!([
                "document-retirement-v1",
                p.project_id,
                p.workspace_id,
                subject
            ]))
            .unwrap()
        )
    )
}
// Durable observations belong to the existing append-only control event ledger,
// not a disposable code cache or a second memory engine. They grant no authority.
fn retirement_history(
    p: &Project,
    observed: &[ObservedDecision],
) -> Result<BTreeMap<String, Vec<Retirement>>> {
    if observed.is_empty() {
        return Ok(BTreeMap::new());
    }
    p.check_deadline()?;
    let mut db = crate::work::connect(p)?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch("CREATE INDEX IF NOT EXISTS events_document_lineage ON events(type,entity);")?;
    let policy = p.policy_hash();
    let mut subjects = observed
        .iter()
        .map(|d| d.id.clone())
        .collect::<BTreeSet<_>>();
    for d in observed {
        if !["proposed", "rejected"].contains(&d.status.as_str()) {
            subjects.extend(d.supersedes.iter().cloned());
        }
    }
    if subjects.len() > 2048 {
        return Err(Error::new(
            "PARTIAL_RESULT",
            "Document retirement subjects exceed bounded limits",
            3,
        ));
    }
    let mut history = BTreeMap::new();
    let mut history_bytes = 0usize;
    for subject in subjects {
        p.check_deadline()?;
        // Reject oversized or non-TEXT storage before transferring payloads to
        // Rust. BLOB length measures UTF8 bytes, not SQLite's character count.
        let mut statement = tx.prepare("SELECT CASE WHEN typeof(payload)='text' AND length(CAST(payload AS BLOB))<=4096 THEN payload ELSE NULL END FROM events WHERE type='document_retirement_observed' AND entity=?1 ORDER BY seq LIMIT 1025")?;
        let mut rows = statement.query([retirement_entity(p, &subject)])?;
        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            p.check_deadline()?;
            if records.len() >= 1024 {
                return Err(Error::new(
                    "PARTIAL_RESULT",
                    "Document retirement history exceeds bounded limits",
                    3,
                ));
            }
            let payload = row
                .get::<_, Option<String>>(0)?
                .ok_or_else(|| Error::new("DB_ERROR", "Invalid document lineage observation", 7))?;
            history_bytes = history_bytes.saturating_add(payload.len());
            if history_bytes > 8 * 1024 * 1024 {
                return Err(Error::new(
                    "PARTIAL_RESULT",
                    "Document lineage exceeds aggregate metadata limit",
                    3,
                ));
            }
            if payload.len() > 4096 {
                return Err(Error::new(
                    "DB_ERROR",
                    "Invalid document lineage observation",
                    7,
                ));
            }
            let record: Retirement = serde_json::from_str(&payload)
                .map_err(|_| Error::new("DB_ERROR", "Invalid document lineage observation", 7))?;
            if record.version != 1
                || record.project != p.project_id
                || record.workspace != p.workspace_id
                || record.subject != subject
                || record.replacement == subject
                || record.policy.len() != 64
                || !record.policy.bytes().all(|b| b.is_ascii_hexdigit())
                || record.replacement_hash.len() != 64
                || !record
                    .replacement_hash
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit())
                || ![
                    "accepted",
                    "superseded",
                    "deprecated",
                    "cancelled",
                    "completed",
                ]
                .contains(&record.status.as_str())
            {
                return Err(Error::new(
                    "DB_ERROR",
                    "Invalid document lineage observation",
                    7,
                ));
            }
            identity(&record.replacement)
                .map_err(|_| Error::new("DB_ERROR", "Invalid document lineage identity", 7))?;
            match reader::policy_allows(p, &record.replacement_path) {
                Ok(()) => {}
                Err(e) if e.code == "POLICY_DENIED" => {}
                Err(e) if e.code == "PATH_OUTSIDE_ROOT" => {
                    return Err(Error::new("DB_ERROR", "Invalid document lineage path", 7));
                }
                Err(e) => return Err(e),
            }
            records.push(record);
        }
        history.insert(subject, records);
    }
    let mut additions = 0;
    for d in observed
        .iter()
        .filter(|d| !["proposed", "rejected"].contains(&d.status.as_str()))
    {
        for subject in &d.supersedes {
            p.check_deadline()?;
            let records = history.get_mut(subject).unwrap();
            if records
                .iter()
                .any(|r| r.replacement == d.id && r.policy == policy)
            {
                continue;
            }
            additions += 1;
            if additions > 2048 || records.len() >= 1024 {
                return Err(Error::new(
                    "PARTIAL_RESULT",
                    "Document retirement relations exceed bounded limits",
                    3,
                ));
            }
            let record = Retirement {
                version: 1,
                project: p.project_id.clone(),
                workspace: p.workspace_id.clone(),
                subject: subject.clone(),
                replacement: d.id.clone(),
                replacement_path: d.path.clone(),
                replacement_hash: d.hash.clone(),
                status: d.status.clone(),
                policy: policy.clone(),
            };
            let payload = serde_json::to_string(&record)?;
            if payload.len() > 4096 {
                return Err(config("Document lineage metadata exceeds bounded limits"));
            }
            history_bytes = history_bytes.saturating_add(payload.len());
            if history_bytes > 8 * 1024 * 1024 {
                return Err(Error::new(
                    "PARTIAL_RESULT",
                    "Document lineage exceeds aggregate metadata limit",
                    3,
                ));
            }
            tx.execute("INSERT INTO events(entity,type,payload,created) VALUES(?1,'document_retirement_observed',?2,?3)",
                rusqlite::params![retirement_entity(p, subject), payload, crate::domain::now()])?;
            records.push(record);
        }
    }
    // Validate restoration against the same transaction, including observations
    // made in this load. Invalid requests must not publish partial history.
    for d in observed {
        for requested in &d.reinstates {
            if !history
                .get(&d.id)
                .is_some_and(|records| records.iter().any(|r| &r.replacement == requested))
            {
                return Err(config(
                    "Reinstatement refers to an unobserved replacement in this workspace",
                ));
            }
        }
    }
    p.check_deadline()?;
    tx.commit()?;
    Ok(history)
}
fn supersedes<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Ids {
        One(String),
        Many(Vec<String>),
        None(()),
    }
    Ok(match Ids::deserialize(d)? {
        Ids::One(id) => vec![id],
        Ids::Many(ids) => ids,
        Ids::None(()) => vec![],
    })
}
fn mask(value: &mut Value) {
    match value {
        Value::String(s) => *s = reader::redact(s).0,
        Value::Array(a) => {
            for v in a {
                mask(v)
            }
        }
        Value::Object(m) => {
            for (key, v) in m {
                if ["password", "token", "secret", "api_key", "credential"]
                    .contains(&key.to_ascii_lowercase().as_str())
                {
                    *v = json!("[REDACTED]")
                } else {
                    mask(v)
                }
            }
        }
        _ => {}
    }
}
fn cycles(
    graph: &BTreeMap<String, Vec<String>>,
    id: &str,
    active: &mut BTreeSet<String>,
    done: &mut BTreeSet<String>,
) -> Result<()> {
    if done.contains(id) {
        return Ok(());
    }
    if !active.insert(id.into()) {
        return Err(config("Decision supersedes cycle"));
    }
    if let Some(previous) = graph.get(id) {
        for next in previous {
            if graph.contains_key(next) {
                cycles(graph, next, active, done)?;
            }
        }
    }
    active.remove(id);
    done.insert(id.into());
    Ok(())
}
fn all() -> Vec<String> {
    vec!["**".into()]
}
fn yes() -> bool {
    true
}
fn version() -> u32 {
    1
}
fn config(message: &str) -> Error {
    Error::new("INVALID_CONFIG", message, 2)
}
fn identity(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 256 || reader::redact(id).1 || id.chars().any(char::is_control) {
        return Err(config("Invalid or sensitive document identity"));
    }
    Ok(())
}
fn patterns(values: &[String]) -> Result<globset::GlobSet> {
    if values.is_empty() {
        return Err(config("Document scope cannot be empty"));
    }
    let mut b = globset::GlobSetBuilder::new();
    for value in values {
        if value.is_empty()
            || value.starts_with('/')
            || value.contains('\\')
            || value.split('/').any(|p| p == ".." || p == ".")
        {
            return Err(config("Document scope must be a relative glob"));
        }
        b.add(globset::Glob::new(value).map_err(|_| config("Invalid document scope glob"))?);
    }
    b.build().map_err(|_| config("Invalid document scope"))
}
fn requests(values: &[String]) -> Result<globset::GlobSet> {
    if values.is_empty() {
        return patterns(&all());
    }
    let mut expanded = vec![];
    for v in values {
        expanded.push(v.clone());
        if !v.contains(['*', '?', '[', '{']) {
            expanded.push(format!("{}/**", v.trim_end_matches('/')));
        }
    }
    patterns(&expanded)
}
/// Tags and anchors are rejected lexically before a YAML parser can expand them.
fn yaml_preflight(text: &str) -> Result<()> {
    if text.len() > 16384 || text.lines().count() > 256 {
        return Err(config("Front matter exceeds bounded YAML limits"));
    }
    let mut quote = None;
    let mut escaped = false;
    let mut previous = '\n';
    for c in text.chars() {
        if let Some(q) = quote {
            if q == '"' && c == '\\' && !escaped {
                escaped = true;
                continue;
            }
            if c == q && !escaped {
                quote = None;
            }
            escaped = false;
            previous = c;
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
        } else if ['!', '&', '*'].contains(&c)
            && (previous.is_whitespace() || ['[', '{', ',', ':'].contains(&previous))
        {
            return Err(config("YAML tags, anchors and aliases are not supported"));
        }
        previous = c;
    }
    if quote.is_some() {
        return Err(config("Unterminated quoted front matter"));
    }
    if text.lines().any(|line| {
        line.trim_start().starts_with('%') || line.trim() == "---" || line.trim() == "..."
    }) {
        return Err(config(
            "YAML directives and multiple documents are forbidden",
        ));
    }
    Ok(())
}
fn yaml_value(value: serde_yaml_ng::Value, depth: usize, nodes: &mut usize) -> Result<Value> {
    *nodes += 1;
    if depth > 8 || *nodes > 512 {
        return Err(config("YAML tree exceeds depth or node limits"));
    }
    use serde_yaml_ng::Value as Y;
    Ok(match value {
        Y::Null => Value::Null,
        Y::Bool(v) => json!(v),
        Y::Number(n) => {
            if n.as_f64().is_some_and(|f| !f.is_finite()) {
                return Err(config("Nonfinite YAML numbers forbidden"));
            }
            serde_json::to_value(n).map_err(|_| config("Unsupported YAML number"))?
        }
        Y::String(s) => json!(s),
        Y::Sequence(a) => Value::Array(
            a.into_iter()
                .map(|v| yaml_value(v, depth + 1, nodes))
                .collect::<Result<Vec<_>>>()?,
        ),
        Y::Mapping(m) => {
            let mut out = serde_json::Map::new();
            for (k, v) in m {
                let Y::String(key) = k else {
                    return Err(config("YAML keys must be strings"));
                };
                if key == "<<" {
                    return Err(config("YAML merge keys are forbidden"));
                }
                if out.insert(key, yaml_value(v, depth + 1, nodes)?).is_some() {
                    return Err(config("Duplicate YAML key"));
                }
            }
            Value::Object(out)
        }
        Y::Tagged(_) => return Err(config("YAML tags are forbidden")),
    })
}
fn frontmatter(text: &str) -> Result<Option<Value>> {
    let clean = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = clean.split_inclusive('\n');
    if lines
        .next()
        .is_none_or(|l| l.trim_end_matches(['\r', '\n']) != "---")
    {
        return Ok(None);
    }
    let mut header = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            closed = true;
            break;
        }
        header.push_str(line);
        if header.len() > 16384 {
            return Err(config("Front matter exceeds 16 KiB"));
        }
    }
    if !closed {
        return Err(config("Unclosed YAML front matter"));
    }
    yaml_preflight(&header)?;
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&header)
        .map_err(|_| config("Invalid or duplicate-key YAML front matter"))?;
    Ok(Some(yaml_value(yaml, 0, &mut 0)?))
}
fn prefix(glob: &str) -> &str {
    glob.split(['*', '?', '[', '{'])
        .next()
        .unwrap_or("")
        .trim_end_matches('/')
}
fn possible_overlap(a: &[String], b: &[String]) -> bool {
    a.iter().any(|a| {
        b.iter().any(|b| {
            let a = prefix(a);
            let b = prefix(b);
            a.is_empty()
                || b.is_empty()
                || a == b
                || a.starts_with(&format!("{b}/"))
                || b.starts_with(&format!("{a}/"))
        })
    })
}
fn applicable(scope: &[String], wanted: &[String], relevant: &[String]) -> Result<bool> {
    let set = patterns(scope)?;
    if wanted.is_empty() {
        return Ok(scope.iter().any(|s| s == "**" || s == "**/*"));
    }
    Ok(relevant.iter().any(|p| set.is_match(p)) || possible_overlap(scope, wanted))
}
fn overlap(a: &[String], b: &[String], relevant: &[String]) -> Result<bool> {
    let ga = patterns(a)?;
    let gb = patterns(b)?;
    Ok(relevant.iter().any(|p| ga.is_match(p) && gb.is_match(p)) || possible_overlap(a, b))
}
fn provenance(path: &str, hash: &str, text: &str) -> Value {
    let (content, redacted) = reader::redact(text);
    json!({"path":path,"file_hash":hash,"content":content,"redacted":redacted,"source":"user_managed_project_document","instruction_priority":"external_project_data","evidence_status":"observed","freshness":"current","implementation_status":"not_verified_by_document"})
}
pub fn load(p: &Project, scope: &[String]) -> Result<Value> {
    let request = requests(scope)?;
    let inventory = reader::inventory(p, false)?;
    if !inventory.skipped.is_empty() {
        return Err(Error::new(
            "PARTIAL_RESULT",
            "Project document inventory incomplete",
            3,
        ));
    }
    let relevant = inventory
        .paths
        .iter()
        .filter(|path| request.is_match(path))
        .cloned()
        .collect::<Vec<_>>();
    let mut rules = vec![];
    let mut decisions = vec![];
    let mut instructions = vec![];
    let mut rule_defs: Vec<Rule> = vec![];
    let mut rule_ids = BTreeSet::new();
    let mut decision_ids = BTreeSet::new();
    let mut source_hashes = vec![];
    let mut decision_graph = BTreeMap::new();
    let mut decision_states = BTreeMap::new();
    let mut observed_decisions = Vec::new();
    let mut document_bytes = 0u64;
    for path in &inventory.paths {
        if !path.starts_with(".pctx/rules/") && !path.starts_with(".pctx/decisions/") {
            continue;
        }
        let f = reader::read(p, path)?;
        document_bytes += f.size_bytes;
        if source_hashes.len() >= 512 || document_bytes > 8388608 {
            return Err(Error::new(
                "PARTIAL_RESULT",
                "Project document corpus exceeds bounded loader limits",
                3,
            ));
        }
        let meta = frontmatter(&f.text)?;
        let mut item = provenance(path, &f.hash, &f.text);
        source_hashes.push((path.clone(), f.hash));
        if path.starts_with(".pctx/rules/") {
            if let Some(meta) = meta {
                let rule: Rule = serde_json::from_value(meta)
                    .map_err(|_| config("Invalid rule front matter fields"))?;
                identity(&rule.id)?;
                if let Some(topics) = &rule.topics {
                    crate::source_delivery::validate_topics(topics)?;
                }
                item["topics"] = json!(rule.topics);
                if rule.schema_version != 1 || !rule_ids.insert(rule.id.clone()) {
                    return Err(config("Unsupported rule schema or duplicate rule ID"));
                }
                patterns(&rule.scope)?;
                for id in &rule.conflicts_with {
                    identity(id)?;
                }
                if !applicable(&rule.scope, scope, &relevant)? {
                    continue;
                }
                if rule.properties.keys().any(|key| reader::redact(key).1) {
                    return Err(config("Sensitive structured property key"));
                }
                item["id"] = json!(rule.id);
                item["scope"] = json!(rule.scope);
                item["required"] = json!(rule.required);
                item["properties"] = json!(rule.properties);
                item["conflicts_with"] = json!(rule.conflicts_with);
                rules.push(item);
                rule_defs.push(rule);
            } else {
                item["required"] = json!(false);
                item["unstructured"] = json!(true);
                instructions.push(item);
            }
        } else {
            let meta = meta.ok_or_else(|| config("Decision record requires front matter"))?;
            let mut decision: Decision = serde_json::from_value(meta)
                .map_err(|_| config("Invalid decision front matter fields"))?;
            identity(&decision.id)?;
            if let Some(topics) = &decision.topics {
                crate::source_delivery::validate_topics(topics)?;
            }
            item["topics"] = json!(decision.topics);
            if decision.schema_version != 1
                || !decision_ids.insert(decision.id.clone())
                || ![
                    "proposed",
                    "accepted",
                    "superseded",
                    "deprecated",
                    "rejected",
                    "cancelled",
                    "completed",
                ]
                .contains(&decision.status.as_str())
                || chrono::NaiveDate::parse_from_str(&decision.date, "%Y-%m-%d").is_err()
            {
                return Err(config("Invalid decision ID/status/date/schema"));
            }
            patterns(&decision.scope)?;
            decision.supersedes.sort();
            decision.supersedes.dedup();
            for id in &decision.supersedes {
                identity(id)?;
                if id == &decision.id {
                    return Err(config("Decision cannot supersede itself"));
                }
            }
            decision.reinstates.sort();
            decision.reinstates.dedup();
            for id in &decision.reinstates {
                identity(id)?;
                if id == &decision.id || decision.status != "accepted" {
                    return Err(config(
                        "Reinstatement requires an accepted decision and another replacement ID",
                    ));
                }
            }
            observed_decisions.push(ObservedDecision {
                id: decision.id.clone(),
                path: path.clone(),
                hash: item["file_hash"].as_str().unwrap().to_string(),
                status: decision.status.clone(),
                supersedes: decision.supersedes.clone(),
                reinstates: decision.reinstates.clone(),
            });
            decision_states.insert(decision.id.clone(), decision.status.clone());
            decision_graph.insert(decision.id.clone(), decision.supersedes.clone());
            if !applicable(&decision.scope, scope, &relevant)? {
                continue;
            }
            item["id"] = json!(decision.id);
            item["status"] = json!(decision.status);
            item["date"] = json!(decision.date);
            item["supersedes"] = json!(decision.supersedes);
            item["scope"] = json!(decision.scope);
            item["reinstates"] = json!(decision.reinstates);
            decisions.push(item);
        }
    }
    for (i, a) in rule_defs.iter().enumerate() {
        if !a.required {
            continue;
        }
        for b in rule_defs.iter().skip(i + 1).filter(|b| b.required) {
            if !overlap(&a.scope, &b.scope, &relevant)? {
                continue;
            }
            if a.conflicts_with.contains(&b.id)
                || b.conflicts_with.contains(&a.id)
                || a.properties
                    .iter()
                    .any(|(key, v)| b.properties.get(key).is_some_and(|other| other != v))
            {
                return Err(Error::new(
                    "RULE_CONFLICT",
                    "Applicable required rules explicitly conflict",
                    10,
                ));
            }
        }
    }
    let mut done = BTreeSet::new();
    for id in decision_graph.keys() {
        cycles(&decision_graph, id, &mut BTreeSet::new(), &mut done)?;
    }
    // Only validated source observations are recorded; a later rendering failure
    // does not erase an observed retirement and does not acknowledge any body.
    for (path, expected) in &source_hashes {
        if reader::read(p, path)?.hash != *expected {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Project document changed before observation",
                4,
            ));
        }
    }
    let history = retirement_history(p, &observed_decisions)?;
    let policy = p.policy_hash();
    for item in &mut decisions {
        let id = item["id"].as_str().unwrap_or("");
        let records = history.get(id).map(Vec::as_slice).unwrap_or(&[]);
        let reinstates = item["reinstates"].as_array().unwrap();
        for requested in reinstates {
            if !records
                .iter()
                .any(|r| r.replacement == requested.as_str().unwrap_or(""))
            {
                return Err(config(
                    "Reinstatement refers to an unobserved replacement in this workspace",
                ));
            }
        }
        let mut effective = BTreeSet::new();
        let mut visible_history = BTreeSet::new();
        let mut hidden_effective = false;
        for record in records {
            p.check_deadline()?;
            let visible = if record.policy != policy {
                false
            } else {
                match reader::policy_allows(p, &record.replacement_path) {
                    Ok(()) => true,
                    Err(e) if e.code == "POLICY_DENIED" => false,
                    Err(e) => return Err(e),
                }
            };
            if visible {
                visible_history.insert(record.replacement.clone());
            }
            // Absence from a policy-filtered inventory cannot prove deletion.
            // Restore only a history relation visible under its bound policy.
            let restored = visible
                && reinstates.iter().any(|r| r == &record.replacement)
                && decision_states
                    .get(&record.replacement)
                    .is_none_or(|s| s != "accepted");
            if restored {
                continue;
            }
            if visible {
                effective.insert(record.replacement.clone());
            } else {
                hidden_effective = true;
            }
        }
        let replacing = decision_graph
            .iter()
            .filter(|(_, previous)| previous.iter().any(|s| s == id))
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let unresolved = item["supersedes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| !decision_graph.contains_key(id.as_str().unwrap_or("")))
            .cloned()
            .collect::<Vec<_>>();
        item["unresolved_supersedes"] = json!(unresolved);
        item["superseded_by"] = json!(replacing);
        let retired = [
            "superseded",
            "deprecated",
            "rejected",
            "cancelled",
            "completed",
        ]
        .contains(&item["status"].as_str().unwrap_or(""));
        item["historical"] = json!(retired || !effective.is_empty() || hidden_effective);
        item["current_guidance"] =
            json!(item["status"] == "accepted" && item["historical"] == false);
        item["effective_superseded_by"] = json!(effective);
        item["observed_superseded_by"] = json!(visible_history);
        item["source_id"] = item["id"].clone();
        item["validity_basis"] = json!({"status_source":"document_frontmatter",
            "supersession_source":"workspace_control_observations", "implementation_verified":false,
            "permission_granted":false,"retirement_history_outside_current_policy":hidden_effective});
    }
    for (path, expected) in source_hashes {
        if reader::read(p, &path)?.hash != expected {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Project document changed during load",
                4,
            ));
        }
    }
    let mut value = json!({"rules":rules,"decisions":decisions,"instructions":instructions,"scope_uncertain":scope.is_empty(),"source_semantics":"document claims do not establish current implementation behavior","natural_language_conflict_detection":false});
    mask(&mut value);
    Ok(value)
}

/// Read bounded authored classification from a managed document without loading
/// the decision ledger or granting instruction/exception authority.
pub(crate) fn source_topics(p: &Project, path: &str) -> Result<Option<Vec<String>>> {
    if !path.starts_with(".pctx/rules/") && !path.starts_with(".pctx/decisions/") {
        return Ok(None);
    }
    let file = reader::read(p, path)?;
    let Some(meta) = frontmatter(&file.text)? else {
        return Ok(None);
    };
    let topics = if path.starts_with(".pctx/rules/") {
        serde_json::from_value::<Rule>(meta)
            .map_err(|_| config("Invalid rule front matter fields"))?
            .topics
    } else {
        serde_json::from_value::<Decision>(meta)
            .map_err(|_| config("Invalid decision front matter fields"))?
            .topics
    };
    if let Some(topics) = &topics {
        crate::source_delivery::validate_topics(topics)?;
    }
    Ok(topics)
}

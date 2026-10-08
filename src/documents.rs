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
    id: String,
    status: String,
    date: String,
    #[serde(default, deserialize_with = "supersedes")]
    supersedes: Vec<String>,
    #[serde(default = "all")]
    scope: Vec<String>,
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
            if decision.schema_version != 1
                || !decision_ids.insert(decision.id.clone())
                || ![
                    "proposed",
                    "accepted",
                    "superseded",
                    "deprecated",
                    "rejected",
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
            decision_graph.insert(decision.id.clone(), decision.supersedes.clone());
            if !applicable(&decision.scope, scope, &relevant)? {
                continue;
            }
            item["id"] = json!(decision.id);
            item["status"] = json!(decision.status);
            item["date"] = json!(decision.date);
            item["supersedes"] = json!(decision.supersedes);
            item["scope"] = json!(decision.scope);
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
    for item in &mut decisions {
        let id = item["id"].as_str().unwrap_or("");
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
        item["historical"] = json!(
            !item["superseded_by"].as_array().unwrap().is_empty() || item["status"] == "superseded"
        );
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

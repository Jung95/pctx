use crate::{
    domain::{Error, Result, hash},
    project::Project,
    reader, storage,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
#[derive(Debug, clap::Args)]
pub struct BuildRequest {
    #[arg(long,conflicts_with_all=["task_file","task_id"])]
    pub task: Option<String>,
    #[arg(long,conflicts_with_all=["task","task_id"])]
    pub task_file: Option<String>,
    #[arg(long,conflicts_with_all=["task","task_file"])]
    pub task_id: Option<String>,
    #[arg(long)]
    pub seed: Vec<String>,
    #[arg(long, default_value_t = 12000)]
    pub budget_bytes: usize,
    #[arg(long)]
    pub budget_tokens: Option<usize>,
    #[arg(long)]
    pub tokenizer: Option<String>,
    #[arg(long)]
    pub handoff: Option<String>,
    #[arg(long, default_value = "implementer")]
    pub role: String,
    #[arg(long, default_value = "adaptive")]
    pub detail: String,
    #[arg(long)]
    pub changed_since: Option<String>,
    #[arg(long, default_value_t = 0)]
    pub dependency_depth: usize,
    #[arg(long)]
    pub explain: bool,
    #[arg(long)]
    pub require_complete: bool,
}
pub fn build(p: &Project, r: &BuildRequest) -> Result<Value> {
    if r.budget_tokens.is_some() || r.tokenizer.is_some() {
        return Err(Error::new(
            "CAPABILITY_UNAVAILABLE",
            "No tokenizer is registered; use an explicit byte budget",
            6,
        ));
    }
    if r.budget_bytes < 512 {
        return Err(Error::new(
            "BUDGET_TOO_SMALL",
            "Budget cannot hold the minimum envelope",
            8,
        ));
    }
    if r.dependency_depth > 2 {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Dependency depth is limited to 2",
            2,
        ));
    }
    if !["adaptive", "full_span", "signature", "outline", "reference"].contains(&r.detail.as_str())
    {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Unknown detail representation",
            2,
        ));
    }
    let mut task_scope = r.seed.clone();
    let task = if let Some(task) = &r.task {
        reader::redact(task).0
    } else if let Some(path) = &r.task_file {
        let text = if path == "-" {
            use std::io::Read;
            let mut text = String::new();
            std::io::stdin().take(1048577).read_to_string(&mut text)?;
            text
        } else {
            let b = std::fs::read(path)?;
            if b.len() > 1048576 {
                return Err(Error::new(
                    "FILE_TOO_LARGE",
                    "Task document exceeds limit",
                    2,
                ));
            }
            String::from_utf8(b)
                .map_err(|_| Error::new("UNSUPPORTED_ENCODING", "Task document must be UTF-8", 2))?
        };
        if text.len() > 1048576 {
            return Err(Error::new(
                "FILE_TOO_LARGE",
                "Task document exceeds limit",
                2,
            ));
        }
        reader::redact(&text).0
    } else if let Some(id) = &r.task_id {
        let view = crate::work::execute(
            p,
            &crate::work::WorkCommand::Task {
                command: crate::work::TaskCommand::Show { task: id.clone() },
            },
        )?;
        if let Some(scopes) = view["definition"]["scope"].as_array() {
            task_scope.extend(scopes.iter().filter_map(|s| s.as_str().map(str::to_string)));
        }
        reader::redact(&serde_json::to_string(&view)?).0
    } else if let Some(h) = &r.handoff {
        storage::handoff_show(p, h, true)?["content"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    } else {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Provide --task, --task-file, --task-id or --handoff",
            2,
        ));
    };
    let indexed = storage::update(p)?;
    if indexed["coverage"] == "partial" {
        return Err(Error::new(
            "PARTIAL_RESULT",
            "Strict context inventory is incomplete",
            3,
        ));
    }
    let (_, files) = storage::snapshot(p)?;
    let mut items = Vec::new();
    let mut selected_hashes = Vec::new();
    let mut omitted = Vec::new();
    let mut seen = BTreeSet::new();
    let docs = crate::documents::load(p, &task_scope)?;
    for rule in docs["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["required"] == true)
    {
        let mut item = rule.clone();
        item["representation"] = json!("full_span");
        item["reason"] = json!("required_rule");
        let path = item["path"].as_str().unwrap().to_string();
        let digest = item["file_hash"].as_str().unwrap().to_string();
        items.push(item);
        selected_hashes.push((path.clone(), digest));
        seen.insert(path);
    }
    let mut seeds = r.seed.clone();
    for section in ["rules", "decisions", "instructions"] {
        for doc in docs[section].as_array().into_iter().flatten() {
            if let Some(path) = doc["path"].as_str()
                && !seen.contains(path)
            {
                seeds.push(path.into());
            }
        }
    }
    if let Some(cp) = &r.changed_since {
        let c = storage::changes(p, cp)?;
        for item in c["items"].as_array().unwrap_or(&vec![]) {
            if ["added", "modified"].contains(&item["change"].as_str().unwrap_or("")) {
                seeds.push(item["path"].as_str().unwrap_or("").into());
            }
        }
    }
    seeds.sort();
    seeds.dedup();
    let explicit = seeds.clone();
    let mut graph_sources = Vec::new();
    if r.dependency_depth > 0 {
        for path in &explicit {
            let g = crate::graph::dependencies(p, path, r.dependency_depth, 200)?;
            for node in g["nodes"].as_array().into_iter().flatten() {
                if let Some(path) = node["path"].as_str() {
                    seeds.push(path.into());
                }
            }
            graph_sources.push(g);
        }
        seeds.sort();
        seeds.dedup();
    }
    let mut candidates = seeds
        .iter()
        .map(|s| (s.clone(), "explicit_seed"))
        .collect::<Vec<_>>();
    let terms = task
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|s| s.len() > 2)
        .take(16)
        .collect::<Vec<_>>();
    for f in &files {
        if terms.iter().any(|t| {
            f.path.to_lowercase().contains(t)
                || f.symbols.iter().any(|s| s.name.to_lowercase().contains(t))
        }) {
            candidates.push((f.path.clone(), "lexical_match"));
        }
    }
    let mut data = json!({"task":task,"role":r.role,"items":items,"omitted_items":[],"selection_complete":true,"search_coverage":{"status":"partial","reasons":["lexical_candidates_only"]},"rule_scope_uncertain":docs["scope_uncertain"],"source_versions":{"policy_hash":p.policy_hash()},"import_expansions":graph_sources,"budget":{"limit":r.budget_bytes,"used":0,"unit":"bytes"}});
    // Reserve the public envelope and remeasure the exact serialized result below.
    if measured(p, &data)? > r.budget_bytes {
        return Err(Error::new(
            "BUDGET_TOO_SMALL",
            "Required rules and task exceed the output budget",
            8,
        ));
    }
    let mut candidate_seen = BTreeSet::new();
    candidates.retain(|(path, _)| candidate_seen.insert(path.clone()));
    for (path, _) in candidates.iter().skip(200) {
        omitted.push(json!({"path":path,"reason":"candidate_limit"}));
    }
    candidates.truncate(200);
    for (path, reason) in candidates {
        if !seen.insert(path.clone()) {
            continue;
        }
        let f = reader::read(p, &path)?;
        let e = files.iter().find(|e| e.path == path);
        if e.is_some_and(|e| e.file_hash != f.hash) {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Context structure changed during selection",
                4,
            ));
        }
        let (content, redacted) = reader::redact(&f.text);
        let short = content.lines().take(80).collect::<Vec<_>>().join("\n");
        let mut item = json!({"path":path,"file_hash":f.hash,"representation":"full_span","required":false,"content":short,"redacted":redacted,"body_omitted":content.lines().count()>80,"reason":reason,"next_read":format!("pctx read {}",path)});
        if r.detail == "reference" {
            item["content"] = Value::Null;
            item["representation"] = json!("reference");
            item["body_omitted"] = json!(true);
        }
        if ["outline", "signature"].contains(&r.detail.as_str()) {
            item["content"] = Value::Null;
            item["symbols"] = json!(e.map(|e| &e.symbols));
            item["representation"] = json!(r.detail);
            item["body_omitted"] = json!(true);
        }
        data["items"].as_array_mut().unwrap().push(item.clone());
        if measured(p, &data)? > r.budget_bytes {
            data["items"].as_array_mut().unwrap().pop();
            item["content"] = Value::Null;
            item["representation"] = json!("reference");
            item["body_omitted"] = json!(true);
            data["items"].as_array_mut().unwrap().push(item);
        }
        if measured(p, &data)? > r.budget_bytes {
            data["items"].as_array_mut().unwrap().pop();
            omitted.push(json!({"path":path,"reason":"budget"}));
        } else {
            selected_hashes.push((path, f.hash));
        }
    }
    data["omitted_items"] = json!(omitted);
    data["selection_complete"] = json!(omitted.is_empty());
    if r.require_complete && !omitted.is_empty() {
        return Err(Error::new(
            "PARTIAL_RESULT",
            "Some requested context items exceed the budget",
            3,
        ));
    }
    for (path, expected) in selected_hashes {
        if reader::read(p, &path)?.hash != expected {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Selected source changed before output",
                4,
            ));
        }
    }
    data["context_fingerprint"] = json!(hash(serde_json::to_vec(&data)?));
    for _ in 0..3 {
        let used = measured(p, &data)?;
        if used > r.budget_bytes {
            return Err(Error::new(
                "BUDGET_TOO_SMALL",
                "Serialized context including omissions exceeds budget",
                8,
            ));
        }
        if data["budget"]["used"].as_u64() == Some(used as u64) {
            break;
        }
        data["budget"]["used"] = json!(used);
    }
    Ok(data)
}
fn measured(p: &Project, data: &Value) -> Result<usize> {
    Ok(serde_json::to_vec(&crate::domain::envelope("build", Some(p), data.clone()))?.len() + 1)
}

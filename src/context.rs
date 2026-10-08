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
    build_with_format(p, r, crate::render::Format::Json)
}
pub fn build_with_format(
    p: &Project,
    r: &BuildRequest,
    format: crate::render::Format,
) -> Result<Value> {
    let mut envelope = crate::domain::envelope("build", Some(p), Value::Null);
    envelope["validation"]["checked_at"] = json!("2000-01-01T00:00:00.000Z");
    envelope["validation"]["mode"] = json!("strict");
    select_with_measurement(p, r, format, |data| {
        envelope["data"] = data.clone();
        Ok(crate::render::render(&envelope, format)?.len())
    })
}

/// Shared adaptive selection for an application delivery envelope. The consumer
/// measures its complete escaped output, including its envelope and newline.
/// The measurement may run repeatedly as optional representations are lowered;
/// it must not publish receipts, acknowledge content or execute external work.
/// Selection still owns policy, source validation and the original deadline.
pub fn select_with_measurement(
    p: &Project,
    r: &BuildRequest,
    format: crate::render::Format,
    mut measure: impl FnMut(&Value) -> Result<usize>,
) -> Result<Value> {
    let mut scoped;
    let p = if p.deadline.is_none() {
        scoped = p.clone();
        scoped.deadline = Some(crate::deadline::Deadline::from_millis(120_000)?);
        &scoped
    } else {
        p
    };
    p.check_deadline()?;
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
        let deadline = p.deadline.ok_or_else(|| {
            Error::new(
                "INVALID_CONFIG",
                "Task input requires the context request deadline",
                2,
            )
        })?;
        let text = crate::input::task_document(path, deadline)?;
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
    p.check_deadline()?;
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
    let mut alternatives: Vec<Vec<Value>> = Vec::new();
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
    // Decision records may carry blocking constraints; retain their claims in full rather than
    // inferring which natural-language statements are blocking.
    for decision in docs["decisions"].as_array().into_iter().flatten() {
        let path = decision["path"].as_str().unwrap().to_string();
        if seen.insert(path.clone()) {
            let mut item = decision.clone();
            item["required"] = json!(true);
            item["representation"] = json!("full_span");
            item["reason"] = json!("required_decision");
            selected_hashes.push((path, item["file_hash"].as_str().unwrap().to_string()));
            items.push(item);
        }
    }
    let mandatory_count = items.len();
    let mut explicit = r.seed.clone();
    explicit.sort();
    explicit.dedup();
    let mut documents = Vec::new();
    for section in ["rules", "instructions"] {
        for doc in docs[section].as_array().into_iter().flatten() {
            if let Some(path) = doc["path"].as_str()
                && !seen.contains(path)
            {
                documents.push(path.to_string());
            }
        }
    }
    documents.sort();
    documents.dedup();
    let mut changed = Vec::new();
    if let Some(cp) = &r.changed_since {
        let changes = storage::changes(p, cp)?;
        for item in changes["items"].as_array().into_iter().flatten() {
            if ["added", "modified"].contains(&item["change"].as_str().unwrap_or("")) {
                let path = item["path"].as_str().unwrap_or("");
                if task_scope.iter().any(|s| {
                    path == s
                        || path.starts_with(&format!("{s}/"))
                        || globset::Glob::new(s).is_ok_and(|g| g.compile_matcher().is_match(path))
                }) {
                    changed.push(path.to_string());
                }
            }
        }
    }
    changed.sort();
    changed.dedup();
    let mut imports = Vec::new();
    let mut graph_sources = Vec::new();
    if r.dependency_depth > 0 {
        let graph_seeds: BTreeSet<_> = explicit.iter().chain(changed.iter()).collect();
        for path in graph_seeds {
            let g = crate::graph::dependencies(p, path, r.dependency_depth, 200)?;
            for node in g["nodes"].as_array().into_iter().flatten() {
                if let Some(path) = node["path"].as_str() {
                    imports.push(path.to_string());
                }
            }
            graph_sources.push(g);
        }
    }
    imports.sort();
    imports.dedup();
    let mut candidates = explicit
        .into_iter()
        .map(|p| (p, "explicit_seed"))
        .chain(changed.into_iter().map(|p| (p, "task_related_change")))
        .chain(imports.into_iter().map(|p| (p, "import_dependency")))
        .chain(documents.into_iter().map(|p| (p, "related_document")))
        .collect::<Vec<_>>();
    let terms = task
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|s| s.len() > 2)
        .take(16)
        .collect::<Vec<_>>();
    for f in &files {
        p.check_deadline()?;
        if terms.iter().any(|t| {
            f.path.to_lowercase().contains(t)
                || f.symbols.iter().any(|s| s.name.to_lowercase().contains(t))
        }) {
            candidates.push((f.path.clone(), "lexical_match"));
        }
    }
    let mut data = json!({"task":task,"role":r.role,"items":items,"omitted_items":[],"selection_complete":true,"search_coverage":{"status":"partial","reasons":["lexical_candidates_only"]},"rule_scope_uncertain":docs["scope_uncertain"],"source_versions":{"policy_hash":p.policy_hash()},"import_expansions":graph_sources,"budget":{"limit":r.budget_bytes,"used":0,"unit":"bytes"}});
    let mut candidate_seen = BTreeSet::new();
    candidates.retain(|(path, _)| !seen.contains(path) && candidate_seen.insert(path.clone()));
    for (path, _) in candidates.iter().skip(200) {
        omitted.push(json!({"path":path,"reason":"candidate_limit"}));
    }
    candidates.truncate(200);
    for (path, reason) in candidates {
        p.check_deadline()?;
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
        let tiers = representations(p, &path, &f.hash, &f.text, e, reason)?;
        let start = match r.detail.as_str() {
            "reference" => tiers.len() - 1,
            "outline" => tiers
                .iter()
                .position(|v| v["representation"] == "outline")
                .unwrap(),
            "signature" => tiers
                .iter()
                .position(|v| v["representation"] == "signature")
                .unwrap_or(1),
            "adaptive" if reason == "import_dependency" => tiers
                .iter()
                .position(|v| v["representation"] == "signature")
                .unwrap_or(1),
            "adaptive" if reason == "lexical_match" => tiers
                .iter()
                .position(|v| v["representation"] == "outline")
                .unwrap(),
            _ => 0,
        };
        let mut chosen = tiers[start..].to_vec();
        if r.detail == "signature" && chosen[0]["representation"] != "signature" {
            chosen[0]["fallback"] = json!("signature_unsupported_outline");
        }
        data["items"]
            .as_array_mut()
            .unwrap()
            .push(chosen[0].clone());
        alternatives.push(chosen);
        selected_hashes.push((path, f.hash));
    }
    data["selection_inputs"] = json!({"detail":r.detail,"seed":r.seed,"task_scope":task_scope,
        "changed_since":r.changed_since,"dependency_depth":r.dependency_depth,
        "parser_set":storage::PARSER_SET,"selector_version":"adaptive-v2",
        "format": match format { crate::render::Format::Markdown => "markdown", _ => "json" }});
    finalize(
        p,
        &mut data,
        mandatory_count,
        &mut alternatives,
        &mut omitted,
        r.budget_bytes,
        &mut measure,
    )?;
    if r.require_complete && !omitted.is_empty() {
        return Err(Error::new(
            "PARTIAL_RESULT",
            "Requested context items were omitted",
            3,
        ));
    }
    let selected_paths: BTreeSet<_> = data["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["path"].as_str())
        .collect();
    for (path, expected) in selected_hashes {
        if !selected_paths.contains(path.as_str()) {
            continue;
        }
        p.check_deadline()?;
        if reader::read(p, &path)?.hash != expected {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Selected source changed before output",
                4,
            ));
        }
    }
    p.check_deadline()?;
    Ok(data)
}

fn range(text: &str, start: usize, end: usize) -> Value {
    let start_line = text[..start].bytes().filter(|b| *b == b'\n').count() + 1;
    let end_line = start_line
        + text[start..end]
            .trim_end_matches('\n')
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
    json!({"start_byte":start,"end_byte":end,"start_line":start_line,"end_line":end_line})
}

fn representations(
    p: &Project,
    path: &str,
    digest: &str,
    text: &str,
    entry: Option<&crate::domain::FileEntry>,
    reason: &str,
) -> Result<Vec<Value>> {
    let end = text
        .split_inclusive('\n')
        .take(80)
        .map(str::len)
        .sum::<usize>();
    let (content, redacted) = reader::redact_span(text, 0, end);
    let base = json!({"path":path,"file_hash":digest,"required":false,"reason":reason,
        "next_read":format!("pctx read {}",path),"parser_set":storage::PARSER_SET});
    let mut full = base.clone();
    full["representation"] = json!("full_span");
    full["content"] = json!(content);
    full["range"] = range(text, 0, end);
    full["redacted"] = json!(redacted);
    full["body_omitted"] = json!(end < text.len());
    full["completeness"] = json!(if end < text.len() {
        "partial"
    } else {
        "complete"
    });
    let mut tiers = vec![full];
    if let Some(e) = entry {
        let signatures = signatures(p, text, e)?;
        if !signatures.is_empty() {
            let mut signature = base.clone();
            signature["representation"] = json!("signature");
            signature["signatures"] = json!(signatures);
            signature["body_omitted"] = json!(true);
            tiers.push(signature);
        }
    }
    let mut outline = base.clone();
    outline["representation"] = json!("outline");
    outline["symbols"] = json!(entry.map(|e| e.symbols.iter().map(|s|
        json!({"name":reader::redact(&s.name).0,"kind":s.kind,"qualified_name":reader::redact(&s.qualified_name).0,
            "start_line":s.start_line,"end_line":s.end_line})).collect::<Vec<_>>()).unwrap_or_default());
    outline["body_omitted"] = json!(true);
    outline["structure_status"] = json!(
        entry
            .map(|e| e.parse_status.as_str())
            .unwrap_or("unsupported")
    );
    if tiers.len() == 1 {
        outline["fallback"] = json!("signature_unsupported_outline");
    }
    tiers.push(outline);
    let mut reference = base;
    reference["representation"] = json!("reference");
    reference["body_omitted"] = json!(true);
    tiers.push(reference);
    Ok(tiers)
}

// Only grammar-declared bodies establish a signature boundary. No line/regex
// guesses, and no claim that a partially parsed declaration has a complete header.
fn signatures(p: &Project, text: &str, e: &crate::domain::FileEntry) -> Result<Vec<Value>> {
    if e.parse_status != "complete" {
        return Ok(Vec::new());
    }
    let language = match e.language.as_str() {
        "python" => tree_sitter_python::LANGUAGE.into(),
        "javascript" | "jsx" => tree_sitter_javascript::LANGUAGE.into(),
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
        _ => return Ok(Vec::new()),
    };
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language)
        .map_err(|_| Error::new("INVALID_CONFIG", "Grammar initialization failed", 2))?;
    let mut cancelled = |_: &tree_sitter::ParseState| p.check_deadline().is_err();
    let tree = parser.parse_with_options(
        &mut |offset: usize, _: tree_sitter::Point| text.as_bytes().get(offset..).unwrap_or(&[]),
        None,
        Some(tree_sitter::ParseOptions::new().progress_callback(&mut cancelled)),
    );
    p.check_deadline()?;
    let tree =
        tree.ok_or_else(|| Error::new("PARTIAL_RESULT", "Signature parser did not complete", 3))?;
    if tree.root_node().has_error() {
        return Ok(Vec::new());
    }
    let mut stack = vec![tree.root_node()];
    let mut result = Vec::new();
    while let Some(node) = stack.pop() {
        p.check_deadline()?;
        if let Some(symbol) = e
            .symbols
            .iter()
            .find(|s| s.start_byte == node.start_byte() && s.end_byte == node.end_byte())
            && let Some(body) = node.child_by_field_name("body")
            && body.start_byte() > node.start_byte()
        {
            let start = node.start_byte();
            let end = start + text[start..body.start_byte()].trim_end().len();
            let (content, redacted) = reader::redact_span(text, start, end);
            result.push(json!({"symbol_id":symbol.id,"range":range(text,start,end),"content":content,"redacted":redacted}));
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    result.sort_by_key(|v| v["range"]["start_byte"].as_u64());
    Ok(result)
}

fn finalize(
    p: &Project,
    data: &mut Value,
    mandatory: usize,
    alternatives: &mut Vec<Vec<Value>>,
    omitted: &mut Vec<Value>,
    limit: usize,
    measure: &mut impl FnMut(&Value) -> Result<usize>,
) -> Result<()> {
    let mut omission_details = true;
    loop {
        p.check_deadline()?;
        data["omitted_items"] = if omission_details {
            json!(omitted)
        } else {
            json!([])
        };
        data["omission_details_omitted"] = json!(!omission_details);
        data["omission_reasons"] = json!(
            omitted
                .iter()
                .filter_map(|v| v["reason"].as_str())
                .collect::<BTreeSet<_>>()
        );
        data["omitted_count"] = json!(omitted.len());
        data["selection_complete"] = json!(omitted.is_empty());
        data["budget"]["used"] = json!(0);
        let mut fingerprint = data.clone();
        fingerprint
            .as_object_mut()
            .unwrap()
            .remove("context_fingerprint");
        fingerprint["budget"]
            .as_object_mut()
            .unwrap()
            .remove("used");
        data["context_fingerprint"] = json!(hash(serde_json::to_vec(&fingerprint)?));
        let mut stable = false;
        for _ in 0..3 {
            p.check_deadline()?;
            let measured = measure(data);
            p.check_deadline()?;
            let used = measured?;
            if data["budget"]["used"].as_u64() == Some(used as u64) {
                stable = used <= limit;
                break;
            }
            data["budget"]["used"] = json!(used);
        }
        if stable {
            return Ok(());
        }
        // Reverse selection order is deterministic lowest priority first. Each
        // retry advances a finite tier or removes an optional item.
        let index = mandatory + alternatives.len().saturating_sub(1);
        if let Some(tiers) = alternatives.last_mut() {
            if tiers.len() > 1 {
                tiers.remove(0);
                data["items"][index] = tiers[0].clone();
                data["items"][index]["downgrade_reason"] = json!("budget");
            } else {
                let item = data["items"].as_array_mut().unwrap().pop().unwrap();
                omitted.push(
                    json!({"path":item["path"],"reason":"budget","next_read":item["next_read"]}),
                );
                alternatives.pop();
            }
        } else if omission_details && !omitted.is_empty() {
            omission_details = false;
        } else {
            return Err(Error::new(
                "BUDGET_TOO_SMALL",
                "Mandatory content and omission accounting exceed budget or failed to stabilize",
                8,
            ));
        }
    }
}

//! Explicit source packs. Plans contain hashes and spans, never source text.
use crate::{
    context,
    domain::{Error, Result, Symbol, hash, id},
    project::{Project, atomic_write, private_dir},
    reader, search, storage, work,
};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};
use tree_sitter::{Node, Parser};
const VERSION: &str = "pack-v1";
const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
const MAX_PARTS: usize = 9999;
const MAX_MANIFEST_BYTES: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, Subcommand)]
pub enum PackCommand {
    Plan {
        #[arg(long)]
        task_id: String,
        #[arg(long = "scope", required = true)]
        scopes: Vec<String>,
        #[arg(long,default_value="metadata",value_parser=["metadata","signatures","selected","full"])]
        content: String,
        #[arg(long, default_value_t = 64000)]
        budget_bytes: usize,
        #[arg(long)]
        split_bytes: Option<usize>,
    },
    Create {
        #[arg(long)]
        plan: String,
        #[arg(long)]
        expect_hash: String,
        #[arg(long)]
        output: PathBuf,
    },
    Inspect {
        path: PathBuf,
    },
    Verify {
        path: PathBuf,
        #[arg(long,value_parser=["current"])]
        against: Option<String>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Span {
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
    kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Item {
    path: String,
    source_hash: String,
    representation: String,
    spans: Vec<Span>,
    symbols: Vec<Symbol>,
    parse_status: String,
    content_hash: String,
    serialized_bytes: usize,
    redacted: bool,
    body_omitted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Plan {
    schema_version: u32,
    id: String,
    project_id: String,
    workspace_id: String,
    task_id: String,
    task_revision: i64,
    definition_revision: i64,
    task_hash: String,
    policy_hash: String,
    config_hash: String,
    parser_hash: String,
    manifest_hash: String,
    scopes: Vec<String>,
    content: String,
    budget_bytes: usize,
    split_bytes: Option<usize>,
    items: Vec<Item>,
    estimated_directory_bytes: usize,
    estimated_single_json_bytes: usize,
    omissions: Vec<Value>,
    plan_hash: String,
}
fn invalid(s: &str) -> Error {
    Error::new("INVALID_ARGUMENT", s, 2)
}
fn stale(s: &str) -> Error {
    Error::new("PACK_PLAN_STALE", s, 9)
}
fn budget(s: &str) -> Error {
    Error::new("BUDGET_TOO_SMALL", s, 8)
}
fn sanitized(v: Value) -> Value {
    match v {
        Value::String(s) => {
            let masked = reader::redact(&s).0;
            static ABSOLUTE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            let absolute = ABSOLUTE.get_or_init(|| {
                regex::Regex::new(r#"(^|[\s'"(=])(?:[A-Za-z]:[\\/]|/)[^\s'")<>]+"#)
                    .expect("fixed absolute path pattern")
            });
            json!(
                absolute
                    .replace_all(&masked, "${1}[LOCAL PATH REDACTED]")
                    .into_owned()
            )
        }
        Value::Array(a) => Value::Array(a.into_iter().map(sanitized).collect()),
        Value::Object(m) => Value::Object(
            m.into_iter()
                .map(|(k, v)| {
                    let key = reader::redact(&k).0;
                    let value = if [
                        "password",
                        "secret",
                        "api_key",
                        "token",
                        "credential",
                        "authorization",
                        "capability",
                    ]
                    .contains(&k.to_lowercase().as_str())
                    {
                        json!("[REDACTED]")
                    } else {
                        sanitized(v)
                    };
                    (key, value)
                })
                .collect(),
        ),
        v => v,
    }
}
fn task(p: &Project, name: &str) -> Result<Value> {
    work::execute(
        p,
        &work::WorkCommand::Task {
            command: work::TaskCommand::Show { task: name.into() },
        },
    )
}
fn scopes(values: &[String]) -> Result<Vec<String>> {
    if values.is_empty() {
        return Err(invalid("Pack requires explicit --scope"));
    }
    let mut out = values.to_vec();
    for s in &out {
        if s.is_empty()
            || s.contains('\\')
            || Path::new(s).is_absolute()
            || Path::new(s)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(invalid(
                "Scope must be normalized project-relative file or directory",
            ));
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}
fn selected(path: &str, scopes: &[String]) -> bool {
    scopes
        .iter()
        .any(|s| path == s || path.starts_with(&format!("{s}/")))
}
fn forbidden(p: &Project, path: &str) -> bool {
    let full = p.root.join(path);
    full.starts_with(&p.data_dir)
        || ["db", "sqlite", "sqlite3", "log"].contains(&path.rsplit('.').next().unwrap_or(""))
        || path.starts_with(".pctx/artifacts/")
        || path.starts_with(".pctx/credentials/")
}
fn plan_hash(plan: &Plan) -> Result<String> {
    let mut copy = plan.clone();
    copy.plan_hash.clear();
    Ok(hash(serde_json::to_vec(&copy)?))
}
fn config_hash(p: &Project) -> Result<String> {
    let mut configs = BTreeMap::new();
    for path in reader::inventory(p, false)?.paths {
        if path == ".pctx/config.toml"
            || path.ends_with("package.json")
            || path.ends_with("tsconfig.json")
        {
            configs.insert(path.clone(), reader::read(p, &path)?.hash);
        }
    }
    Ok(hash(serde_json::to_vec(
        &json!({"effective":p.config,"files":configs}),
    )?))
}
fn parser_hash(p: &Project) -> Result<String> {
    Ok(hash(format!(
        "{}:{}",
        storage::PARSER_SET,
        serde_json::to_string(&p.config.index)?
    )))
}
fn span(text: &str, a: usize, b: usize, kind: &str) -> Span {
    Span {
        start_byte: a,
        end_byte: b,
        start_line: text[..a].bytes().filter(|c| *c == b'\n').count() + 1,
        end_line: text[..b].bytes().filter(|c| *c == b'\n').count()
            + usize::from(b == 0 || !text[..b].ends_with('\n')),
        kind: kind.into(),
    }
}
fn notices(node: Node<'_>, text: &str, out: &mut Vec<Span>) {
    if node.kind().contains("comment")
        || (node.kind() == "expression_statement"
            && node.named_child(0).is_some_and(|n| n.kind() == "string"))
    {
        out.push(span(
            text,
            node.start_byte(),
            node.end_byte(),
            "comment_notice",
        ));
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        notices(child, text, out);
    }
}
fn signatures(path: &str, text: &str, symbols: &[Symbol]) -> Result<Vec<Span>> {
    let ext = path.rsplit('.').next().unwrap_or("");
    let grammar = match ext {
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "js" | "mjs" | "cjs" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => None,
    };
    let mut ranges = Vec::new();
    if let Some(grammar) = grammar {
        let mut parser = Parser::new();
        parser
            .set_language(&grammar)
            .map_err(|_| invalid("Parser unavailable"))?;
        let tree = parser
            .parse(text, None)
            .ok_or_else(|| invalid("Parser failed"))?;
        notices(tree.root_node(), text, &mut ranges);
        for s in symbols {
            let Some(mut node) = tree
                .root_node()
                .descendant_for_byte_range(s.start_byte, s.end_byte)
            else {
                continue;
            };
            if node.kind() == "variable_declarator"
                && let Some(value) = node.child_by_field_name("value")
            {
                node = value;
            }
            let end = node
                .child_by_field_name("body")
                .map(|n| n.start_byte())
                .unwrap_or(s.end_byte);
            ranges.push(span(text, s.start_byte, end, "signature"));
        }
    } else if matches!(ext, "md" | "markdown") {
        for s in symbols {
            let end = text[s.start_byte..]
                .find('\n')
                .map(|n| s.start_byte + n)
                .unwrap_or(text.len());
            ranges.push(span(text, s.start_byte, end, "heading"));
        }
    }
    ranges.sort_by_key(|s| (s.start_byte, s.end_byte));
    ranges.dedup_by_key(|s| (s.start_byte, s.end_byte));
    Ok(ranges)
}
fn item_value(item: &Item, text: &str) -> Result<Value> {
    let mut excerpts = Vec::new();
    let mut redacted = false;
    for range in &item.spans {
        if range.start_byte > range.end_byte || range.end_byte > text.len() {
            return Err(stale("Planned source span is no longer valid"));
        }
        let (content, masked) = reader::redact_span(text, range.start_byte, range.end_byte);
        redacted |= masked;
        excerpts.push(json!({"range":range,"text":content}));
    }
    Ok(
        json!({"path":item.path,"source_hash":item.source_hash,"representation":item.representation,"ranges":item.spans,"symbols":item.symbols,"parse_status":item.parse_status,"excerpts":excerpts,"redacted":redacted,"semantic_change_possible":redacted,"body_omitted":item.body_omitted,"omission_marker":if item.body_omitted{Some("[body omitted; use the original source and hash reference]")}else{None},"source":"external_project_data","executable_source":false}),
    )
}
fn source_set(p: &Project, scope: &[String]) -> Result<BTreeMap<String, String>> {
    let inventory = reader::inventory(p, false)?;
    if !inventory.skipped.is_empty() {
        return Err(Error::new(
            "PARTIAL_RESULT",
            "Pack scope enumeration incomplete",
            3,
        ));
    }
    let mut out = BTreeMap::new();
    for path in inventory.paths {
        if selected(&path, scope) && !forbidden(p, &path) {
            if reader::redact(&path).1 {
                return Err(Error::new(
                    "POLICY_DENIED",
                    "Sensitive source metadata rejected",
                    5,
                ));
            }
            let file = reader::read(p, &path)?;
            out.insert(path, file.hash);
        }
    }
    Ok(out)
}
type RenderedPack = (BTreeMap<String, Vec<u8>>, Vec<u8>);
fn render(plan: &Plan, items: Vec<Value>, task: Value) -> Result<RenderedPack> {
    let mut files = BTreeMap::new();
    let mut parts = Vec::new();
    let mut groups: Vec<Vec<Value>> = Vec::new();
    if let Some(limit) = plan.split_bytes {
        let mut group = Vec::new();
        for item in &items {
            let mut candidate = group.clone();
            candidate.push(item.clone());
            let trial = json!({"pack_id":plan.id,"sequence":999999,"part_count":999999,"metadata_ref":"manifest.json","items":candidate});
            if serde_json::to_vec(&trial)?.len() > limit {
                if group.is_empty() {
                    return Err(budget(
                        "One item exceeds --split-bytes; narrow scope or increase part bound",
                    ));
                }
                groups.push(group);
                group = vec![item.clone()];
                if serde_json::to_vec(&json!({"pack_id":plan.id,"sequence":999999,"part_count":999999,"metadata_ref":"manifest.json","items":group}))?.len()>limit{return Err(budget("One item exceeds --split-bytes"));}
            } else {
                group = candidate;
            }
        }
        if !group.is_empty() {
            groups.push(group);
        }
        if groups.is_empty() {
            groups.push(Vec::new());
        }
        let count = groups.len();
        if count > 9999 {
            return Err(budget("Pack exceeds 9999 part bound"));
        }
        for (i, group) in groups.into_iter().enumerate() {
            let name = format!("part-{:04}.json", i + 1);
            let bytes = serde_json::to_vec(
                &json!({"pack_id":plan.id,"sequence":i+1,"part_count":count,"metadata_ref":"manifest.json","items":group}),
            )?;
            parts.push(json!({"path":name,"sequence":i+1,"count":count,"hash":hash(&bytes),"bytes":bytes.len()}));
            files.insert(name, bytes);
        }
    } else {
        let name = "context.json";
        let bytes = serde_json::to_vec(
            &json!({"pack_id":plan.id,"task":sanitized(task.clone()),"items":items}),
        )?;
        parts.push(
            json!({"path":name,"sequence":1,"count":1,"hash":hash(&bytes),"bytes":bytes.len()}),
        );
        files.insert(name.into(), bytes);
    }
    let mut manifest = json!({"schema_version":1,"manifest_integrity_hash":"0".repeat(64),"pack_id":plan.id,"plan_hash":plan.plan_hash,"project_id":plan.project_id,"workspace_fingerprint":hash(&plan.workspace_id),"task_id":plan.task_id,"task_revision":plan.task_revision,"definition_revision":plan.definition_revision,"task_hash":plan.task_hash,"task":sanitized(task),"tool_version":VERSION,"parser_hash":plan.parser_hash,"policy_hash":plan.policy_hash,"config_hash":plan.config_hash,"source_manifest_hash":plan.manifest_hash,"scope":plan.scopes,"content":plan.content,"files":plan.items.iter().map(|i|json!({"path":i.path,"source_hash":i.source_hash,"ranges":i.spans,"representation":i.representation,"redacted":i.redacted,"semantic_change_possible":i.redacted,"body_omitted":i.body_omitted,"content_hash":i.content_hash})).collect::<Vec<_>>(),"parts":parts,"budget":{"limit_bytes":plan.budget_bytes,"directory_bytes":0,"single_json_bytes":0,"token_measurement":"unavailable"},"omissions":plan.omissions,"security_scan":"configured_policy_and_pattern_redaction","author_authenticated":false,"acknowledged":false,"restores_authority":false});
    let mut single;
    for _ in 0..8 {
        let mut digest = manifest.clone();
        digest
            .as_object_mut()
            .unwrap()
            .remove("manifest_integrity_hash");
        manifest["manifest_integrity_hash"] = json!(hash(serde_json::to_vec(&digest)?));
        let manifest_bytes = serde_json::to_vec(&manifest)?;
        if manifest_bytes.len() > MAX_MANIFEST_BYTES {
            return Err(budget("Pack manifest exceeds trusted metadata bound"));
        }
        let total = manifest_bytes.len() + files.values().map(Vec::len).sum::<usize>();
        let data: BTreeMap<_, _> = files
            .iter()
            .map(|(name, bytes)| Ok((name.clone(), serde_json::from_slice::<Value>(bytes)?)))
            .collect::<Result<_>>()?;
        single = serde_json::to_vec(&json!({"manifest":manifest,"parts":data}))?;
        if manifest["budget"]["directory_bytes"] == json!(total)
            && manifest["budget"]["single_json_bytes"] == json!(single.len())
        {
            files.insert("manifest.json".into(), manifest_bytes);
            return Ok((files, single));
        }
        manifest["budget"]["directory_bytes"] = json!(total);
        manifest["budget"]["single_json_bytes"] = json!(single.len());
    }
    Err(budget("Pack size metadata did not converge"))
}
fn plan_dir(p: &Project) -> Result<PathBuf> {
    let dir = p.control_dir.join("pack-plans");
    if fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::new("POLICY_DENIED", "Linked plan storage denied", 5));
    }
    private_dir(&dir)?;
    Ok(dir)
}
fn plan_path(p: &Project, name: &str) -> Result<PathBuf> {
    if !name.starts_with("PACKPLAN-")
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.len() > 128
    {
        return Err(invalid("Invalid plan ID"));
    }
    Ok(plan_dir(p)?.join(format!("{name}.json")))
}
fn load_plan(p: &Project, name: &str) -> Result<Plan> {
    let path = plan_path(p, name)?;
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err(Error::new("POLICY_DENIED", "Linked plan denied", 5));
    }
    let bytes = fs::read(path)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(invalid("Plan exceeds size bound"));
    }
    let plan: Plan = serde_json::from_slice(&bytes)?;
    if plan.schema_version != 1 || plan.id != name || plan.plan_hash != plan_hash(&plan)? {
        return Err(Error::new(
            "PACK_INTEGRITY_FAILED",
            "Plan integrity mismatch",
            9,
        ));
    }
    Ok(plan)
}
fn create_plan(
    p: &Project,
    task_id: &str,
    scope: &[String],
    content: &str,
    budget_bytes: usize,
    split_bytes: Option<usize>,
) -> Result<Value> {
    let scope = scopes(scope)?;
    if !["metadata", "signatures", "selected", "full"].contains(&content) {
        return Err(invalid("Unknown pack representation"));
    }
    if !(1024..=64 * 1024 * 1024).contains(&budget_bytes)
        || split_bytes.is_some_and(|n| n < 512 || n > budget_bytes)
    {
        return Err(budget(
            "Pack budget must be 1024..67108864 bytes and split bound 512..budget",
        ));
    }
    let task = task(p, task_id)?;
    let source_hashes = source_set(p, &scope)?;
    if source_hashes.is_empty() {
        return Err(invalid("No allowed files in the explicit scope"));
    }
    let chosen = if content == "selected" {
        let request = context::BuildRequest {
            session: None,
            topic: None,
            task: Some(format!(
                "{}\n{}",
                task["title"].as_str().unwrap_or(""),
                task["definition"]["description"].as_str().unwrap_or("")
            )),
            task_file: None,
            task_id: None,
            seed: source_hashes.keys().cloned().collect(),
            budget_bytes,
            budget_tokens: None,
            tokenizer: None,
            handoff: None,
            role: "implementer".into(),
            detail: "full_span".into(),
            changed_since: None,
            dependency_depth: 0,
            explain: true,
            require_complete: false,
        };
        Some(context::build(p, &request)?)
    } else {
        None
    };
    let mut items = Vec::new();
    let mut values = Vec::new();
    let mut omissions = Vec::new();
    for (path, expected) in &source_hashes {
        let f = reader::read(p, path)?;
        if &f.hash != expected {
            return Err(stale("Source changed during planning"));
        }
        let mut analysis = search::analyze(path, &f.hash, &f.text)?;
        analysis
            .symbols
            .retain(|s| !reader::redact(&s.name).1 && !reader::redact(&s.qualified_name).1);
        let mut item = Item {
            path: path.clone(),
            source_hash: f.hash,
            representation: content.into(),
            spans: Vec::new(),
            symbols: if content == "signatures" {
                analysis.symbols.clone()
            } else {
                Vec::new()
            },
            parse_status: analysis.parse_status,
            content_hash: String::new(),
            serialized_bytes: 0,
            redacted: false,
            body_omitted: content != "full",
        };
        if content == "full" {
            item.spans
                .push(span(&f.text, 0, f.text.len(), "full_source"));
            item.body_omitted = false;
        } else if content == "signatures" {
            item.spans = signatures(path, &f.text, &analysis.symbols)?;
        } else if let Some(selection) = &chosen {
            let found = selection["items"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|v| v["path"] == *path);
            if let Some(found) = found {
                if found["content"].is_string() {
                    let end = f.text.split_inclusive('\n').take(80).map(str::len).sum();
                    item.spans
                        .push(span(&f.text, 0, end, "verified_build_selection"));
                    item.body_omitted = end < f.text.len();
                } else {
                    item.body_omitted = true;
                }
            } else {
                omissions.push(json!({"path":path,"reason":"not_selected_by_verified_build"}));
                continue;
            }
        }
        let value = item_value(&item, &f.text)?;
        item.redacted = value["redacted"].as_bool().unwrap_or(false);
        item.serialized_bytes = serde_json::to_vec(&value)?.len();
        item.content_hash = hash(serde_json::to_vec(&value)?);
        values.push(value);
        items.push(item);
    }
    let mut plan = Plan {
        schema_version: 1,
        id: id("PACKPLAN"),
        project_id: p.project_id.clone(),
        workspace_id: p.workspace_id.clone(),
        task_id: task["task_id"]
            .as_str()
            .ok_or_else(|| invalid("Task ID missing"))?
            .into(),
        task_revision: task["task_revision"].as_i64().unwrap_or(0),
        definition_revision: task["definition_revision"].as_i64().unwrap_or(0),
        task_hash: hash(task.to_string()),
        policy_hash: p.policy_hash(),
        config_hash: config_hash(p)?,
        parser_hash: parser_hash(p)?,
        manifest_hash: hash(serde_json::to_vec(&source_hashes)?),
        scopes: scope,
        content: content.into(),
        budget_bytes,
        split_bytes,
        items,
        estimated_directory_bytes: 0,
        estimated_single_json_bytes: 0,
        omissions,
        plan_hash: String::new(),
    };
    // A fixed-width provisional hash keeps estimated serialization exact without a circular digest.
    plan.plan_hash = "0".repeat(64);
    let (files, single) = render(&plan, values, task)?;
    let directory_bytes = files.values().map(Vec::len).sum();
    if directory_bytes > budget_bytes {
        return Err(budget(
            "The complete pack directory exceeds the total byte budget",
        ));
    }
    plan.estimated_directory_bytes = directory_bytes;
    plan.estimated_single_json_bytes = single.len();
    plan.plan_hash = plan_hash(&plan)?;
    validate_inputs(p, &plan)?;
    let path = plan_path(p, &plan.id)?;
    atomic_write(&path, &serde_json::to_vec(&plan)?, false)?;
    Ok(
        json!({"plan_id":plan.id,"plan_hash":plan.plan_hash,"content":plan.content,"scope":plan.scopes,"items":plan.items,"budget":{"limit_bytes":budget_bytes,"directory_bytes":directory_bytes,"single_json_bytes":single.len(),"split_bytes":split_bytes,"token_measurement":"unavailable"},"security_scan":"configured_policy_and_pattern_redaction","source_persisted_in_plan":false,"single_json_available":single.len()<=budget_bytes}),
    )
}
fn output_path(p: &Project, path: &Path, create_parents: bool) -> Result<PathBuf> {
    let relative = path
        .to_str()
        .ok_or_else(|| invalid("UTF-8 pack path required"))?;
    reader::policy_allows(p, relative)?;
    if reader::redact(&path.to_string_lossy()).1
        || path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path.as_os_str().is_empty()
    {
        return Err(invalid(
            "Pack artifact path must be normalized and project-relative",
        ));
    }
    if path.starts_with(".git") || path.starts_with(".pctx") {
        return Err(Error::new(
            "POLICY_DENIED",
            "Artifact configuration/control path denied",
            5,
        ));
    }
    if create_parents {
        create_output_parents(p, path)?;
    }
    let mut current = p.root.clone();
    let parts: Vec<_> = path.components().collect();
    for (i, part) in parts.iter().enumerate() {
        current.push(part.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(Error::new(
                        "POLICY_DENIED",
                        "Linked artifact path denied",
                        5,
                    ));
                }
                if i + 1 < parts.len() && !meta.is_dir() {
                    return Err(invalid("Artifact parent is not a directory"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if i + 1 < parts.len() {
                    return Err(e.into());
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    if current.starts_with(&p.data_dir) || path.starts_with(".git") || path.starts_with(".pctx") {
        return Err(Error::new(
            "POLICY_DENIED",
            "Pack outputs cannot use private control or configuration storage",
            5,
        ));
    }
    Ok(current)
}
fn validate_inputs(p: &Project, plan: &Plan) -> Result<(Vec<Value>, Value)> {
    if plan.project_id != p.project_id
        || plan.workspace_id != p.workspace_id
        || plan.policy_hash != p.policy_hash()
        || plan.config_hash != config_hash(p)?
        || plan.parser_hash != parser_hash(p)?
    {
        return Err(stale("Project/workspace/policy/config/parser changed"));
    }
    let task = task(p, &plan.task_id)?;
    if plan.task_hash != hash(task.to_string())
        || task["task_revision"].as_i64() != Some(plan.task_revision)
        || task["definition_revision"].as_i64() != Some(plan.definition_revision)
    {
        return Err(stale("Task definition or operational state changed"));
    }
    let manifest = source_set(p, &plan.scopes)
        .map_err(|_| stale("Source scope is no longer safely readable"))?;
    if hash(serde_json::to_vec(&manifest)?) != plan.manifest_hash {
        return Err(stale("Source scope inventory or content changed"));
    }
    let mut values = Vec::new();
    for item in &plan.items {
        let f = reader::read(p, &item.path)
            .map_err(|_| stale("Planned source is no longer accessible"))?;
        if f.hash != item.source_hash {
            return Err(stale("Source hash changed"));
        }
        let value = item_value(item, &f.text)?;
        if hash(serde_json::to_vec(&value)?) != item.content_hash {
            return Err(stale("Planned representation changed"));
        }
        values.push(value);
    }
    Ok((values, task))
}
#[cfg(unix)]
fn pinned_publish(
    p: &Project,
    output: &Path,
    destination: &Path,
    plan: &Plan,
    files: &BTreeMap<String, Vec<u8>>,
    single: &[u8],
    is_single: bool,
) -> Result<()> {
    use std::{
        ffi::CString,
        io::{Read, Write},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{
                ffi::OsStrExt,
                fs::{MetadataExt, OpenOptionsExt},
            },
        },
    };
    fn name(path: &Path) -> Result<CString> {
        CString::new(path.as_os_str().as_bytes()).map_err(|_| invalid("Invalid artifact component"))
    }
    fn open_at(parent: &fs::File, path: &Path, flags: i32) -> Result<fs::File> {
        let n = name(path)?;
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                n.as_ptr(),
                flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(Error::new(
                "POLICY_DENIED",
                "Artifact open refused or failed",
                5,
            ));
        }
        Ok(unsafe { fs::File::from_raw_fd(fd) })
    }
    fn write_at(parent: &fs::File, path: &Path, bytes: &[u8]) -> Result<()> {
        let mut file = open_at(parent, path, libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    }
    fn read_at(parent: &fs::File, path: &Path) -> Result<Vec<u8>> {
        let mut file = open_at(parent, path, libc::O_RDONLY)?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(budget("Staged artifact exceeded bound"));
        }
        Ok(bytes)
    }
    let parent_path = destination
        .parent()
        .ok_or_else(|| invalid("Output parent missing"))?;
    let mut opts = fs::OpenOptions::new();
    opts.read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let mut parent = opts.open(&p.root)?;
    let relative = parent_path
        .strip_prefix(&p.root)
        .map_err(|_| invalid("Output outside root"))?;
    for component in relative.components() {
        parent = open_at(
            &parent,
            Path::new(component.as_os_str()),
            libc::O_RDONLY | libc::O_DIRECTORY,
        )?;
    }
    let parent_meta = parent.metadata()?;
    let stage_name = PathBuf::from(format!(".pctx-pack-stage-{}", uuid::Uuid::new_v4()));
    let stage_c = name(&stage_name)?;
    let destination_name = destination
        .file_name()
        .ok_or_else(|| invalid("Output name missing"))?;
    let destination_c = name(Path::new(destination_name))?;
    let result = (|| {
        if is_single {
            write_at(&parent, &stage_name, single)?;
            let bytes = read_at(&parent, &stage_name)?;
            if bytes != single {
                return Err(Error::new(
                    "PACK_INTEGRITY_FAILED",
                    "Staged single pack differs",
                    9,
                ));
            }
        } else {
            if unsafe { libc::mkdirat(parent.as_raw_fd(), stage_c.as_ptr(), 0o700) } != 0 {
                return Err(Error::new(
                    "PACK_PUBLICATION_FAILED",
                    "Cannot create private staging directory",
                    7,
                ));
            }
            let stage = open_at(&parent, &stage_name, libc::O_RDONLY | libc::O_DIRECTORY)?;
            for (filename, bytes) in files {
                write_at(&stage, Path::new(filename), bytes)?;
                if read_at(&stage, Path::new(filename))? != *bytes {
                    return Err(Error::new(
                        "PACK_INTEGRITY_FAILED",
                        "Staged part differs",
                        9,
                    ));
                }
            }
            stage.sync_all()?;
        }
        let artifact = if is_single {
            let wrapper: Value = serde_json::from_slice(single)?;
            let parts: BTreeMap<_, _> = wrapper["parts"]
                .as_object()
                .ok_or_else(|| invalid("Missing parts"))?
                .iter()
                .map(|(k, v)| Ok((k.clone(), serde_json::to_vec(v)?)))
                .collect::<Result<_>>()?;
            Artifact {
                manifest: wrapper["manifest"].clone(),
                parts,
                actual_bytes: single.len(),
            }
        } else {
            Artifact {
                manifest: serde_json::from_slice(&files["manifest.json"])?,
                parts: files
                    .iter()
                    .filter(|(k, _)| k.as_str() != "manifest.json")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                actual_bytes: files.values().map(Vec::len).sum(),
            }
        };
        verify_integrity(&artifact)?;
        validate_inputs(p, plan)?;
        output_path(p, output, false)?;
        let current = fs::symlink_metadata(parent_path)?;
        if current.file_type().is_symlink()
            || current.ino() != parent_meta.ino()
            || current.dev() != parent_meta.dev()
        {
            return Err(stale("Output parent was replaced"));
        }
        if is_single {
            if unsafe {
                libc::linkat(
                    parent.as_raw_fd(),
                    stage_c.as_ptr(),
                    parent.as_raw_fd(),
                    destination_c.as_ptr(),
                    0,
                )
            } != 0
            {
                return Err(Error::new(
                    "PACK_PUBLICATION_FAILED",
                    "Atomic no-replace file publication failed",
                    7,
                ));
            }
            if unsafe { libc::unlinkat(parent.as_raw_fd(), stage_c.as_ptr(), 0) } != 0 {
                return Err(Error::new(
                    "PACK_PUBLICATION_FAILED",
                    "Staging cleanup failed",
                    7,
                ));
            }
        } else {
            #[cfg(target_os = "macos")]
            let code = unsafe {
                libc::renameatx_np(
                    parent.as_raw_fd(),
                    stage_c.as_ptr(),
                    parent.as_raw_fd(),
                    destination_c.as_ptr(),
                    libc::RENAME_EXCL,
                )
            };
            #[cfg(target_os = "linux")]
            let code = unsafe {
                libc::renameat2(
                    parent.as_raw_fd(),
                    stage_c.as_ptr(),
                    parent.as_raw_fd(),
                    destination_c.as_ptr(),
                    libc::RENAME_NOREPLACE,
                )
            };
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            return Err(Error::new(
                "CAPABILITY_UNAVAILABLE",
                "Atomic no-replace directory publication unavailable",
                6,
            ));
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            if code != 0 {
                return Err(Error::new(
                    "PACK_PUBLICATION_FAILED",
                    "Atomic no-replace directory publication failed",
                    7,
                ));
            }
        }
        parent.sync_all()?;
        let current = fs::symlink_metadata(parent_path)?;
        if current.file_type().is_symlink()
            || current.ino() != parent_meta.ino()
            || current.dev() != parent_meta.dev()
        {
            return Err(stale("Output parent moved during publication"));
        }
        Ok(())
    })();
    if result.is_err() {
        if is_single {
            unsafe { libc::unlinkat(parent.as_raw_fd(), stage_c.as_ptr(), 0) };
        } else {
            if let Ok(stage) = open_at(&parent, &stage_name, libc::O_RDONLY | libc::O_DIRECTORY) {
                for filename in files.keys() {
                    let n = name(Path::new(filename))?;
                    unsafe { libc::unlinkat(stage.as_raw_fd(), n.as_ptr(), 0) };
                }
            }
            unsafe { libc::unlinkat(parent.as_raw_fd(), stage_c.as_ptr(), libc::AT_REMOVEDIR) };
        }
    }
    result
}
#[cfg(not(unix))]
fn pinned_publish(
    _p: &Project,
    _output: &Path,
    _destination: &Path,
    _plan: &Plan,
    _files: &BTreeMap<String, Vec<u8>>,
    _single: &[u8],
    _is_single: bool,
) -> Result<()> {
    Err(Error::new(
        "CAPABILITY_UNAVAILABLE",
        "Pack publication requires a validated pinned platform adapter",
        6,
    ))
}
fn create(p: &Project, name: &str, expected: &str, output: &Path) -> Result<Value> {
    let plan = load_plan(p, name)?;
    if expected != plan.plan_hash {
        return Err(stale("Expected plan hash differs"));
    }
    let (values, task) = validate_inputs(p, &plan)?;
    let candidate = p.root.join(output);
    for scope in &plan.scopes {
        let input = p.root.join(scope);
        if candidate == input || candidate.starts_with(&input) || input.starts_with(&candidate) {
            return Err(Error::new(
                "POLICY_DENIED",
                "Pack output overlaps requested source scope",
                5,
            ));
        }
    }
    let destination = output_path(p, output, true)?;
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(Error::new(
            "REVISION_CONFLICT",
            "Pack output already exists",
            9,
        ));
    }
    let source_paths = source_set(p, &plan.scopes)?;
    for source in source_paths.keys() {
        let input = p.root.join(source);
        if input == destination
            || input.starts_with(&destination)
            || destination.starts_with(&input)
        {
            return Err(Error::new(
                "POLICY_DENIED",
                "Pack output overlaps selected source",
                5,
            ));
        }
    }
    let (files, single) = render(&plan, values, task)?;
    let is_single = output.extension().is_some_and(|s| s == "json");
    let actual = if is_single {
        single.len()
    } else {
        files.values().map(Vec::len).sum()
    };
    if actual > plan.budget_bytes {
        return Err(budget("Actual complete output exceeds pack budget"));
    }
    if (!is_single && actual != plan.estimated_directory_bytes)
        || (is_single && actual != plan.estimated_single_json_bytes)
    {
        return Err(stale("Rendered output size differs from immutable plan"));
    }
    pinned_publish(p, output, &destination, &plan, &files, &single, is_single)?;
    Ok(
        json!({"pack_id":plan.id,"plan_hash":plan.plan_hash,"output":output.to_string_lossy(),"format":if is_single{"single_json"}else{"directory"},"actual_bytes":actual,"budget_bytes":plan.budget_bytes,"integrity":"verified","freshness":"current","published":true,"acknowledged":false,"authority_restored":false}),
    )
}

struct Artifact {
    manifest: Value,
    parts: BTreeMap<String, Vec<u8>>,
    actual_bytes: usize,
}
fn part_name(name: &str) -> bool {
    name == "context.json"
        || (name.starts_with("part-")
            && name.ends_with(".json")
            && name.len() == 14
            && name[5..9].bytes().all(|c| c.is_ascii_digit()))
}
fn artifact_json(bytes: &[u8]) -> Result<Value> {
    // Bound JSON allocation amplification before serde creates arrays/objects.
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0usize;
    let mut nodes = 0usize;
    for byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match *byte {
            b'"' => quoted = true,
            b'[' | b'{' => {
                depth += 1;
                nodes += 1;
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            b',' => nodes += 1,
            _ => {}
        }
        if depth > 64 || nodes > 200000 {
            return Err(budget("Artifact JSON exceeds trusted complexity bound"));
        }
    }
    serde_json::from_slice(bytes).map_err(Into::into)
}
#[cfg(unix)]
fn artifact_policy(p: &Project, manifest: &Value) -> Result<()> {
    for item in manifest["files"]
        .as_array()
        .ok_or_else(|| invalid("Missing source metadata"))?
    {
        reader::policy_allows(
            p,
            item["path"]
                .as_str()
                .ok_or_else(|| invalid("Invalid source reference"))?,
        )?;
    }
    for scope in manifest["scope"]
        .as_array()
        .ok_or_else(|| invalid("Missing pack scope"))?
    {
        reader::policy_allows(
            p,
            scope
                .as_str()
                .ok_or_else(|| invalid("Invalid pack scope"))?,
        )?;
    }
    Ok(())
}
fn declared_parts(manifest: &Value) -> Result<Vec<String>> {
    if manifest["schema_version"] != 1 {
        return Err(invalid("Unsupported pack schema"));
    }
    let limit = manifest["budget"]["limit_bytes"]
        .as_u64()
        .ok_or_else(|| invalid("Missing pack budget"))?;
    if limit == 0 || limit > MAX_ARTIFACT_BYTES as u64 {
        return Err(budget("Declared budget exceeds the trusted artifact limit"));
    }
    let parts = manifest["parts"]
        .as_array()
        .ok_or_else(|| invalid("Manifest lacks parts"))?;
    if parts.is_empty() || parts.len() > MAX_PARTS {
        return Err(budget("Manifest exceeds trusted part count"));
    }
    let mut names = BTreeSet::new();
    let mut total = 0u64;
    for part in parts {
        let name = part["path"]
            .as_str()
            .ok_or_else(|| invalid("Missing part path"))?;
        if !part_name(name) || !names.insert(name.to_string()) {
            return Err(invalid("Unsafe or duplicate part name"));
        }
        let bytes = part["bytes"]
            .as_u64()
            .ok_or_else(|| invalid("Missing part size"))?;
        total = total
            .checked_add(bytes)
            .ok_or_else(|| budget("Declared part sizes overflow"))?;
        if total > MAX_ARTIFACT_BYTES as u64 || total > limit {
            return Err(budget("Declared parts exceed trusted aggregate budget"));
        }
    }
    Ok(names.into_iter().collect())
}
#[cfg(unix)]
fn read_bytes(mut file: fs::File, remaining: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(Error::new(
            "POLICY_DENIED",
            "Artifact entry must be a regular file",
            5,
        ));
    }
    if meta.len() > remaining as u64 {
        return Err(budget("Artifact exceeds trusted aggregate size"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(remaining as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > remaining {
        return Err(budget("Artifact grew past aggregate size bound"));
    }
    Ok(bytes)
}
#[cfg(unix)]
fn open_artifact(p: &Project, path: &Path) -> Result<fs::File> {
    use std::{
        ffi::CString,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::OpenOptionsExt},
        },
    };
    let mut opts = fs::OpenOptions::new();
    opts.read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC);
    let mut current = opts.open(&p.root)?;
    let components = path.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        if !matches!(component, Component::Normal(_)) {
            return Err(invalid("Invalid artifact component"));
        }
        let name = CString::new(component.as_os_str().as_bytes())
            .map_err(|_| invalid("Invalid artifact name"))?;
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if index + 1 < components.len() {
                libc::O_DIRECTORY
            } else {
                0
            };
        let fd = unsafe { libc::openat(current.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(Error::new(
                "POLICY_DENIED",
                "Pinned artifact open denied",
                5,
            ));
        }
        current = unsafe { fs::File::from_raw_fd(fd) };
    }
    Ok(current)
}
#[cfg(unix)]
fn read_directory(p: &Project, path: &Path, parent: fs::File) -> Result<Artifact> {
    use std::{
        ffi::CString,
        os::fd::{AsRawFd, FromRawFd},
    };
    fn open(parent: &fs::File, name: &str) -> Result<fs::File> {
        let name = CString::new(name).map_err(|_| invalid("Invalid artifact entry"))?;
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(Error::new(
                "PACK_INTEGRITY_FAILED",
                "Required artifact entry is missing or linked",
                9,
            ));
        }
        Ok(unsafe { fs::File::from_raw_fd(fd) })
    }
    reader::policy_allows(
        p,
        path.join("manifest.json")
            .to_str()
            .ok_or_else(|| invalid("UTF-8 artifact path required"))?,
    )?;
    let manifest_bytes = read_bytes(open(&parent, "manifest.json")?, MAX_MANIFEST_BYTES)?;
    let manifest = artifact_json(&manifest_bytes)?;
    artifact_policy(p, &manifest)?;
    let names = declared_parts(&manifest)?;
    let mut total = manifest_bytes.len();
    let mut parts = BTreeMap::new();
    let declared_limit = manifest["budget"]["limit_bytes"].as_u64().unwrap() as usize;
    let declared_total = manifest["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["bytes"].as_u64().unwrap() as usize)
        .sum::<usize>();
    if total.saturating_add(declared_total) > declared_limit
        || total.saturating_add(declared_total) > MAX_ARTIFACT_BYTES
    {
        return Err(budget(
            "Manifest and parts exceed aggregate budget before reads",
        ));
    }
    for name in &names {
        reader::policy_allows(
            p,
            path.join(name)
                .to_str()
                .ok_or_else(|| invalid("UTF-8 artifact path required"))?,
        )?;
    }
    for name in names {
        let declared = manifest["parts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|part| part["path"] == name)
            .and_then(|part| part["bytes"].as_u64())
            .unwrap() as usize;
        let remaining = MAX_ARTIFACT_BYTES
            .saturating_sub(total)
            .min(declared_limit.saturating_sub(total))
            .min(declared);
        let bytes = read_bytes(open(&parent, &name)?, remaining)?;
        total += bytes.len();
        parts.insert(name, bytes);
    }
    // fdopendir owns only a duplicate; enumerate the same pinned directory as reads.
    let duplicated = unsafe { libc::dup(parent.as_raw_fd()) };
    if duplicated < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let directory = unsafe { libc::fdopendir(duplicated) };
    if directory.is_null() {
        unsafe { libc::close(duplicated) };
        return Err(std::io::Error::last_os_error().into());
    }
    let expected: BTreeSet<_> = parts
        .keys()
        .map(String::as_str)
        .chain(std::iter::once("manifest.json"))
        .collect();
    let mut unexpected = false;
    loop {
        let entry = unsafe { libc::readdir(directory) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name != b"."
            && name != b".."
            && !std::str::from_utf8(name).is_ok_and(|name| expected.contains(name))
        {
            unexpected = true;
            break;
        }
    }
    unsafe { libc::closedir(directory) };
    if unexpected {
        return Err(Error::new(
            "PACK_INTEGRITY_FAILED",
            "Unexpected artifact entry",
            9,
        ));
    }
    Ok(Artifact {
        manifest,
        parts,
        actual_bytes: total,
    })
}
#[cfg(unix)]
fn read_single(p: &Project, bytes: Vec<u8>) -> Result<Artifact> {
    let wrapper = artifact_json(&bytes)?;
    let manifest = wrapper
        .get("manifest")
        .cloned()
        .ok_or_else(|| invalid("Single pack lacks manifest"))?;
    artifact_policy(p, &manifest)?;
    declared_parts(&manifest)?;
    let data = wrapper["parts"]
        .as_object()
        .ok_or_else(|| invalid("Single pack lacks parts"))?;
    if data.len() > MAX_PARTS {
        return Err(budget("Single pack exceeds part count"));
    }
    let mut parts = BTreeMap::new();
    let mut total = 0usize;
    for (name, value) in data {
        if !part_name(name) {
            return Err(invalid("Unsafe part name"));
        }
        let part = serde_json::to_vec(value)?;
        total = total
            .checked_add(part.len())
            .ok_or_else(|| budget("Single parts overflow"))?;
        if total > MAX_ARTIFACT_BYTES {
            return Err(budget("Single pack parts exceed aggregate bound"));
        }
        parts.insert(name.clone(), part);
    }
    Ok(Artifact {
        manifest,
        parts,
        actual_bytes: bytes.len(),
    })
}
fn read_artifact(p: &Project, path: &Path) -> Result<Artifact> {
    #[cfg(unix)]
    {
        let file = open_artifact(p, path)?;
        let metadata = file.metadata()?;
        if metadata.is_file() {
            return read_single(p, read_bytes(file, MAX_ARTIFACT_BYTES)?);
        }
        if metadata.is_dir() {
            return read_directory(p, path, file);
        }
        Err(invalid("Pack artifact must be a file or directory"))
    }
    #[cfg(not(unix))]
    {
        let _ = (p, path);
        Err(Error::new(
            "CAPABILITY_UNAVAILABLE",
            "Pinned artifact reads require a validated platform adapter",
            6,
        ))
    }
}
fn verify_integrity(artifact: &Artifact) -> Result<()> {
    let m = &artifact.manifest;
    declared_parts(m)?;
    if artifact.actual_bytes > MAX_ARTIFACT_BYTES {
        return Err(budget("Artifact exceeds trusted aggregate limit"));
    }
    let mut digest = m.clone();
    digest
        .as_object_mut()
        .ok_or_else(|| invalid("Invalid manifest"))?
        .remove("manifest_integrity_hash");
    if m["manifest_integrity_hash"].as_str() != Some(hash(serde_json::to_vec(&digest)?).as_str()) {
        return Err(Error::new(
            "PACK_INTEGRITY_FAILED",
            "Manifest checksum mismatch",
            9,
        ));
    }
    if m["schema_version"] != 1 {
        return Err(invalid("Unsupported pack schema"));
    }
    let declared = m["parts"]
        .as_array()
        .ok_or_else(|| invalid("Manifest parts missing"))?;
    if declared.len() != artifact.parts.len() {
        return Err(Error::new(
            "PACK_INTEGRITY_FAILED",
            "Part count mismatch",
            9,
        ));
    }
    let mut names = BTreeSet::new();
    for (i, part) in declared.iter().enumerate() {
        let name = part["path"]
            .as_str()
            .ok_or_else(|| invalid("Part path missing"))?;
        if !names.insert(name) {
            return Err(Error::new("PACK_INTEGRITY_FAILED", "Duplicate part", 9));
        }
        let bytes = artifact
            .parts
            .get(name)
            .ok_or_else(|| Error::new("PACK_INTEGRITY_FAILED", "Required part missing", 9))?;
        if part["hash"].as_str() != Some(&hash(bytes))
            || part["bytes"].as_u64() != Some(bytes.len() as u64)
            || part["sequence"].as_u64() != Some(i as u64 + 1)
            || part["count"].as_u64() != Some(declared.len() as u64)
        {
            return Err(Error::new(
                "PACK_INTEGRITY_FAILED",
                "Part hash/size/sequence mismatch",
                9,
            ));
        }
        let data = artifact_json(bytes)?;
        if data["pack_id"] != m["pack_id"] {
            return Err(Error::new(
                "PACK_INTEGRITY_FAILED",
                "Part belongs to another pack",
                9,
            ));
        }
        if name.starts_with("part-")
            && (data["sequence"] != part["sequence"] || data["part_count"] != part["count"])
        {
            return Err(Error::new(
                "PACK_INTEGRITY_FAILED",
                "Part metadata mismatch",
                9,
            ));
        }
    }
    if artifact.actual_bytes > m["budget"]["limit_bytes"].as_u64().unwrap_or(0) as usize {
        return Err(budget("Artifact exceeds its declared total byte budget"));
    }
    Ok(())
}
fn inspection_metadata(mut manifest: Value) -> Value {
    if let Some(object) = manifest.as_object_mut() {
        object.retain(|key, _| {
            [
                "schema_version",
                "manifest_integrity_hash",
                "pack_id",
                "plan_hash",
                "project_id",
                "workspace_fingerprint",
                "task_id",
                "task_revision",
                "definition_revision",
                "task_hash",
                "task",
                "tool_version",
                "parser_hash",
                "policy_hash",
                "config_hash",
                "source_manifest_hash",
                "scope",
                "content",
                "files",
                "parts",
                "budget",
                "omissions",
                "security_scan",
                "author_authenticated",
                "acknowledged",
                "restores_authority",
            ]
            .contains(&key.as_str())
        });
    }
    if let Some(files) = manifest["files"].as_array_mut() {
        for file in files {
            if let Some(object) = file.as_object_mut() {
                object.retain(|key, _| {
                    [
                        "path",
                        "source_hash",
                        "ranges",
                        "representation",
                        "redacted",
                        "semantic_change_possible",
                        "body_omitted",
                        "content_hash",
                    ]
                    .contains(&key.as_str())
                });
            }
        }
    }
    for field in ["author_authenticated", "acknowledged", "restores_authority"] {
        manifest[field] = json!(false);
    }
    sanitized(manifest)
}
fn inspect(p: &Project, path: &Path, against: bool) -> Result<Value> {
    output_path(p, path, false)?;
    let artifact = read_artifact(p, path)?;
    verify_integrity(&artifact)?;
    let sources = artifact.manifest["files"]
        .as_array()
        .ok_or_else(|| invalid("Missing source metadata"))?;
    for item in sources {
        let source = item["path"]
            .as_str()
            .ok_or_else(|| invalid("Invalid source reference"))?;
        reader::policy_allows(p, source)?;
    }
    for scope in artifact.manifest["scope"]
        .as_array()
        .ok_or_else(|| invalid("Missing pack scope"))?
    {
        reader::policy_allows(
            p,
            scope
                .as_str()
                .ok_or_else(|| invalid("Invalid pack scope"))?,
        )?;
    }
    let mut statuses = Vec::new();
    let mut freshness = "unknown";
    if against {
        let mut changed = false;
        let mut unknown = false;
        for item in sources {
            let source = item["path"].as_str().unwrap();
            let status = match reader::read(p, source) {
                Ok(f) => {
                    if item["source_hash"].as_str() == Some(&f.hash) {
                        "current"
                    } else {
                        changed = true;
                        "stale"
                    }
                }
                Err(_) => {
                    unknown = true;
                    "unknown"
                }
            };
            statuses.push(json!({"path":source,"freshness":status}));
        }
        if artifact.manifest["project_id"] != p.project_id
            || artifact.manifest["workspace_fingerprint"] != hash(&p.workspace_id)
            || artifact.manifest["tool_version"] != VERSION
            || artifact.manifest["policy_hash"] != p.policy_hash()
            || artifact.manifest["config_hash"] != config_hash(p)?
            || artifact.manifest["parser_hash"] != parser_hash(p)?
        {
            changed = true;
        }
        match artifact.manifest["task_id"]
            .as_str()
            .map(|name| task(p, name))
            .transpose()
        {
            Ok(Some(current)) => {
                if artifact.manifest["task_revision"] != current["task_revision"]
                    || artifact.manifest["definition_revision"] != current["definition_revision"]
                    || artifact.manifest["task_hash"] != hash(current.to_string())
                {
                    changed = true;
                }
            }
            _ => unknown = true,
        }
        if !unknown {
            let scope = artifact.manifest["scope"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect::<Vec<_>>();
            match source_set(p, &scope) {
                Ok(current) => {
                    if artifact.manifest["source_manifest_hash"]
                        != hash(serde_json::to_vec(&current)?)
                    {
                        changed = true;
                    }
                }
                Err(_) => unknown = true,
            }
        }
        freshness = if changed {
            "stale"
        } else if unknown {
            "unknown"
        } else {
            "current"
        };
    }
    Ok(
        json!({"manifest":inspection_metadata(artifact.manifest),"integrity":"verified","freshness":freshness,"source_validation":sanitized(json!(statuses)),"actual_bytes":artifact.actual_bytes,"acknowledged":false,"author_authenticated":false,"restores_authority":false}),
    )
}
pub fn execute(p: &Project, c: &PackCommand) -> Result<Value> {
    match c {
        PackCommand::Plan {
            task_id,
            scopes,
            content,
            budget_bytes,
            split_bytes,
        } => create_plan(p, task_id, scopes, content, *budget_bytes, *split_bytes),
        PackCommand::Create {
            plan,
            expect_hash,
            output,
        } => create(p, plan, expect_hash, output),
        PackCommand::Inspect { path } => inspect(p, path, false),
        PackCommand::Verify { path, against } => {
            inspect(p, path, against.as_deref() == Some("current"))
        }
    }
}

#[cfg(unix)]
fn create_output_parents(p: &Project, path: &Path) -> Result<()> {
    use std::{
        ffi::CString,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        },
    };
    let mut dir = fs::File::open(&p.root)?;
    let components = path.components().collect::<Vec<_>>();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        let name = CString::new(component.as_os_str().as_bytes())
            .map_err(|_| invalid("Invalid parent directory"))?;
        let code = unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o700) };
        if code != 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(std::io::Error::last_os_error().into());
        }
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(Error::new(
                "POLICY_DENIED",
                "Linked output parent denied",
                5,
            ));
        }
        dir = unsafe { fs::File::from_raw_fd(fd) };
    }
    Ok(())
}
#[cfg(not(unix))]
fn create_output_parents(_p: &Project, _path: &Path) -> Result<()> {
    Err(Error::new(
        "CAPABILITY_UNAVAILABLE",
        "Pinned output parent creation is unavailable on this platform",
        6,
    ))
}

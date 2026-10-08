//! Static import graph, recomputed from verified files in this workspace.
use crate::{
    deadline::Deadline,
    domain::{Error, Result, hash},
    project::Project,
    reader, storage,
};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tree_sitter::{Node, ParseOptions, Parser};
const VERSION: &str = "static-import-v1";
#[derive(Debug, Clone, Subcommand)]
pub enum GraphCommand {
    Trace {
        path: String,
        #[arg(long,default_value="outgoing",value_parser=["outgoing","incoming"])]
        direction: String,
        #[arg(long, default_value_t = 2)]
        depth: usize,
    },
    Impact {
        path: String,
        #[arg(long, default_value_t = 2)]
        depth: usize,
    },
    Refs {
        path: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Edge {
    id: String,
    source: String,
    source_hash: String,
    target: Option<String>,
    target_hash: Option<String>,
    specifier: String,
    relation: String,
    evidence_status: String,
    resolution: String,
    reason: Option<String>,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
    resolver_version: String,
    config_hashes: BTreeMap<String, String>,
    resolver_fingerprint: String,
}
struct Graph {
    edges: Vec<Edge>,
    sources: BTreeMap<String, String>,
    config_hashes: BTreeMap<String, String>,
    generation: Option<String>,
    warnings: Vec<Value>,
    manifest_hash: String,
}
#[derive(Debug, Clone)]
struct Config {
    path: String,
    dir: String,
    body: Value,
}
fn invalid(s: &str) -> Error {
    Error::new("INVALID_ARGUMENT", s, 2)
}
fn validate_bounds(direction: &str, depth: usize, max_nodes: usize) -> Result<()> {
    if !["incoming", "outgoing"].contains(&direction) {
        return Err(invalid("Direction must be incoming or outgoing"));
    }
    if depth > 32 || max_nodes == 0 || max_nodes > 1000 {
        return Err(invalid("Graph bounds: depth <=32, max_nodes 1..1000"));
    }
    Ok(())
}
/// Pure existing graph grammar, before filesystem authorization or index access.
pub fn validate_request(command: &GraphCommand) -> Result<()> {
    let path = match command {
        GraphCommand::Trace {
            path,
            direction,
            depth,
        } => {
            validate_bounds(direction, *depth, 1000)?;
            path
        }
        GraphCommand::Impact { path, depth } => {
            validate_bounds("incoming", *depth, 1000)?;
            path
        }
        GraphCommand::Refs { path } => path,
    };
    reader::validate_relative_path(path)
}

fn directory(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(d, _)| d.into())
        .unwrap_or_default()
}
fn join(base: &str, rel: &str) -> Option<String> {
    if rel.starts_with('/') || rel.contains('\\') || rel.contains('\0') {
        return None;
    }
    let mut stack: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    for p in rel.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                stack.pop()?;
            }
            _ => stack.push(p),
        }
    }
    Some(stack.join("/"))
}
fn source_language(path: &str) -> Option<&'static str> {
    match path.rsplit('.').next()? {
        "js" | "mjs" | "cjs" | "jsx" => Some("javascript"),
        "ts" => Some("typescript"),
        "tsx" => Some("tsx"),
        _ => None,
    }
}
type ImportSpan = (String, usize, usize, usize, usize, Option<String>);
fn extract<'a>(p: &Project, node: Node<'a>, text: &str, out: &mut Vec<ImportSpan>) -> Result<()> {
    p.check_deadline()?;
    let mut found = None;
    if matches!(node.kind(), "import_statement" | "export_statement") {
        if let Some(source) = node.child_by_field_name("source") {
            found = Some((source, None));
        }
    } else if node.kind() == "call_expression"
        && let Some(function) = node.child_by_field_name("function")
    {
        let name = function.utf8_text(text.as_bytes()).unwrap_or("");
        if (function.kind() == "import" || name == "import" || name == "require")
            && let Some(args) = node.child_by_field_name("arguments")
        {
            let source = args.named_child(0).unwrap_or(args);
            found = Some((
                source,
                Some(
                    if name == "require" {
                        "commonjs_require_unsupported"
                    } else {
                        "dynamic_import"
                    }
                    .into(),
                ),
            ));
        }
    }
    if let Some((source, reason)) = found {
        let raw = source.utf8_text(text.as_bytes()).unwrap_or("");
        let spec = if source.kind() == "string" && raw.len() >= 2 {
            raw[1..raw.len() - 1].to_owned()
        } else {
            "<dynamic-expression>".into()
        };
        out.push((
            spec,
            source.start_byte(),
            source.end_byte(),
            source.start_position().row + 1,
            source.end_position().row + 1,
            reason,
        ));
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        extract(p, child, text, out)?;
    }
    p.check_deadline()
}
fn candidate(base: &str, paths: &BTreeSet<String>) -> Option<String> {
    if paths.contains(base) {
        return Some(base.into());
    }
    for ext in ["ts", "tsx", "js", "jsx", "mjs", "cjs", "json"] {
        let p = format!("{base}.{ext}");
        if paths.contains(&p) {
            return Some(p);
        }
    }
    if base.ends_with(".js") {
        let prefix = base.trim_end_matches(".js");
        for ext in ["ts", "tsx"] {
            let p = format!("{prefix}.{ext}");
            if paths.contains(&p) {
                return Some(p);
            }
        }
    }
    for ext in ["ts", "tsx", "js", "jsx", "mjs", "cjs"] {
        let p = format!("{base}/index.{ext}");
        if paths.contains(&p) {
            return Some(p);
        }
    }
    None
}
fn mapping(pattern: &str, spec: &str) -> Option<String> {
    if let Some((a, b)) = pattern.split_once('*') {
        if b.contains('*') {
            return None;
        }
        let capture = spec.strip_prefix(a)?.strip_suffix(b)?;
        Some(capture.into())
    } else if pattern == spec {
        Some(String::new())
    } else {
        None
    }
}
fn resolve(
    p: &Project,
    source: &str,
    spec: &str,
    paths: &BTreeSet<String>,
    configs: &[Config],
) -> Result<(Option<String>, String, BTreeSet<String>)> {
    p.check_deadline()?;
    let mut proof = BTreeSet::new();
    if spec.starts_with('.') {
        return Ok((
            join(&directory(source), spec).and_then(|b| candidate(&b, paths)),
            "relative_import".into(),
            proof,
        ));
    }
    if spec.starts_with('/')
        || spec.contains(':')
        || spec.contains('\\')
        || spec.contains('#')
        || spec.contains('?')
    {
        return Ok((None, "unsupported_specifier".into(), proof));
    }
    let mut config = None;
    for c in configs {
        p.check_deadline()?;
        if c.path.ends_with("tsconfig.json")
            && (c.dir.is_empty() || source.starts_with(&format!("{}/", c.dir)))
            && config.is_none_or(|old: &Config| c.dir.len() >= old.dir.len())
        {
            config = Some(c);
        }
    }
    if let Some(c) = config
        && let Some(maps) = c
            .body
            .pointer("/compilerOptions/paths")
            .and_then(Value::as_object)
    {
        let mut matched = Vec::new();
        for (key, v) in maps {
            p.check_deadline()?;
            if let Some(capture) = mapping(key, spec) {
                matched.push((key, capture, v));
            }
        }
        matched.sort_by(|a, b| {
            b.0.trim_end_matches('*')
                .len()
                .cmp(&a.0.trim_end_matches('*').len())
                .then_with(|| a.0.cmp(b.0))
        });
        p.check_deadline()?;
        if let Some((_, capture, value)) = matched.first() {
            proof.insert(c.path.clone());
            let base = c
                .body
                .pointer("/compilerOptions/baseUrl")
                .and_then(Value::as_str)
                .unwrap_or(".");
            let mut targets = BTreeSet::new();
            if let Some(values) = value.as_array() {
                for value in values {
                    p.check_deadline()?;
                    let Some(target) = value.as_str() else {
                        continue;
                    };
                    if let Some(target) = join(&c.dir, base)
                        .and_then(|b| join(&b, &target.replace('*', capture)))
                        .and_then(|b| candidate(&b, paths))
                    {
                        targets.insert(target);
                    }
                }
            }
            return Ok(if targets.len() == 1 {
                (targets.into_iter().next(), "tsconfig_paths".into(), proof)
            } else {
                (
                    None,
                    if targets.len() > 1 {
                        "ambiguous_tsconfig_paths"
                    } else {
                        "tsconfig_target_missing"
                    }
                    .into(),
                    proof,
                )
            });
        }
    }
    let mut packages = Vec::new();
    for c in configs {
        p.check_deadline()?;
        if c.path.ends_with("package.json")
            && let Some(name) = c.body["name"].as_str()
            && (spec == name || spec.starts_with(&format!("{name}/")))
        {
            packages.push((c, name));
        }
    }
    packages.sort_by_key(|(_, name)| std::cmp::Reverse(name.len()));
    p.check_deadline()?;
    if let Some((c, name)) = packages.first() {
        if packages.iter().filter(|(_, n)| n == name).count() > 1 {
            return Ok((None, "ambiguous_workspace_package".into(), proof));
        }
        proof.insert(c.path.clone());
        let key = if spec == *name {
            ".".to_owned()
        } else {
            format!(".{}", &spec[name.len()..])
        };
        let exports = &c.body["exports"];
        let value = if exports.is_string() && key == "." {
            Some((exports.clone(), String::new()))
        } else if let Some(m) = exports.as_object() {
            let mut options = Vec::new();
            for (pattern, value) in m {
                p.check_deadline()?;
                if let Some(cap) = mapping(pattern, &key) {
                    options.push((pattern, value, cap));
                }
            }
            options.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(b.0)));
            p.check_deadline()?;
            options
                .first()
                .map(|(_, v, cap)| ((*v).clone(), cap.clone()))
        } else {
            None
        };
        if let Some((value, capture)) = value {
            if let Some(target) = value.as_str() {
                if !target.starts_with("./") || target.split('/').any(|part| part == "..") {
                    return Ok((None, "invalid_package_export".into(), proof));
                }
                return Ok((
                    join(&c.dir, &target.replace('*', &capture)).and_then(|b| candidate(&b, paths)),
                    "workspace_package_export".into(),
                    proof,
                ));
            }
            return Ok((None, "conditional_package_export_unsupported".into(), proof));
        }
        return Ok((
            None,
            if exports.is_object()
                && exports
                    .as_object()
                    .unwrap()
                    .keys()
                    .any(|k| !k.starts_with('.'))
            {
                "conditional_package_export_unsupported"
            } else {
                "workspace_export_missing"
            }
            .into(),
            proof,
        ));
    }
    p.check_deadline()?;
    Ok((None, "external_or_unmapped_package".into(), proof))
}
fn graph(p: &Project) -> Result<Graph> {
    let (generation, _cached) = storage::snapshot(p)?;
    let inventory = reader::inventory(p, false)?;
    let mut warnings = inventory.skipped;
    let mut sources = BTreeMap::new();
    let mut texts = BTreeMap::new();
    let mut configs = Vec::new();
    let mut config_hashes = BTreeMap::new();
    let paths: BTreeSet<_> = inventory.paths.iter().cloned().collect();
    for path in &paths {
        p.check_deadline()?;
        if source_language(path).is_none()
            && !path.ends_with("package.json")
            && !path.ends_with("tsconfig.json")
        {
            continue;
        }
        if texts.len() >= 10000 {
            warnings.push(json!({"reason":"graph_scan_limit"}));
            break;
        }
        let f = match reader::read(p, path) {
            Ok(v) => v,
            Err(e) => {
                if e.code == "TIMEOUT" {
                    return Err(e);
                }
                p.check_deadline()?;
                warnings.push(json!({"path":path,"reason":e.code}));
                continue;
            }
        };
        sources.insert(path.clone(), f.hash.clone());
        if path.ends_with("package.json") || path.ends_with("tsconfig.json") {
            config_hashes.insert(path.clone(), f.hash.clone());
            match serde_json::from_str::<Value>(&f.text) {
                Ok(body) => {
                    if body.get("extends").is_some() {
                        warnings.push(json!({"path":path,"reason":"tsconfig_extends_unsupported"}));
                    }
                    configs.push(Config {
                        path: path.clone(),
                        dir: directory(path),
                        body,
                    });
                }
                Err(_) => {
                    warnings.push(json!({"path":path,"reason":"configuration_not_plain_json"}))
                }
            }
        }
        p.check_deadline()?;
        if source_language(path).is_some() {
            texts.insert(path.clone(), f.text);
        }
    }
    let manifest_hash = hash(serde_json::to_vec(
        &json!({"workspace":p.workspace_id,"sources":sources,"config_hashes":config_hashes,"policy":p.policy_hash(),"resolver":VERSION}),
    )?);
    p.check_deadline()?;
    let mut edges = Vec::new();
    for (path, text) in texts {
        p.check_deadline()?;
        let mut parser = Parser::new();
        let grammar = match source_language(&path).unwrap() {
            "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
            _ => tree_sitter_javascript::LANGUAGE.into(),
        };
        parser
            .set_language(&grammar)
            .map_err(|_| invalid("Grammar unavailable"))?;
        let mut progress = |_: &tree_sitter::ParseState| p.check_deadline().is_err();
        let mut input = |offset, _| &text.as_bytes()[offset..];
        let tree = parser.parse_with_options(
            &mut input,
            None,
            Some(ParseOptions::new().progress_callback(&mut progress)),
        );
        p.check_deadline()?;
        let Some(tree) = tree else {
            warnings.push(json!({"path":path,"reason":"parse_failed"}));
            continue;
        };
        if tree.root_node().has_error() {
            warnings.push(json!({"path":path,"reason":"parse_partial"}));
        }
        let mut found = Vec::new();
        extract(p, tree.root_node(), &text, &mut found)?;
        for (raw, begin, end, first, last, unsupported) in found {
            p.check_deadline()?;
            let (specifier, redacted) = reader::redact(&raw);
            let (mut target, resolution, proof) = if redacted {
                (None, "sensitive_specifier_redacted".into(), BTreeSet::new())
            } else if raw.contains('\\') {
                (
                    None,
                    "escaped_specifier_unsupported".into(),
                    BTreeSet::new(),
                )
            } else if let Some(reason) = unsupported {
                (None, reason, BTreeSet::new())
            } else {
                resolve(p, &path, &specifier, &paths, &configs)?
            };
            p.check_deadline()?;
            // A resolved file is accepted only after the same reader verifies its policy and hash.
            let target_hash = if let Some(t) = &target {
                match reader::read(p, t) {
                    Ok(file) => {
                        if sources
                            .get(t)
                            .is_some_and(|expected| expected != &file.hash)
                        {
                            return Err(Error::new(
                                "CONCURRENT_MODIFICATION",
                                "Graph target changed after its source was parsed",
                                4,
                            ));
                        }
                        sources.insert(t.clone(), file.hash.clone());
                        Some(file.hash)
                    }
                    Err(e) => {
                        if e.code == "TIMEOUT" {
                            return Err(e);
                        }
                        p.check_deadline()?;
                        warnings.push(json!({"path":t,"reason":e.code}));
                        None
                    }
                }
            } else {
                None
            };
            if target_hash.is_none() {
                target = None;
            }
            let proof_hashes: BTreeMap<_, _> = proof
                .into_iter()
                .filter_map(|name| config_hashes.get(&name).map(|h| (name.clone(), h.clone())))
                .collect();
            let source_hash = sources[&path].clone();
            let edge_id = format!(
                "EDGE-{}",
                hash(format!(
                    "{}\0{path}\0{source_hash}\0{begin}\0{specifier}\0{manifest_hash}",
                    p.workspace_id
                ))
            );
            let reason = if target.is_none() {
                Some(
                    if matches!(
                        resolution.as_str(),
                        "relative_import" | "workspace_package_export"
                    ) {
                        "target_missing_or_excluded".into()
                    } else {
                        resolution.clone()
                    },
                )
            } else {
                None
            };
            edges.push(Edge {
                id: edge_id,
                source: path.clone(),
                source_hash,
                target,
                target_hash,
                specifier,
                relation: "imports".into(),
                evidence_status: "observed".into(),
                resolution,
                reason,
                start_byte: begin,
                end_byte: end,
                start_line: first,
                end_line: last,
                resolver_version: VERSION.into(),
                config_hashes: proof_hashes,
                resolver_fingerprint: manifest_hash.clone(),
            });
        }
    }
    edges.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then_with(|| a.start_byte.cmp(&b.start_byte))
            .then_with(|| a.specifier.cmp(&b.specifier))
    });
    p.check_deadline()?;
    validate_sources(p, &sources)?;
    Ok(Graph {
        edges,
        sources,
        config_hashes,
        generation,
        warnings,
        manifest_hash,
    })
}
fn validate_sources(p: &Project, sources: &BTreeMap<String, String>) -> Result<()> {
    p.check_deadline()?;
    for (path, expected) in sources {
        if reader::read(p, path)?.hash != *expected {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Graph source/config changed while resolving",
                4,
            ));
        }
    }
    p.check_deadline()?;
    reader::validate_root(p)
}

fn traverse(
    p: &Project,
    path: &str,
    direction: &str,
    depth: usize,
    max_nodes: usize,
) -> Result<Value> {
    validate_bounds(direction, depth, max_nodes)?;
    reader::authorize(p, path)?;
    let g = graph(p)?;
    let mut visited = BTreeSet::from([path.to_owned()]);
    let mut queue = VecDeque::from([(path.to_owned(), 0usize)]);
    let mut nodes = Vec::new();
    let mut edge_ids = BTreeSet::new();
    let mut selected = Vec::new();
    let mut truncated = false;
    while let Some((current, d)) = queue.pop_front() {
        p.check_deadline()?;
        if d >= depth {
            continue;
        }
        for edge in &g.edges {
            p.check_deadline()?;
            let matches = if direction == "outgoing" {
                edge.source == current
            } else {
                edge.target.as_deref() == Some(current.as_str())
            };
            if !matches {
                continue;
            }
            if edge_ids.insert(edge.id.clone()) {
                selected.push(edge.clone());
            }
            let next = if direction == "outgoing" {
                edge.target.as_ref()
            } else {
                Some(&edge.source)
            };
            if let Some(next) = next {
                if visited.contains(next) {
                    continue;
                }
                if visited.len() >= max_nodes {
                    truncated = true;
                    continue;
                }
                visited.insert(next.clone());
                nodes.push(json!({"path":next,"file_hash":g.sources.get(next),"depth":d+1,"status":if direction=="incoming"{"potentially_affected"}else{"dependency"},"evidence_status":if direction=="incoming"{"inferred"}else{"observed"}}));
                queue.push_back((next.clone(), d + 1));
            }
        }
    }
    nodes.sort_by(|a, b| {
        a["depth"]
            .as_u64()
            .cmp(&b["depth"].as_u64())
            .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
    });
    selected.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then_with(|| a.start_byte.cmp(&b.start_byte))
    });
    p.check_deadline()?;
    let unresolved = selected.iter().filter(|e| e.target.is_none()).count();
    let partial = truncated || !g.warnings.is_empty() || unresolved > 0;
    let data = json!({"root":path,"direction":direction,"depth":depth,"max_nodes":max_nodes,"nodes":nodes,"edges":selected,"visited_count":visited.len(),"unresolved_count":unresolved,"truncated":truncated,"workspace_id":p.workspace_id,"generation_id":g.generation,"resolver_version":VERSION,"resolver_fingerprint":g.manifest_hash,"config_hashes":g.config_hashes,"freshness":"current","workspace_atomic":false,"coverage":{"status":if partial{"partial"}else{"complete"},"warnings":g.warnings,"capability":"static_imports_only","gaps":["runtime_injection","reflection","conditional_exports","python_resolver","symbol_reference_resolution"]},"test_exclusion_safe":false});
    validate_sources(p, &g.sources)?;
    Ok(data)
}
fn query_project(p: &Project) -> Result<Project> {
    let mut p = p.clone();
    if p.deadline.is_none() {
        p.deadline = Some(Deadline::from_millis(10_000)?);
    }
    p.check_deadline()?;
    Ok(p)
}
pub fn dependencies(p: &Project, path: &str, depth: usize, max_nodes: usize) -> Result<Value> {
    validate_bounds("outgoing", depth, max_nodes)?;
    reader::validate_relative_path(path)?;
    let p = query_project(p)?;
    traverse(&p, path, "outgoing", depth, max_nodes)
}
pub fn execute(p: &Project, c: &GraphCommand) -> Result<Value> {
    validate_request(c)?;
    let scoped = query_project(p)?;
    let p = &scoped;
    let result = match c {
        GraphCommand::Trace {
            path,
            direction,
            depth,
        } => traverse(p, path, direction, *depth, 1000),
        GraphCommand::Impact { path, depth } => traverse(p, path, "incoming", *depth, 1000),
        GraphCommand::Refs { path } => {
            reader::authorize(p, path)?;
            let g = graph(p)?;
            let mut refs = Vec::new();
            for edge in &g.edges {
                p.check_deadline()?;
                if edge.source == *path || edge.target.as_deref() == Some(path.as_str()) {
                    refs.push(edge);
                }
            }
            let data = json!({"path":path,"edges":refs,"workspace_id":p.workspace_id,"resolver_fingerprint":g.manifest_hash,"freshness":"current","coverage":{"status":if g.warnings.is_empty()&&!refs.iter().any(|e|e.target.is_none()){"complete"}else{"partial"},"warnings":g.warnings},"capability":"observed_import_edges","symbol_references_supported":false,"test_exclusion_safe":false});
            validate_sources(p, &g.sources)?;
            Ok(data)
        }
    }?;
    p.check_deadline()?;
    reader::validate_root(p)?;
    Ok(result)
}

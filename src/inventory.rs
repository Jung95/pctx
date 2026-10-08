//! Static, policy-checked project evidence and document correction proposals.
//! Project profiles are claims, never operation permission or local-owner identity.
use crate::{
    domain::{Error, Result, hash},
    project::Project,
    reader, search,
};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
const MAX_REFS: usize = 256;
#[derive(Debug, Subcommand)]
pub enum InventoryCommand {
    Scan {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long, default_value_t = 2000)]
        max_files: usize,
        #[arg(long, default_value_t = 2097152)]
        max_bytes: usize,
    },
    Profile {
        #[arg(long)]
        path: String,
    },
    Audit {
        #[arg(long)]
        registry: String,
        #[arg(long)]
        since: Option<String>,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    schema_version: u32,
    id: String,
    #[serde(default)]
    expectations: Vec<Expectation>,
    #[serde(default)]
    roles: Vec<Role>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirmation {
    source: String,
    hash: String,
    authority: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expectation {
    source: String,
    selector: String,
    expected: Value,
    #[serde(default)]
    confirmation: Option<Confirmation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Role {
    id: String,
    scope: Vec<String>,
    #[serde(default)]
    confirmation: Option<Confirmation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: u32,
    #[serde(default)]
    since_event: Option<String>,
    source_refs: Vec<SourceRef>,
    #[serde(default)]
    known_proposal_ids: Vec<String>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceRef {
    document: String,
    section: String,
    source: String,
    selector: String,
    expected: Value,
    rendered: String,
    template: String,
    source_hash: String,
    document_hash: String,
    #[serde(default)]
    decision_id: Option<String>,
}
fn invalid(s: &str) -> Error {
    Error::new("INVALID_ARGUMENT", s, 2)
}
fn safe(value: &Value) -> Value {
    match value {
        Value::String(s) => {
            static URL: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
                regex::Regex::new(r"(?i)(https?://)[^/\s:@]+:[^/\s@]+@").unwrap()
            });
            let masked = URL
                .replace_all(&reader::redact(s).0, "${1}[REDACTED]@")
                .to_string();
            if masked.chars().count() > 2048 {
                json!(format!(
                    "{} [omitted]",
                    masked.chars().take(2048).collect::<String>()
                ))
            } else {
                json!(masked)
            }
        }
        Value::Array(a) => json!(a.iter().map(safe).collect::<Vec<_>>()),
        Value::Object(o) => {
            let mut m = serde_json::Map::new();
            for (k, v) in o {
                if sensitive(k) {
                    m.insert(reader::redact(k).0, json!("[REDACTED]"));
                } else {
                    m.insert(reader::redact(k).0, safe(v));
                }
            }
            Value::Object(m)
        }
        _ => value.clone(),
    }
}
fn sensitive(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    [
        "password",
        "secret",
        "token",
        "api_key",
        "apikey",
        "credential",
        "authorization",
        "dsn",
        "connection_string",
    ]
    .iter()
    .any(|word| s.contains(word))
}
fn label(s: &str) -> Result<()> {
    if s.trim().is_empty()
        || s.len() > 256
        || s.chars().any(char::is_control)
        || reader::redact(s).1
    {
        return Err(invalid("Invalid or sensitive metadata identity"));
    }
    Ok(())
}
fn proof(path: &str, hash: &str) -> Value {
    json!({"path":path,"hash":hash,"freshness":"verified_current_read","authority":"observed_project_file","operation_authorized":false})
}
fn static_value(path: &str, text: &str) -> Result<Value> {
    if path.ends_with(".json") {
        serde_json::from_str(text)
            .map_err(|_| Error::new("UNSUPPORTED_MANIFEST", "Invalid static JSON", 3))
    } else if path.ends_with(".toml") || path.ends_with("Cargo.lock") {
        let value: toml::Value = toml::from_str(text)
            .map_err(|_| Error::new("UNSUPPORTED_MANIFEST", "Invalid static TOML", 3))?;
        Ok(serde_json::to_value(value)?)
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(text)
            .map_err(|_| Error::new("UNSUPPORTED_MANIFEST", "Invalid static YAML", 3))?;
        Ok(serde_json::to_value(value)?)
    } else {
        Err(Error::new(
            "CAPABILITY_UNVERIFIED",
            "Dynamic or unregistered manifest format is not evaluated",
            6,
        ))
    }
}
fn observe(p: &Project, path: &str, selector: &str) -> Result<(Value, String)> {
    reader::policy_allows(p, path)?;
    if selector.split('/').any(sensitive) {
        return Err(Error::new(
            "POLICY_DENIED",
            "Sensitive selector is not inventoried",
            5,
        ));
    }
    let source = reader::read(p, path)?;
    if selector.len() > 1024 {
        return Err(invalid("Selector bound exceeded"));
    }
    let value = if selector == "$path" {
        json!(source.path)
    } else if selector == "$hash" {
        json!(source.hash)
    } else if let Some(name) = selector.strip_prefix("$symbol/") {
        let entry = search::analyze(path, &source.hash, &source.text)?;
        if entry.parse_status != "complete" {
            return Err(Error::new(
                "CAPABILITY_UNVERIFIED",
                "Source structure is unsupported or incomplete",
                6,
            ));
        }
        let matches: Vec<_> = entry
            .symbols
            .into_iter()
            .filter(|s| s.name == name || s.qualified_name == name)
            .map(|s| json!({"name":s.name,"qualified_name":s.qualified_name,"kind":s.kind}))
            .collect();
        json!(matches)
    } else if selector.starts_with('/') {
        static_value(path, &source.text)?
            .pointer(selector)
            .cloned()
            .ok_or_else(|| Error::new("SOURCE_UNAVAILABLE", "Static selector is absent", 6))?
    } else {
        return Err(Error::new(
            "CAPABILITY_UNVERIFIED",
            "Unknown source selector",
            6,
        ));
    };
    if serde_json::to_vec(&value)?.len() > 16384 {
        return Err(Error::new(
            "BUDGET_TOO_SMALL",
            "Selected metadata exceeds 16 KiB",
            8,
        ));
    }
    let clean = safe(&value);
    if clean != value {
        return Err(Error::new(
            "POLICY_DENIED",
            "Sensitive metadata cannot establish equality or drift",
            5,
        ));
    }
    Ok((clean, source.hash))
}
fn confirmation(p: &Project, c: Option<&Confirmation>) -> Result<Value> {
    let Some(c) = c else {
        return Ok(json!({"status":"unconfirmed","owner_authority_verified":false}));
    };
    label(&c.authority)?;
    match reader::read(p, &c.source) {
        Ok(source) => Ok(
            json!({"status":if source.hash==c.hash{"hash_matched_claim"}else{"stale_claim"},"declared_authority":c.authority,"owner_authority_verified":false,"proof":proof(&c.source,&source.hash),"expected_hash":c.hash}),
        ),
        Err(e) => {
            Ok(json!({"status":"unconfirmed","reason":e.code,"owner_authority_verified":false}))
        }
    }
}
pub fn profile(p: &Project, path: &str) -> Result<Value> {
    let file = reader::read(p, path)?;
    let profile: Profile =
        serde_json::from_str(&file.text).map_err(|_| invalid("Invalid explicit profile schema"))?;
    if profile.schema_version != 1
        || profile.expectations.len() > MAX_REFS
        || profile.roles.len() > 128
    {
        return Err(invalid("Unsupported profile schema or bound"));
    }
    label(&profile.id)?;
    let mut claims = Vec::new();
    for claim in profile.expectations {
        if serde_json::to_vec(&claim.expected)?.len() > 8192 {
            return Err(invalid("Profile expected metadata exceeds 8 KiB"));
        }
        let evidence = confirmation(p, claim.confirmation.as_ref())?;
        let observed = observe(p, &claim.source, &claim.selector);
        let record = match observed {
            Ok((actual, h)) => {
                let matches = actual == safe(&claim.expected);
                let status = if !matches {
                    "conflicting"
                } else if evidence["status"] == "hash_matched_claim" {
                    "confirmed"
                } else {
                    "unconfirmed"
                };
                json!({"source":claim.source,"selector":claim.selector,"expected":safe(&claim.expected),"observed":actual,"status":status,"proof":proof(&claim.source,&h),"confirmation":evidence,"confirmation_meaning":"observed_claim_only","operation_authorized":false})
            }
            Err(e) => {
                json!({"source":claim.source,"selector":claim.selector,"status":"unconfirmed","reason":e.code,"confirmation":evidence,"operation_authorized":false})
            }
        };
        claims.push(record);
    }
    let files = reader::inventory(p, false)?;
    let mut roles = Vec::new();
    for role in profile.roles {
        label(&role.id)?;
        if role.scope.len() > 64 {
            return Err(invalid("Role scope bound exceeded"));
        }
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in &role.scope {
            if pattern.starts_with('/') || pattern.contains("..") || pattern.contains('\\') {
                return Err(invalid("Role scope must remain relative"));
            }
            builder
                .add(globset::Glob::new(pattern).map_err(|_| invalid("Invalid role scope glob"))?);
        }
        let scopes = builder
            .build()
            .map_err(|_| invalid("Invalid role scopes"))?;
        let related: Vec<_> = files
            .paths
            .iter()
            .filter(|path| scopes.is_match(path))
            .take(512)
            .cloned()
            .collect();
        let evidence = confirmation(p, role.confirmation.as_ref())?;
        roles.push(json!({"role_id":role.id,"scope":role.scope,"related_paths":related,"status":if evidence["status"]=="hash_matched_claim"{"confirmed"}else{"unconfirmed"},"confirmation":evidence,"confirmation_meaning":"declared_scope_with_current_evidence","operation_authorized":false,"scope_coverage":if related.len()==512{"bounded"}else{"observed"}}));
    }
    let conflicting = claims.iter().any(|c| c["status"] == "conflicting");
    Ok(
        json!({"schema_version":1,"profile_id":profile.id,"profile_proof":proof(path,&file.hash),"expectations":claims,"roles":roles,"status":if conflicting{"conflicting"}else if claims.iter().all(|c|c["status"]=="confirmed")&&roles.iter().all(|r|r["status"]=="confirmed"){"confirmed"}else{"unconfirmed"},"operation_authorized":false,"owner_identity_verified":false,"profile_applied":false}),
    )
}
fn category(path: &str) -> Option<&'static str> {
    let base = path.rsplit('/').next().unwrap_or(path);
    if base == "package.json" {
        Some("npm_package")
    } else if base == "Cargo.toml" {
        Some("cargo_package")
    } else if base == "pnpm-workspace.yaml" {
        Some("pnpm_workspace")
    } else if path.starts_with(".github/workflows/")
        && (path.ends_with(".yaml") || path.ends_with(".yml"))
    {
        Some("ci_workflow")
    } else if ["AGENTS.md", "CLAUDE.md"].contains(&base) {
        Some("rule_document")
    } else if path.ends_with(".md") {
        Some("document")
    } else if base.ends_with(".config.js")
        || base.ends_with(".config.ts")
        || base == "app.config.js"
        || base == "app.config.ts"
    {
        Some("dynamic_config")
    } else if [
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "Cargo.lock",
    ]
    .contains(&base)
    {
        Some("lockfile")
    } else if path == ".cargo/config.toml" || path == ".cargo/config" {
        Some("cargo_config")
    } else if base.starts_with("tsconfig") && base.ends_with(".json") {
        Some("typescript_config")
    } else if path.ends_with(".json") && (base == "app.json" || base == "eas.json") {
        Some("static_config")
    } else if [
        ".py", ".js", ".jsx", ".ts", ".tsx", ".rs", ".swift", ".tf", ".sql",
    ]
    .iter()
    .any(|suffix| path.ends_with(suffix))
    {
        Some("source_metadata")
    } else {
        None
    }
}
fn script_check(path: &str, key: &str, command: &str, source_hash: &str) -> Value {
    let words = command.split_whitespace().collect::<Vec<_>>();
    let families = [
        ("jest", "jest"),
        ("jest-expo", "jest-expo"),
        ("vitest", "vitest"),
        ("playwright", "playwright"),
        ("eslint", "eslint"),
        ("gitleaks", "gitleaks"),
        ("actionlint", "actionlint"),
        ("maestro", "maestro"),
        ("tsc", "typescript"),
        ("cargo", "cargo"),
    ];
    let parser = families
        .iter()
        .find(|(token, _)| words.contains(token))
        .map(|(_, name)| *name)
        .unwrap_or("unknown");
    json!({"check_key":format!("manifest:{}:{key}",hash(path)),"source_script":key,"command":safe(&json!(command.chars().take(1024).collect::<String>())),"command_hash":hash(command),"scope":path.rsplit_once('/').map(|(dir,_)|dir).unwrap_or("."),"parser_candidate":parser,"resource_candidate":if ["jest","jest-expo","vitest","playwright","maestro","typescript","cargo"].contains(&parser){"heavy_compute"}else{"unknown"},"policy":"unregistered_requires_confirmation","executed":false,"gate_checks_omission_authorized":false,"proof":proof(path,source_hash)})
}
pub fn scan(
    p: &Project,
    profile_path: Option<&str>,
    max_files: usize,
    max_bytes: usize,
) -> Result<Value> {
    if !(1..=4096).contains(&max_files) || !(1..=16777216).contains(&max_bytes) {
        return Err(invalid(
            "Inventory limits: 1..4096 files, 1..16777216 bytes",
        ));
    }
    let inventory = reader::inventory(p, false)?;
    let mut items = Vec::new();
    let mut checks = Vec::new();
    let mut packages = Vec::new();
    let mut problems = inventory.skipped;
    let mut spent = 0usize;
    let mut sources = BTreeMap::new();
    let mut npm_members = Vec::new();
    let mut cargo_members = Vec::new();
    let mut attempted = 0usize;
    for path in inventory.paths {
        let Some(kind) = category(&path) else {
            continue;
        };
        if attempted >= max_files {
            problems.push(json!({"reason":"file_bound","remaining_candidate_coverage":"unknown"}));
            break;
        }
        attempted += 1;
        let selected = match reader::authorize(p, &path) {
            Ok(v) => v,
            Err(e) => {
                problems.push(json!({"path":path,"reason":e.code}));
                continue;
            }
        };
        let size = match std::fs::metadata(selected) {
            Ok(v) => v.len(),
            Err(_) => {
                problems.push(json!({"path":path,"reason":"metadata_unavailable"}));
                continue;
            }
        };
        if size > max_bytes.saturating_sub(spent) as u64 {
            problems.push(json!({"reason":"byte_bound","path":path}));
            break;
        }
        let file = match reader::read(p, &path) {
            Ok(file) => file,
            Err(e) => {
                problems.push(json!({"path":path,"reason":e.code}));
                continue;
            }
        };
        spent = spent.saturating_add(file.text.len());
        if spent > max_bytes {
            problems.push(json!({"reason":"byte_bound_after_concurrent_size_change","path":path,"actual_read_bytes":spent}));
            break;
        }
        sources.insert(path.clone(), file.hash.clone());
        let mut item = json!({"path":path,"kind":kind,"size_bytes":file.size_bytes,"proof":proof(&path,&file.hash)});
        match kind {
            "npm_package" => match static_value(&path, &file.text) {
                Ok(value) => {
                    let value = safe(&value);
                    let name = value["name"].as_str().unwrap_or("unnamed");
                    item["name"] = json!(name);
                    item["version"] = value["version"].clone();
                    item["license"] = value["license"].clone();
                    item["engines"] = value["engines"].clone();
                    item["package_manager"] = value["packageManager"].clone();
                    let deps:Vec<Value>=["dependencies","devDependencies","peerDependencies","optionalDependencies"].iter().flat_map(|kind|value.get(kind).and_then(Value::as_object).into_iter().flat_map(move|map|map.iter().map(move|(name,version)|json!({"name":name,"version":version,"kind":kind})))).collect();
                    item["dependencies"] = json!(deps);
                    packages.push(json!({"name":name,"path":path,"kind":"npm","dependencies":deps,"proof":proof(&path,&file.hash)}));
                    if let Some(scripts) = value["scripts"].as_object() {
                        for (key, command) in scripts {
                            if let Some(command) = command.as_str() {
                                checks.push(script_check(&path, key, command, &file.hash));
                            } else {
                                problems.push(json!({"path":path,"reason":"unsupported_script_form","key":key}));
                            }
                        }
                    }
                    if value.get("workspaces").is_some() {
                        let members = value["workspaces"]
                            .as_array()
                            .or_else(|| value["workspaces"]["packages"].as_array());
                        if let Some(members) = members {
                            for member in members {
                                if let Some(s) = member.as_str() {
                                    npm_members.push((path.clone(), s.to_string()));
                                } else {
                                    problems.push(json!({"path":path,"reason":"unsupported_workspace_member"}));
                                }
                            }
                        } else {
                            problems
                                .push(json!({"path":path,"reason":"unsupported_workspace_form"}));
                        }
                    }
                }
                Err(e) => {
                    item["coverage"] = json!("unknown");
                    problems.push(json!({"path":path,"reason":e.code}));
                }
            },
            "cargo_package" => match static_value(&path, &file.text) {
                Ok(value) => {
                    item["package"] = safe(
                        &json!({"name":value["package"]["name"],"version":value["package"]["version"],"edition":value["package"]["edition"],"rust_version":value["package"]["rust-version"],"license":value["package"]["license"],"license_file":value["package"]["license-file"]}),
                    );
                    if value["package"]["version"].is_object() {
                        problems.push(
                            json!({"path":path,"reason":"workspace_inheritance_not_resolved"}),
                        );
                    }
                    let mut deps = Vec::new();
                    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
                        if let Some(map) = value[key].as_object() {
                            for (name, v) in map {
                                deps.push(safe(&json!({"name":name,"version":v.as_str().map(Value::from).unwrap_or(v["version"].clone()),"path":v["path"],"kind":key})));
                            }
                        }
                    }
                    item["dependencies"] = json!(deps);
                    item["virtual_workspace"] = json!(value.get("package").is_none());
                    if value["package"]["name"].is_string() {
                        packages.push(json!({"name":value["package"]["name"],"path":path,"kind":"cargo","dependencies":deps,"proof":proof(&path,&file.hash)}));
                    }
                    if value.get("target").is_some() {
                        problems.push(json!({"path":path,"reason":"target_conditioned_dependencies_not_resolved"}));
                    }
                    if value["workspace"].get("exclude").is_some() {
                        problems.push(
                            json!({"path":path,"reason":"workspace_exclusions_not_resolved"}),
                        );
                    }
                    if let Some(members) = value["workspace"]["members"].as_array() {
                        for m in members {
                            if let Some(s) = m.as_str() {
                                cargo_members.push((path.clone(), s.into()));
                            }
                        }
                    }
                    checks.push(script_check(&path, "cargo-test", "cargo test", &file.hash));
                    checks.push(script_check(
                        &path,
                        "cargo-check",
                        "cargo check",
                        &file.hash,
                    ));
                }
                Err(e) => problems.push(json!({"path":path,"reason":e.code})),
            },
            "pnpm_workspace" => match static_value(&path, &file.text) {
                Ok(v) => {
                    if let Some(members) = v["packages"].as_array() {
                        for m in members {
                            if let Some(s) = m.as_str() {
                                npm_members.push((path.clone(), s.into()));
                            } else {
                                problems.push(
                                    json!({"path":path,"reason":"unsupported_workspace_member"}),
                                );
                            }
                        }
                    } else {
                        problems.push(json!({"path":path,"reason":"unsupported_workspace_form"}));
                    }
                }
                Err(e) => problems.push(json!({"path":path,"reason":e.code})),
            },
            "ci_workflow" => match static_value(&path, &file.text) {
                Ok(v) => {
                    item["workflow_name"] = safe(&v["name"]);
                    let mut jobs = Vec::new();
                    if let Some(map) = v["jobs"].as_object() {
                        for (key, job) in map {
                            let mut steps = Vec::new();
                            for (at, step) in
                                job["steps"].as_array().into_iter().flatten().enumerate()
                            {
                                let run = step["run"].as_str();
                                steps.push(json!({"index":at,"uses":safe(&step["uses"]),"run_hash":run.map(hash),"run_dynamic":run.is_some_and(|s|s.contains("${{")),"environment_values_omitted":true}));
                                if let Some(run) = run {
                                    checks.push(script_check(
                                        &path,
                                        &format!("{key}/{at}"),
                                        run,
                                        &file.hash,
                                    ));
                                    if run.contains("${{") {
                                        problems.push(json!({"path":path,"reason":"dynamic_workflow_expression"}));
                                    }
                                }
                            }
                            jobs.push(
                                json!({"id":key,"runs_on":safe(&job["runs-on"]),"steps":steps}),
                            );
                        }
                    } else {
                        problems.push(json!({"path":path,"reason":"unsupported_jobs"}));
                    }
                    item["jobs"] = json!(jobs);
                }
                Err(e) => problems.push(json!({"path":path,"reason":e.code})),
            },
            "cargo_config" => {
                match static_value(
                    if path.ends_with(".toml") {
                        &path
                    } else {
                        "config.toml"
                    },
                    &file.text,
                ) {
                    Ok(v) => {
                        item["build_target"] = safe(&v["build"]["target"]);
                        item["aliases"] = safe(&v["alias"]);
                        if let Some(aliases) = v["alias"].as_object() {
                            for (key, value) in aliases {
                                if let Some(command) = value.as_str() {
                                    checks.push(script_check(&path, key, command, &file.hash));
                                } else {
                                    problems.push(json!({"path":path,"reason":"unsupported_cargo_alias_form"}));
                                }
                            }
                        }
                        item["environment_values_omitted"] = json!(true);
                    }
                    Err(e) => problems.push(json!({"path":path,"reason":e.code})),
                }
            }
            "typescript_config" | "static_config" => match static_value(&path, &file.text) {
                Ok(v) => {
                    item["metadata"] = if kind == "typescript_config" {
                        safe(
                            &json!({"extends":v["extends"],"base_url":v["compilerOptions"]["baseUrl"],"paths":v["compilerOptions"]["paths"]}),
                        )
                    } else {
                        json!({"declared_static":true,"content_values_omitted":true})
                    };
                    if v.get("extends").is_some() {
                        problems
                            .push(json!({"path":path,"reason":"config_inheritance_not_resolved"}));
                    }
                }
                Err(e) => problems.push(json!({"path":path,"reason":e.code})),
            },
            "rule_document" | "document" => {
                item["headings"]=json!(file.text.lines().enumerate().filter_map(|(i,line)|{let line=line.trim_start();if line.starts_with('#'){Some(json!({"line":i+1,"title":safe(&json!(line.trim_start_matches('#').trim().chars().take(256).collect::<String>()))}))}else{None}}).take(128).collect::<Vec<_>>());
                item["native_loading"] = json!("unknown");
                item["operation_authorized"] = json!(false);
            }
            "source_metadata" => {
                let entry = search::analyze(&path, &file.hash, &file.text)?;
                item["language"] = json!(entry.language);
                item["parse_status"] = json!(entry.parse_status);
                item["symbols"]=safe(&json!(entry.symbols.into_iter().take(128).map(|s|json!({"name":s.name,"qualified_name":s.qualified_name,"kind":s.kind,"start_line":s.start_line,"end_line":s.end_line})).collect::<Vec<_>>()));
                item["body_omitted"] = json!(true);
                if entry.parse_status != "complete" {
                    problems.push(
                        json!({"path":path,"reason":"source_structure_partial_or_unsupported"}),
                    );
                }
            }
            "dynamic_config" => {
                item["coverage"] = json!("unknown");
                problems.push(json!({"path":path,"reason":"dynamic_config_not_executed"}));
            }
            _ => {
                item["body_omitted"] = json!(true);
            }
        }
        items.push(item);
    }
    let mut workspaces = Vec::new();
    for (patterns, kind) in [(&npm_members, "npm"), (&cargo_members, "cargo")] {
        for (manifest, pattern) in patterns {
            if pattern.starts_with('!') {
                problems.push(json!({"path":manifest,"reason":"workspace_exclusion_requires_resolution","pattern":pattern}));
                continue;
            }
            if pattern.starts_with('/') || pattern.contains("..") || pattern.contains('\\') {
                problems.push(json!({"path":manifest,"reason":"unsafe_workspace_pattern"}));
                continue;
            }
            let base = manifest
                .rsplit_once('/')
                .map(|(d, _)| format!("{d}/"))
                .unwrap_or_default();
            let glob = globset::Glob::new(&format!("{base}{pattern}")).map(|g| g.compile_matcher());
            let paths: Vec<_> = packages
                .iter()
                .filter(|p| p["kind"] == kind)
                .filter_map(|p| p["path"].as_str())
                .filter(|path| {
                    glob.as_ref().is_ok_and(|g| {
                        g.is_match(path.rsplit_once('/').map(|(d, _)| d).unwrap_or("."))
                    })
                })
                .collect();
            workspaces.push(json!({"manifest":manifest,"pattern":pattern,"members":paths,"kind":kind,"proof":sources.get(manifest).map(|h|proof(manifest,h))}));
        }
    }
    let mut reverse = BTreeMap::<String, Vec<String>>::new();
    let names: BTreeSet<_> = packages.iter().filter_map(|p| p["name"].as_str()).collect();
    for package in &packages {
        for dep in package["dependencies"].as_array().into_iter().flatten() {
            if let Some(name) = dep["name"].as_str().filter(|n| names.contains(n)) {
                reverse
                    .entry(name.into())
                    .or_default()
                    .push(package["path"].as_str().unwrap_or("").into());
            }
        }
    }
    for v in reverse.values_mut() {
        v.sort();
        v.dedup();
    }
    let result = json!({"schema_version":1,"inventory_id":hash(serde_json::to_vec(&json!({"sources":sources,"policy_hash":p.policy_hash(),"serializer":"inventory-static-v1"}))?),"policy_hash":p.policy_hash(),"sources":sources,"items":items,"packages":packages,"workspace_declarations":workspaces,"reverse_dependency_candidates":reverse,"check_candidates":checks,"profile":profile_path.map(|path|profile(p,path)).transpose()?,"coverage":{"status":if problems.is_empty(){"complete_for_static_subset"}else{"partial"},"reasons":problems},"read_bytes":spent,"attempted_files":attempted,"operation_authorized":false,"scripts_executed":false,"native_rule_loading":"unknown"});
    Ok(safe(&result))
}

fn section<'a>(text: &'a str, name: &str) -> Result<&'a str> {
    if name.is_empty() {
        return Ok(text);
    }
    let mut start = None;
    let mut end = text.len();
    let mut offset = 0usize;
    let mut matching = 0;
    let mut fence: Option<char> = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let marker = trimmed.chars().next().unwrap();
            if fence == Some(marker) {
                fence = None;
            } else if fence.is_none() {
                fence = Some(marker);
            }
            offset += line.len();
            continue;
        }
        if fence.is_some() || line.len() - trimmed.len() > 3 {
            offset += line.len();
            continue;
        }
        let level = trimmed.bytes().take_while(|b| *b == b'#').count();
        if level > 0 && level <= 6 && trimmed.as_bytes().get(level) == Some(&b' ') {
            let title = trimmed[level..].trim().trim_end_matches('#').trim();
            if title == name {
                matching += 1;
                if matching > 1 {
                    return Err(Error::new(
                        "SOURCE_UNAVAILABLE",
                        "Document section is ambiguous",
                        6,
                    ));
                }
                start = Some((offset, level));
            } else if let Some((_, start_level)) = start
                && level <= start_level
                && end == text.len()
            {
                end = offset;
            }
        }
        offset += line.len();
    }
    let (start, _) =
        start.ok_or_else(|| Error::new("SOURCE_UNAVAILABLE", "Document section absent", 6))?;
    Ok(&text[start..end])
}
fn rendered(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}
pub fn audit(p: &Project, path: &str, since: Option<&str>) -> Result<Value> {
    let file = reader::read(p, path)?;
    let registry: Registry = serde_json::from_str(&file.text)
        .map_err(|_| invalid("Invalid document source_refs schema"))?;
    if registry.schema_version != 1
        || registry.source_refs.len() > MAX_REFS
        || registry.known_proposal_ids.len() > 1024
    {
        return Err(invalid("Unsupported registry schema or bound"));
    }
    if since.is_some() && since != registry.since_event.as_deref() {
        return Err(Error::new(
            "BASELINE_MISMATCH",
            "Requested event does not match registry's explicit hash baseline",
            9,
        ));
    }
    let mut changed = BTreeSet::new();
    let mut affected = BTreeSet::new();
    let mut mismatch = Vec::new();
    let mut review = Vec::new();
    let mut proposals = BTreeMap::new();
    for r in registry.source_refs {
        if r.rendered.len() > 8192
            || r.template.len() > 8192
            || r.section.len() > 256
            || r.template.matches("{value}").count() != 1
        {
            return Err(invalid(
                "Document claim/template bounds or placeholder invalid",
            ));
        }
        reader::policy_allows(p, &r.document)?;
        reader::policy_allows(p, &r.source)?;
        let observed = observe(p, &r.source, &r.selector);
        let (actual, current_hash) = match observed {
            Ok(v) => v,
            Err(e) => {
                let current = reader::read(p, &r.source).ok();
                if current.as_ref().is_some_and(|f| f.hash != r.source_hash) {
                    changed.insert(r.source.clone());
                    affected.insert(r.document.clone());
                }
                review.push(json!({"document":r.document,"source":r.source,"reason":e.code,"proof":current.as_ref().map(|f|proof(&r.source,&f.hash)),"operation_authorized":false}));
                continue;
            }
        };
        let changed_source = current_hash != r.source_hash;
        if changed_source {
            changed.insert(r.source.clone());
            affected.insert(r.document.clone());
        }
        let document = match reader::read(p, &r.document) {
            Ok(v) => v,
            Err(e) => {
                review.push(json!({"document":r.document,"source":r.source,"reason":e.code}));
                continue;
            }
        };
        let part = match section(&document.text, &r.section) {
            Ok(part) => part,
            Err(e) => {
                review.push(json!({"document":r.document,"section":r.section,"reason":e.code}));
                continue;
            }
        };
        if actual == safe(&r.expected) {
            if changed_source {
                review.push(json!({"document":r.document,"source":r.source,"reason":"source_hash_changed_claim_still_matches","proof":proof(&r.source,&current_hash)}));
            }
            continue;
        }
        if document.hash != r.document_hash {
            review.push(json!({"document":r.document,"source":r.source,"reason":"document_baseline_stale","expected_hash":r.document_hash,"current_hash":document.hash}));
            continue;
        }
        let before_expected =
            rendered(&safe(&r.expected)).map(|v| r.template.replace("{value}", &v));
        if before_expected.as_deref() != Some(r.rendered.as_str())
            || part.matches(&r.rendered).count() != 1
        {
            review.push(json!({"document":r.document,"source":r.source,"reason":"claimed_before_text_not_unique_or_template_mismatch"}));
            continue;
        }
        let Some(value) = rendered(&actual) else {
            review.push(json!({"document":r.document,"source":r.source,"reason":"non_scalar_fact_requires_review"}));
            continue;
        };
        let after = r.template.replace("{value}", &value);
        if reader::redact(&r.rendered).1 || reader::redact(&after).1 {
            review.push(json!({"document":r.document,"source":r.source,"reason":"sensitive_claim_not_exported"}));
            continue;
        }
        affected.insert(r.document.clone());
        let identity = json!({"project_id":p.project_id,"coordination_id":p.coordination_id,"document":r.document,"document_hash":document.hash,"section":r.section,"source":r.source,"selector":r.selector,"source_hash":current_hash,"before":r.rendered,"after":after,"policy_hash":p.policy_hash()});
        let proposal_id = format!("DOC-{}", hash(serde_json::to_vec(&identity)?));
        let known = registry.known_proposal_ids.contains(&proposal_id);
        let proof = json!({"registry":proof(path,&file.hash),"document":proof(&r.document,&document.hash),"source":proof(&r.source,&current_hash),"baseline_source_hash":r.source_hash,"policy_hash":p.policy_hash()});
        let proposal = json!({"proposal_id":proposal_id,"status":if known{"known_in_registry"}else{"proposed"},"kind":"verified_structural_mismatch","document":r.document,"section":r.section,"source":r.source,"selector":r.selector,"decision_id":r.decision_id,"decision_link":"declared_reference_not_control_verified","existing_task_verified":false,"before":r.rendered,"after":after,"proof":proof,"task_intent":{"title":format!("Synchronize {}: {}",r.document,r.section),"kind":"DOC","scope":[r.document],"proposal_id":proposal_id,"requires_current_hash_revalidation":true},"document_modified":false,"task_created":false,"operation_authorized":false});
        mismatch.push(proposal_id.clone());
        proposals.entry(proposal_id).or_insert(proposal);
    }
    mismatch.sort();
    mismatch.dedup();
    Ok(safe(
        &json!({"schema_version":1,"audit_id":hash(serde_json::to_vec(&proposals)?),"registry_proof":proof(path,&file.hash),"baseline_event":registry.since_event,"baseline_event_provenance":"registry_claim_not_event_history_replay","changed_sources":changed,"affected_documents":affected,"verified_mismatch":mismatch,"review_needed":review,"proposals":proposals.into_values().collect::<Vec<_>>(),"documents_modified":false,"tasks_created":false,"coverage":{"status":if review.is_empty(){"complete_for_registered_refs"}else{"partial"},"unregistered_document_semantics":"not_inferred"}}),
    ))
}
pub fn execute(p: &Project, c: &InventoryCommand) -> Result<Value> {
    match c {
        InventoryCommand::Scan {
            profile,
            max_files,
            max_bytes,
        } => scan(p, profile.as_deref(), *max_files, *max_bytes),
        InventoryCommand::Profile { path } => profile(p, path),
        InventoryCommand::Audit { registry, since } => audit(p, registry, since.as_deref()),
    }
}

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
fn safe(p: &Project, value: &Value) -> Result<Value> {
    p.check_deadline()?;
    let masked = match value {
        Value::String(s) => {
            static URL: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
                regex::Regex::new(r"(?i)(https?://)[^/\s:@]+:[^/\s@]+@").unwrap()
            });
            let masked = URL
                .replace_all(&reader::redact(s).0, "${1}[REDACTED]@")
                .to_string();
            p.check_deadline()?;
            if masked.chars().count() > 2048 {
                json!(format!(
                    "{} [omitted]",
                    masked.chars().take(2048).collect::<String>()
                ))
            } else {
                json!(masked)
            }
        }
        Value::Array(a) => {
            let mut out = Vec::new();
            for v in a {
                p.check_deadline()?;
                out.push(safe(p, v)?);
            }
            Value::Array(out)
        }
        Value::Object(o) => {
            let mut out = serde_json::Map::new();
            for (k, v) in o {
                p.check_deadline()?;
                if sensitive(k) {
                    out.insert(reader::redact(k).0, json!("[REDACTED]"));
                } else {
                    out.insert(reader::redact(k).0, safe(p, v)?);
                }
            }
            Value::Object(out)
        }
        _ => value.clone(),
    };
    p.check_deadline()?;
    Ok(masked)
}
/// Root/configuration failures are request failures, not per-item coverage.
fn partial_error(p: &Project, e: Error) -> Result<Error> {
    p.check_deadline()?;
    reader::validate_root(p)?;
    reader::validate_policy(p)?;
    if matches!(
        e.code.as_str(),
        "TIMEOUT"
            | "INVALID_CONFIG"
            | "POLICY_UNAVAILABLE"
            | "CONFIG_CHANGED"
            | "WORKSPACE_MISMATCH"
    ) {
        Err(e)
    } else {
        Ok(e)
    }
}
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Debug)]
enum Phase {
    ScanAdmitted,
    ScanWork,
    ProfileAdmitted,
    ProfileWork,
    AuditAdmitted,
    AuditWork,
}
#[cfg(test)]
type Observer = Box<dyn FnMut(Phase, Option<crate::deadline::Deadline>)>;
#[cfg(test)]
thread_local! {static OBSERVER:std::cell::RefCell<Option<Observer>>=const{std::cell::RefCell::new(None)};}
#[cfg(test)]
struct ObserverGuard;
#[cfg(test)]
impl Drop for ObserverGuard {
    fn drop(&mut self) {
        OBSERVER.with(|s| *s.borrow_mut() = None);
    }
}
#[cfg(test)]
fn observe_phase(
    callback: impl FnMut(Phase, Option<crate::deadline::Deadline>) + 'static,
) -> ObserverGuard {
    OBSERVER.with(|s| {
        assert!(s.borrow().is_none());
        *s.borrow_mut() = Some(Box::new(callback));
    });
    ObserverGuard
}
#[cfg(test)]
fn phase_observation(p: &Project, phase: Phase) {
    OBSERVER.with(|s| {
        if let Some(callback) = s.borrow_mut().as_mut() {
            callback(phase, p.deadline);
        }
    });
}
fn query_scope(p: &Project) -> Result<Project> {
    let mut scope = p.clone();
    if scope.deadline.is_none() {
        scope.deadline = Some(crate::deadline::Deadline::from_millis(10000)?);
    }
    scope.check_deadline()?;
    reader::validate_root(&scope)?;
    reader::validate_policy(&scope)?;
    Ok(scope)
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
fn static_value(p: &Project, path: &str, text: &str) -> Result<Value> {
    p.check_deadline()?;
    let result = static_value_inner(path, text);
    p.check_deadline()?;
    result
}
fn static_value_inner(path: &str, text: &str) -> Result<Value> {
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
    p.check_deadline()?;
    if selector.len() > 1024 {
        return Err(invalid("Selector bound exceeded"));
    }
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
        p.check_deadline()?;
        let analyzed = search::analyze(path, &source.hash, &source.text);
        p.check_deadline()?;
        let entry = analyzed?;
        if entry.parse_status != "complete" {
            return Err(Error::new(
                "CAPABILITY_UNVERIFIED",
                "Source structure is unsupported or incomplete",
                6,
            ));
        }
        let mut matches = Vec::new();
        for symbol in entry.symbols {
            p.check_deadline()?;
            if symbol.name == name || symbol.qualified_name == name {
                matches.push(json!({"name":symbol.name,"qualified_name":symbol.qualified_name,"kind":symbol.kind}));
            }
        }
        json!(matches)
    } else if selector.starts_with('/') {
        static_value(p, path, &source.text)?
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
    let clean = safe(p, &value)?;
    if clean != value {
        return Err(Error::new(
            "POLICY_DENIED",
            "Sensitive metadata cannot establish equality or drift",
            5,
        ));
    }
    p.check_deadline()?;
    Ok((clean, source.hash))
}
fn confirmation(p: &Project, c: Option<&Confirmation>) -> Result<Value> {
    p.check_deadline()?;
    let Some(c) = c else {
        return Ok(json!({"status":"unconfirmed","owner_authority_verified":false}));
    };
    label(&c.authority)?;
    match reader::read(p, &c.source) {
        Ok(source) => Ok(
            json!({"status":if source.hash==c.hash{"hash_matched_claim"}else{"stale_claim"},"declared_authority":c.authority,"owner_authority_verified":false,"proof":proof(&c.source,&source.hash),"expected_hash":c.hash}),
        ),
        Err(e) => {
            let e = partial_error(p, e)?;
            Ok(json!({"status":"unconfirmed","reason":e.code,"owner_authority_verified":false}))
        }
    }
}
pub fn profile(p: &Project, path: &str) -> Result<Value> {
    let scope = query_scope(p)?;
    let result = profile_inner(&scope, path);
    scope.check_deadline()?;
    reader::validate_root(&scope)?;
    result
}
fn profile_inner(p: &Project, path: &str) -> Result<Value> {
    p.check_deadline()?;
    let file = reader::read(p, path)?;
    let profile: Profile =
        serde_json::from_str(&file.text).map_err(|_| invalid("Invalid explicit profile schema"))?;
    p.check_deadline()?;
    if profile.schema_version != 1
        || profile.expectations.len() > MAX_REFS
        || profile.roles.len() > 128
    {
        return Err(invalid("Unsupported profile schema or bound"));
    }
    label(&profile.id)?;
    let mut claims = Vec::new();
    for claim in profile.expectations {
        #[cfg(test)]
        phase_observation(p, Phase::ProfileAdmitted);
        p.check_deadline()?;
        #[cfg(test)]
        phase_observation(p, Phase::ProfileWork);
        if serde_json::to_vec(&claim.expected)?.len() > 8192 {
            return Err(invalid("Profile expected metadata exceeds 8 KiB"));
        }
        let evidence = confirmation(p, claim.confirmation.as_ref())?;
        let observed = observe(p, &claim.source, &claim.selector);
        let record = match observed {
            Ok((actual, h)) => {
                let matches = actual == safe(p, &claim.expected)?;
                let status = if !matches {
                    "conflicting"
                } else if evidence["status"] == "hash_matched_claim" {
                    "confirmed"
                } else {
                    "unconfirmed"
                };
                json!({"source":claim.source,"selector":claim.selector,"expected":safe(p, &claim.expected)?,"observed":actual,"status":status,"proof":proof(&claim.source,&h),"confirmation":evidence,"confirmation_meaning":"observed_claim_only","operation_authorized":false})
            }
            Err(e) => {
                let e = partial_error(p, e)?;
                json!({"source":claim.source,"selector":claim.selector,"status":"unconfirmed","reason":e.code,"confirmation":evidence,"operation_authorized":false})
            }
        };
        claims.push(record);
    }
    let files = reader::inventory(p, false)?;
    let mut roles = Vec::new();
    for role in profile.roles {
        p.check_deadline()?;
        label(&role.id)?;
        if role.scope.len() > 64 {
            return Err(invalid("Role scope bound exceeded"));
        }
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in &role.scope {
            p.check_deadline()?;
            if pattern.starts_with('/') || pattern.contains("..") || pattern.contains('\\') {
                return Err(invalid("Role scope must remain relative"));
            }
            builder
                .add(globset::Glob::new(pattern).map_err(|_| invalid("Invalid role scope glob"))?);
        }
        let scopes = builder
            .build()
            .map_err(|_| invalid("Invalid role scopes"))?;
        let mut related = Vec::new();
        for path in &files.paths {
            p.check_deadline()?;
            if scopes.is_match(path) {
                related.push(path.clone());
                if related.len() == 512 {
                    break;
                }
            }
        }
        let evidence = confirmation(p, role.confirmation.as_ref())?;
        roles.push(json!({"role_id":role.id,"scope":role.scope,"related_paths":related,"status":if evidence["status"]=="hash_matched_claim"{"confirmed"}else{"unconfirmed"},"confirmation":evidence,"confirmation_meaning":"declared_scope_with_current_evidence","operation_authorized":false,"scope_coverage":if related.len()==512{"bounded"}else{"observed"}}));
    }
    let mut conflicting = false;
    let mut confirmed = true;
    for c in &claims {
        p.check_deadline()?;
        conflicting |= c["status"] == "conflicting";
        confirmed &= c["status"] == "confirmed";
    }
    for r in &roles {
        p.check_deadline()?;
        confirmed &= r["status"] == "confirmed";
    }
    Ok(
        json!({"schema_version":1,"profile_id":profile.id,"profile_proof":proof(path,&file.hash),"expectations":claims,"roles":roles,"status":if conflicting{"conflicting"}else if confirmed{"confirmed"}else{"unconfirmed"},"operation_authorized":false,"owner_identity_verified":false,"profile_applied":false}),
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
fn script_check(
    p: &Project,
    path: &str,
    key: &str,
    command: &str,
    source_hash: &str,
) -> Result<Value> {
    p.check_deadline()?;
    let mut words = Vec::new();
    for word in command.split_whitespace() {
        p.check_deadline()?;
        words.push(word);
    }
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
    let mut parser = "unknown";
    'family: for (token, name) in families {
        p.check_deadline()?;
        for word in &words {
            p.check_deadline()?;
            if *word == token {
                parser = name;
                break 'family;
            }
        }
    }
    let value = json!({"check_key":format!("manifest:{}:{key}",hash(path)),"source_script":key,"command":safe(p, &json!(command.chars().take(1024).collect::<String>()))?,"command_hash":hash(command),"scope":path.rsplit_once('/').map(|(dir,_)|dir).unwrap_or("."),"parser_candidate":parser,"resource_candidate":if ["jest","jest-expo","vitest","playwright","maestro","typescript","cargo"].contains(&parser){"heavy_compute"}else{"unknown"},"policy":"unregistered_requires_confirmation","executed":false,"gate_checks_omission_authorized":false,"proof":proof(path,source_hash)});
    p.check_deadline()?;
    Ok(value)
}
pub fn validate_limits(max_files: usize, max_bytes: usize) -> Result<()> {
    if !(1..=4096).contains(&max_files) || !(1..=16777216).contains(&max_bytes) {
        Err(invalid(
            "Inventory limits: 1..4096 files, 1..16777216 bytes",
        ))
    } else {
        Ok(())
    }
}
pub fn scan(
    p: &Project,
    profile_path: Option<&str>,
    max_files: usize,
    max_bytes: usize,
) -> Result<Value> {
    validate_limits(max_files, max_bytes)?;
    let scope = query_scope(p)?;
    let result = scan_inner(&scope, profile_path, max_files, max_bytes);
    scope.check_deadline()?;
    reader::validate_root(&scope)?;
    result
}
fn scan_inner(
    p: &Project,
    profile_path: Option<&str>,
    max_files: usize,
    max_bytes: usize,
) -> Result<Value> {
    p.check_deadline()?;
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
        p.check_deadline()?;
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
                let e = partial_error(p, e)?;
                problems.push(json!({"path":path,"reason":e.code}));
                continue;
            }
        };
        p.check_deadline()?;
        let metadata = std::fs::metadata(selected);
        p.check_deadline()?;
        let size = match metadata {
            Ok(v) => v.len(),
            Err(_) => {
                reader::validate_root(p)?;
                reader::validate_policy(p)?;
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
                let e = partial_error(p, e)?;
                problems.push(json!({"path":path,"reason":e.code}));
                continue;
            }
        };
        spent = spent.saturating_add(file.text.len());
        if spent > max_bytes {
            problems.push(json!({"reason":"byte_bound_after_concurrent_size_change","path":path,"actual_read_bytes":spent}));
            break;
        }
        #[cfg(test)]
        phase_observation(p, Phase::ScanAdmitted);
        p.check_deadline()?;
        #[cfg(test)]
        phase_observation(p, Phase::ScanWork);
        sources.insert(path.clone(), file.hash.clone());
        let mut item = json!({"path":path,"kind":kind,"size_bytes":file.size_bytes,"proof":proof(&path,&file.hash)});
        match kind {
            "npm_package" => match static_value(p, &path, &file.text) {
                Ok(value) => {
                    let value = safe(p, &value)?;
                    let name = value["name"].as_str().unwrap_or("unnamed");
                    item["name"] = json!(name);
                    item["version"] = value["version"].clone();
                    item["license"] = value["license"].clone();
                    item["engines"] = value["engines"].clone();
                    item["package_manager"] = value["packageManager"].clone();
                    let mut deps = Vec::new();
                    for kind in [
                        "dependencies",
                        "devDependencies",
                        "peerDependencies",
                        "optionalDependencies",
                    ] {
                        p.check_deadline()?;
                        if let Some(map) = value.get(kind).and_then(Value::as_object) {
                            for (name, version) in map {
                                p.check_deadline()?;
                                deps.push(json!({"name":name,"version":version,"kind":kind}));
                            }
                        }
                    }
                    item["dependencies"] = json!(deps);
                    packages.push(json!({"name":name,"path":path,"kind":"npm","dependencies":deps,"proof":proof(&path,&file.hash)}));
                    if let Some(scripts) = value["scripts"].as_object() {
                        for (key, command) in scripts {
                            p.check_deadline()?;
                            if let Some(command) = command.as_str() {
                                checks.push(script_check(p, &path, key, command, &file.hash)?);
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
                                p.check_deadline()?;
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
                    let e = partial_error(p, e)?;
                    item["coverage"] = json!("unknown");
                    problems.push(json!({"path":path,"reason":e.code}));
                }
            },
            "cargo_package" => match static_value(p, &path, &file.text) {
                Ok(value) => {
                    item["package"] = safe(
                        p,
                        &json!({"name":value["package"]["name"],"version":value["package"]["version"],"edition":value["package"]["edition"],"rust_version":value["package"]["rust-version"],"license":value["package"]["license"],"license_file":value["package"]["license-file"]}),
                    )?;
                    if value["package"]["version"].is_object() {
                        problems.push(
                            json!({"path":path,"reason":"workspace_inheritance_not_resolved"}),
                        );
                    }
                    let mut deps = Vec::new();
                    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
                        p.check_deadline()?;
                        if let Some(map) = value[key].as_object() {
                            for (name, v) in map {
                                p.check_deadline()?;
                                deps.push(safe(p, &json!({"name":name,"version":v.as_str().map(Value::from).unwrap_or(v["version"].clone()),"path":v["path"],"kind":key}))?);
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
                            p.check_deadline()?;
                            if let Some(s) = m.as_str() {
                                cargo_members.push((path.clone(), s.into()));
                            }
                        }
                    }
                    checks.push(script_check(
                        p,
                        &path,
                        "cargo-test",
                        "cargo test",
                        &file.hash,
                    )?);
                    checks.push(script_check(
                        p,
                        &path,
                        "cargo-check",
                        "cargo check",
                        &file.hash,
                    )?);
                }
                Err(e) => {
                    let e = partial_error(p, e)?;
                    problems.push(json!({"path":path,"reason":e.code}));
                }
            },
            "pnpm_workspace" => match static_value(p, &path, &file.text) {
                Ok(v) => {
                    if let Some(members) = v["packages"].as_array() {
                        for m in members {
                            p.check_deadline()?;
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
                Err(e) => {
                    let e = partial_error(p, e)?;
                    problems.push(json!({"path":path,"reason":e.code}));
                }
            },
            "ci_workflow" => match static_value(p, &path, &file.text) {
                Ok(v) => {
                    item["workflow_name"] = safe(p, &v["name"])?;
                    let mut jobs = Vec::new();
                    if let Some(map) = v["jobs"].as_object() {
                        for (key, job) in map {
                            p.check_deadline()?;
                            let mut steps = Vec::new();
                            for (at, step) in
                                job["steps"].as_array().into_iter().flatten().enumerate()
                            {
                                p.check_deadline()?;
                                let run = step["run"].as_str();
                                steps.push(json!({"index":at,"uses":safe(p, &step["uses"])?,"run_hash":run.map(hash),"run_dynamic":run.is_some_and(|s|s.contains("${{")),"environment_values_omitted":true}));
                                if let Some(run) = run {
                                    checks.push(script_check(
                                        p,
                                        &path,
                                        &format!("{key}/{at}"),
                                        run,
                                        &file.hash,
                                    )?);
                                    if run.contains("${{") {
                                        problems.push(json!({"path":path,"reason":"dynamic_workflow_expression"}));
                                    }
                                }
                            }
                            jobs.push(
                                json!({"id":key,"runs_on":safe(p, &job["runs-on"])?,"steps":steps}),
                            );
                        }
                    } else {
                        problems.push(json!({"path":path,"reason":"unsupported_jobs"}));
                    }
                    item["jobs"] = json!(jobs);
                }
                Err(e) => {
                    let e = partial_error(p, e)?;
                    problems.push(json!({"path":path,"reason":e.code}));
                }
            },
            "cargo_config" => {
                match static_value(
                    p,
                    if path.ends_with(".toml") {
                        &path
                    } else {
                        "config.toml"
                    },
                    &file.text,
                ) {
                    Ok(v) => {
                        item["build_target"] = safe(p, &v["build"]["target"])?;
                        item["aliases"] = safe(p, &v["alias"])?;
                        if let Some(aliases) = v["alias"].as_object() {
                            for (key, value) in aliases {
                                p.check_deadline()?;
                                if let Some(command) = value.as_str() {
                                    checks.push(script_check(p, &path, key, command, &file.hash)?);
                                } else {
                                    problems.push(json!({"path":path,"reason":"unsupported_cargo_alias_form"}));
                                }
                            }
                        }
                        item["environment_values_omitted"] = json!(true);
                    }
                    Err(e) => {
                        let e = partial_error(p, e)?;
                        problems.push(json!({"path":path,"reason":e.code}));
                    }
                }
            }
            "typescript_config" | "static_config" => match static_value(p, &path, &file.text) {
                Ok(v) => {
                    item["metadata"] = if kind == "typescript_config" {
                        safe(
                            p,
                            &json!({"extends":v["extends"],"base_url":v["compilerOptions"]["baseUrl"],"paths":v["compilerOptions"]["paths"]}),
                        )?
                    } else {
                        json!({"declared_static":true,"content_values_omitted":true})
                    };
                    if v.get("extends").is_some() {
                        problems
                            .push(json!({"path":path,"reason":"config_inheritance_not_resolved"}));
                    }
                }
                Err(e) => {
                    let e = partial_error(p, e)?;
                    problems.push(json!({"path":path,"reason":e.code}));
                }
            },
            "rule_document" | "document" => {
                let mut headings = Vec::new();
                for (i, line) in file.text.lines().enumerate() {
                    p.check_deadline()?;
                    let line = line.trim_start();
                    if line.starts_with('#') {
                        headings.push(json!({"line":i+1,"title":safe(p, &json!(line.trim_start_matches('#').trim().chars().take(256).collect::<String>()))?}));
                        if headings.len() == 128 {
                            break;
                        }
                    }
                }
                item["headings"] = json!(headings);
                item["native_loading"] = json!("unknown");
                item["operation_authorized"] = json!(false);
            }
            "source_metadata" => {
                p.check_deadline()?;
                let analyzed = search::analyze(&path, &file.hash, &file.text);
                p.check_deadline()?;
                let entry = analyzed?;
                item["language"] = json!(entry.language);
                item["parse_status"] = json!(entry.parse_status);
                let mut symbols = Vec::new();
                for symbol in entry.symbols.into_iter().take(128) {
                    p.check_deadline()?;
                    symbols.push(json!({"name":symbol.name,"qualified_name":symbol.qualified_name,"kind":symbol.kind,"start_line":symbol.start_line,"end_line":symbol.end_line}));
                }
                item["symbols"] = safe(p, &json!(symbols))?;
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
            p.check_deadline()?;
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
            let mut paths = Vec::new();
            for package in &packages {
                p.check_deadline()?;
                if package["kind"] == kind
                    && let Some(path) = package["path"].as_str()
                    && glob.as_ref().is_ok_and(|g| {
                        g.is_match(path.rsplit_once('/').map(|(d, _)| d).unwrap_or("."))
                    })
                {
                    paths.push(path);
                }
            }
            workspaces.push(json!({"manifest":manifest,"pattern":pattern,"members":paths,"kind":kind,"proof":sources.get(manifest).map(|h|proof(manifest,h))}));
        }
    }
    let mut reverse = BTreeMap::<String, Vec<String>>::new();
    let mut names = BTreeSet::new();
    for package in &packages {
        p.check_deadline()?;
        if let Some(name) = package["name"].as_str() {
            names.insert(name);
        }
    }
    for package in &packages {
        p.check_deadline()?;
        for dep in package["dependencies"].as_array().into_iter().flatten() {
            p.check_deadline()?;
            if let Some(name) = dep["name"].as_str().filter(|n| names.contains(n)) {
                reverse
                    .entry(name.into())
                    .or_default()
                    .push(package["path"].as_str().unwrap_or("").into());
            }
        }
    }
    for v in reverse.values_mut() {
        p.check_deadline()?;
        v.sort();
        v.dedup();
    }
    p.check_deadline()?;
    let result = json!({"schema_version":1,"inventory_id":hash(serde_json::to_vec(&json!({"sources":sources,"policy_hash":p.policy_hash(),"serializer":"inventory-static-v1"}))?),"policy_hash":p.policy_hash(),"sources":sources,"items":items,"packages":packages,"workspace_declarations":workspaces,"reverse_dependency_candidates":reverse,"check_candidates":checks,"profile":profile_path.map(|path|profile(p,path)).transpose()?,"coverage":{"status":if problems.is_empty(){"complete_for_static_subset"}else{"partial"},"reasons":problems},"read_bytes":spent,"attempted_files":attempted,"operation_authorized":false,"scripts_executed":false,"native_rule_loading":"unknown"});
    safe(p, &result)
}

fn section<'a>(p: &Project, text: &'a str, name: &str) -> Result<&'a str> {
    p.check_deadline()?;
    if name.is_empty() {
        return Ok(text);
    }
    let mut start = None;
    let mut end = text.len();
    let mut offset = 0usize;
    let mut matching = 0;
    let mut fence: Option<char> = None;
    for line in text.split_inclusive('\n') {
        p.check_deadline()?;
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
    let scope = query_scope(p)?;
    let result = audit_inner(&scope, path, since);
    scope.check_deadline()?;
    reader::validate_root(&scope)?;
    result
}
fn audit_inner(p: &Project, path: &str, since: Option<&str>) -> Result<Value> {
    p.check_deadline()?;
    let file = reader::read(p, path)?;
    let registry: Registry = serde_json::from_str(&file.text)
        .map_err(|_| invalid("Invalid document source_refs schema"))?;
    p.check_deadline()?;
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
        #[cfg(test)]
        phase_observation(p, Phase::AuditAdmitted);
        p.check_deadline()?;
        #[cfg(test)]
        phase_observation(p, Phase::AuditWork);
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
                let e = partial_error(p, e)?;
                let current = match reader::read(p, &r.source) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        partial_error(p, e)?;
                        None
                    }
                };
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
                let e = partial_error(p, e)?;
                review.push(json!({"document":r.document,"source":r.source,"reason":e.code}));
                continue;
            }
        };
        let part = match section(p, &document.text, &r.section) {
            Ok(part) => part,
            Err(e) => {
                let e = partial_error(p, e)?;
                review.push(json!({"document":r.document,"section":r.section,"reason":e.code}));
                continue;
            }
        };
        if actual == safe(p, &r.expected)? {
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
            rendered(&safe(p, &r.expected)?).map(|v| r.template.replace("{value}", &v));
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
    p.check_deadline()?;
    mismatch.sort();
    mismatch.dedup();
    let audit_id = hash(serde_json::to_vec(&proposals)?);
    p.check_deadline()?;
    let mut proposal_values = Vec::new();
    for proposal in proposals.into_values() {
        p.check_deadline()?;
        proposal_values.push(proposal);
    }
    safe(
        p,
        &json!({"schema_version":1,"audit_id":audit_id,"registry_proof":proof(path,&file.hash),"baseline_event":registry.since_event,"baseline_event_provenance":"registry_claim_not_event_history_replay","changed_sources":changed,"affected_documents":affected,"verified_mismatch":mismatch,"review_needed":review,"proposals":proposal_values,"documents_modified":false,"tasks_created":false,"coverage":{"status":if review.is_empty(){"complete_for_registered_refs"}else{"partial"},"unregistered_document_semantics":"not_inferred"}}),
    )
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

#[cfg(test)]
mod deadline_tests {
    use super::*;
    use crate::project::{Config, ProjectConfig, RootAnchor};
    use std::{
        cell::RefCell,
        fs,
        rc::Rc,
        time::{Duration, Instant},
    };
    fn fixture() -> (tempfile::TempDir, Project) {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("project");
        fs::create_dir_all(&root).unwrap();
        let p = Project {
            deadline: None,
            root_anchor: RootAnchor::capture(&root).unwrap(),
            root,
            data_dir: base.join("data"),
            workspace_dir: base.join("data/workspace"),
            control_dir: base.join("data/control"),
            project_id: "fixture".into(),
            workspace_id: "workspace".into(),
            coordination_id: "coordination".into(),
            config: Config {
                schema_version: 1,
                project: ProjectConfig {
                    id: "fixture".into(),
                    name: "fixture".into(),
                },
                index: Default::default(),
                policy: Default::default(),
                search: Default::default(),
                context: Default::default(),
                roles: Default::default(),
            },
        };
        (temp, p)
    }
    fn snapshot(p: &Project) -> BTreeMap<String, String> {
        reader::manifest(&Project {
            deadline: None,
            ..p.clone()
        })
        .unwrap()
    }
    fn assert_unchanged(p: &Project, before: &BTreeMap<String, String>) {
        assert_eq!(&snapshot(p), before);
        assert!(!p.data_dir.exists());
        assert!(!p.control_db().exists());
        assert!(!p.index_db().exists());
        assert!(!p.root.join("EXECUTED").exists());
    }
    fn expiring(
        p: &mut Project,
        admitted: Phase,
        work: Phase,
        call: impl FnOnce(&Project) -> Result<Value>,
    ) {
        let before = snapshot(p);
        let original = crate::deadline::Deadline::from_millis(1000).unwrap();
        p.deadline = Some(original);
        let trace = Rc::new(RefCell::new((0usize, 0usize)));
        let observed = trace.clone();
        let start = Instant::now();
        {
            let _guard = observe_phase(move |phase, deadline| {
                assert_eq!(deadline.unwrap().instant(), original.instant());
                let mut trace = observed.borrow_mut();
                if phase == admitted {
                    trace.0 += 1;
                    if trace.0 == 17 {
                        while let Ok(left) = original.remaining() {
                            std::thread::sleep(left.min(Duration::from_millis(5)));
                        }
                    }
                } else if phase == work {
                    trace.1 += 1;
                }
            });
            let e = call(p).unwrap_err();
            assert_eq!(e.code, "TIMEOUT");
            assert_eq!(e.exit, 7);
        }
        assert_eq!(*trace.borrow(), (17, 16));
        assert_eq!(p.deadline.unwrap().instant(), original.instant());
        assert!(start.elapsed() < Duration::from_secs(3));
        assert_unchanged(p, &before);
    }
    #[test]
    fn scan_original_budget_expires_after_real_source_items_before_next_processing() {
        let (_temp, mut p) = fixture();
        for i in 0..128 {
            let dir = p.root.join(format!("packages/{i:03}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("package.json"),json!({"name":format!("package-{i}"),"version":"1.0.0","scripts":{"test":"touch EXECUTED"}}).to_string()).unwrap();
        }
        let before = snapshot(&p);
        let scopes = Rc::new(RefCell::new(Vec::new()));
        let seen = scopes.clone();
        let start = Instant::now();
        {
            let _guard = observe_phase(move |phase, deadline| {
                if phase == Phase::ScanWork {
                    seen.borrow_mut().push(deadline.unwrap().instant());
                }
            });
            let v = scan(&p, None, 4096, 16777216).unwrap();
            assert_eq!(v["packages"].as_array().unwrap().len(), 128);
            assert_eq!(v["attempted_files"], 128);
            assert_eq!(v["scripts_executed"], false);
        }
        let scopes = scopes.borrow();
        assert_eq!(scopes.len(), 128);
        assert!(scopes.iter().all(|end| *end == scopes[0]));
        assert!(scopes[0] >= start + Duration::from_secs(9));
        drop(scopes);
        assert!(p.deadline.is_none());
        assert_unchanged(&p, &before);
        expiring(&mut p, Phase::ScanAdmitted, Phase::ScanWork, |p| {
            scan(p, None, 4096, 16777216)
        });
    }
    fn profile_fixture(p: &Project) {
        fs::write(p.root.join("fact.json"), r#"{"version":"1.0.0"}"#).unwrap();
        let source_hash = hash(fs::read(p.root.join("fact.json")).unwrap());
        let claims:Vec<_>=(0..MAX_REFS).map(|_|json!({"source":"fact.json","selector":"/version","expected":"1.0.0","confirmation":{"source":"fact.json","hash":source_hash,"authority":"declared-owner"}})).collect();
        fs::write(
            p.root.join("profile.json"),
            json!({"schema_version":1,"id":"fixture","expectations":claims,"roles":[]}).to_string(),
        )
        .unwrap();
    }
    #[test]
    fn profile_original_budget_expires_after_actual_confirmed_claims() {
        let (_temp, mut p) = fixture();
        profile_fixture(&p);
        let before = snapshot(&p);
        let v = profile(&p, "profile.json").unwrap();
        assert_eq!(v["expectations"].as_array().unwrap().len(), MAX_REFS);
        assert_eq!(v["status"], "confirmed");
        assert_eq!(v["owner_identity_verified"], false);
        assert!(p.deadline.is_none());
        assert_unchanged(&p, &before);
        expiring(&mut p, Phase::ProfileAdmitted, Phase::ProfileWork, |p| {
            profile(p, "profile.json")
        });
    }
    #[test]
    fn audit_original_budget_expires_after_actual_claim_reads() {
        let (_temp, mut p) = fixture();
        fs::write(p.root.join("fact.json"), r#"{"version":"1.0.0"}"#).unwrap();
        fs::write(p.root.join("README.md"), "# Version\nVersion 1.0.0\n").unwrap();
        let source_hash = hash(fs::read(p.root.join("fact.json")).unwrap());
        let document_hash = hash(fs::read(p.root.join("README.md")).unwrap());
        let refs:Vec<_>=(0..MAX_REFS).map(|_|json!({"document":"README.md","section":"Version","source":"fact.json","selector":"/version","expected":"1.0.0","rendered":"Version 1.0.0","template":"Version {value}","source_hash":source_hash,"document_hash":document_hash})).collect();
        fs::write(
            p.root.join("registry.json"),
            json!({"schema_version":1,"since_event":"fixture","source_refs":refs}).to_string(),
        )
        .unwrap();
        let before = snapshot(&p);
        let actual = Rc::new(RefCell::new(Vec::new()));
        let observed = actual.clone();
        let v = {
            let _guard = observe_phase(move |phase, deadline| {
                if phase == Phase::AuditWork {
                    observed.borrow_mut().push(deadline.unwrap().instant());
                }
            });
            audit(&p, "registry.json", Some("fixture")).unwrap()
        };
        {
            let actual = actual.borrow();
            assert_eq!(actual.len(), MAX_REFS);
            assert!(actual.iter().all(|end| *end == actual[0]));
        }

        assert_eq!(v["coverage"]["status"], "complete_for_registered_refs");
        assert!(v["review_needed"].as_array().unwrap().is_empty());
        assert!(v["proposals"].as_array().unwrap().is_empty());
        assert_eq!(v["documents_modified"], false);
        assert!(p.deadline.is_none());
        assert_unchanged(&p, &before);
        expiring(&mut p, Phase::AuditAdmitted, Phase::AuditWork, |p| {
            audit(p, "registry.json", Some("fixture"))
        });
    }
    #[test]
    fn confirmation_and_partial_fallback_cannot_absorb_expired_or_changed_root() {
        let (_temp, mut p) = fixture();
        let c = Confirmation {
            source: "missing.json".into(),
            hash: "fixture".into(),
            authority: "declared".into(),
        };
        p.deadline = Some(crate::deadline::Deadline::from_instant(Instant::now()));
        assert_eq!(confirmation(&p, Some(&c)).unwrap_err().code, "TIMEOUT");
        let incidental = Error::new("SOURCE_UNAVAILABLE", "Missing fixture", 6);
        assert_eq!(partial_error(&p, incidental).unwrap_err().code, "TIMEOUT");
        p.deadline = None;
        let old = p.root.with_file_name("old");
        fs::rename(&p.root, &old).unwrap();
        fs::create_dir(&p.root).unwrap();
        assert_eq!(
            confirmation(&p, Some(&c)).unwrap_err().code,
            "POLICY_DENIED"
        );
        assert_eq!(
            partial_error(&p, Error::new("POLICY_DENIED", "Excluded fixture", 5))
                .unwrap_err()
                .code,
            "POLICY_DENIED"
        );
        assert!(!p.data_dir.exists());
    }
}

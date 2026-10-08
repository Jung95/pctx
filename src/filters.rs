//! Inert, bounded presentation rules. Filter success is never check evidence.
use crate::{
    domain::{Error, Result, hash},
    project::{Project, atomic_write, private_dir},
    reader,
};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Digest;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_INPUT: usize = 1024 * 1024;
const MAX_RECORD: usize = 256 * 1024;
const MAX_RECORDS: usize = 8192;
const REQUIRED_CASES: [&str; 11] = [
    "success",
    "nonzero",
    "warning-only",
    "empty",
    "timeout",
    "malformed",
    "large",
    "unicode",
    "cr-progress",
    "streams",
    "split-secret",
];
#[derive(Debug, Subcommand)]
pub enum FilterCommand {
    Validate {
        path: PathBuf,
    },
    Apply {
        #[arg(long)]
        filter: String,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        child_exit: i32,
    },
    Test {
        path: PathBuf,
        #[arg(long)]
        fixtures: PathBuf,
    },
    Activate {
        id: String,
        #[arg(long)]
        expect_hash: String,
    },
    Explain {
        #[arg(last = true, required = true)]
        argv: Vec<String>,
    },
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    schema_version: u32,
    id: String,
    version: String,
    priority: i32,
    #[serde(rename = "match")]
    matcher: Matcher,
    parse: Parse,
    render: Render,
    rules: Vec<Rule>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matcher {
    program: String,
    argv_prefix: Vec<String>,
    stream: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parse {
    kind: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Render {
    max_bytes: usize,
    keep_head_lines: usize,
    keep_tail_lines: usize,
    show_omission_counts: bool,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Rule {
    Protect {
        pattern: String,
    },
    Drop {
        pattern: String,
    },
    SelectFields {
        fields: Vec<String>,
    },
    GroupBy {
        fields: Vec<String>,
    },
    DeduplicateExact {
        show_count: bool,
    },
    #[serde(alias = "bounded_excerpt")]
    Excerpt {
        keep_head_lines: usize,
        keep_tail_lines: usize,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilterRecord {
    pub stream: String,
    pub sequence: u64,
    pub text: String,
    pub start_byte: usize,
    pub end_byte: usize,
}
#[derive(Clone, Serialize)]
struct ViewRecord {
    stream: String,
    text: String,
    sequence: u64,
    ranges: Vec<(usize, usize)>,
    count: usize,
    protected: bool,
    fields: Option<Value>,
    group: Option<String>,
}
struct Loaded {
    definition: Definition,
    digest: String,
    path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Suite {
    schema_version: u32,
    executable: String,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    input: String,
    #[serde(default)]
    stderr: Option<String>,
    child_exit: i32,
    #[serde(default = "exited")]
    termination: String,
    must_keep: Vec<String>,
    minimum_protected: usize,
    expected_view_hash: String,
}
fn exited() -> String {
    "exited".into()
}
#[derive(Serialize, Deserialize)]
struct Report {
    schema_version: u32,
    filter_id: String,
    filter_hash: String,
    policy_hash: String,
    workspace_id: String,
    manifest_path: String,
    manifest_hash: String,
    input_hashes: BTreeMap<String, String>,
    executable: String,
    executable_hash: String,
    scripts: BTreeMap<String, String>,
    cases: Vec<String>,
    all_passed: bool,
}
#[derive(Serialize, Deserialize)]
struct Binding {
    filter_id: String,
    filter_hash: String,
    filter_path: String,
    policy_hash: String,
    workspace_id: String,
    executable: String,
    executable_hash: String,
    scripts: BTreeMap<String, String>,
    fixture_report_hash: String,
}
fn err(code: &str, msg: &str, exit: i32) -> Error {
    Error::new(code, msg, exit)
}
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn relative(p: &Project, path: &Path) -> Result<String> {
    let path = if path.is_absolute() {
        path.strip_prefix(&p.root).map_err(|_| {
            err(
                "POLICY_DENIED",
                "Filter and fixture paths must remain in this project",
                5,
            )
        })?
    } else {
        path
    };
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| err("INVALID_ARGUMENT", "UTF-8 path required", 2))
}
fn filter_path(p: &Project, value: &str) -> Result<String> {
    if safe_id(value) {
        return Ok(format!(".pctx/filters/{value}.toml"));
    }
    let path = relative(p, Path::new(value))?;
    if !path.starts_with(".pctx/filters/") || !path.ends_with(".toml") {
        return Err(err(
            "POLICY_DENIED",
            "Project filter must be under .pctx/filters",
            5,
        ));
    }
    Ok(path)
}
fn regex(pattern: &str) -> Result<regex::Regex> {
    if pattern.len() > 4096 {
        return Err(err(
            "FILTER_LIMIT_EXCEEDED",
            "Filter pattern exceeds limit",
            2,
        ));
    }
    regex::RegexBuilder::new(pattern)
        .size_limit(1024 * 1024)
        .dfa_size_limit(1024 * 1024)
        .build()
        .map_err(|_| err("FILTER_INVALID", "Unsupported or invalid regex", 2))
}
fn load(p: &Project, value: &str) -> Result<Loaded> {
    let path = filter_path(p, value)?;
    let file = reader::read(p, &path)?;
    if file.text.len() > 65536 {
        return Err(err(
            "FILTER_LIMIT_EXCEEDED",
            "Filter definition exceeds 64KiB",
            2,
        ));
    }
    let definition: Definition = toml::from_str(&file.text).map_err(|_| {
        err(
            "FILTER_INVALID",
            "Invalid filter schema, key or operation",
            2,
        )
    })?;
    if definition.schema_version != 1
        || !safe_id(&definition.id)
        || definition.version.is_empty()
        || definition.matcher.program.is_empty()
        || definition.matcher.argv_prefix.len() > 64
        || !["stdout", "stderr", "both"].contains(&definition.matcher.stream.as_str())
        || !["lines", "json"].contains(&definition.parse.kind.as_str())
        || definition.render.max_bytes < 2048
        || definition.render.max_bytes > 65536
        || definition.render.keep_head_lines + definition.render.keep_tail_lines > 1000
        || !definition.render.show_omission_counts
        || definition.rules.len() > 64
    {
        return Err(err(
            "FILTER_INVALID",
            "Unsupported filter values or required omission accounting",
            2,
        ));
    }
    for rule in &definition.rules {
        match rule {
            Rule::Protect { pattern } | Rule::Drop { pattern } => {
                let _ = regex(pattern)?;
            }
            Rule::SelectFields { fields } | Rule::GroupBy { fields } => {
                if fields.is_empty()
                    || fields.len() > 32
                    || fields.iter().any(|f| f.is_empty() || f.len() > 128)
                {
                    return Err(err("FILTER_INVALID", "Bounded field names required", 2));
                }
            }
            Rule::DeduplicateExact { show_count } => {
                if !show_count {
                    return Err(err(
                        "FILTER_INVALID",
                        "Deduplication must preserve occurrence count",
                        2,
                    ));
                }
            }
            Rule::Excerpt {
                keep_head_lines,
                keep_tail_lines,
            } => {
                if keep_head_lines + keep_tail_lines > 1000 {
                    return Err(err("FILTER_INVALID", "Excerpt bounds exceed limit", 2));
                }
            }
        }
    }
    Ok(Loaded {
        definition,
        digest: file.hash,
        path,
    })
}
fn inputs(text: &str, stream: &str) -> Vec<FilterRecord> {
    let mut records = Vec::new();
    let mut start = 0;
    let mut private_key = false;
    for raw in text.split_inclusive(['\n', '\r']) {
        let end = start + raw.len();
        let content = raw.trim_end_matches(['\n', '\r']);
        if content.contains("-----BEGIN") && content.contains("PRIVATE KEY-----") {
            private_key = true;
        }
        let masked = if private_key {
            "[REDACTED PRIVATE KEY RECORD]".into()
        } else {
            reader::redact(content).0
        };
        records.push(FilterRecord {
            stream: stream.into(),
            sequence: records.len() as u64,
            text: masked,
            start_byte: start,
            end_byte: end,
        });
        if content.contains("-----END") && content.contains("PRIVATE KEY-----") {
            private_key = false;
        }
        start = end;
    }
    records
}
fn deadline(start: Instant) -> Result<()> {
    if start.elapsed() > Duration::from_secs(1) {
        Err(err(
            "FILTER_LIMIT_EXCEEDED",
            "Filter processing exceeded one second",
            3,
        ))
    } else {
        Ok(())
    }
}
fn render(
    p: &Project,
    loaded: &Loaded,
    input: &[FilterRecord],
    child_exit: i32,
    termination: &str,
) -> Result<Value> {
    let started = Instant::now();
    let d = &loaded.definition;
    if input.len() > MAX_RECORDS || input.iter().map(|r| r.text.len()).sum::<usize>() > MAX_INPUT {
        return Err(err(
            "FILTER_LIMIT_EXCEEDED",
            "Preview input exceeds bounded parser memory",
            3,
        ));
    }
    let ansi = regex(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\))")?;
    let typed = regex(
        r"(?i)\berror\b|\bfailed\b|\bfailure\b|\bwarning\b|\bsignal\b|\bsummary\b|\bdenied\b|\btimeout\b|\bpanic\b",
    )?;
    let protects = d
        .rules
        .iter()
        .filter_map(|r| {
            if let Rule::Protect { pattern } = r {
                Some(regex(pattern))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let mut records = Vec::new();
    let mut omitted = 0usize;
    let mut private_key = false;
    for record in input {
        deadline(started)?;
        if record.text.len() > MAX_RECORD {
            omitted += 1;
            continue;
        }
        if !["stdout", "stderr"].contains(&record.stream.as_str())
            || record.end_byte < record.start_byte
        {
            return Err(err("FILTER_INVALID", "Invalid stream or source range", 2));
        }
        let clean = ansi.replace_all(&record.text, "");
        let clean = clean
            .chars()
            .filter(|c| *c == '\t' || !c.is_control())
            .collect::<String>();
        if clean.contains("-----BEGIN") && clean.contains("PRIVATE KEY-----") {
            private_key = true;
        }
        let text = if private_key {
            "[REDACTED PRIVATE KEY RECORD]".to_string()
        } else {
            reader::redact(&clean).0
        };
        if clean.contains("-----END") && clean.contains("PRIVATE KEY-----") {
            private_key = false;
        }
        let fields = if d.parse.kind == "json" {
            serde_json::from_str::<Value>(&text)
                .ok()
                .filter(Value::is_object)
        } else {
            None
        };
        let protected = typed.is_match(&text)
            || protects.iter().any(|re| re.is_match(&text))
            || fields.as_ref().is_some_and(|v| {
                matches!(v["severity"].as_str(), Some("error" | "warning"))
                    || matches!(
                        v["type"].as_str(),
                        Some("error" | "warning" | "signal" | "summary")
                    )
            });
        records.push(ViewRecord {
            stream: record.stream.clone(),
            sequence: record.sequence,
            text,
            ranges: vec![(record.start_byte, record.end_byte)],
            count: 1,
            protected,
            fields,
            group: None,
        });
    }
    let protected_input = records.iter().filter(|r| r.protected).count();
    let mut trace = Vec::new();
    for rule in &d.rules {
        deadline(started)?;
        let before = records.len();
        match rule {
            Rule::Protect { .. } => {}
            Rule::Drop { pattern } => {
                let pattern = regex(pattern)?;
                records.retain(|r| {
                    r.protected
                        || (d.matcher.stream != "both" && d.matcher.stream != r.stream)
                        || !pattern.is_match(&r.text)
                });
            }
            Rule::SelectFields { fields } => {
                for record in &mut records {
                    if !record.protected
                        && (d.matcher.stream == "both" || d.matcher.stream == record.stream)
                        && let Some(v) = &record.fields
                    {
                        let mut selected = serde_json::Map::new();
                        for field in fields {
                            if let Some(value) = v.get(field) {
                                selected.insert(field.clone(), value.clone());
                            }
                        }
                        record.fields = Some(Value::Object(selected));
                        record.text = record.fields.as_ref().unwrap().to_string();
                    }
                }
            }
            Rule::GroupBy { fields } => {
                for record in &mut records {
                    if let Some(v) = &record.fields {
                        record.group = Some(
                            fields
                                .iter()
                                .map(|f| format!("{f}={}", v.get(f).unwrap_or(&Value::Null)))
                                .collect::<Vec<_>>()
                                .join(";"),
                        );
                    }
                }
            }
            Rule::DeduplicateExact { .. } => {
                let mut seen = BTreeMap::<(String, String, bool), usize>::new();
                let mut grouped: Vec<ViewRecord> = Vec::new();
                for record in records {
                    deadline(started)?;
                    let key = (record.stream.clone(), record.text.clone(), record.protected);
                    if let Some(index) = seen.get(&key) {
                        grouped[*index].ranges.extend(record.ranges);
                        grouped[*index].count += record.count;
                    } else {
                        seen.insert(key, grouped.len());
                        grouped.push(record);
                    }
                }
                records = grouped;
            }
            Rule::Excerpt {
                keep_head_lines,
                keep_tail_lines,
            } => {
                let len = records.len();
                records = records
                    .into_iter()
                    .enumerate()
                    .filter(|(i, r)| {
                        r.protected
                            || *i < *keep_head_lines
                            || *i >= len.saturating_sub(*keep_tail_lines)
                    })
                    .map(|(_, r)| r)
                    .collect();
            }
        }
        let op = match rule {
            Rule::Protect { .. } => "protect",
            Rule::Drop { .. } => "drop",
            Rule::SelectFields { .. } => "select_fields",
            Rule::GroupBy { .. } => "group_by",
            Rule::DeduplicateExact { .. } => "deduplicate_exact",
            Rule::Excerpt { .. } => "excerpt",
        };
        trace.push(json!({"op":op,"input_records":before,"output_records":records.len()}));
    }
    let len = records.len();
    let head = d.render.keep_head_lines;
    let tail = d.render.keep_tail_lines;
    records = records
        .into_iter()
        .enumerate()
        .filter(|(i, r)| r.protected || *i < head || *i >= len.saturating_sub(tail))
        .map(|(_, r)| r)
        .collect();
    let mut kept = Vec::new();
    let mut used = 0;
    let budget = d.render.max_bytes.saturating_sub(1600 + trace.len() * 100);
    let mut protected_omitted = 0;
    records.sort_by_key(|r| (!r.protected, r.sequence));
    for record in records {
        let size = serde_json::to_vec(&record)?.len();
        if used + size <= budget {
            used += size;
            kept.push(record);
        } else {
            omitted += record.count;
            if record.protected {
                protected_omitted += record.count;
            }
        }
    }
    kept.sort_by_key(|r| r.sequence);
    let preserved = kept.iter().map(|r| r.count).sum::<usize>();
    let mut value = json!({"filter_id":d.id,"filter_version":d.version,"filter_hash":loaded.digest,"policy_hash":p.policy_hash(),"capability":"presentation_only","evidence_origin":"fixture_claim","child_exit_code":child_exit,"termination":termination,"execution_started":false,"command_rerun":false,"parse_status":if d.parse.kind=="json"&&kept.iter().any(|r|r.fields.is_none()){"partial"}else{"complete"},"records":kept,"input_records":input.len(),"preserved_occurrences":preserved,"omitted_occurrences":input.len().saturating_sub(preserved).max(omitted),"protected_occurrences":protected_input,"protected_omitted":protected_omitted,"source_ref":"explicit_preview_input","trace":trace,"test_result":"not_evaluated","task_completion":"not_evaluated"});
    while serde_json::to_vec(&value)?.len() > d.render.max_bytes {
        let rec = value["records"].as_array_mut().unwrap();
        if rec.is_empty() {
            return Err(err(
                "BUDGET_TOO_SMALL",
                "Filter metadata exceeds output budget",
                8,
            ));
        }
        let removed = rec.pop().unwrap();
        let count = removed["count"].as_u64().unwrap_or(1);
        value["omitted_occurrences"] =
            json!(value["omitted_occurrences"].as_u64().unwrap_or(0) + count);
        if removed["protected"] == true {
            value["protected_omitted"] =
                json!(value["protected_omitted"].as_u64().unwrap_or(0) + count);
        }
    }
    deadline(started)?;
    Ok(value)
}
fn explicit_input(p: &Project, path: &Path) -> Result<String> {
    if path == Path::new("-") {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(MAX_INPUT as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_INPUT {
            return Err(err(
                "FILTER_LIMIT_EXCEEDED",
                "Preview stdin exceeds 1MiB",
                3,
            ));
        }
        return String::from_utf8(bytes)
            .map_err(|_| err("UNSUPPORTED_ENCODING", "Preview requires UTF-8", 3));
    }
    // Explicit input is still policy-checked, unlike an arbitrary filter rule file read.
    let path = relative(p, path)?;
    let f = reader::read(p, &path)?;
    if f.text.len() > MAX_INPUT || f.text.contains('\0') {
        return Err(err(
            "FILTER_LIMIT_EXCEEDED",
            "Preview input exceeds bounds or is binary",
            3,
        ));
    }
    Ok(f.text)
}
fn private_path(p: &Project, area: &str, id: &str) -> Result<PathBuf> {
    if !safe_id(id) {
        return Err(err("FILTER_INVALID", "Invalid filter identifier", 2));
    }
    let dir = p.data_dir.join(area);
    for ancestor in dir.ancestors() {
        if fs::symlink_metadata(ancestor).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(err("POLICY_DENIED", "Linked user filter store denied", 5));
        }
    }
    private_dir(&dir)?;
    Ok(dir.join(format!("{id}.json")))
}
fn executable(program: &str) -> Result<(String, String)> {
    let path = if Path::new(program).is_absolute() {
        PathBuf::from(program)
    } else if program.contains('/') || program.contains('\\') {
        return Err(err(
            "FILTER_INVALID",
            "Executable must use PATH name or absolute path",
            2,
        ));
    } else {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|p| p.join(program))
            .find(|p| p.is_file())
            .ok_or_else(|| {
                err(
                    "CAPABILITY_UNAVAILABLE",
                    "Filter executable not installed",
                    6,
                )
            })?
    };
    let path = fs::canonicalize(path)?;
    if !path.is_file() || fs::metadata(&path)?.len() > 128 * 1024 * 1024 {
        return Err(err(
            "FILTER_LIMIT_EXCEEDED",
            "Executable fingerprint exceeds bounds",
            3,
        ));
    }
    let mut file = fs::File::open(&path)?;
    let mut digest = sha2::Sha256::new();
    let mut chunk = [0u8; 65536];
    loop {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        digest.update(&chunk[..n]);
    }
    Ok((
        path.to_string_lossy().into_owned(),
        format!("{:x}", digest.finalize()),
    ))
}
fn prefix_hashes(p: &Project, loaded: &Loaded) -> Result<BTreeMap<String, String>> {
    let mut scripts = BTreeMap::new();
    for token in &loaded.definition.matcher.argv_prefix {
        if !token.starts_with('-') && p.root.join(token).is_file() {
            let path = relative(p, Path::new(token))?;
            let f = reader::read(p, &path)?;
            scripts.insert(path, f.hash);
        }
    }
    Ok(scripts)
}
fn checked_binding(p: &Project, loaded: &Loaded) -> Result<Binding> {
    let path = private_path(p, "filter-bindings", &loaded.definition.id)?;
    let binding: Binding = serde_json::from_slice(
        &fs::read(path).map_err(|_| err("POLICY_DENIED", "Filter is not activated", 5))?,
    )?;
    let current = executable(&loaded.definition.matcher.program)?;
    if binding.filter_id != loaded.definition.id
        || binding.filter_hash != loaded.digest
        || binding.policy_hash != p.policy_hash()
        || binding.workspace_id != p.workspace_id
        || binding.executable != current.0
        || binding.executable_hash != current.1
        || binding.scripts != prefix_hashes(p, loaded)?
    {
        return Err(err(
            "CONFIG_CHANGED",
            "Filter or executable trust binding changed",
            9,
        ));
    }
    let report_bytes = fs::read(private_path(
        p,
        "filter-fixture-reports",
        &loaded.definition.id,
    )?)
    .map_err(|_| err("CONFIG_CHANGED", "Activated fixture report unavailable", 9))?;
    if hash(&report_bytes) != binding.fixture_report_hash {
        return Err(err("CONFIG_CHANGED", "Activated fixture report changed", 9));
    }
    let report: Report = serde_json::from_slice(&report_bytes)?;
    if reader::read(p, &report.manifest_path)?.hash != report.manifest_hash {
        return Err(err(
            "CONFIG_CHANGED",
            "Activated fixture manifest changed",
            9,
        ));
    }
    for (path, digest) in &report.input_hashes {
        if reader::read(p, path)?.hash != *digest {
            return Err(err("CONFIG_CHANGED", "Activated fixture input changed", 9));
        }
    }
    Ok(binding)
}
/// Explicit preview is permitted before activation and cannot execute the matched program.
pub fn preview_records(
    p: &Project,
    filter_id: &str,
    records: &[FilterRecord],
    child_exit: i32,
    termination: &str,
) -> Result<Value> {
    if !["exited", "timed_out", "signaled", "cancelled"].contains(&termination) {
        return Err(err("FILTER_INVALID", "Invalid claimed termination", 2));
    }
    let loaded = load(p, filter_id)?;
    render(p, &loaded, records, child_exit, termination)
}
pub fn apply_records(
    p: &Project,
    filter_id: &str,
    records: &[FilterRecord],
    child_exit: i32,
) -> Result<Value> {
    apply_observed(p, filter_id, records, Some(child_exit), "exited")
}
/// Rendering observed artifacts preserves native termination and never creates check evidence.
pub fn apply_observed(
    p: &Project,
    filter_id: &str,
    records: &[FilterRecord],
    child_exit: Option<i32>,
    termination: &str,
) -> Result<Value> {
    if !["exited", "timed_out", "signaled", "cancelled"].contains(&termination) {
        return Err(err("FILTER_INVALID", "Unknown observed termination", 2));
    }
    let loaded = load(p, filter_id)?;
    checked_binding(p, &loaded)?;
    let mut result = render(p, &loaded, records, child_exit.unwrap_or(128), termination)?;
    result["child_exit_code"] = json!(child_exit);
    result["evidence_origin"] = json!("stored_observed_output");
    Ok(result)
}
pub fn execute(p: &Project, command: &FilterCommand) -> Result<Value> {
    match command {
        FilterCommand::Validate { path } => {
            let loaded = load(p, &relative(p, path)?)?;
            Ok(
                json!({"valid":true,"id":loaded.definition.id,"filter_hash":loaded.digest,"capability":"presentation_only","execution_started":false,"rules":loaded.definition.rules.len()}),
            )
        }
        FilterCommand::Apply {
            filter,
            input,
            child_exit,
        } => {
            let loaded = load(p, filter)?;
            let text = explicit_input(p, input)?;
            let records = inputs(&text, "stdout");
            render(p, &loaded, &records, *child_exit, "exited")
        }
        FilterCommand::Test { path, fixtures } => test(p, path, fixtures),
        FilterCommand::Activate { id, expect_hash } => activate(p, id, expect_hash),
        FilterCommand::Explain { argv } => explain(p, argv),
    }
}
fn test(p: &Project, path: &Path, fixtures: &Path) -> Result<Value> {
    let loaded = load(p, &relative(p, path)?)?;
    let manifest_path = relative(p, &fixtures.join("manifest.json"))?;
    let manifest = reader::read(p, &manifest_path)?;
    let suite: Suite = serde_json::from_str(&manifest.text)
        .map_err(|_| err("FILTER_INVALID", "Invalid fixture manifest", 2))?;
    let names = suite
        .cases
        .iter()
        .map(|c| c.name.as_str())
        .collect::<BTreeSet<_>>();
    if suite.schema_version != 1
        || suite.cases.len() != REQUIRED_CASES.len()
        || names != REQUIRED_CASES.into_iter().collect()
    {
        return Err(err(
            "FILTER_INVALID",
            "Fixture suite must cover every required case exactly once",
            2,
        ));
    }
    let current = executable(&suite.executable)?;
    let expected = executable(&loaded.definition.matcher.program)?;
    if current != expected {
        return Err(err(
            "FILTER_INVALID",
            "Fixture executable must match declared program",
            2,
        ));
    }
    let mut results = Vec::new();
    let mut input_hashes = BTreeMap::new();
    let mut all_passed = true;
    for case in &suite.cases {
        if case.expected_view_hash.len() != 64
            || !case
                .expected_view_hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err(err(
                "FILTER_INVALID",
                "Each fixture requires an exact expected view hash",
                2,
            ));
        }
        let input_path = relative(p, &fixtures.join(&case.input))?;
        let raw = reader::read(p, &input_path)?;
        input_hashes.insert(input_path, raw.hash);
        let mut records = inputs(&raw.text, "stdout");
        let mut stderr = String::new();
        if let Some(path) = &case.stderr {
            let path = relative(p, &fixtures.join(path))?;
            let f = reader::read(p, &path)?;
            input_hashes.insert(path, f.hash);
            stderr = f.text;
            records.extend(inputs(&stderr, "stderr"));
        }
        let scenario = match case.name.as_str() {
            "success" => case.child_exit == 0,
            "nonzero" => case.child_exit != 0 && !case.must_keep.is_empty(),
            "warning-only" => {
                case.child_exit == 0 && raw.text.to_ascii_lowercase().contains("warning")
            }
            "empty" => raw.text.is_empty() && stderr.is_empty(),
            "timeout" => case.termination == "timed_out",
            "malformed" => serde_json::from_str::<Value>(&raw.text).is_err(),
            "large" => raw.text.len() >= 65536,
            "unicode" => !raw.text.is_ascii(),
            "cr-progress" => raw.text.contains('\r'),
            "streams" => !raw.text.is_empty() && !stderr.is_empty(),
            "split-secret" => reader::redact(&raw.text).1,
            _ => false,
        };
        let value = render(p, &loaded, &records, case.child_exit, &case.termination)?;
        let view_hash = hash(serde_json::to_vec(&value)?);
        let body = value["records"].to_string();
        let preserves = case.must_keep.iter().all(|id| body.contains(id));
        let passed = scenario
            && preserves
            && value["protected_occurrences"].as_u64().unwrap_or(0)
                >= case.minimum_protected as u64
            && view_hash == case.expected_view_hash
            && value["test_result"] == "not_evaluated"
            && value["child_exit_code"] == case.child_exit;
        all_passed &= passed;
        results.push(json!({"case":case.name,"passed":passed,"scenario_valid":scenario,"view_hash":view_hash,"required_items_preserved":preserves}));
    }
    let report = Report {
        schema_version: 1,
        filter_id: loaded.definition.id.clone(),
        filter_hash: loaded.digest.clone(),
        policy_hash: p.policy_hash(),
        workspace_id: p.workspace_id.clone(),
        manifest_path,
        manifest_hash: manifest.hash,
        input_hashes,
        executable: current.0,
        executable_hash: current.1,
        scripts: prefix_hashes(p, &loaded)?,
        cases: suite.cases.into_iter().map(|c| c.name).collect(),
        all_passed,
    };
    let bytes = serde_json::to_vec(&report)?;
    let report_hash = hash(&bytes);
    atomic_write(
        &private_path(p, "filter-fixture-reports", &loaded.definition.id)?,
        &bytes,
        true,
    )?;
    Ok(
        json!({"filter_id":loaded.definition.id,"filter_hash":loaded.digest,"all_passed":all_passed,"cases":results,"fixture_report_hash":report_hash,"execution_started":false,"test_result":"presentation_fixtures_only"}),
    )
}
fn activate(p: &Project, id: &str, expect_hash: &str) -> Result<Value> {
    if std::env::var("PCTX_ACTOR").unwrap_or_else(|_| "owner".into()) != "owner" {
        return Err(err(
            "POLICY_DENIED",
            "Filter activation requires local owner",
            5,
        ));
    }
    let loaded = load(p, id)?;
    if loaded.digest != expect_hash {
        return Err(err("CONFIG_CHANGED", "Filter changed since validation", 9));
    }
    let bytes = fs::read(private_path(p, "filter-fixture-reports", id)?).map_err(|_| {
        err(
            "POLICY_DENIED",
            "A passing complete fixture report is required",
            5,
        )
    })?;
    let report: Report = serde_json::from_slice(&bytes)?;
    if !report.all_passed
        || report.schema_version != 1
        || report.filter_id != id
        || report.filter_hash != loaded.digest
        || report.policy_hash != p.policy_hash()
        || report.workspace_id != p.workspace_id
        || report
            .cases
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != REQUIRED_CASES.into_iter().collect()
    {
        return Err(err(
            "CONFIG_CHANGED",
            "Fixture report does not match this filter scope",
            9,
        ));
    }
    if reader::read(p, &report.manifest_path)?.hash != report.manifest_hash {
        return Err(err("CONFIG_CHANGED", "Fixture manifest changed", 9));
    }
    for (path, digest) in &report.input_hashes {
        if reader::read(p, path)?.hash != *digest {
            return Err(err("CONFIG_CHANGED", "Fixture input changed", 9));
        }
    }
    let current = executable(&loaded.definition.matcher.program)?;
    if current.0 != report.executable
        || current.1 != report.executable_hash
        || report.scripts != prefix_hashes(p, &loaded)?
    {
        return Err(err("CONFIG_CHANGED", "Fixture executable changed", 9));
    }
    let binding = Binding {
        filter_id: id.into(),
        filter_hash: loaded.digest.clone(),
        filter_path: loaded.path.clone(),
        policy_hash: p.policy_hash(),
        workspace_id: p.workspace_id.clone(),
        executable: current.0,
        executable_hash: current.1,
        scripts: prefix_hashes(p, &loaded)?,
        fixture_report_hash: hash(&bytes),
    };
    atomic_write(
        &private_path(p, "filter-bindings", id)?,
        &serde_json::to_vec(&binding)?,
        true,
    )?;
    Ok(
        json!({"filter_id":id,"activated":true,"capability":"presentation_only","execution_started":false,"fixture_report_hash":binding.fixture_report_hash}),
    )
}
fn explain(p: &Project, argv: &[String]) -> Result<Value> {
    if argv.is_empty() || argv.len() > 256 {
        return Err(err("INVALID_ARGUMENT", "Bounded argv required", 2));
    }
    let actual = executable(&argv[0])?;
    let dir = p.data_dir.join("filter-bindings");
    let mut matches = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_symlink()) {
                return Err(err("POLICY_DENIED", "Linked filter binding denied", 5));
            }
            let Some(id) = entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_owned)
            else {
                continue;
            };
            if !safe_id(&id) {
                continue;
            }
            let loaded = load(p, &id)?;
            let binding = checked_binding(p, &loaded)?;
            if binding.executable == actual.0
                && binding.executable_hash == actual.1
                && argv[1..].starts_with(&loaded.definition.matcher.argv_prefix)
            {
                matches.push((loaded.definition.priority, id, loaded.digest));
            }
        }
    }
    matches.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    if matches.len() > 1 && matches[0].0 == matches[1].0 {
        return Err(err(
            "FILTER_AMBIGUOUS",
            "Multiple filters match at the same priority",
            2,
        ));
    }
    Ok(
        json!({"selected":matches.first().map(|m|json!({"priority":m.0,"id":m.1,"filter_hash":m.2})),"execution_started":false,"argv_rewritten":false,"capability":"presentation_only"}),
    )
}

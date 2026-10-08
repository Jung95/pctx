//! Inert, bounded presentation rules. Filter success is never check evidence.
use crate::{
    domain::{Error, Result, hash},
    project::{Project, atomic_write, private_dir},
    reader,
};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
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
    p.check_deadline()?;
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
    p.check_deadline()?;
    let path = filter_path(p, value)?;
    let file = reader::read(p, &path)?;
    if file.text.len() > 65536 {
        return Err(err(
            "FILTER_LIMIT_EXCEEDED",
            "Filter definition exceeds 64KiB",
            2,
        ));
    }
    p.check_deadline()?;
    let definition: Definition = toml::from_str(&file.text).map_err(|_| {
        err(
            "FILTER_INVALID",
            "Invalid filter schema, key or operation",
            2,
        )
    })?;
    p.check_deadline()?;
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
        p.check_deadline()?;
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
    p.check_deadline()?;
    Ok(Loaded {
        definition,
        digest: file.hash,
        path,
    })
}
#[cfg(test)]
#[derive(Clone, Copy, PartialEq)]
enum FilterPhase {
    InputAdmitted,
    InputWork,
    RecordAdmitted,
    RecordWork,
}
#[cfg(test)]
type PhaseObserver = Box<dyn FnMut(FilterPhase, Option<crate::deadline::Deadline>)>;
#[cfg(test)]
thread_local! {static PHASE_OBSERVER:std::cell::RefCell<Option<PhaseObserver>>=const{std::cell::RefCell::new(None)};}
#[cfg(test)]
struct PhaseGuard;
#[cfg(test)]
impl Drop for PhaseGuard {
    fn drop(&mut self) {
        PHASE_OBSERVER.with(|s| *s.borrow_mut() = None);
    }
}
#[cfg(test)]
fn observe_phase(
    callback: impl FnMut(FilterPhase, Option<crate::deadline::Deadline>) + 'static,
) -> PhaseGuard {
    PHASE_OBSERVER.with(|s| {
        assert!(s.borrow().is_none());
        *s.borrow_mut() = Some(Box::new(callback));
    });
    PhaseGuard
}
#[cfg(test)]
fn phase_observation(p: &Project, phase: FilterPhase) {
    PHASE_OBSERVER.with(|s| {
        if let Some(callback) = s.borrow_mut().as_mut() {
            callback(phase, p.deadline);
        }
    });
}
fn inputs(p: &Project, text: &str, stream: &str) -> Result<Vec<FilterRecord>> {
    p.check_deadline()?;
    let mut records = Vec::new();
    let mut start = 0;
    let mut private_key = false;
    for raw in text.split_inclusive(['\n', '\r']) {
        #[cfg(test)]
        phase_observation(p, FilterPhase::InputAdmitted);
        p.check_deadline()?;
        #[cfg(test)]
        phase_observation(p, FilterPhase::InputWork);
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
    p.check_deadline()?;
    Ok(records)
}
fn deadline(p: &Project, start: Instant) -> Result<()> {
    p.check_deadline()?;
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
    p.check_deadline()?;
    let started = Instant::now();
    let d = &loaded.definition;
    let mut input_bytes = 0usize;
    for r in input {
        deadline(p, started)?;
        input_bytes = input_bytes.saturating_add(r.text.len());
    }
    if input.len() > MAX_RECORDS || input_bytes > MAX_INPUT {
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
    let mut protects = Vec::new();
    for rule in &d.rules {
        deadline(p, started)?;
        if let Rule::Protect { pattern } = rule {
            protects.push(regex(pattern)?);
        }
    }
    let mut records = Vec::new();
    let mut omitted = 0usize;
    let mut private_key = false;
    for record in input {
        #[cfg(test)]
        phase_observation(p, FilterPhase::RecordAdmitted);
        deadline(p, started)?;
        #[cfg(test)]
        phase_observation(p, FilterPhase::RecordWork);
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
    let mut protected_input = 0;
    for r in &records {
        deadline(p, started)?;
        if r.protected {
            protected_input += 1;
        }
    }
    let mut trace = Vec::new();
    for rule in &d.rules {
        deadline(p, started)?;
        let before = records.len();
        match rule {
            Rule::Protect { .. } => {}
            Rule::Drop { pattern } => {
                let pattern = regex(pattern)?;
                let mut retained = Vec::new();
                for r in records {
                    deadline(p, started)?;
                    if r.protected
                        || (d.matcher.stream != "both" && d.matcher.stream != r.stream)
                        || !pattern.is_match(&r.text)
                    {
                        retained.push(r);
                    }
                }
                records = retained;
            }
            Rule::SelectFields { fields } => {
                for record in &mut records {
                    deadline(p, started)?;
                    if !record.protected
                        && (d.matcher.stream == "both" || d.matcher.stream == record.stream)
                        && let Some(v) = &record.fields
                    {
                        let mut selected = serde_json::Map::new();
                        for field in fields {
                            deadline(p, started)?;
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
                    deadline(p, started)?;
                    if let Some(v) = &record.fields {
                        let mut parts = Vec::new();
                        for f in fields {
                            deadline(p, started)?;
                            parts.push(format!("{f}={}", v.get(f).unwrap_or(&Value::Null)));
                        }
                        record.group = Some(parts.join(";"));
                    }
                }
            }
            Rule::DeduplicateExact { .. } => {
                let mut seen = BTreeMap::<(String, String, bool), usize>::new();
                let mut grouped: Vec<ViewRecord> = Vec::new();
                for record in records {
                    deadline(p, started)?;
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
                let mut retained = Vec::new();
                for (i, r) in records.into_iter().enumerate() {
                    deadline(p, started)?;
                    if r.protected
                        || i < *keep_head_lines
                        || i >= len.saturating_sub(*keep_tail_lines)
                    {
                        retained.push(r);
                    }
                }
                records = retained;
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
    let mut retained = Vec::new();
    for (i, r) in records.into_iter().enumerate() {
        deadline(p, started)?;
        if r.protected || i < head || i >= len.saturating_sub(tail) {
            retained.push(r);
        }
    }
    records = retained;
    let mut kept = Vec::new();
    let mut used = 0;
    let budget = d.render.max_bytes.saturating_sub(1600 + trace.len() * 100);
    let mut protected_omitted = 0;
    records.sort_by_key(|r| (!r.protected, r.sequence));
    for record in records {
        deadline(p, started)?;
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
    let mut preserved = 0;
    let mut parse_partial = false;
    for r in &kept {
        deadline(p, started)?;
        preserved += r.count;
        parse_partial |= r.fields.is_none();
    }
    let mut value = json!({"filter_id":d.id,"filter_version":d.version,"filter_hash":loaded.digest,"policy_hash":p.policy_hash(),"capability":"presentation_only","evidence_origin":"fixture_claim","child_exit_code":child_exit,"termination":termination,"execution_started":false,"command_rerun":false,"parse_status":if d.parse.kind=="json"&&parse_partial{"partial"}else{"complete"},"records":kept,"input_records":input.len(),"preserved_occurrences":preserved,"omitted_occurrences":input.len().saturating_sub(preserved).max(omitted),"protected_occurrences":protected_input,"protected_omitted":protected_omitted,"source_ref":"explicit_preview_input","trace":trace,"test_result":"not_evaluated","task_completion":"not_evaluated"});
    loop {
        deadline(p, started)?;
        let size = serde_json::to_vec(&value)?.len();
        deadline(p, started)?;
        if size <= d.render.max_bytes {
            break;
        }
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
    deadline(p, started)?;
    Ok(value)
}
fn explicit_input(p: &Project, path: &Path) -> Result<String> {
    p.check_deadline()?;
    if path == Path::new("-") {
        let original = p.deadline.ok_or_else(|| {
            err(
                "INVALID_ARGUMENT",
                "Finite preview input requires a request deadline",
                2,
            )
        })?;
        let bytes = crate::input::read_stdin(original).map_err(|e| {
            if e.code == "FILE_TOO_LARGE" {
                err("FILTER_LIMIT_EXCEEDED", "Preview stdin exceeds 1MiB", 3)
            } else {
                e
            }
        })?;
        p.check_deadline()?;
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
    p.check_deadline()?;
    Ok(f.text)
}
fn private_store(p: &Project, area: &str) -> Result<PathBuf> {
    p.check_deadline()?;
    let dir = p.data_dir.join(area);
    for ancestor in dir.ancestors() {
        p.check_deadline()?;
        match fs::symlink_metadata(ancestor) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(err("POLICY_DENIED", "Linked user filter store denied", 5));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    p.check_deadline()?;
    Ok(dir)
}
fn private_location(p: &Project, area: &str, id: &str) -> Result<PathBuf> {
    if !safe_id(id) {
        return Err(err("FILTER_INVALID", "Invalid filter identifier", 2));
    }
    Ok(private_store(p, area)?.join(format!("{id}.json")))
}
fn private_path(p: &Project, area: &str, id: &str) -> Result<PathBuf> {
    let path = private_location(p, area, id)?;
    private_dir(path.parent().unwrap())?;
    p.check_deadline()?;
    Ok(path)
}
fn private_read(p: &Project, area: &str, id: &str) -> Result<Vec<u8>> {
    let path = private_location(p, area, id)?;
    match fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(err(
                "FILTER_METADATA_MISSING",
                "Filter metadata unavailable",
                6,
            ));
        }
        Err(e) => return Err(e.into()),
    }
    p.check_deadline()?;
    if p.deadline.is_none() {
        return Ok(fs::read(path)?);
    }
    use std::io::Read;
    let parent = path.parent().unwrap();
    let leaf = path.file_name().unwrap().to_str().unwrap();
    let anchor = crate::project::RootAnchor::capture(parent)?;
    let mut file = reader::anchored_open_deadline(parent, &anchor, leaf, p.deadline)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() > MAX_INPUT as u64 {
        return Err(err(
            "FILTER_LIMIT_EXCEEDED",
            "Filter metadata must be a bounded regular file",
            3,
        ));
    }
    let identity = same_file::Handle::from_file(file.try_clone()?)?;
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        p.check_deadline()?;
        let count = file.read(&mut chunk);
        p.check_deadline()?;
        let count = count?;
        if count == 0 {
            break;
        }
        if bytes.len() + count > MAX_INPUT {
            return Err(err(
                "FILTER_LIMIT_EXCEEDED",
                "Filter metadata exceeds 1MiB",
                3,
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    let after = file.metadata()?;
    let reopened = reader::anchored_open_deadline(parent, &anchor, leaf, p.deadline)?;
    let current = reopened.metadata()?;
    if identity != same_file::Handle::from_file(reopened)?
        || before.len() != after.len()
        || before.len() != current.len()
        || before.modified().ok() != after.modified().ok()
        || before.modified().ok() != current.modified().ok()
    {
        return Err(err(
            "CONCURRENT_MODIFICATION",
            "Filter metadata changed during read",
            4,
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (before.ctime(), before.ctime_nsec()) != (after.ctime(), after.ctime_nsec())
            || (before.ctime(), before.ctime_nsec()) != (current.ctime(), current.ctime_nsec())
        {
            return Err(err(
                "CONCURRENT_MODIFICATION",
                "Filter metadata changed during read",
                4,
            ));
        }
    }
    p.check_deadline()?;
    Ok(bytes)
}
fn executable(p: &Project, program: &str) -> Result<(String, String)> {
    p.check_deadline()?;
    let path = if Path::new(program).is_absolute() {
        PathBuf::from(program)
    } else if program.contains('/') || program.contains('\\') {
        return Err(err(
            "FILTER_INVALID",
            "Executable must use PATH name or absolute path",
            2,
        ));
    } else {
        let mut resolved = None;
        for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
            p.check_deadline()?;
            let candidate = dir.join(program);
            match fs::metadata(&candidate) {
                Ok(m) if m.is_file() => {
                    resolved = Some(candidate);
                    break;
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            p.check_deadline()?;
        }
        resolved.ok_or_else(|| {
            err(
                "CAPABILITY_UNAVAILABLE",
                "Filter executable not installed",
                6,
            )
        })?
    };
    p.check_deadline()?;
    let path = fs::canonicalize(path)?;
    p.check_deadline()?;
    let metadata = fs::metadata(&path)?;
    p.check_deadline()?;
    if !metadata.is_file() || metadata.len() > 128 * 1024 * 1024 {
        return Err(err(
            "FILTER_LIMIT_EXCEEDED",
            "Executable fingerprint exceeds bounds",
            3,
        ));
    }
    p.check_deadline()?;
    let digest = crate::output::hash_executable_with_limit_error(p, &path, |_| {
        err(
            "FILTER_LIMIT_EXCEEDED",
            "Executable fingerprint exceeds bounds",
            3,
        )
    })?;
    p.check_deadline()?;
    Ok((path.to_string_lossy().into_owned(), digest))
}
fn prefix_hashes(p: &Project, loaded: &Loaded) -> Result<BTreeMap<String, String>> {
    let mut scripts = BTreeMap::new();
    for token in &loaded.definition.matcher.argv_prefix {
        p.check_deadline()?;
        if !token.starts_with('-') && p.root.join(token).is_file() {
            let path = relative(p, Path::new(token))?;
            let f = reader::read(p, &path)?;
            scripts.insert(path, f.hash);
        }
    }
    Ok(scripts)
}
fn checked_binding(p: &Project, loaded: &Loaded) -> Result<Binding> {
    p.check_deadline()?;
    let binding: Binding = serde_json::from_slice(
        &private_read(p, "filter-bindings", &loaded.definition.id).map_err(|e| {
            if e.code == "FILTER_METADATA_MISSING" {
                err("POLICY_DENIED", "Filter is not activated", 5)
            } else {
                e
            }
        })?,
    )?;
    p.check_deadline()?;
    let current = executable(p, &loaded.definition.matcher.program)?;
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
    let report_bytes =
        private_read(p, "filter-fixture-reports", &loaded.definition.id).map_err(|e| {
            if e.code == "FILTER_METADATA_MISSING" {
                err("CONFIG_CHANGED", "Activated fixture report unavailable", 9)
            } else {
                e
            }
        })?;
    p.check_deadline()?;
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
        p.check_deadline()?;
        if reader::read(p, path)?.hash != *digest {
            return Err(err("CONFIG_CHANGED", "Activated fixture input changed", 9));
        }
    }
    p.check_deadline()?;
    Ok(binding)
}
/// Explicit preview is permitted before activation and cannot execute the matched program.
fn query_scope(p: &Project) -> Result<Project> {
    let mut scope = p.clone();
    if scope.deadline.is_none() {
        scope.deadline = Some(crate::deadline::Deadline::from_millis(10000)?);
    }
    scope.check_deadline()?;
    Ok(scope)
}
pub fn preview_records(
    p: &Project,
    filter_id: &str,
    records: &[FilterRecord],
    child_exit: i32,
    termination: &str,
) -> Result<Value> {
    let scope = query_scope(p)?;
    let result = preview_records_inner(&scope, filter_id, records, child_exit, termination);
    scope.check_deadline()?;
    result
}
fn preview_records_inner(
    p: &Project,
    filter_id: &str,
    records: &[FilterRecord],
    child_exit: i32,
    termination: &str,
) -> Result<Value> {
    p.check_deadline()?;
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
    p.check_deadline()?;
    if !["exited", "timed_out", "signaled", "cancelled"].contains(&termination) {
        return Err(err("FILTER_INVALID", "Unknown observed termination", 2));
    }
    let loaded = load(p, filter_id)?;
    checked_binding(p, &loaded)?;
    let mut result = render(p, &loaded, records, child_exit.unwrap_or(128), termination)?;
    result["child_exit_code"] = json!(child_exit);
    result["evidence_origin"] = json!("stored_observed_output");
    p.check_deadline()?;
    Ok(result)
}
pub fn execute(p: &Project, command: &FilterCommand) -> Result<Value> {
    if matches!(
        command,
        FilterCommand::Validate { .. }
            | FilterCommand::Apply { .. }
            | FilterCommand::Explain { .. }
    ) {
        let scope = query_scope(p)?;
        let result = execute_inner(&scope, command);
        scope.check_deadline()?;
        result
    } else {
        execute_inner(p, command)
    }
}
fn execute_inner(p: &Project, command: &FilterCommand) -> Result<Value> {
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
            let records = inputs(p, &text, "stdout")?;
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
    let current = executable(p, &suite.executable)?;
    let expected = executable(p, &loaded.definition.matcher.program)?;
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
        p.check_deadline()?;
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
        let mut records = inputs(p, &raw.text, "stdout")?;
        let mut stderr = String::new();
        if let Some(path) = &case.stderr {
            let path = relative(p, &fixtures.join(path))?;
            let f = reader::read(p, &path)?;
            input_hashes.insert(path, f.hash);
            stderr = f.text;
            records.extend(inputs(p, &stderr, "stderr")?);
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
    p.check_deadline()?;
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
    let bytes = private_read(p, "filter-fixture-reports", id).map_err(|e| {
        if e.code == "FILTER_METADATA_MISSING" {
            err(
                "POLICY_DENIED",
                "A passing complete fixture report is required",
                5,
            )
        } else {
            e
        }
    })?;
    p.check_deadline()?;
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
        p.check_deadline()?;
        if reader::read(p, path)?.hash != *digest {
            return Err(err("CONFIG_CHANGED", "Fixture input changed", 9));
        }
    }
    let current = executable(p, &loaded.definition.matcher.program)?;
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
    p.check_deadline()?;
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
    let actual = executable(p, &argv[0])?;
    p.check_deadline()?;
    let dir = private_store(p, "filter-bindings")?;
    let mut matches = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => Some(entries),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    p.check_deadline()?;
    if let Some(entries) = entries {
        for entry in entries {
            p.check_deadline()?;
            let entry = entry?;
            if entry.file_type()?.is_symlink() {
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
    p.check_deadline()?;
    matches.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    p.check_deadline()?;
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

#[cfg(test)]
mod deadline_tests {
    use super::*;
    use crate::project::{Config, ProjectConfig, RootAnchor};
    use std::{cell::RefCell, rc::Rc};
    fn fixture() -> (tempfile::TempDir, Project) {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("project");
        fs::create_dir_all(root.join(".pctx/filters")).unwrap();
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
        fs::write(
            p.root.join(".pctx/filters/fixture.toml"),
            r#"schema_version=1
id="fixture"
version="1"
priority=0
rules=[]
[match]
program="fixture"
argv_prefix=[]
stream="both"
[parse]
kind="lines"
[render]
max_bytes=8192
keep_head_lines=10
keep_tail_lines=10
show_omission_counts=true
"#,
        )
        .unwrap();
        (temp, p)
    }
    fn records() -> Vec<FilterRecord> {
        (0..1000)
            .map(|i| FilterRecord {
                stream: "stdout".into(),
                sequence: i,
                text: format!("warning record {i}"),
                start_byte: i as usize * 20,
                end_byte: i as usize * 20 + 19,
            })
            .collect()
    }
    #[test]
    fn preview_default_scope_is_one_budget_over_all_actual_records() {
        let (_temp, p) = fixture();
        let records = records();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let observed = seen.clone();
        let start = Instant::now();
        {
            let _guard = observe_phase(move |phase, deadline| {
                if phase == FilterPhase::RecordWork {
                    observed.borrow_mut().push(deadline.unwrap().instant());
                }
            });
            let value = preview_records(&p, "fixture", &records, 17, "exited").unwrap();
            assert_eq!(value["input_records"], 1000);
            assert_eq!(value["child_exit_code"], 17);
            assert_eq!(value["test_result"], "not_evaluated");
        }
        let seen = seen.borrow();
        assert_eq!(seen.len(), 1000);
        assert!(seen.iter().all(|end| *end == seen[0]));
        assert!(seen[0] >= start + Duration::from_secs(9));
        assert!(p.deadline.is_none());
        assert!(!p.data_dir.exists());
    }
    #[cfg(unix)]
    #[test]
    fn explain_preserves_admitted_executable_symlink_denial_and_size_limit() {
        let (_temp, p) = fixture();
        let executable = p.root.join("fingerprint-fixture");
        fs::write(&executable, vec![42u8; 65536 + 7]).unwrap();
        let command = FilterCommand::Explain {
            argv: vec![executable.to_str().unwrap().into()],
        };
        let positive = execute(&p, &command).unwrap();
        assert!(positive["selected"].is_null());
        assert_eq!(positive["execution_started"], false);
        assert!(!p.data_dir.exists());
        let alternate = p.root.join("alternate-fixture");
        fs::write(&alternate, b"alternate executable").unwrap();
        let named = executable.clone();
        let target = alternate.clone();
        let mut replaced = false;
        {
            let _guard = crate::output::observe_hash(move |phase, count, deadline| {
                if matches!(phase, crate::output::HashPhase::ChunkRead) && !replaced {
                    assert!(count > 0);
                    deadline.unwrap().check().unwrap();
                    fs::remove_file(&named).unwrap();
                    std::os::unix::fs::symlink(&target, &named).unwrap();
                    replaced = true;
                }
            });
            let error = execute(&p, &command).unwrap_err();
            assert_eq!(error.code, "POLICY_DENIED");
            assert_eq!(error.exit, 5);
        }
        assert!(
            fs::symlink_metadata(&executable)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&alternate).unwrap(), b"alternate executable");
        fs::remove_file(&executable).unwrap();
        fs::File::create(&executable)
            .unwrap()
            .set_len(128 * 1024 * 1024 + 1)
            .unwrap();
        let error = execute(&p, &command).unwrap_err();
        assert_eq!(error.code, "FILTER_LIMIT_EXCEEDED");
        assert_eq!(error.exit, 3);
        assert!(!p.data_dir.exists());
        assert!(p.deadline.is_none());
    }
    #[test]
    fn original_budget_expires_during_actual_preview_record_admission() {
        let (_temp, mut p) = fixture();
        let records = records();
        let positive = preview_records(&p, "fixture", &records, 19, "exited").unwrap();
        assert_eq!(positive["input_records"], 1000);
        assert_eq!(positive["child_exit_code"], 19);
        let original = crate::deadline::Deadline::from_millis(250).unwrap();
        p.deadline = Some(original);
        let trace = Rc::new(RefCell::new((0usize, 0usize)));
        let observed = trace.clone();
        {
            let _guard = observe_phase(move |phase, deadline| {
                assert_eq!(deadline.unwrap().instant(), original.instant());
                let mut trace = observed.borrow_mut();
                match phase {
                    FilterPhase::RecordAdmitted => {
                        trace.0 += 1;
                        if trace.0 == 1 {
                            while let Ok(left) = original.remaining() {
                                std::thread::sleep(left.min(Duration::from_millis(5)));
                            }
                        }
                    }
                    FilterPhase::RecordWork => trace.1 += 1,
                    _ => {}
                }
            });
            let e = preview_records(&p, "fixture", &records, 19, "exited").unwrap_err();
            assert_eq!(e.code, "TIMEOUT");
            assert_eq!(e.exit, 7);
        }
        assert_eq!(*trace.borrow(), (1, 0));
        assert_eq!(p.deadline.unwrap().instant(), original.instant());
        assert!(!p.data_dir.exists());
    }
    #[test]
    fn original_budget_expires_during_actual_loaded_input_record_admission() {
        let (_temp, mut p) = fixture();
        let path = p.root.join("preview.txt");
        let body = "warning input\n".repeat(1000);
        fs::write(&path, &body).unwrap();
        let before = fs::read(&path).unwrap();
        let command = FilterCommand::Apply {
            filter: "fixture".into(),
            input: PathBuf::from("preview.txt"),
            child_exit: 23,
        };
        let positive = execute(&p, &command).unwrap();
        assert_eq!(positive["input_records"], 1000);
        assert_eq!(positive["child_exit_code"], 23);
        let original = crate::deadline::Deadline::from_millis(250).unwrap();
        p.deadline = Some(original);
        let trace = Rc::new(RefCell::new((0usize, 0usize)));
        let observed = trace.clone();
        {
            let _guard = observe_phase(move |phase, deadline| {
                assert_eq!(deadline.unwrap().instant(), original.instant());
                let mut trace = observed.borrow_mut();
                match phase {
                    FilterPhase::InputAdmitted => {
                        trace.0 += 1;
                        if trace.0 == 1 {
                            while let Ok(left) = original.remaining() {
                                std::thread::sleep(left.min(Duration::from_millis(5)));
                            }
                        }
                    }
                    FilterPhase::InputWork => trace.1 += 1,
                    _ => {}
                }
            });
            let e = execute(&p, &command).unwrap_err();
            assert_eq!(e.code, "TIMEOUT");
            assert_eq!(e.exit, 7);
        }
        assert_eq!(*trace.borrow(), (1, 0));
        assert_eq!(p.deadline.unwrap().instant(), original.instant());
        assert_eq!(fs::read(path).unwrap(), before);
        assert!(!p.data_dir.exists());
    }
}

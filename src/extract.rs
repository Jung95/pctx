use crate::{
    deadline::Deadline,
    domain::{Error, Result},
    project::Project,
    reader, storage,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Debug, clap::Args)]
pub struct ExtractRequest {
    #[arg(long)]
    pub from_output: Option<String>,
    #[arg(long)]
    pub location: Vec<String>,
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long)]
    pub line: Option<usize>,
    #[arg(long)]
    pub symbol_id: Vec<String>,
    #[arg(long,default_value="enclosing",value_parser=["enclosing","lines"])]
    pub unit: String,
    #[arg(long,default_value="full_span",value_parser=["full_span","signature","reference"])]
    pub view: String,
    #[arg(long, default_value_t = 10000)]
    pub budget_bytes: usize,
}
pub fn extract(p: &Project, r: &ExtractRequest) -> Result<Value> {
    let mut scoped = p.clone();
    if scoped.deadline.is_none() {
        scoped.deadline = Some(Deadline::from_millis(10_000)?);
    }
    let p = &scoped;
    p.check_deadline()?;
    if r.budget_bytes < 512 {
        return Err(Error::new(
            "BUDGET_TOO_SMALL",
            "Budget below minimum response size",
            8,
        ));
    }
    storage::update(p)?;
    let (_, files) = storage::snapshot(p)?;
    let mut locations = Vec::new();
    let diagnostics = if let Some(id) = &r.from_output {
        let result = crate::output::diagnostic_locations(p, id)?;
        for location in result["locations"].as_array().into_iter().flatten() {
            p.check_deadline()?;
            locations.push((
                location["path"].as_str().unwrap().to_string(),
                location["line"].as_u64().unwrap() as usize,
            ));
        }
        Some(result)
    } else {
        None
    };
    for value in &r.location {
        p.check_deadline()?;
        let (path, line) = value
            .rsplit_once(':')
            .ok_or_else(|| Error::new("INVALID_ARGUMENT", "Location requires path:line", 2))?;
        let line = line
            .parse::<usize>()
            .map_err(|_| Error::new("INVALID_ARGUMENT", "Invalid location line", 2))?;
        locations.push((path.to_owned(), line));
    }
    if let (Some(path), Some(line)) = (&r.path, r.line) {
        locations.push((path.clone(), line));
    } else if r.path.is_some() || r.line.is_some() {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "--path and --line must be used together",
            2,
        ));
    }
    for key in &r.symbol_id {
        p.check_deadline()?;
        let mut selected = None;
        for file in &files {
            p.check_deadline()?;
            for symbol in &file.symbols {
                p.check_deadline()?;
                if &symbol.id == key {
                    selected = Some(symbol);
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        let sym = selected
            .ok_or_else(|| Error::new("STALE_INDEX", "Exact symbol version is unavailable", 4))?;
        locations.push((sym.path.clone(), sym.start_line));
    }
    if locations.is_empty() {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Provide a location or symbol ID",
            2,
        ));
    }
    let mut ranges: BTreeMap<String, Vec<(usize, usize, Option<String>)>> = BTreeMap::new();
    for (path, line) in locations {
        p.check_deadline()?;
        let file = reader::read(p, &path)?;
        let mut line_count = 0;
        for _ in file.text.lines() {
            p.check_deadline()?;
            line_count += 1;
        }
        if line == 0 || line > line_count {
            return Err(Error::new(
                "INVALID_ARGUMENT",
                "Location is outside source lines",
                2,
            ));
        }
        let mut symbol: Option<&crate::domain::Symbol> = None;
        if r.unit == "enclosing" {
            for indexed in &files {
                p.check_deadline()?;
                if indexed.path != path {
                    continue;
                }
                for candidate in &indexed.symbols {
                    p.check_deadline()?;
                    if candidate.start_line <= line
                        && candidate.end_line >= line
                        && symbol.is_none_or(|old: &crate::domain::Symbol| {
                            candidate.end_byte - candidate.start_byte
                                < old.end_byte - old.start_byte
                        })
                    {
                        symbol = Some(candidate);
                    }
                }
                break;
            }
        }
        let range = if let Some(s) = symbol {
            (s.start_line, s.end_line, Some(s.id.clone()))
        } else {
            (
                line.saturating_sub(3).max(1),
                (line + 3).min(line_count),
                None,
            )
        };
        ranges.entry(path).or_default().push(range);
    }
    let mut items = Vec::new();
    let mut manifests = Vec::new();
    for (path, mut spans) in ranges {
        p.check_deadline()?;
        spans.sort_by_key(|s| (s.0, s.1));
        p.check_deadline()?;
        let mut merged: Vec<(usize, usize, Option<String>)> = Vec::new();
        for span in spans {
            p.check_deadline()?;
            if let Some(last) = merged.last_mut()
                && last.1 >= span.0
            {
                last.1 = last.1.max(span.1);
                if last.2 != span.2 {
                    last.2 = None;
                }
                continue;
            }
            merged.push(span);
        }
        let file = reader::read(p, &path)?;
        let mut offsets = Vec::new();
        for (i, _) in file.text.match_indices('\n') {
            p.check_deadline()?;
            offsets.push(i + 1);
        }
        offsets.insert(0, 0);
        for (start, end, sym) in merged {
            p.check_deadline()?;
            let a = offsets[start - 1];
            let b = offsets.get(end).copied().unwrap_or(file.text.len());
            let (text, redacted) = reader::redact_span(&file.text, a, b);
            p.check_deadline()?;
            let mut content = Some(text);
            let representation = r.view.as_str();
            if representation == "reference" {
                content = None;
            } else if representation == "signature" {
                content = Some(content.unwrap().lines().next().unwrap_or("").into());
            }
            items.push(json!({"path":path,"symbol_id":sym,"range":{"start_byte":a,"end_byte":b,"start_line":start,"end_line":end},"file_hash":file.hash,"representation":representation,"body_omitted":representation!="full_span","content":content,"redacted":redacted,"completeness":"complete","fallback":if sym.is_none(){Some("line_window")}else{None},"evidence_status":"observed","freshness":"current","reason":"explicit_location","parser_set":storage::PARSER_SET}));
        }
        manifests.push((path, file.hash));
    }
    p.check_deadline()?;
    let mut data =
        json!({"items":items,"omissions":[],"budget":{"limit":r.budget_bytes,"unit":"bytes"}});
    let measured =
        serde_json::to_vec(&crate::domain::envelope("extract", Some(p), data.clone()))?.len() + 1;
    p.check_deadline()?;
    if measured > r.budget_bytes {
        for item in data["items"].as_array_mut().unwrap() {
            p.check_deadline()?;
            if item["representation"] == "full_span" {
                item["content"] = json!(
                    item["content"]
                        .as_str()
                        .unwrap_or("")
                        .lines()
                        .next()
                        .unwrap_or("")
                );
                item["representation"] = json!("signature");
                item["body_omitted"] = json!(true);
                item["reason"] = json!("budget_downgrade");
            }
        }
    }
    let measured =
        serde_json::to_vec(&crate::domain::envelope("extract", Some(p), data.clone()))?.len() + 1;
    p.check_deadline()?;
    if measured > r.budget_bytes {
        return Err(Error::new(
            "BUDGET_TOO_SMALL",
            "Requested extract metadata exceeds budget",
            8,
        ));
    }
    for (path, expected) in manifests {
        p.check_deadline()?;
        if reader::read(p, &path)?.hash != expected {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Extract source changed before output",
                4,
            ));
        }
    }
    data["diagnostic_sources"] = json!(diagnostics);
    let measured =
        serde_json::to_vec(&crate::domain::envelope("extract", Some(p), data.clone()))?.len() + 1;
    p.check_deadline()?;
    if measured > r.budget_bytes {
        return Err(Error::new(
            "BUDGET_TOO_SMALL",
            "Diagnostic provenance exceeds requested budget",
            8,
        ));
    }
    p.check_deadline()?;
    reader::validate_root(p)?;
    Ok(data)
}

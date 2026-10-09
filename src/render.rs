//! Pure final-document rendering. Budgets are enforced by the caller on returned bytes.
//! Source strings are already masked by producers; this module never reads or executes them.
use crate::domain::{Error, Result};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Compact,
    Json,
    Markdown,
}

/// The Markdown contract is deliberately limited to these exact CLI command names.
/// Call before executing the command so an unsupported format cannot cause side effects.
pub fn supports(command: &str) -> bool {
    matches!(
        command,
        "build"
            | "outline"
            | "read"
            | "handoff"
            | "handoff create"
            | "handoff update"
            | "handoff show"
    )
}

/// Reduce an oversized presentation while preserving independently observed execution
/// and typed refusal truth. The caller measures final bytes and chooses this path.
/// This function neither executes children nor establishes budget admission.
pub fn budget_fallback(
    command: &str,
    original: &Value,
    execution_pointer: &str,
    mut exit: i32,
) -> (Value, i32) {
    let e = Error::new("BUDGET_TOO_SMALL", "Output exceeds byte budget", 8);
    // Keep the durable reread handle and original execution outcome even when
    // presentation cannot fit. The wrapper failure never replaces child truth.
    let mut proof = serde_json::Map::new();
    if let Some(execution) = original.pointer(execution_pointer)
        && (execution["spawned"].is_boolean() || execution["output_id"].is_string())
    {
        for key in [
            "output_id",
            "query_ref",
            "spawned",
            "child_exit_code",
            "signal",
            "termination",
            "pctx_error",
            "capture_complete",
            "raw_available",
            "task_completion",
            "test_result",
            "execution_status",
            "delivery_kind",
        ] {
            if let Some(value) = execution.get(key) {
                proof.insert(key.into(), value.clone());
            }
        }
    }
    let previous_errors = original["errors"].clone();
    let preserve_refusal = original["status"] == "error"
        && previous_errors
            .as_array()
            .is_some_and(|errors| !errors.is_empty());
    let mut response = crate::domain::envelope(command, None, Value::Object(proof));
    for key in ["project_id", "workspace_id", "generation_id", "validation"] {
        if let Some(value) = original.get(key) {
            response[key] = value.clone();
        }
    }
    if let Some(truncation) = original.get("truncation").filter(|value| value.is_object()) {
        response["truncation"] = truncation.clone();
    }
    response["truncation"]["truncated"] = json!(true);
    if !response["truncation"]["reasons"].is_array() {
        response["truncation"]["reasons"] = json!([]);
    }
    let reasons = response["truncation"]["reasons"].as_array_mut().unwrap();
    if !reasons.contains(&json!("presentation_budget")) {
        reasons.push(json!("presentation_budget"));
    }
    if let Some(warnings) = original["warnings"]
        .as_array()
        .filter(|warnings| !warnings.is_empty())
    {
        // This is an exact count of withheld warning records, not an estimate
        // of source results or candidates that were never counted.
        response["truncation"]["warnings_omitted"] = json!(warnings.len());
    }
    response["status"] = json!("error");
    response["coverage"] = json!({"status":"partial","reasons":["presentation_budget"]});
    if preserve_refusal {
        response["errors"] = previous_errors;
        response["errors"][0]["message"] = json!("Request refused");
    } else {
        response["errors"] = json!([e]);
        exit = 8;
    }
    (response, exit)
}

/// Returns the entire UTF-8 document, including its final newline. Never truncates data.
pub fn render(envelope: &Value, format: Format) -> Result<Vec<u8>> {
    if matches!(format, Format::Compact | Format::Json) {
        return Ok(json_document(envelope));
    }
    let mut document = match format {
        Format::Compact => json_text(envelope, false)?,
        Format::Json => json_text(envelope, false)?,
        Format::Markdown => markdown(envelope)?,
    };
    if !document.ends_with('\n') {
        document.push('\n');
    }
    Ok(document.into_bytes())
}

// JSON C0 escapes come from serde_json. Escape C1, bidi formatting and line separators
// too: they are valid JSON characters, but should not control a terminal or review UI.
// These JSON escapes round-trip to the identical original Value.
fn hidden(c: char) -> bool {
    matches!(c, '\u{7f}'..='\u{9f}' | '\u{200e}' | '\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}
fn json_text(value: &Value, pretty: bool) -> Result<String> {
    let serialized = if pretty {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    };
    Ok(escape_json_controls(&serialized))
}

/// A serde_json Value is already a valid JSON tree. Its infallible Display
/// serialization lets error delivery share the same reversible escaping.
pub fn json_document(value: &Value) -> Vec<u8> {
    let mut text = escape_json_controls(&value.to_string());
    text.push('\n');
    text.into_bytes()
}
fn escape_json_controls(serialized: &str) -> String {
    let mut text = String::with_capacity(serialized.len());
    for c in serialized.chars() {
        if hidden(c) {
            text.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            text.push(c);
        }
    }
    text
}
/// Escape terminal controls in literal source and plain CLI diagnostics.
pub fn diagnostic_text(source: &str) -> String {
    let mut text = String::with_capacity(source.len());
    for c in source.chars() {
        if (c.is_control() && c != '\n' && c != '\t') || hidden(c) {
            text.push_str(&format!("\\u{{{:04x}}}", c as u32));
        } else {
            text.push(c);
        }
    }
    text
}
fn longest_run(text: &str, marker: char) -> usize {
    let mut run = 0;
    let mut longest = 0;
    for c in text.chars() {
        if c == marker {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}
fn fence(out: &mut String, text: &str, language: &str) {
    // Fixed info strings only; source cannot choose a language or close the fence.
    let length = 3.max(
        longest_run(text, '`')
            .max(longest_run(text, '~'))
            .saturating_add(1),
    );
    let delimiter = "`".repeat(length);
    out.push_str(&delimiter);
    out.push_str(language);
    out.push('\n');
    out.push_str(text);
    if !text.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&delimiter);
    out.push_str("\n\n");
}
fn inline(value: &Value) -> Result<String> {
    let text = json_text(value, false)?;
    let delimiter = "`".repeat(longest_run(&text, '`').saturating_add(1));
    // Padding protects literal leading/trailing backticks in CommonMark code spans.
    Ok(format!("{delimiter} {text} {delimiter}"))
}
fn field(out: &mut String, label: &str, value: Option<&Value>) -> Result<()> {
    if let Some(value) = value {
        out.push_str("- ");
        out.push_str(label);
        out.push_str(": ");
        out.push_str(&inline(value)?);
        out.push('\n');
    }
    Ok(())
}
fn json_section(out: &mut String, title: &str, value: &Value) -> Result<()> {
    out.push_str("## ");
    out.push_str(title);
    out.push_str("\n\n");
    fence(out, &json_text(value, true)?, "json");
    Ok(())
}
fn indicators(value: &Value, partial: &mut bool, failure: &mut bool) {
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                if matches!(key.as_str(), "status" | "completeness" | "coverage")
                    && child.as_str() == Some("partial")
                    || matches!(
                        key.as_str(),
                        "truncated" | "body_omitted" | "returned_excerpt"
                    ) && child == &Value::Bool(true)
                    || matches!(
                        key.as_str(),
                        "selection_complete" | "required_minimum_complete" | "capture_complete"
                    ) && child == &Value::Bool(false)
                {
                    *partial = true;
                }
                if matches!(key.as_str(), "status" | "result" | "state")
                    && child
                        .as_str()
                        .is_some_and(|s| matches!(s, "error" | "failed" | "timed_out"))
                    || matches!(key.as_str(), "child_exit_code" | "exit_code")
                        && child.as_i64().is_some_and(|n| n != 0)
                    || key == "pctx_error" && !child.is_null()
                {
                    *failure = true;
                }
                indicators(child, partial, failure);
            }
        }
        Value::Array(values) => {
            for child in values {
                indicators(child, partial, failure);
            }
        }
        _ => {}
    }
}
fn provenance(out: &mut String, item: &Value) -> Result<()> {
    for (key, label) in [
        ("path", "Source path"),
        ("file_hash", "Source SHA-256"),
        ("hash", "Hash (reported)"),
        ("range", "Source range (reported)"),
        ("symbol_id", "Symbol ID"),
        ("representation", "Representation"),
        ("reason", "Selection reason"),
        ("freshness", "Freshness"),
        ("evidence_status", "Evidence status"),
        ("evidence_origin", "Evidence origin"),
        ("source", "Source (reported)"),
        ("parse_status", "Parser status"),
        ("language", "Language"),
        ("completeness", "Completeness"),
        ("coverage", "Coverage"),
        ("truncated", "Truncated"),
        ("body_omitted", "Body omitted"),
        ("details_omitted", "Details omitted"),
        ("omitted_count", "Omitted count"),
        ("omission_reasons", "Omission reasons"),
        ("redacted", "Redaction applied"),
        ("next_read", "Follow-up read reference"),
        ("query_ref", "Follow-up query reference"),
        ("checkpoint_id", "Checkpoint ID"),
        ("start_byte", "Start byte (zero based)"),
        ("end_byte", "End byte (exclusive)"),
        ("start_line", "Start line (one based)"),
        ("end_line", "End line (inclusive)"),
    ] {
        field(out, label, item.get(key))?;
    }
    out.push('\n');
    Ok(())
}
fn source_section(out: &mut String, item: &Value) -> Result<()> {
    provenance(out, item)?;
    for key in ["content", "text"] {
        if let Some(text) = item.get(key).and_then(Value::as_str) {
            out.push_str("Source/content (literal, already masked):\n\n");
            fence(out, &diagnostic_text(text), "text");
        }
    }
    if let Some(signatures) = item.get("signatures").and_then(Value::as_array) {
        for signature in signatures {
            out.push_str("Verified signature:\n\n");
            provenance(out, signature)?;
            if let Some(content) = signature.get("content").and_then(Value::as_str) {
                fence(out, &diagnostic_text(content), "text");
            }
        }
    }
    if let Some(symbols) = item.get("symbols") {
        json_section(out, "Selected outline metadata", symbols)?;
    }
    Ok(())
}
fn markdown(envelope: &Value) -> Result<String> {
    let command = envelope
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            Error::new(
                "INVALID_ARGUMENT",
                "Markdown requires an envelope command",
                2,
            )
        })?;
    if !supports(command) {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Markdown is supported only for build, outline, read and handoff",
            2,
        ));
    }
    let mut out = format!("# PCTX {command}\n\n");
    field(
        &mut out,
        "Request status (reported)",
        envelope.get("status"),
    )?;
    for (key, label) in [
        ("schema_version", "Schema version"),
        ("project_id", "Project ID"),
        ("workspace_id", "Workspace ID"),
        ("generation_id", "Generation ID"),
    ] {
        field(&mut out, label, envelope.get(key))?;
    }
    let mut partial = false;
    let mut failure = false;
    indicators(envelope, &mut partial, &mut failure);
    if envelope
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(|e| !e.is_empty())
    {
        failure = true;
    }
    out.push('\n');
    if failure {
        out.push_str("**Failure/error indicators are present in the supplied envelope.**\n\n");
    }
    if partial {
        out.push_str("**Partial, truncated or omitted content is reported.**\n\n");
    }
    out.push_str("Rendering preserves reported evidence; it does not establish test pass, task completion or source freshness.\n\n");
    let status_metadata = Value::Object(
        ["validation", "coverage", "truncation", "warnings", "errors"]
            .into_iter()
            .filter_map(|key| {
                envelope
                    .get(key)
                    .map(|value| (key.to_owned(), value.clone()))
            })
            .collect::<Map<String, Value>>(),
    );
    json_section(
        &mut out,
        "Validation, coverage, warnings and errors",
        &status_metadata,
    )?;
    if let Some(data) = envelope.get("data") {
        match command {
            "read" => {
                out.push_str("## Selected source\n\n");
                source_section(&mut out, data)?;
            }
            "build" => {
                out.push_str("## Context selection\n\n");
                for (key, label) in [
                    ("task", "Task"),
                    ("role", "Role"),
                    ("selection_complete", "Selection complete"),
                    ("context_fingerprint", "Context fingerprint"),
                    ("source_versions", "Source versions"),
                    ("budget", "Reported budget"),
                ] {
                    field(&mut out, label, data.get(key))?;
                }
                out.push('\n');
                if let Some(items) = data.get("items").and_then(Value::as_array) {
                    for (index, item) in items.iter().enumerate() {
                        out.push_str(&format!("### Context item {}\n\n", index + 1));
                        source_section(&mut out, item)?;
                    }
                }
                if let Some(omissions) = data.get("omitted_items") {
                    json_section(&mut out, "Omitted context items", omissions)?;
                }
            }
            "outline" => {
                out.push_str("## Structure\n\n");
                if let Some(files) = data.get("files").and_then(Value::as_array) {
                    for (index, file) in files.iter().enumerate() {
                        out.push_str(&format!("### Source file {}\n\n", index + 1));
                        provenance(&mut out, file)?;
                        if let Some(symbols) = file.get("symbols").and_then(Value::as_array) {
                            if symbols.is_empty() {
                                out.push_str("No symbols returned. Parser and coverage status remain as reported above.\n\n");
                            }
                            for symbol in symbols {
                                for (key, label) in [
                                    ("name", "Symbol"),
                                    ("qualified_name", "Qualified name"),
                                    ("kind", "Kind"),
                                    ("id", "ID"),
                                    ("parent_symbol_id", "Parent symbol ID"),
                                ] {
                                    field(&mut out, label, symbol.get(key))?;
                                }
                                provenance(&mut out, symbol)?;
                            }
                        }
                    }
                }
            }
            _ => {
                out.push_str("## Handoff\n\n");
                source_section(&mut out, data)?;
                if let Some(metadata) = data.get("metadata") {
                    json_section(&mut out, "Handoff metadata", metadata)?;
                }
                if let Some(validation) = data.get("validation") {
                    json_section(&mut out, "Handoff validation", validation)?;
                }
            }
        }
    }
    out.push_str("## Complete envelope metadata\n\nThis JSON appendix preserves every supplied field and the exact source strings. Control escapes in the readable source display are presentation only. No fields are silently dropped.\n\n");
    fence(&mut out, &json_text(envelope, true)?, "json");
    Ok(out)
}

//! Read-only presentation stream. Observation frames never advance durable cursors.
use crate::{
    domain::{Error, Result},
    project::Project,
    work,
};
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    thread,
    time::{Duration, Instant},
};
const POLL: Duration = Duration::from_secs(2);
fn text(value: &Value) -> String {
    crate::render::diagnostic_text(value.as_str().unwrap_or("-"))
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .chars()
        .take(70)
        .collect()
}
fn compact(board: &Value) -> String {
    let mut output = format!(
        "PCTX {}  tasks {}  done {}  valid {}\nID  STATE  AGENT  STAGE  ACCEPTANCE  CHECKS  ACTIVITY\n",
        text(&board["coordination_id"]),
        board["tasks"].as_array().map_or(0, Vec::len),
        board["done_count"],
        board["valid_done_count"]
    );
    for task in board["tasks"].as_array().into_iter().flatten() {
        let acceptance = match (
            task["checklist"]["accepted_weight"].as_u64(),
            task["checklist"]["required_weight"].as_u64(),
        ) {
            (Some(a), Some(n)) => format!("{a}/{n}"),
            _ => "unknown".into(),
        };
        let checks = if let Some(checks) = task.get("checks").and_then(Value::as_array) {
            checks
                .iter()
                .map(|c| format!("{}:{}", text(&c["key"]), text(&c["result"])))
                .collect::<Vec<_>>()
                .join(",")
        } else {
            task["definition"]["checks"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|c| format!("{}:unknown", text(&c["key"])))
                .collect::<Vec<_>>()
                .join(",")
        };
        output.push_str(&format!(
            "{}  {}  {}  {}  {}  {}  {}\n",
            text(&task["display_id"]),
            text(&task["state"]),
            text(&task["agent_id"]),
            text(&task["run"]["stage"]),
            acceptance,
            if checks.is_empty() { "none" } else { &checks },
            text(&task["run"]["activity_status"])
        ));
    }
    output
}
fn emit(writer: &mut impl Write, value: &Value, ndjson: bool, board: bool) -> Result<()> {
    if ndjson {
        writer.write_all(&crate::render::render(value, crate::render::Format::Json)?)?;
    } else if board {
        writer.write_all(compact(&value["data"]).as_bytes())?;
    } else {
        writer.write_all(
            format!(
                "{} {} {}\n",
                value["event_seq"],
                text(&value["type"]),
                text(&value["entity_id"])
            )
            .as_bytes(),
        )?;
    }
    writer.flush()?;
    Ok(())
}
/// Stream durable work events, with separate ephemeral board observations every two seconds.
/// The caller handles argument/format/output-file admission and writes diagnostics to stderr.
pub fn run(project: &Project, board: bool, since: i64, follow: bool, ndjson: bool) -> Result<()> {
    project.check_deadline()?;
    work::validate_activity_cursor(since)?;
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut cursor = since;
    if board {
        let snapshot = work::board(project)?;
        cursor = snapshot["as_of_seq"]
            .as_i64()
            .ok_or_else(|| Error::new("DB_ERROR", "Board snapshot cursor missing", 7))?;
        let frame = json!({"schema_version":"1.0","event_namespace":"work","coordination_id":project.coordination_id,"event_seq":cursor,"type":"board_snapshot","data":snapshot});
        emit(&mut writer, &frame, ndjson, true)?;
    }
    let mut next_observation = Instant::now() + POLL;
    loop {
        project.check_deadline()?;
        let page = work::activity(project, cursor)?;
        for event in page["events"].as_array().into_iter().flatten() {
            if ndjson || !board {
                let mut event = event.clone();
                event["event_namespace"] = json!("work");
                event["coordination_id"] = json!(project.coordination_id);
                emit(&mut writer, &event, ndjson, false)?;
            }
        }
        cursor = page["next_cursor"]
            .as_i64()
            .ok_or_else(|| Error::new("DB_ERROR", "Activity page cursor missing", 7))?;
        if board && follow && Instant::now() >= next_observation {
            let snapshot = work::board(project)?;
            let frame = json!({"schema_version":"1.0","event_namespace":"work","coordination_id":project.coordination_id,"event_seq":null,"type":"board_observation","observed_at":chrono::Utc::now().to_rfc3339(),"persistent_cursor":cursor,"data":snapshot});
            emit(&mut writer, &frame, ndjson, true)?;
            next_observation = Instant::now() + POLL;
        }
        if page["has_more"] == true {
            continue;
        }
        if !follow {
            return Ok(());
        }
        // No independent daemon, upstream polling, durable writes, or model wakeup.
        let wake = Instant::now() + POLL;
        while Instant::now() < wake {
            project.check_deadline()?;
            thread::sleep(Duration::from_millis(20));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames_escape_hidden_controls_without_changing_ndjson_values() {
        let hidden = "stage\u{202e}\u{85}\u{2028}\n";
        let frame = json!({"schema_version":"1.0","event_namespace":"work","event_seq":3,"type":hidden,"entity_id":hidden});
        let mut json_bytes = Vec::new();
        emit(&mut json_bytes, &frame, true, false).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&json_bytes).unwrap(), frame);
        let json_text = String::from_utf8(json_bytes).unwrap();
        assert_eq!(json_text.lines().count(), 1);
        for control in ['\u{202e}', '\u{85}', '\u{2028}'] {
            assert!(!json_text.contains(control));
        }
        let mut plain = Vec::new();
        emit(&mut plain, &frame, false, false).unwrap();
        let plain = String::from_utf8(plain).unwrap();
        assert_eq!(plain.lines().count(), 1);
        for control in ['\u{202e}', '\u{85}', '\u{2028}'] {
            assert!(!plain.contains(control));
        }
        assert!(plain.contains("\\u{202e}") && plain.contains("\\n"));
    }
    #[test]
    fn fixed_controls_in_event_error_and_board_frames_remain_safe() {
        let controls: String = (0..=31)
            .chain(127..=159)
            .chain([0x200e, 0x200f])
            .chain(0x2028..=0x202e)
            .chain(0x2066..=0x2069)
            .map(|n| char::from_u32(n).unwrap())
            .collect();
        for kind in ["event", "error"] {
            let frame = json!({"schema_version":"1.0","type":controls,"event_seq":1,"entity_id":controls,"data":{"kind":kind,"message":controls}});
            let mut bytes = Vec::new();
            emit(&mut bytes, &frame, true, false).unwrap();
            assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), frame);
            assert_eq!(bytes.iter().filter(|b| **b == b'\n').count(), 1);
            let mut plain = Vec::new();
            emit(&mut plain, &frame, false, false).unwrap();
            assert_eq!(plain.iter().filter(|b| **b == b'\n').count(), 1);
            for c in controls.chars().filter(|c| *c != '\n') {
                assert!(!std::str::from_utf8(&plain).unwrap().contains(c));
            }
        }
        for c in controls.chars() {
            // One control per short value prevents the row's deliberate70-char
            // projection from hiding an untested control later in the fixture.
            let value = format!("value{c}tail");
            let frame = json!({"data":{"coordination_id":value,"done_count":0,"valid_done_count":0,"tasks":[{"display_id":value,"state":value,"agent_id":value,"run":{"stage":value,"activity_status":value},"checks":[{"key":value,"result":value}]}]}});
            let mut plain = Vec::new();
            emit(&mut plain, &frame, false, true).unwrap();
            let text = std::str::from_utf8(&plain).unwrap();
            assert_eq!(text.lines().count(), 3);
            assert!(text.contains("tail"));
            if c != '\n' {
                assert!(!text.contains(c));
            }
            let event = json!({"event_seq":1,"type":value,"entity_id":value});
            let mut row = Vec::new();
            emit(&mut row, &event, false, false).unwrap();
            let text = std::str::from_utf8(&row).unwrap();
            assert_eq!(text.lines().count(), 1);
            assert!(text.contains("tail"));
            if c != '\n' {
                assert!(!text.contains(c));
            }
        }
    }
    #[test]
    fn frame_delivery_and_flush_failures_propagate_io_errors() {
        struct Fail {
            flush: bool,
        }
        impl Write for Fail {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.flush {
                    Ok(bytes.len())
                } else {
                    Err(io::Error::from(io::ErrorKind::BrokenPipe))
                }
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::from(io::ErrorKind::BrokenPipe))
            }
        }
        for ndjson in [false, true] {
            for flush in [false, true] {
                let error =
                    emit(&mut Fail { flush }, &json!({"type":"event"}), ndjson, false).unwrap_err();
                assert_eq!(error.code, "IO_ERROR");
                assert_eq!(error.exit, 7);
            }
        }
    }
}

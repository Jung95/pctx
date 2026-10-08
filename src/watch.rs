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
    value
        .as_str()
        .unwrap_or("-")
        .chars()
        .filter(|c| !c.is_control())
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
        writer.write_all(&serde_json::to_vec(value)?)?;
        writer.write_all(b"\n")?;
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
    if since < 0 {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Sequence cannot be negative",
            2,
        ));
    }
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
        thread::sleep(POLL);
    }
}

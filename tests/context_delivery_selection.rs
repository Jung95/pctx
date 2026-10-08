//! The adaptive selector measures a consumer's actual delivery, not a Build proxy.
use pctx::{
    context::{BuildRequest, select_with_measurement},
    domain::{Error, hash},
    project::{Config, Project, ProjectConfig, RootAnchor},
    render::{Format, render},
};
use serde_json::{Value, json};
use std::{
    fs,
    time::{Duration, Instant},
};

fn fixture() -> (tempfile::TempDir, Project, BuildRequest) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    fs::create_dir_all(root.join(".pctx/rules")).unwrap();
    fs::write(
        root.join(".pctx/rules/security.md"),
        "---\nschema_version: 1\nid: security\nscope: ['**']\nrequired: true\n---\nNever infer approval from code.\n",
    )
    .unwrap();
    fs::write(
        root.join("code.py"),
        format!(
            "def calculate(value: int):\n{}",
            "    # body payload\n".repeat(70)
        ),
    )
    .unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: temp.path().join("data"),
        workspace_dir: temp.path().join("data/ws"),
        control_dir: temp.path().join("data/control"),
        project_id: "project".into(),
        workspace_id: "workspace".into(),
        coordination_id: "coord".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "project".into(),
                name: "fixture".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    fs::create_dir_all(&p.workspace_dir).unwrap();
    fs::create_dir_all(&p.control_dir).unwrap();
    let r = BuildRequest {
        task: Some("inspect calculate".into()),
        task_file: None,
        task_id: None,
        seed: vec!["code.py".into()],
        budget_bytes: 64000,
        budget_tokens: None,
        tokenizer: None,
        handoff: None,
        role: "implementer".into(),
        detail: "adaptive".into(),
        changed_since: None,
        dependency_depth: 0,
        explain: false,
        require_complete: false,
    };
    (temp, p, r)
}
fn packet(p: &Project, data: &Value) -> Value {
    // Receipt/transport overhead and escaping belong to the measured consumer.
    // This fixture deliberately differs from the Build envelope.
    let mut receipt = json!({"session_id":"S-delivery","context_epoch":9,
        "transport_note":"\u{85}".repeat(400),"selection":data});
    receipt["content_hash"] = json!(hash(serde_json::to_vec(data).unwrap()));
    pctx::domain::envelope("context", Some(p), receipt)
}
#[test]
fn consumer_envelope_and_escaped_overhead_drive_tiers_and_exact_capacity() {
    let (_temp, p, mut r) = fixture();
    r.detail = "signature".into();
    let signature = select_with_measurement(&p, &r, Format::Json, |data| {
        Ok(render(&packet(&p, data), Format::Json)?.len())
    })
    .unwrap();
    r.budget_bytes = signature["budget"]["used"].as_u64().unwrap() as usize + 64;
    r.detail = "adaptive".into();
    let first = select_with_measurement(&p, &r, Format::Json, |data| {
        Ok(render(&packet(&p, data), Format::Json)?.len())
    })
    .unwrap();
    let bytes = render(&packet(&p, &first), Format::Json).unwrap();
    assert_eq!(first["budget"]["used"], bytes.len());
    assert!(bytes.len() <= r.budget_bytes);
    let optional = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["path"] == "code.py")
        .unwrap();
    assert_eq!(optional["representation"], "signature");
    assert_eq!(optional["downgrade_reason"], "budget");
    assert!(first["items"].as_array().unwrap().iter().any(|v| {
        v["required"] == true
            && v["content"]
                .as_str()
                .unwrap()
                .contains("Never infer approval")
    }));
    let second = select_with_measurement(&p, &r, Format::Json, |data| {
        Ok(render(&packet(&p, data), Format::Json)?.len())
    })
    .unwrap();
    assert_eq!(first, second);
}
#[test]
fn consumer_failure_and_original_deadline_cannot_publish_a_selection() {
    let (_temp, p, r) = fixture();
    let error = select_with_measurement(&p, &r, Format::Json, |_| {
        Err(Error::new(
            "DELIVERY_UNAVAILABLE",
            "fixture renderer failed",
            7,
        ))
    })
    .unwrap_err();
    assert_eq!(error.code, "DELIVERY_UNAVAILABLE");
    let mut p = p;
    // Expire the same request from within delivery preparation, after selection.
    p.deadline = Some(pctx::deadline::Deadline::from_millis(1000).unwrap());
    let end = p.deadline.unwrap().instant();
    let mut measured = false;
    let error = select_with_measurement(&p, &r, Format::Json, |_| {
        measured = true;
        std::thread::sleep(
            end.saturating_duration_since(Instant::now()) + Duration::from_millis(2),
        );
        Ok(1)
    })
    .unwrap_err();
    assert!(
        measured,
        "fixture must reach delivery preparation before expiry"
    );
    assert_eq!(error.code, "TIMEOUT");
}
#[test]
fn consumer_overhead_cannot_remove_mandatory_rules_or_fake_a_fitting_packet() {
    let (_temp, p, mut r) = fixture();
    r.budget_bytes = 512;
    let error = select_with_measurement(&p, &r, Format::Json, |data| {
        Ok(render(&packet(&p, data), Format::Json)?.len())
    })
    .unwrap_err();
    assert_eq!(error.code, "BUDGET_TOO_SMALL");
}

#[test]
fn source_changed_during_delivery_preparation_cannot_publish_stale_spans() {
    let (_temp, p, r) = fixture();
    let mut changed = false;
    let error = select_with_measurement(&p, &r, Format::Json, |data| {
        if !changed {
            fs::write(
                p.root.join("code.py"),
                "def replacement():\n    return 42\n",
            )?;
            changed = true;
        }
        Ok(render(&packet(&p, data), Format::Json)?.len())
    })
    .unwrap_err();
    assert!(changed);
    assert_eq!(error.code, "CONCURRENT_MODIFICATION");
}

//! PCTX01-G03-D04 caller syntax/shape/size and original admission precedence.
use rusqlite::Connection;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

type Files = BTreeMap<PathBuf, Option<Vec<u8>>>;
type Tables = BTreeMap<String, Vec<Vec<String>>>;
#[derive(Debug, PartialEq)]
struct Snapshot {
    files: Files,
    databases: BTreeMap<PathBuf, Tables>,
}
fn snapshot(root: &Path) -> Snapshot {
    fn walk(root: &Path, path: &Path, out: &mut Snapshot) {
        if !path.exists() {
            return;
        }
        let key = path.strip_prefix(root).unwrap().to_path_buf();
        if path.is_dir() {
            out.files.insert(key, None);
            for entry in fs::read_dir(path).unwrap() {
                walk(root, &entry.unwrap().path(), out);
            }
            return;
        }
        let name = path.file_name().unwrap().to_string_lossy();
        if name.ends_with(".sqlite3-wal") || name.ends_with(".sqlite3-shm") {
            return;
        }
        if name.ends_with(".sqlite3") {
            let db = Connection::open(path).unwrap();
            let mut tables = Tables::new();
            let schema: Vec<(String, String, String)> = db
                .prepare("SELECT type,name,coalesce(sql,'') FROM sqlite_master ORDER BY type,name")
                .unwrap()
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            tables.insert(
                "schema".into(),
                schema
                    .iter()
                    .map(|(t, n, s)| vec![t.clone(), n.clone(), s.clone()])
                    .collect(),
            );
            for (kind, name, _) in schema {
                if kind != "table" {
                    continue;
                }
                let sql = format!("SELECT * FROM \"{}\"", name.replace('"', "\"\""));
                let mut statement = db.prepare(&sql).unwrap();
                let columns = statement.column_count();
                let mut rows: Vec<Vec<String>> = statement
                    .query_map([], |r| {
                        (0..columns)
                            .map(|i| Ok(format!("{:?}", r.get_ref(i)?)))
                            .collect()
                    })
                    .unwrap()
                    .map(Result::unwrap)
                    .collect();
                rows.sort();
                tables.insert(format!("table:{name}"), rows);
            }
            out.databases.insert(key, tables);
        } else {
            out.files.insert(key, Some(fs::read(path).unwrap()));
        }
    }
    let mut out = Snapshot {
        files: Files::new(),
        databases: BTreeMap::new(),
    };
    walk(root, root, &mut out);
    out
}
struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    root: PathBuf,
    data: PathBuf,
    task: String,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let mut f = Self {
            root: base.join("project"),
            data: base.join("data"),
            base,
            _temp: temp,
            task: String::new(),
        };
        fs::create_dir(&f.root).unwrap();
        fs::write(f.root.join("code.py"), "def code():\n    return 1\n").unwrap();
        assert!(f.run("json", "owner", &["init"]).status.success());
        fs::write(f.base.join("seed.json"), json!({"schema_version":1,"title":"Input fixture","scope":["code.py"],"acceptance":[{"id":"AC1","description":"Finish","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test"}]}).to_string()).unwrap();
        let output = f.run(
            "json",
            "owner",
            &["task", "create", "--from-file", "seed.json"],
        );
        assert!(output.status.success(), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        f.task = value["data"]["task_id"].as_str().unwrap().into();
        // Prepare databases before snapshots; no first-use rollback claim.
        let _ = f.run("json", "owner", &["quota", "report"]);
        fs::write(f.base.join("schedule.json"), json!({"schema_version":1,"namespace":"fixture","id":"digest","timezone":"UTC","cadence":{"kind":"daily","at":"12:00"},"valid_from":"2024-01-01T00:00:00Z","job":"read_query","bridge":"manual","role":"assistant","recipient":"owner","topic":"digest"}).to_string()).unwrap();
        let output = f.run(
            "json",
            "owner",
            &[
                "schedule",
                "add",
                "--from-file",
                "schedule.json",
                "--idempotency-key",
                "seed",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        f
    }
    fn run(&self, format: &str, actor: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .current_dir(&self.base)
            .args(["--root"])
            .arg(&self.root)
            .args(["--format", format])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_USER_CONFIG", self.base.join("absent-config"))
            .env("PCTX_ACTOR", actor)
            .env_remove("PCTX_RUN_CAPABILITY")
            .output()
            .unwrap()
    }
    fn state(&self) -> (Snapshot, Snapshot) {
        (snapshot(&self.root), snapshot(&self.data))
    }
    fn routes(&self) -> Vec<(Vec<&str>, usize)> {
        vec![
            (
                vec!["task", "create", "--from-file", "caller.json"],
                1048576,
            ),
            (
                vec![
                    "task",
                    "edit",
                    &self.task,
                    "--expect-revision",
                    "1",
                    "--from-file",
                    "caller.json",
                ],
                1048576,
            ),
            (
                vec!["check", "record", "missing", "--from-file", "caller.json"],
                1048576,
            ),
            (
                vec!["control", "restore", "--input", "caller.json"],
                268435456,
            ),
            (
                vec![
                    "quota",
                    "ingest",
                    "--idempotency-key",
                    "input-contract",
                    "--from-file",
                    "caller.json",
                ],
                1048576,
            ),
            (
                vec![
                    "schedule",
                    "add",
                    "--idempotency-key",
                    "input-contract",
                    "--from-file",
                    "caller.json",
                ],
                65536,
            ),
            (
                vec![
                    "schedule",
                    "update",
                    "--namespace",
                    "fixture",
                    "digest",
                    "--expect-revision",
                    "1",
                    "--from-file",
                    "caller.json",
                ],
                65536,
            ),
            (
                vec![
                    "schedule",
                    "install",
                    "--expect-hash",
                    "hash",
                    "--from-file",
                    "caller.json",
                ],
                262144,
            ),
        ]
    }
}
fn refusal(output: &Output, exit: i32, code: &str) -> Value {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "error");
    assert_eq!(value["errors"][0]["code"], code);
    assert_eq!(
        value["coverage"],
        json!({"status":"partial","reasons":[code]})
    );
    assert!(value["data"].is_null());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PCTX_CALLER_SENTINEL"));
    value
}

#[test]
fn caller_json_syntax_and_typed_shape_are_input_two_and_preserve_prepared_state() {
    let f = Fixture::new();
    let before = f.state();
    let mut attempts = 0;
    for body in [
        b"{PCTX_CALLER_SENTINEL".as_slice(),
        b"null",
        b"[]",
        b"{}",
        b"\xff",
    ] {
        fs::write(f.base.join("caller.json"), body).unwrap();
        for (args, _) in f.routes() {
            for format in ["json", "compact"] {
                refusal(&f.run(format, "owner", &args), 2, "INVALID_ARGUMENT");
                assert_eq!(f.state(), before, "{args:?}");
                attempts += 1;
            }
        }
    }
    eprintln!("Caller syntax/shape native combinations:{attempts}");
}

#[test]
fn exact_file_caps_reach_json_decoder_and_overflow_preserves_existing_input_errors() {
    let f = Fixture::new();
    let before = f.state();
    for (args, limit) in f.routes() {
        let path = f.base.join("caller.json");
        fs::File::create(&path)
            .unwrap()
            .set_len((limit + 1) as u64)
            .unwrap();
        let value = refusal(&f.run("json", "owner", &args), 2, "INVALID_ARGUMENT");
        assert!(
            value["errors"][0]["message"]
                .as_str()
                .unwrap()
                .contains(if limit == 1048576 {
                    "one MiB"
                } else if limit == 268435456 {
                    "256 MiB"
                } else if limit == 65536 {
                    "64 KiB"
                } else {
                    "bounded"
                })
        );
        assert_eq!(f.state(), before, "{args:?}");
        // The archive metadata-before-body bound has a private unit proof.
        // Avoid allocating256MiB merely to retest that common primitive.
        if limit == 268435456 {
            continue;
        }
        let mut bytes = vec![b' '; limit];
        bytes[0] = b'{';
        fs::write(path, bytes).unwrap();
        let value = refusal(&f.run("json", "owner", &args), 2, "INVALID_ARGUMENT");
        assert_eq!(value["errors"][0]["message"], "Invalid JSON data");
        assert_eq!(f.state(), before, "{args:?}");
    }
}

#[test]
fn caller_owner_and_task_revision_priorities_do_not_become_universal_input_first() {
    let f = Fixture::new();
    fs::write(f.base.join("caller.json"), b"{PCTX_CALLER_SENTINEL").unwrap();
    let before = f.state();
    for (args, _) in f.routes() {
        // Check Record's existing input parse precedes its report capability checks.
        if args.starts_with(&["check", "record"]) {
            continue;
        }
        refusal(&f.run("json", "agent", &args), 5, "POLICY_DENIED");
        assert_eq!(f.state(), before, "{args:?}");
    }
    refusal(
        &f.run(
            "json",
            "owner",
            &[
                "task",
                "edit",
                &f.task,
                "--expect-revision",
                "99",
                "--from-file",
                "caller.json",
            ],
        ),
        9,
        "REVISION_CONFLICT",
    );
    assert_eq!(f.state(), before);
    fs::remove_file(f.base.join("caller.json")).unwrap();
    for (args, _) in f.routes() {
        refusal(&f.run("json", "owner", &args), 7, "IO_ERROR");
        assert_eq!(f.state(), before, "{args:?}");
    }
}

#[test]
fn authored_task_and_schedule_schema_refusals_preserve_prepared_state() {
    let f = Fixture::new();
    let before = f.state();
    let task: Value = serde_json::from_slice(&fs::read(f.base.join("seed.json")).unwrap()).unwrap();
    let schedule: Value =
        serde_json::from_slice(&fs::read(f.base.join("schedule.json")).unwrap()).unwrap();
    let task_changes = [
        ("/schema_version", json!(2)),
        ("/schema_version", json!("1")),
        ("/title", json!(" ")),
        ("/priority", json!("P4")),
        ("/scope", json!([])),
        ("/scope", json!(["../escape"])),
        ("/acceptance/0/weight", json!(0)),
        ("/acceptance/0/evidence_check_keys", json!(["unregistered"])),
        ("/checks/0/kind", json!("unknown")),
        ("/checks/0/allowed_sources", json!(["claimed_runner"])),
    ];
    let schedule_changes = [
        ("/schema_version", json!(2)),
        ("/schema_version", json!("1")),
        ("/timezone", json!("unsupported-zone")),
        ("/cadence/at", json!("25:00")),
        ("/valid_from", json!("yesterday")),
        ("/job", json!("unknown")),
        ("/bridge", json!("unknown")),
        ("/misfire", json!("run_all")),
    ];
    let mut attempts = 0;
    for (seed, changes, selected) in [
        (&task, task_changes.as_slice(), [0, 1]),
        (&schedule, schedule_changes.as_slice(), [5, 6]),
    ] {
        for (pointer, value) in changes {
            let mut body = seed.clone();
            // Defaulted fields can be absent in the valid authored seed.
            if let Some(field) = body.pointer_mut(pointer) {
                *field = value.clone();
            } else {
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                body.pointer_mut(parent).unwrap()[key] = value.clone();
            }
            fs::write(f.base.join("caller.json"), body.to_string()).unwrap();
            for route in selected {
                let routes = f.routes();
                let args = &routes[route].0;
                for format in ["json", "compact"] {
                    refusal(&f.run(format, "owner", args), 2, "INVALID_ARGUMENT");
                    assert_eq!(f.state(), before, "{pointer} {args:?}");
                    attempts += 1;
                }
            }
        }
    }
    eprintln!("Authored Task/Schedule schema native combinations:{attempts}");
}

#[test]
fn quota_batch_schema_and_restore_identity_refusals_are_safe_input_errors() {
    let f = Fixture::new();
    let before = f.state();
    for body in [
        json!({"schema_version":2,"observations":[]}),
        json!({"schema_version":1,"observations":[]}),
        json!({"schema_version":"1","observations":[]}),
        json!({"schema_version":1,"observations":null}),
        json!({"schema_version":1,"observations":[{}]}),
        json!({"schema_version":1,"observations":[],"PCTX_CALLER_SENTINEL":true}),
    ] {
        fs::write(f.base.join("caller.json"), body.to_string()).unwrap();
        for format in ["json", "compact"] {
            refusal(
                &f.run(format, "owner", &f.routes()[4].0),
                2,
                "INVALID_ARGUMENT",
            );
            assert_eq!(f.state(), before);
        }
    }
    // Produce a real archive rather than inventing a checksum-valid fixture.
    let backup_path = f.base.join("backup.json");
    let output = f.run(
        "json",
        "owner",
        &[
            "control",
            "backup",
            "--output",
            backup_path.to_str().unwrap(),
        ],
    );
    assert!(output.status.success(), "{output:?}");
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["status"], "ok");
    assert_eq!(response["command"], "work");
    assert!(output.stderr.is_empty());
    let archive: Value = serde_json::from_slice(&fs::read(backup_path).unwrap()).unwrap();
    let after_backup = f.state();
    let original_archive = fs::read(f.base.join("backup.json")).unwrap();
    // Artifact collision is an error envelope on stdout, never a second write
    // of either a response or a newly generated archive over the first backup.
    refusal(
        &f.run(
            "json",
            "owner",
            &[
                "control",
                "backup",
                "--output",
                f.base.join("backup.json").to_str().unwrap(),
            ],
        ),
        9,
        "REVISION_CONFLICT",
    );
    assert_eq!(
        fs::read(f.base.join("backup.json")).unwrap(),
        original_archive
    );
    assert_eq!(f.state(), after_backup);
    for (key, value) in [
        ("schema_version", json!(2)),
        ("project_id", json!("other-project")),
        ("database_hash", json!("wrong-checksum")),
    ] {
        let mut body = archive.clone();
        body[key] = value;
        fs::write(f.base.join("caller.json"), body.to_string()).unwrap();
        for format in ["json", "compact"] {
            refusal(
                &f.run(format, "owner", &f.routes()[3].0),
                2,
                "INVALID_ARCHIVE",
            );
            assert_eq!(f.state(), after_backup, "{key}");
        }
    }
}

#[test]
fn operations_raw_and_sanitized_consumers_reject_bad_json_before_effects() {
    let f = Fixture::new();
    let prepared = f.run("json", "owner", &["role", "list"]);
    assert!(prepared.status.success(), "{prepared:?}");
    let before = f.state();
    let routes: [&[&str]; 10] = [
        &["policy", "evaluate", "--action-file", "caller.json"],
        &["policy", "attest-owner", "--from-file", "caller.json"],
        &["policy", "release", "--from-file", "caller.json"],
        &["policy", "exception-record", "--from-file", "caller.json"],
        &["policy", "exception-revoke", "--from-file", "caller.json"],
        &["policy", "report-evaluate", "--from-file", "caller.json"],
        &["policy", "report-fingerprint", "--from-file", "caller.json"],
        &["decision", "request", "--from-file", "caller.json"],
        &[
            "decision",
            "record",
            "missing",
            "--from-file",
            "caller.json",
        ],
        &[
            "message",
            "send",
            "--to-role",
            "assistant",
            "--from-file",
            "caller.json",
        ],
    ];
    let mut combinations = 0;
    for body in [
        b"{PCTX_CALLER_SENTINEL".as_slice(),
        b"null",
        b"[]",
        b"{}",
        b"\xff",
    ] {
        fs::write(f.base.join("caller.json"), body).unwrap();
        for args in routes {
            for format in ["json", "compact"] {
                let code = if body == b"\xff" {
                    "UNSUPPORTED_ENCODING"
                } else {
                    "INVALID_ARGUMENT"
                };
                refusal(&f.run(format, "owner", args), 2, code);
                assert_eq!(f.state(), before, "{args:?}");
                combinations += 1;
            }
        }
    }
    for length in [65536, 65537] {
        let mut bytes = vec![b' '; length];
        bytes[0] = b'{';
        fs::write(f.base.join("caller.json"), bytes).unwrap();
        for args in routes {
            let value = refusal(&f.run("json", "owner", args), 2, "INVALID_ARGUMENT");
            assert_eq!(
                value["errors"][0]["message"],
                if length == 65536 {
                    "Invalid JSON data"
                } else {
                    "Operations input exceeds 64 KiB"
                }
            );
            assert_eq!(f.state(), before, "{args:?}");
        }
    }
    // Owner authority precedes the file read only on owner-dependent routes.
    for route in [5, 8] {
        refusal(&f.run("json", "agent", routes[route]), 5, "POLICY_DENIED");
        assert_eq!(f.state(), before);
    }
    eprintln!("Operations raw/sanitized native combinations:{combinations}; cap20; owner2");
}

#[test]
fn hook_required_shapes_and_unsupported_protocol_keep_distinct_refusals() {
    let f = Fixture::new();
    let before = f.state();
    let routes: [&[&str]; 2] = [
        &[
            "adapter",
            "claude",
            "event",
            "--agent",
            "fixture",
            "--from-file",
            "caller.json",
        ],
        &[
            "adapter",
            "claude",
            "protocol-fixture",
            "--from-file",
            "caller.json",
        ],
    ];
    let cases = [
        (json!(null), 2, "INVALID_ARGUMENT"),
        (json!([]), 2, "INVALID_ARGUMENT"),
        (json!({}), 2, "INVALID_ARGUMENT"),
        (json!({"hook_event_name":2}), 2, "INVALID_ARGUMENT"),
        (json!({"hook_event_name":"Stop"}), 2, "INVALID_ARGUMENT"),
        (
            json!({"hook_event_name":"Stop","session_id":2}),
            2,
            "INVALID_ARGUMENT",
        ),
        (
            json!({"hook_event_name":"Unknown","session_id":"native"}),
            6,
            "CAPABILITY_UNAVAILABLE",
        ),
        (
            json!({"hook_event_name":"SessionStart","session_id":"native","source":"unknown"}),
            6,
            "CAPABILITY_UNAVAILABLE",
        ),
        (
            json!({"hook_event_name":"PreCompact","session_id":"native","trigger":"unknown"}),
            6,
            "CAPABILITY_UNAVAILABLE",
        ),
    ];
    for (body, exit, code) in cases {
        fs::write(f.base.join("caller.json"), body.to_string()).unwrap();
        for args in routes {
            for format in ["json", "compact"] {
                refusal(&f.run(format, "owner", args), exit, code);
                assert_eq!(f.state(), before, "{body} {args:?}");
            }
        }
    }
    fs::write(f.base.join("caller.json"), b"{PCTX_CALLER_SENTINEL").unwrap();
    for args in routes {
        refusal(&f.run("json", "owner", args), 2, "INVALID_ARGUMENT");
        assert_eq!(f.state(), before);
    }
}

#[test]
fn authored_inventory_profile_and_registry_refuse_schema_before_publication() {
    let f = Fixture::new();
    let routes: [&[&str]; 2] = [
        &["inventory", "profile", "--path", "caller.json"],
        &["inventory", "audit", "--registry", "caller.json"],
    ];
    for body in [
        b"{PCTX_CALLER_SENTINEL".as_slice(),
        b"null",
        b"[]",
        b"{}",
        b"\xff",
    ] {
        fs::write(f.root.join("caller.json"), body).unwrap();
        let before = f.state();
        for args in routes {
            for format in ["json", "compact"] {
                // reader classifies invalid UTF-8 independently of JSON shape.
                let code = if body == b"\xff" {
                    "UNSUPPORTED_ENCODING"
                } else {
                    "INVALID_ARGUMENT"
                };
                let exit = if body == b"\xff" { 3 } else { 2 };
                refusal(&f.run(format, "owner", args), exit, code);
                assert_eq!(f.state(), before, "{args:?}");
            }
        }
    }
    for (args, body) in routes.into_iter().zip([
        json!({"schema_version":2,"id":"fixture","expectations":[]}),
        json!({"schema_version":2,"source_refs":[]}),
    ]) {
        fs::write(f.root.join("caller.json"), body.to_string()).unwrap();
        let before = f.state();
        for format in ["json", "compact"] {
            let value = refusal(&f.run(format, "owner", args), 2, "INVALID_ARGUMENT");
            assert!(
                value["errors"][0]["message"]
                    .as_str()
                    .unwrap()
                    .contains("schema")
            );
            assert_eq!(f.state(), before);
        }
    }
}

#[test]
fn statusline_shape_and_version_refusals_precede_session_binding() {
    let f = Fixture::new();
    let args = [
        "adapter",
        "claude",
        "statusline",
        "--task-id",
        &f.task,
        "--from-file",
        "caller.json",
        "--pool",
        "main",
        "--session",
        "missing",
        "--epoch",
        "1",
        "--observed-at",
        "2026-10-09T00:00:00Z",
        "--window-start",
        "2026-10-09T00:00:00Z",
        "--window-end",
        "2026-10-10T00:00:00Z",
        "--counter-epoch",
        "fixture",
        "--idempotency-key",
        "fixture",
    ];
    let before = f.state();
    for (body, exit, code) in [
        ("{PCTX_CALLER_SENTINEL", 2, "INVALID_ARGUMENT"),
        ("null", 6, "CAPABILITY_UNAVAILABLE"),
        ("[]", 6, "CAPABILITY_UNAVAILABLE"),
        ("{}", 6, "CAPABILITY_UNAVAILABLE"),
        (r#"{"version":2}"#, 6, "CAPABILITY_UNAVAILABLE"),
        (r#"{"version":"unverified"}"#, 6, "CAPABILITY_UNAVAILABLE"),
        (r#"{"version":"2.1.211"}"#, 2, "INVALID_ARGUMENT"),
        (
            r#"{"version":"2.1.211","session_id":2}"#,
            2,
            "INVALID_ARGUMENT",
        ),
    ] {
        fs::write(f.base.join("caller.json"), body).unwrap();
        for format in ["json", "compact"] {
            refusal(&f.run(format, "owner", &args), exit, code);
            assert_eq!(f.state(), before, "{body}");
        }
    }
    // Policy remains earlier than protocol parsing on this owner-only import.
    refusal(&f.run("json", "agent", &args), 5, "POLICY_DENIED");
    assert_eq!(f.state(), before);
}

#[test]
fn handoff_metadata_syntax_is_distinct_from_permitted_raw_body_and_unvalidated_shape() {
    let f = Fixture::new();
    let directory = f.root.join(".pctx/handoffs");
    fs::create_dir_all(&directory).unwrap();
    for metadata in ["{PCTX_CALLER_SENTINEL", ""] {
        fs::write(
            directory.join("caller.md"),
            format!("---\n{metadata}\n---\nRaw body"),
        )
        .unwrap();
        let before = f.state();
        for format in ["json", "compact"] {
            refusal(
                &f.run(format, "owner", &["handoff", "show", "caller"]),
                2,
                "INVALID_ARGUMENT",
            );
            assert_eq!(f.state(), before);
        }
    }
    // Show without --validate permits arbitrary JSON metadata; do not invent
    // a schema restriction or reject arbitrary prose in the user-authored body.
    for metadata in [json!(null), json!([]), json!({})] {
        fs::write(
            directory.join("caller.md"),
            format!("---\n{metadata}\n---\n{{raw non-JSON body"),
        )
        .unwrap();
        let before = f.state();
        let output = f.run("json", "owner", &["handoff", "show", "caller"]);
        assert!(output.status.success(), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["data"]["metadata"], metadata);
        assert_eq!(f.state(), before);
    }
}

#[test]
fn filter_manifest_refusals_precede_reports_and_external_pack_syntax_is_input_two() {
    let f = Fixture::new();
    let directory = f.root.join(".pctx/filters");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("caller.toml"),
        r#"schema_version = 1
id = "caller"
version = "1.0.0"
priority = 100
[match]
program = "/usr/bin/printf"
argv_prefix = ["status"]
stream = "both"
[parse]
kind = "lines"
[render]
max_bytes = 8192
keep_head_lines = 12
keep_tail_lines = 20
show_omission_counts = true
[[rules]]
op = "protect"
pattern = "error"
"#,
    )
    .unwrap();
    fs::create_dir(f.root.join("fixtures")).unwrap();
    let args = [
        "filter",
        "test",
        ".pctx/filters/caller.toml",
        "--fixtures",
        "fixtures",
    ];
    for body in [
        "{PCTX_CALLER_SENTINEL",
        "null",
        "[]",
        "{}",
        r#"{"schema_version":2,"executable":"/usr/bin/printf","cases":[]}"#,
        r#"{"schema_version":1,"executable":"/usr/bin/printf","cases":[]}"#,
    ] {
        fs::write(f.root.join("fixtures/manifest.json"), body).unwrap();
        let before = f.state();
        for format in ["json", "compact"] {
            refusal(&f.run(format, "owner", &args), 2, "FILTER_INVALID");
            assert_eq!(f.state(), before);
        }
    }
    let artifact = f.root.join("external.json");
    fs::write(&artifact, b"{PCTX_CALLER_SENTINEL").unwrap();
    let before = f.state();
    let bytes = fs::read(&artifact).unwrap();
    for format in ["json", "compact"] {
        refusal(
            &f.run(format, "owner", &["pack", "inspect", "external.json"]),
            2,
            "INVALID_ARGUMENT",
        );
        assert_eq!(f.state(), before);
        assert_eq!(fs::read(&artifact).unwrap(), bytes);
    }
}

#[test]
fn typed_check_report_schema_is_validated_after_existing_check_and_lease() {
    let f = Fixture::new();
    for args in [
        vec!["agent", "register", "--name", "worker", "--kind", "agent"],
        vec!["task", "ready", &f.task],
        vec!["task", "assign", &f.task, "--agent", "worker"],
    ] {
        let output = f.run("json", "owner", &args);
        assert!(output.status.success(), "{output:?}");
    }
    let output = f.run("json", "owner", &["task", "start", &f.task]);
    assert!(output.status.success(), "{output:?}");
    let started: Value = serde_json::from_slice(&output.stdout).unwrap();
    let run = started["data"]["run_id"].as_str().unwrap();
    let output = f.run(
        "json",
        "owner",
        &["check", "begin", &f.task, "--key", "unit", "--run", run],
    );
    assert!(output.status.success(), "{output:?}");
    let begun: Value = serde_json::from_slice(&output.stdout).unwrap();
    let check = begun["data"]["check_id"].as_str().unwrap();
    let report = json!({"schema_version":1,"check_key":"unit","producer":"fixture","source":"external_report","exit_code":0,"tests":1,"passed":1,"failed":0,"errors":0,"skipped":0,"result":"passed","started_at":1,"finished_at":2});
    let before = f.state();
    for (key, value) in [
        ("schema_version", json!(2)),
        ("check_key", json!("other")),
        ("producer", json!(" ")),
        ("finished_at", json!(0)),
        ("result", json!("unknown")),
        ("tests", json!(2)),
        ("passed", json!(u64::MAX)),
    ] {
        let mut body = report.clone();
        body[key] = value;
        fs::write(f.base.join("caller.json"), body.to_string()).unwrap();
        for format in ["json", "compact"] {
            let value = refusal(
                &f.run(
                    format,
                    "owner",
                    &["check", "record", check, "--from-file", "caller.json"],
                ),
                2,
                "INVALID_ARGUMENT",
            );
            assert_eq!(
                value["errors"][0]["message"],
                "Invalid check report schema or counts"
            );
            assert_eq!(f.state(), before, "{key}");
        }
    }
}

#[test]
fn quota_count_ceiling_remains_subordinate_to_original_one_mib_file_cap() {
    let f = Fixture::new();
    // These required string keys have no aliases. Empty strings and omitted
    // optional/defaulted fields are the shortest typed observation encoding.
    let minimum = json!({"observation_id":"","pool_id":"","provider":"","model":"","metric":"","unit":"","source":"","collector":"","source_revision":"","observed_at":"","window_id":"","window_start":"","window_end":"","status":"","kind":""});
    let _: pctx::quota::Observation = serde_json::from_value(minimum.clone()).unwrap();
    let minimum_bytes = minimum.to_string().len();
    assert!(10000 * (minimum_bytes + 1) > 1048576);
    let before = f.state();
    for count in [10000, 10001] {
        let batch = json!({"schema_version":1,"observations":vec![minimum.clone(); count]});
        fs::write(f.base.join("caller.json"), batch.to_string()).unwrap();
        let value = refusal(
            &f.run("json", "owner", &f.routes()[4].0),
            2,
            "INVALID_ARGUMENT",
        );
        assert_eq!(
            value["errors"][0]["message"],
            "Usage import must be regular JSON at most one MiB"
        );
        assert_eq!(f.state(), before);
    }
    eprintln!(
        "Quota required-key minimum bytes:{minimum_bytes};10000/10001 bounded transport refuses before observation/count semantics"
    );
}

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

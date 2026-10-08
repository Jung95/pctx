//! Actual CLI-process regression cases. Each test uses an isolated project and data directory.
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::{Arc, Barrier},
};
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let data = temp.path().join("data");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("code.py"), "def value():\n    return 1\n").unwrap();
        let f = Self {
            _temp: temp,
            root,
            data,
        };
        f.ok(&["init"]);
        f
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_pctx"));
        c.args(["--root", self.root.to_str().unwrap(), "--format", "json"])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .env("PCTX_ACTOR", "owner")
            .env_remove("PCTX_RUN_CAPABILITY");
        c
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
    fn ok(&self, args: &[&str]) -> Value {
        parse_ok(self.run(args), args)
    }
    fn definition(&self, title: &str) -> PathBuf {
        let p = self.data.join("definition.json");
        fs::write(&p,json!({"schema_version":1,"title":title,"scope":["code.py"],"acceptance":[{"id":"behavior","description":"Tests validate behavior","evidence_check_keys":["unit"]}],"checks":[{"key":"unit","kind":"test","output_paths":["reports/**"]}]}).to_string()).unwrap();
        p
    }
    fn create(&self) -> String {
        let path = self.definition("Change value");
        self.ok(&["task", "create", "--from-file", path.to_str().unwrap()])["data"]["task_id"]
            .as_str()
            .unwrap()
            .into()
    }
    fn register(&self, name: &str) {
        self.ok(&["agent", "register", "--name", name, "--kind", "agent"]);
    }
    fn active(&self) -> (String, String, String) {
        let t = self.create();
        self.register("worker");
        self.ok(&["task", "ready", &t]);
        self.ok(&["task", "assign", &t, "--agent", "worker"]);
        let r = self.ok(&["task", "start", &t]);
        (
            t,
            r["data"]["run_id"].as_str().unwrap().into(),
            r["data"]["lease_epoch"].to_string(),
        )
    }
    fn record(&self, t: &str, run: &str) -> String {
        let c = self.ok(&["check", "begin", t, "--key", "unit", "--run", run])["data"]["check_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let path = self.data.join("report.json");
        let now = chrono::Utc::now().timestamp();
        fs::write(&path,json!({"schema_version":1,"check_key":"unit","producer":"synthetic-fixture","source":"external_report","exit_code":0,"tests":1,"passed":1,"failed":0,"errors":0,"skipped":0,"result":"passed","started_at":now,"finished_at":now}).to_string()).unwrap();
        self.ok(&["check", "record", &c, "--from-file", path.to_str().unwrap()]);
        c
    }
}
fn parse(o: &Output) -> Value {
    serde_json::from_slice(&o.stdout).unwrap_or_else(|e| {
        panic!(
            "JSON error {e}: stdout={} stderr={}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
    })
}
fn parse_ok(o: Output, args: &[&str]) -> Value {
    assert!(
        o.status.success(),
        "{args:?}: stdout={} stderr={}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    parse(&o)
}
#[test]
fn two_process_claims_have_one_winner() {
    let f = Fixture::new();
    let t = f.create();
    f.register("alpha");
    f.register("beta");
    f.ok(&["task", "ready", &t]);
    let barrier = Arc::new(Barrier::new(3));
    let mut joins = vec![];
    for a in ["alpha", "beta"] {
        let mut cmd = f.command(&["task", "claim", &t, "--agent", a]);
        let gate = barrier.clone();
        joins.push(std::thread::spawn(move || {
            gate.wait();
            cmd.output().unwrap()
        }));
    }
    barrier.wait();
    let outcomes = joins
        .into_iter()
        .map(|j| j.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|o| o.status.success()).count(), 1);
    let loser = outcomes.iter().find(|o| !o.status.success()).unwrap();
    assert_eq!(loser.status.code(), Some(9));
    assert_eq!(parse(loser)["errors"][0]["code"], "TASK_ALREADY_CLAIMED");
    let winner = parse(outcomes.iter().find(|o| o.status.success()).unwrap());
    let board = f.ok(&["board"]);
    assert_eq!(
        board["data"]["tasks"][0]["run"]["run_id"],
        winner["data"]["run_id"]
    );
    assert_eq!(board["data"]["tasks"][0]["state"], "in_progress");
}
#[test]
fn ten_readers_and_two_writers_see_published_generations() {
    let f = Fixture::new();
    f.ok(&["index", "update"]);
    let gate = Arc::new(Barrier::new(13));
    let mut joins = vec![];
    for n in 0..12 {
        let args = if n < 2 {
            vec!["index", "update"]
        } else {
            vec!["find", "value", "--kind", "symbol"]
        };
        let mut cmd = f.command(&args);
        let gate = gate.clone();
        joins.push(std::thread::spawn(move || {
            gate.wait();
            cmd.output().unwrap()
        }));
    }
    gate.wait();
    for j in joins {
        let o = j.join().unwrap();
        assert!(
            o.status.success(),
            "{} {}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        let value = parse(&o);
        assert_eq!(value["status"], "ok");
        if let Some(items) = value["data"]["items"].as_array() {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0]["path"], "code.py");
            assert_eq!(items[0]["freshness"], "current");
        }
    }
    let status = f.ok(&["status"]);
    let path = status["data"]["local_storage"]["index"].as_str().unwrap();
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let state:String=db.query_row("SELECT g.state FROM workspace_meta m JOIN generations g ON g.id=m.active_generation_id",[],|r|r.get(0)).unwrap();
    assert_eq!(state, "ready");
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM generations WHERE state='ready'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 3);
}
#[test]
fn report_replay_and_revision_conflict_are_visible_at_cli() {
    let f = Fixture::new();
    let (t, run, epoch) = f.active();
    let args = [
        "agent",
        "report",
        "--run",
        &run,
        "--lease-epoch",
        &epoch,
        "--report-seq",
        "1",
        "--idempotency-key",
        "report-one",
        "--stage",
        "implementing",
        "--summary",
        "Synthetic progress",
    ];
    let first = f.ok(&args);
    let board = f.ok(&["board"]);
    let seq = board["data"]["as_of_seq"].as_i64().unwrap().to_string();
    let replay = f.ok(&args);
    assert_eq!(first["data"], replay["data"]);
    assert!(
        f.ok(&["activity", "--since-seq", &seq])["data"]["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let stale = f.run(&["task", "complete", &t, "--expect-revision", "1"]);
    assert_eq!(stale.status.code(), Some(9));
    assert_eq!(parse(&stale)["errors"][0]["code"], "REVISION_CONFLICT");
}
#[test]
fn changed_source_and_definition_do_not_reuse_check_evidence() {
    let f = Fixture::new();
    let (t, run, _) = f.active();
    let check = f.record(&t, &run);
    f.ok(&[
        "task",
        "criterion",
        "accept",
        &t,
        "--criterion",
        "behavior",
        "--evidence",
        &check,
    ]);
    f.ok(&["task", "submit", &t, "--run", &run]);
    f.ok(&[
        "task",
        "review",
        &t,
        "--approve",
        "--note",
        "Synthetic fixture reviewed",
    ]);
    fs::write(f.root.join("code.py"), "def value():\n    return 2\n").unwrap();
    let stale = f.run(&["task", "complete", &t]);
    assert_eq!(stale.status.code(), Some(10));
    assert_eq!(parse(&stale)["errors"][0]["code"], "COMPLETION_GATE_FAILED");
    let shown = f.ok(&["task", "show", &t]);
    let revision = shown["data"]["task_revision"].to_string();
    let path = f.definition("New acceptance definition");
    f.ok(&[
        "task",
        "edit",
        &t,
        "--from-file",
        path.to_str().unwrap(),
        "--expect-revision",
        &revision,
    ]);
    let gate = f.ok(&["task", "complete", &t, "--dry-run"]);
    assert_eq!(gate["data"]["passed"], false);
    assert!(
        gate["data"]["failures"]
            .as_array()
            .unwrap()
            .contains(&json!("submission_missing"))
    );
}
#[test]
fn agent_capability_cannot_grant_owner_authority() {
    let f = Fixture::new();
    let (t, run, epoch) = f.active();
    let args = ["agent", "heartbeat", "--run", &run, "--lease-epoch", &epoch];
    let denied = f
        .command(&args)
        .env("PCTX_ACTOR", "worker")
        .output()
        .unwrap();
    assert_eq!(denied.status.code(), Some(5));
    let workspace = fs::read_dir(f.data.join("workspaces"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let credential: Value = serde_json::from_slice(
        &fs::read(workspace.join("credentials").join(format!("{run}.json"))).unwrap(),
    )
    .unwrap();
    let capability = credential["capability"].as_str().unwrap();
    let heartbeat = f
        .command(&args)
        .env("PCTX_ACTOR", "worker")
        .env("PCTX_RUN_CAPABILITY", capability)
        .output()
        .unwrap();
    assert!(
        heartbeat.status.success(),
        "{}",
        String::from_utf8_lossy(&heartbeat.stdout)
    );
    let denied = f
        .command(&["task", "assign", &t, "--agent", "worker"])
        .env("PCTX_ACTOR", "worker")
        .env("PCTX_RUN_CAPABILITY", capability)
        .output()
        .unwrap();
    assert_eq!(denied.status.code(), Some(5));
    assert_eq!(parse(&denied)["errors"][0]["code"], "POLICY_DENIED");
}
#[test]
fn check_changed_during_external_execution_stays_stale() {
    let f = Fixture::new();
    let (t, run, _) = f.active();
    let c = f.ok(&["check", "begin", &t, "--key", "unit", "--run", &run])["data"]["check_id"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::write(
        f.root.join("code.py"),
        "def changed_during_check():\n    return 3\n",
    )
    .unwrap();
    let now = chrono::Utc::now().timestamp();
    let path = f.data.join("stale-report.json");
    fs::write(&path,json!({"schema_version":1,"check_key":"unit","producer":"synthetic-fixture","source":"external_report","exit_code":0,"tests":1,"passed":1,"failed":0,"errors":0,"skipped":0,"result":"passed","started_at":now,"finished_at":now}).to_string()).unwrap();
    let recorded = f.ok(&["check", "record", &c, "--from-file", path.to_str().unwrap()]);
    assert_eq!(recorded["data"]["result"], "stale");
    let accept = f.run(&[
        "task",
        "criterion",
        "accept",
        &t,
        "--criterion",
        "behavior",
        "--evidence",
        &c,
    ]);
    assert_eq!(accept.status.code(), Some(10));
    f.ok(&["task", "submit", &t, "--run", &run]);
    f.ok(&["task", "review", &t, "--approve"]);
    let gate = f.ok(&["task", "complete", &t, "--dry-run"]);
    assert_eq!(gate["data"]["passed"], false);
    assert!(
        gate["data"]["failures"]
            .as_array()
            .unwrap()
            .contains(&json!("check:unit"))
    );
}
#[test]
fn agent_cannot_authorize_execution_trust() {
    let f = Fixture::new();
    let plan = f.ok(&["trust", "plan", "--", "printf", "synthetic-fixture"]);
    let fingerprint = plan["data"]["fingerprint"].as_str().unwrap();
    let o = f
        .command(&[
            "trust",
            "add",
            "--expect-hash",
            fingerprint,
            "--",
            "printf",
            "synthetic-fixture",
        ])
        .env("PCTX_ACTOR", "worker")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(5));
    assert_eq!(parse(&o)["errors"][0]["code"], "POLICY_DENIED");
}
#[cfg(unix)]
#[test]
fn git_status_does_not_run_unbound_repository_fsmonitor() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let git = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&f.root)
        .output();
    let Ok(git) = git else {
        return;
    };
    assert!(git.status.success());
    let hook = f._temp.path().join("fsmonitor-hook");
    let marker = f.root.join("hook-executed");
    fs::write(
        &hook,
        "#!/bin/sh\nprintf 'unexpected hook execution' > hook-executed\nprintf '\\0'\n",
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    let configured = Command::new("git")
        .args(["config", "core.fsmonitor", hook.to_str().unwrap()])
        .current_dir(&f.root)
        .output()
        .unwrap();
    assert!(configured.status.success());
    let planned = f.run(&["trust", "plan", "--", "git", "status", "--porcelain"]);
    if !planned.status.success() {
        assert!([Some(5), Some(6)].contains(&planned.status.code()));
        return;
    }
    let plan = parse(&planned);
    let fingerprint = plan["data"]["fingerprint"].as_str().unwrap();
    f.ok(&[
        "trust",
        "add",
        "--expect-hash",
        fingerprint,
        "--",
        "git",
        "status",
        "--porcelain",
    ]);
    let _ = f.run(&[
        "run",
        "--budget-bytes",
        "5000",
        "--",
        "git",
        "status",
        "--porcelain",
    ]);
    assert!(
        !marker.exists(),
        "PCTX executed a repo-configured hook not included in the trust binding"
    );
}
#[test]
fn session_ack_and_epoch_cannot_be_changed_by_other_agent() {
    let f = Fixture::new();
    let (task, run, _) = f.active();
    f.register("other");
    let own = f.ok(&[
        "session",
        "attach",
        "--agent",
        "worker",
        "--runtime",
        "manual",
    ])["data"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let foreign = f.ok(&[
        "session",
        "attach",
        "--agent",
        "other",
        "--runtime",
        "manual",
    ])["data"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let workspace = fs::read_dir(f.data.join("workspaces"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let credential: Value = serde_json::from_slice(
        &fs::read(workspace.join("credentials").join(format!("{run}.json"))).unwrap(),
    )
    .unwrap();
    let token = credential["capability"].as_str().unwrap();
    let invoke = |args: &[&str], actor: &str| {
        f.command(args)
            .env("PCTX_ACTOR", actor)
            .env("PCTX_RUN_ID", &run)
            .env("PCTX_RUN_CAPABILITY", token)
            .output()
            .unwrap()
    };
    let own_context = parse_ok(
        invoke(
            &[
                "context",
                "get",
                "--task-id",
                &task,
                "--session",
                &own,
                "--scope",
                "code.py",
            ],
            "worker",
        ),
        &[],
    );
    let context = own_context["data"]["context_id"].as_str().unwrap();
    assert!(
        invoke(
            &["context", "ack", context, "--session", &own, "--epoch", "1"],
            "worker"
        )
        .status
        .success()
    );
    for args in [
        vec!["session", "boundary", "--session", foreign.as_str()],
        vec!["session", "suspend", "--session", foreign.as_str()],
        vec![
            "context",
            "get",
            "--task-id",
            task.as_str(),
            "--session",
            foreign.as_str(),
        ],
        vec![
            "context",
            "ack",
            context,
            "--session",
            foreign.as_str(),
            "--epoch",
            "1",
        ],
    ] {
        let denied = invoke(&args, "worker");
        assert_eq!(denied.status.code(), Some(5));
    }
    let impersonated = invoke(&["session", "boundary", "--session", &own], "other");
    assert_eq!(impersonated.status.code(), Some(5));
    let after = f.ok(&["session", "reconcile", "--session", &foreign]);
    assert_eq!(after["data"]["context_epoch"], 1);
}

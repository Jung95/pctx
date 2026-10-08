use pctx::{
    broker::{self, RepoCommand},
    project::{Config, Project, ProjectConfig},
};
use serde_json::Value;
use std::{
    fs,
    process::Command,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

fn fixture(git: bool) -> (tempfile::TempDir, Project) {
    let temp = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(temp.path()).unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(data.join("control")).unwrap();
    fs::write(root.join("input.txt"), "source\n").unwrap();
    if git {
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
    }
    let p = Project {
        root_anchor: pctx::project::RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspace"),
        control_dir: data.join("control"),
        project_id: "project-test".into(),
        workspace_id: "workspace-test".into(),
        coordination_id: "coordination-test".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "project-test".into(),
                name: "test".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    (temp, p)
}
fn status(p: &Project) -> Value {
    broker::repo(
        p,
        &RepoCommand::Status {
            fields: "branch,dirty,head,counts".into(),
            workspace: "current".into(),
        },
    )
    .unwrap()
}
#[test]
fn concurrent_refresh_is_shared_but_workspace_and_policy_keys_are_isolated() {
    let (_t, p) = fixture(true);
    let barrier = Arc::new(Barrier::new(5));
    let mut children = Vec::new();
    for _ in 0..5 {
        let p = p.clone();
        let barrier = barrier.clone();
        children.push(thread::spawn(move || {
            barrier.wait();
            status(&p)
        }));
    }
    let results = children
        .into_iter()
        .map(|c| c.join().unwrap())
        .collect::<Vec<_>>();
    for value in &results {
        assert_eq!(value["source_revision"], results[0]["source_revision"]);
        assert_eq!(value["source_generation"], 1);
        assert_eq!(value["source_status"], "ok_nonempty");
        assert_eq!(value["items"]["dirty"], true);
    }
    assert_eq!(broker::stats(&p).unwrap()["upstream_refreshes"], 1);
    let mut other = p.clone();
    other.workspace_id = "separate-worktree".into();
    let value = status(&other);
    assert_eq!(value["source_generation"], 1);
    assert_eq!(broker::stats(&other).unwrap()["upstream_refreshes"], 1);
    other.config.policy.exclude.push("input.txt".into());
    let value = status(&other);
    assert_eq!(value["cache_status"], "refreshed");
    assert_eq!(broker::stats(&other).unwrap()["upstream_refreshes"], 1);
}
#[test]
fn non_git_is_unsupported_and_corrupt_git_is_never_empty_or_clean() {
    let (_t, p) = fixture(false);
    let value = status(&p);
    assert_eq!(value["source_status"], "unsupported");
    assert_eq!(value["items"]["reason"], "no_git_repository");
    assert!(value["items"].get("dirty").is_none());
    let (_t, p) = fixture(true);
    fs::write(p.root.join(".git/index"), "not a valid index").unwrap();
    let error = broker::repo(
        &p,
        &RepoCommand::Status {
            fields: "dirty".into(),
            workspace: "current".into(),
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "SOURCE_UNAVAILABLE");
}
#[test]
fn field_selection_and_nul_filenames_do_not_emit_paths_or_source() {
    let (_t, p) = fixture(true);
    fs::write(p.root.join("name with\nnewline.txt"), "private source body").unwrap();
    let value = broker::repo(
        &p,
        &RepoCommand::Status {
            fields: "dirty,counts".into(),
            workspace: "current".into(),
        },
    )
    .unwrap();
    assert_eq!(value["items"]["counts"]["untracked"], 2);
    assert!(value["items"].get("head").is_none());
    let rendered = value.to_string();
    assert!(!rendered.contains("private source body"));
    assert!(!rendered.contains("newline.txt"));
    assert!(!rendered.contains(p.root.to_str().unwrap()));
    assert_eq!(
        broker::repo(
            &p,
            &RepoCommand::Status {
                fields: "password".into(),
                workspace: "current".into()
            }
        )
        .unwrap_err()
        .code,
        "INVALID_ARGUMENT"
    );
}
#[test]
fn live_refresh_owner_is_not_stolen_by_age_and_returns_pending() {
    let (_t, p) = fixture(true);
    let first = status(&p);
    let db = p.connect(true).unwrap();
    db.execute("UPDATE broker_snapshots SET observed_ms=0", [])
        .unwrap();
    db.execute(
        "UPDATE broker_refresh_jobs SET owner_active=1,claimed_ms=0",
        [],
    )
    .unwrap();
    let started = Instant::now();
    let next = broker::repo_with_timeout(
        &p,
        &RepoCommand::Status {
            fields: "branch,dirty,head,counts".into(),
            workspace: "current".into(),
        },
        Duration::from_millis(150),
    )
    .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(150));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "Bounded waiter did not return"
    );
    assert_eq!(next["refresh_status"], "refresh_pending");
    assert_eq!(next["freshness"], "stale");
    assert_eq!(next["source_revision"], first["source_revision"]);
    assert_eq!(broker::stats(&p).unwrap()["upstream_refreshes"], 1);
}
#[cfg(unix)]
#[test]
fn repo_fsmonitor_script_is_never_executed() {
    let (_t, p) = fixture(true);
    let marker = p.root.join("helper-ran");
    let script = p.root.join("monitor.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        Command::new("git")
            .args(["config", "core.fsmonitor", script.to_str().unwrap()])
            .current_dir(&p.root)
            .status()
            .unwrap()
            .success()
    );
    let _ = status(&p);
    assert!(!marker.exists());
}

#[cfg(unix)]
#[test]
fn proven_dead_refresh_owner_can_be_reclaimed_without_ttl_guessing() {
    let (_t, p) = fixture(true);
    let _ = status(&p);
    let mut child = Command::new("/bin/sleep").arg("5").spawn().unwrap();
    let dead_pid = child.id();
    child.kill().unwrap();
    child.wait().unwrap();
    let db = p.connect(true).unwrap();
    db.execute("UPDATE broker_snapshots SET observed_ms=0", [])
        .unwrap();
    db.execute(
        "UPDATE broker_refresh_jobs SET owner_active=1,pid=?1,start_identity='old-process'",
        [dead_pid],
    )
    .unwrap();
    let next = status(&p);
    assert_eq!(next["refresh_status"], "idle");
    assert_eq!(next["source_generation"], 2);
    assert_eq!(broker::stats(&p).unwrap()["upstream_refreshes"], 2);
}

#[test]
fn five_actual_cli_processes_share_one_refresh() {
    let (_t, p) = fixture(true);
    let binary = env!("CARGO_BIN_EXE_pctx");
    let invoke = |args: &[&str]| {
        Command::new(binary)
            .env("PCTX_DATA_DIR", &p.data_dir)
            .env("PCTX_ACTOR", "owner")
            .arg("--root")
            .arg(&p.root)
            .arg("--format")
            .arg("json")
            .args(args)
            .output()
            .unwrap()
    };
    let init = invoke(&["init"]);
    assert!(
        init.status.success(),
        "init status={} stdout={} stderr={}",
        init.status,
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    let mut children = Vec::new();
    for _ in 0..5 {
        children.push(
            Command::new(binary)
                .env("PCTX_DATA_DIR", &p.data_dir)
                .env("PCTX_ACTOR", "owner")
                .arg("--root")
                .arg(&p.root)
                .args(["--format", "json", "repo", "status"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    let results = children
        .into_iter()
        .map(|child| {
            let result = child.wait_with_output().unwrap();
            assert!(
                result.status.success(),
                "repo status={} stdout={} stderr={}",
                result.status,
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            serde_json::from_slice::<Value>(&result.stdout).unwrap()
        })
        .collect::<Vec<_>>();
    for result in &results {
        assert_eq!(result["data"]["source_generation"], 1);
        assert_eq!(
            result["data"]["source_revision"],
            results[0]["data"]["source_revision"]
        );
    }
    let stats = invoke(&["cache", "stats"]);
    assert!(
        stats.status.success(),
        "cache stats status={} stdout={} stderr={}",
        stats.status,
        String::from_utf8_lossy(&stats.stdout),
        String::from_utf8_lossy(&stats.stderr)
    );
    let value: Value = serde_json::from_slice(&stats.stdout).unwrap();
    assert_eq!(value["data"]["upstream_refreshes"], 1);
}

#[test]
fn default_waiter_receives_shared_publication_after_old_half_second_cutoff() {
    let (_t, p) = fixture(true);
    let first = status(&p);
    let db = p.connect(true).unwrap();
    db.execute("UPDATE broker_snapshots SET observed_ms=0", [])
        .unwrap();
    db.execute(
        "UPDATE broker_refresh_jobs SET owner_active=1,claimed_ms=0",
        [],
    )
    .unwrap();
    // A controlled live owner publishes the existing fixture observation after 750ms.
    // This tests the waiter protocol independently of machine-specific Git speed.
    let barrier = Arc::new(Barrier::new(2));
    let publisher_barrier = barrier.clone();
    let publisher_project = p.clone();
    let publisher = thread::spawn(move || {
        publisher_barrier.wait();
        thread::sleep(Duration::from_millis(750));
        let mut db = publisher_project.connect(true).unwrap();
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        assert_eq!(
            tx.execute(
                "UPDATE broker_snapshots SET observed_ms=?1 WHERE generation=1",
                [chrono::Utc::now().timestamp_millis()]
            )
            .unwrap(),
            1
        );
        assert_eq!(tx.execute("UPDATE broker_refresh_jobs SET owner_active=0 WHERE generation=1 AND owner_active=1", []).unwrap(), 1);
        tx.commit().unwrap();
    });
    barrier.wait();
    let started = Instant::now();
    let next = status(&p);
    publisher.join().unwrap();
    assert!(started.elapsed() >= Duration::from_millis(500));
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(next["refresh_status"], "idle");
    assert_eq!(next["freshness"], "current");
    assert_eq!(next["source_status"], "ok_nonempty");
    assert_eq!(next["source_revision"], first["source_revision"]);
    assert_eq!(next["source_generation"], 1);
    assert_eq!(broker::stats(&p).unwrap()["upstream_refreshes"], 1);
}

#[test]
fn bounded_wait_without_snapshot_is_unavailable_and_never_reclaims_live_owner() {
    let (_t, p) = fixture(true);
    let _ = status(&p);
    let db = p.connect(true).unwrap();
    db.execute("DELETE FROM broker_snapshots", []).unwrap();
    db.execute(
        "UPDATE broker_refresh_jobs SET owner_active=1,claimed_ms=0",
        [],
    )
    .unwrap();
    let started = Instant::now();
    let next = broker::repo_with_timeout(
        &p,
        &RepoCommand::Status {
            fields: "dirty,head".into(),
            workspace: "current".into(),
        },
        Duration::from_millis(150),
    )
    .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(150));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(next["refresh_status"], "refresh_pending");
    assert_eq!(next["source_status"], "unavailable");
    assert_eq!(next["coverage"]["status"], "partial");
    assert_eq!(next["freshness"], "stale");
    assert!(next["source_revision"].is_null());
    assert_eq!(next["items"], serde_json::json!({}));
    let stats = broker::stats(&p).unwrap();
    assert_eq!(stats["upstream_refreshes"], 1);
    assert_eq!(stats["refresh_jobs_active_or_unknown"], 1);
    assert_eq!(
        broker::repo_with_timeout(
            &p,
            &RepoCommand::Status {
                fields: "dirty".into(),
                workspace: "current".into(),
            },
            Duration::ZERO
        )
        .unwrap_err()
        .code,
        "INVALID_ARGUMENT"
    );
}

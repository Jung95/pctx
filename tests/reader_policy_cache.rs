use pctx::{
    project::{Config, Project, ProjectConfig},
    reader,
};
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(t.path().join("project")).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: pctx::project::RootAnchor::capture(&t.path().join("project")).unwrap(),
        root: t.path().join("project"),
        data_dir: t.path().join("data"),
        workspace_dir: t.path().join("data/ws"),
        control_dir: t.path().join("data/control"),
        project_id: "project".into(),
        workspace_id: "ws".into(),
        coordination_id: "coord".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "project".into(),
                name: "test".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    std::fs::create_dir_all(&p.root).unwrap();
    (t, p)
}

#[test]
fn compiled_patterns_never_cache_path_access_or_source_across_projects() {
    let (_a, mut a) = fixture();
    let (_b, mut b) = fixture();
    std::fs::write(a.root.join("visible.txt"), "first project").unwrap();
    std::fs::write(b.root.join("visible.txt"), "second project").unwrap();
    assert_eq!(
        reader::read(&a, "visible.txt").unwrap().text,
        "first project"
    );
    a.config.policy.exclude.push("visible.txt".into());
    assert_eq!(
        reader::read(&a, "visible.txt").unwrap_err().code,
        "POLICY_DENIED"
    );
    assert_eq!(
        reader::read(&b, "visible.txt").unwrap().text,
        "second project"
    );
    b.config.policy.exclude.push("[".into());
    assert_eq!(
        reader::read(&b, "visible.txt").unwrap_err().code,
        "INVALID_CONFIG"
    );
    a.config.policy.exclude.clear();
    std::fs::write(a.root.join("visible.txt"), "changed source").unwrap();
    assert_eq!(
        reader::read(&a, "visible.txt").unwrap().text,
        "changed source"
    );
    std::fs::write(a.root.join(".env"), "must stay excluded").unwrap();
    assert_eq!(reader::read(&a, ".env").unwrap_err().code, "POLICY_DENIED");
}
#[cfg(unix)]
#[test]
fn warmed_patterns_do_not_authorize_a_path_replaced_by_a_symlink() {
    let (temp, p) = fixture();
    let path = p.root.join("visible.txt");
    std::fs::write(&path, "inside").unwrap();
    reader::read(&p, "visible.txt").unwrap();
    let outside = temp.path().join("outside.txt");
    std::fs::write(&outside, "outside marker").unwrap();
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    assert_eq!(
        reader::read(&p, "visible.txt").unwrap_err().code,
        "POLICY_DENIED"
    );
}

#[test]
fn concurrent_eviction_keeps_each_exact_policy_and_current_source() {
    let handles: Vec<_> = (0..16)
        .map(|i| {
            std::thread::spawn(move || {
                let (_temp, mut p) = fixture();
                p.config.policy.exclude.push(format!("excluded-{i}.txt"));
                let excluded = format!("excluded-{i}.txt");
                std::fs::write(p.root.join(&excluded), "never readable").unwrap();
                std::fs::write(p.root.join("visible.txt"), format!("source-{i}")).unwrap();
                for _ in 0..20 {
                    assert_eq!(
                        reader::read(&p, &excluded).unwrap_err().code,
                        "POLICY_DENIED"
                    );
                    assert_eq!(
                        reader::read(&p, "visible.txt").unwrap().text,
                        format!("source-{i}")
                    );
                    assert_eq!(reader::read(&p, ".env").unwrap_err().code, "POLICY_DENIED");
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}

#[cfg(unix)]
#[test]
fn warmed_root_and_ancestor_replacement_cannot_redirect_current_source() {
    for replace_ancestor in [false, true] {
        let (temp, mut p) = fixture();
        let owner = temp.path().join("owner");
        std::fs::create_dir(&owner).unwrap();
        p.root = owner.join("project");
        std::fs::create_dir(&p.root).unwrap();
        p.root_anchor = pctx::project::RootAnchor::capture(&p.root).unwrap();
        std::fs::write(p.root.join("visible.txt"), "inside").unwrap();
        assert_eq!(reader::read(&p, "visible.txt").unwrap().text, "inside");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        let victim = if replace_ancestor {
            std::fs::create_dir(outside.join("project")).unwrap();
            owner.clone()
        } else {
            p.root.clone()
        };
        let target = if replace_ancestor {
            outside.join("project")
        } else {
            outside.clone()
        };
        std::fs::write(target.join("visible.txt"), "outside marker").unwrap();
        std::fs::rename(&victim, temp.path().join("original-root")).unwrap();
        std::os::unix::fs::symlink(&outside, &victim).unwrap();
        assert_eq!(
            reader::read(&p, "visible.txt").unwrap_err().code,
            "POLICY_DENIED"
        );
        assert_eq!(
            reader::authorize(&p, "visible.txt").unwrap_err().code,
            "POLICY_DENIED"
        );
        assert_eq!(
            reader::inventory(&p, false).unwrap_err().code,
            "POLICY_DENIED"
        );
        let request = pctx::search::FindRequest {
            query: Some("no-match".into()),
            boolean_query: None,
            kind: "symbol".into(),
            regex: false,
            snippet_lines: None,
            limit: 20,
            scopes: vec![],
            freshness: "off".into(),
            language: None,
            explain: false,
        };
        assert_eq!(
            pctx::search::find(&p, &[], &request).unwrap_err().code,
            "POLICY_DENIED"
        );
    }
}

#[cfg(unix)]
#[test]
fn initialized_project_rejects_normal_root_and_ancestor_replacement() {
    // A subprocess isolates data/user configuration without process-wide env
    // mutation in a concurrent Rust test harness. This uses real init/open and DB.
    if std::env::var("PCTX_ANCHOR_FIXTURE_CHILD").as_deref() != Ok("1") {
        let temp = tempfile::tempdir().unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "initialized_project_rejects_normal_root_and_ancestor_replacement",
                "--nocapture",
            ])
            .env("PCTX_ANCHOR_FIXTURE_CHILD", "1")
            .env("PCTX_ANCHOR_FIXTURE_BASE", temp.path())
            .env("PCTX_DATA_DIR", temp.path().join("data"))
            .env("PCTX_USER_CONFIG", temp.path().join("absent-user.toml"))
            .env_remove("PCTX_MAX_FILE_BYTES")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "status={} stdout={} stderr={}",
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    let base = std::path::PathBuf::from(std::env::var_os("PCTX_ANCHOR_FIXTURE_BASE").unwrap());
    for ancestor in [false, true] {
        let owner = base.join(if ancestor {
            "ancestor-case"
        } else {
            "root-case"
        });
        let root = owner.join("project");
        std::fs::create_dir_all(&root).unwrap();
        Project::init(&root).unwrap();
        let p = Project::open(&root).unwrap();
        std::fs::write(root.join("auth.py"), "def auth():\n    pass\n").unwrap();
        pctx::storage::update(&p).unwrap();
        let (_, files) = pctx::storage::snapshot(&p).unwrap();
        let old_owner = base.join(if ancestor {
            "original-ancestor"
        } else {
            "original-root"
        });
        if ancestor {
            std::fs::rename(&owner, &old_owner).unwrap();
            std::fs::create_dir(&owner).unwrap();
            // Preserve the root inode while replacing only its ancestor instance.
            // Comparing just the final root handle would miss this replacement.
            std::fs::rename(old_owner.join("project"), &root).unwrap();
        } else {
            std::fs::rename(&root, &old_owner).unwrap();
            std::fs::create_dir(&root).unwrap();
            std::fs::write(
                root.join("auth.py"),
                "def auth():\n    return 'replacement'\n",
            )
            .unwrap();
        }
        for code in [
            reader::read(&p, "auth.py").unwrap_err().code,
            reader::inventory(&p, false).unwrap_err().code,
            pctx::storage::snapshot(&p).unwrap_err().code,
        ] {
            assert_eq!(code, "POLICY_DENIED");
        }
        let request = pctx::search::FindRequest {
            query: Some("never_matches".into()),
            boolean_query: None,
            kind: "symbol".into(),
            regex: false,
            snippet_lines: None,
            limit: 20,
            scopes: vec![],
            freshness: "off".into(),
            language: None,
            explain: false,
        };
        assert_eq!(
            pctx::search::find(&p, &files, &request).unwrap_err().code,
            "POLICY_DENIED"
        );
        assert_eq!(
            pctx::search::find(&p, &[], &request).unwrap_err().code,
            "POLICY_DENIED"
        );
    }
}

#[test]
fn immutable_root_anchor_accepts_in_place_and_atomic_source_edits() {
    let (_temp, p) = fixture();
    let clone = p.clone();
    let path = p.root.join("visible.txt");
    std::fs::write(&path, "first bytes").unwrap();
    let first = reader::read(&p, "visible.txt").unwrap();
    std::fs::write(&path, "other bytes").unwrap();
    let second = reader::read(&clone, "visible.txt").unwrap();
    assert_ne!(first.hash, second.hash);
    let staging = p.root.join("replacement.txt");
    std::fs::write(&staging, "atomic replacement").unwrap();
    #[cfg(windows)]
    std::fs::remove_file(&path).unwrap();
    std::fs::rename(&staging, &path).unwrap();
    assert_eq!(
        reader::read(&p, "visible.txt").unwrap().text,
        "atomic replacement"
    );
    assert_eq!(
        reader::read(&clone, "visible.txt").unwrap().text,
        "atomic replacement"
    );
}

#[cfg(unix)]
#[test]
fn cloned_anchor_never_rebinds_after_root_is_deleted_and_recreated() {
    let (_temp, p) = fixture();
    let clone = p.clone();
    std::fs::remove_dir(&p.root).unwrap();
    std::fs::create_dir(&p.root).unwrap();
    std::fs::write(p.root.join("visible.txt"), "new root").unwrap();
    // Open anchor handles keep the deleted directory identity pinned against
    // reuse; neither a new path at the same name nor a clone gains authority.
    assert_eq!(
        reader::read(&p, "visible.txt").unwrap_err().code,
        "POLICY_DENIED"
    );
    assert_eq!(
        reader::read(&clone, "visible.txt").unwrap_err().code,
        "POLICY_DENIED"
    );
}

#[cfg(unix)]
#[test]
fn fifo_source_and_configuration_are_rejected_without_blocking() {
    if std::env::var("PCTX_FIFO_FIXTURE_CHILD").as_deref() != Ok("1") {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "fifo_source_and_configuration_are_rejected_without_blocking",
                "--nocapture",
            ])
            .env("PCTX_FIFO_FIXTURE_CHILD", "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let started = std::time::Instant::now();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > std::time::Duration::from_secs(5) {
                child.kill().unwrap();
                let result = child.wait_with_output().unwrap();
                panic!(
                    "FIFO admission blocked: {} {}",
                    String::from_utf8_lossy(&result.stdout),
                    String::from_utf8_lossy(&result.stderr)
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    use std::os::unix::ffi::OsStrExt;
    let (_temp, p) = fixture();
    let fifo = p.root.join("source.txt");
    let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert_eq!(
        reader::read(&p, "source.txt").unwrap_err().code,
        "INVALID_ARGUMENT"
    );
    std::fs::create_dir(p.root.join(".pctx")).unwrap();
    let config = p.root.join(".pctx/config.toml");
    let name = std::ffi::CString::new(config.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert_eq!(Project::open(&p.root).unwrap_err().code, "INVALID_CONFIG");
}

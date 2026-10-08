use pctx::{
    project::{Config, Project, ProjectConfig},
    reader,
};
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    let p = Project {
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

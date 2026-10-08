use pctx::{domain::hash, search::analyze};

#[test]
fn structural_variants_use_real_grammar() {
    for (path, source, expected) in [
        (
            "x.py",
            "class Auth:\n    async def login(self):\n        return 'secret'\n",
            vec!["Auth", "login"],
        ),
        (
            "x.js",
            "class Auth { login() { return 'secret'; } }\nconst run = () => 1;",
            vec!["Auth", "login", "run"],
        ),
        (
            "x.jsx",
            "function Login() { return <div>Hello</div>; }",
            vec!["Login"],
        ),
        (
            "x.ts",
            "interface Auth { login(): void; }\ntype Id = string;\nenum Role { User }",
            vec!["Auth", "login", "Id", "Role"],
        ),
        (
            "x.tsx",
            "const Login = () => <div>Hello</div>;",
            vec!["Login"],
        ),
    ] {
        let entry = analyze(path, &hash(source), source).unwrap();
        assert_eq!(entry.parse_status, "complete", "{path}");
        for name in expected {
            assert!(
                entry.symbols.iter().any(|s| s.name == name),
                "{path} {name}"
            );
        }
        assert!(!serde_json::to_string(&entry).unwrap().contains("secret"));
        for symbol in &entry.symbols {
            assert!(source.get(symbol.start_byte..symbol.end_byte).is_some());
        }
    }
}
#[test]
fn utf8_crlf_locations_and_version_ids() {
    let source = "# 한글 🦀\r\ndef login():\r\n    return 1\r\n";
    let a = analyze("name with space\nand newline.py", &hash(source), source).unwrap();
    let s = &a.symbols[0];
    assert_eq!(s.start_line, 2);
    assert_eq!(s.end_line, 3);
    assert_eq!(
        &source[s.start_byte..s.end_byte],
        "def login():\r\n    return 1"
    );
    let b = analyze(
        &a.path,
        &hash(format!("{source}\n")),
        &format!("{source}\n"),
    )
    .unwrap();
    assert_ne!(s.id, b.symbols[0].id);
}
#[test]
fn unsupported_is_distinct_from_empty_supported() {
    let unsupported = analyze("x.rb", &hash("puts 1"), "puts 1").unwrap();
    assert_eq!(unsupported.parse_status, "unsupported");
    let empty = analyze("x.py", &hash("x=1"), "x=1").unwrap();
    assert_eq!(empty.parse_status, "complete");
    assert!(empty.symbols.is_empty());
}
#[test]
fn markdown_sections_ignore_fenced_examples_and_link_parents() {
    let text = "# Root\nhello\n## Child\n```md\n# Not a heading\n```\n# Next\n";
    let e = analyze("README.md", &hash(text), text).unwrap();
    assert_eq!(e.symbols.len(), 3);
    let root = e.symbols.iter().find(|s| s.name == "Root").unwrap();
    let child = e.symbols.iter().find(|s| s.name == "Child").unwrap();
    assert_eq!(child.parent_symbol_id.as_ref(), Some(&root.id));
    assert_eq!(root.end_line, 6);
}

fn project(root: &std::path::Path) -> pctx::project::Project {
    use pctx::project::{Config, Project, ProjectConfig};
    Project {
        deadline: None,
        root_anchor: pctx::project::RootAnchor::capture(root).unwrap(),
        root: root.to_owned(),
        data_dir: root.join("data"),
        workspace_dir: root.join("workspace"),
        control_dir: root.join("control"),
        project_id: "project".into(),
        workspace_id: "workspace".into(),
        coordination_id: "control".into(),
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
    }
}

#[test]
fn expired_queries_fail_even_when_the_candidate_universe_is_empty() {
    use pctx::{
        deadline::Deadline,
        search::{StructureRequest, find, find_indexed, outline, query_structure, read_selection},
    };
    let temp = tempfile::tempdir().unwrap();
    let mut p = project(temp.path());
    let deadline = Deadline::from_millis(1).unwrap();
    while deadline.check().is_ok() {
        std::hint::spin_loop();
    }
    p.deadline = Some(deadline);
    let req = metadata_request("path", "absent");
    for result in [
        find(&p, &[], &req),
        find_indexed(&p, &req),
        outline(&p, &[], ".", None, "matched"),
        read_selection(&p, &[], Some("absent.py"), None, None, None),
        query_structure(
            &p,
            &[],
            &StructureRequest {
                language: "python".into(),
                kind: "function".into(),
                modifier: None,
                scopes: vec![],
                limit: 20,
                freshness: "matched".into(),
            },
        ),
    ] {
        let error = result.unwrap_err();
        assert_eq!(error.code, "TIMEOUT");
        assert_eq!(error.exit, 7);
    }
}

#[test]
fn deadline_partial_keeps_only_verified_matches_and_never_claims_complete_coverage() {
    use pctx::{deadline::Deadline, search::find};
    let temp = tempfile::tempdir().unwrap();
    let mut p = project(temp.path());
    let source = "verified needle\n";
    std::fs::write(p.root.join("current.txt"), source).unwrap();
    let entry = analyze("current.txt", &hash(source), source).unwrap();
    // Every candidate must undergo real current-source validation. Repetition
    // supplies bounded work to the public provided-index API without a hook,
    // sleeping reader, or assuming that any unobserved candidate is safe.
    let files = vec![entry; 100_001];
    let req = metadata_request("text", "needle");
    p.deadline = Some(Deadline::from_millis(250).unwrap());
    let value = find(&p, &files, &req).unwrap();
    assert_eq!(value["coverage"]["status"], "partial");
    assert!(
        value["coverage"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "timeout")
    );
    assert!(value["omitted_count"].is_null());
    let items = value["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for item in items {
        assert_eq!(item["path"], "current.txt");
        assert_eq!(item["file_hash"], hash(source));
        assert_eq!(item["freshness"], "current");
        assert_eq!(item["line_numbers"], serde_json::json!([1]));
    }
    assert_eq!(
        p.check_deadline().unwrap_err().code,
        "TIMEOUT",
        "Nested search renewed the supplied budget"
    );
    // A zero result limit retains no usable match, even if matches were found
    // before expiry. It must not turn an expired request into an empty partial.
    let mut no_items = req;
    no_items.limit = 0;
    p.deadline = Some(Deadline::from_millis(250).unwrap());
    let error = find(&p, &files, &no_items).unwrap_err();
    assert_eq!(error.code, "TIMEOUT");
    assert_eq!(error.exit, 7);
}

#[test]
fn parsing_cancellation_is_timeout_instead_of_empty_or_partial_ast_success() {
    use pctx::{deadline::Deadline, search::analyze_with_deadline};
    let source = "def expanded():\n    return 1\n".repeat(30_000);
    let digest = hash(&source);
    let error = analyze_with_deadline(
        "expanded.py",
        &digest,
        &source,
        Some(Deadline::from_millis(1).unwrap()),
    )
    .unwrap_err();
    assert_eq!(error.code, "TIMEOUT");
    assert_eq!(error.exit, 7);
}

#[test]
fn candidate_scan_cap_remains_distinct_from_request_timeout() {
    use pctx::{deadline::Deadline, search::find};
    let temp = tempfile::tempdir().unwrap();
    let mut p = project(temp.path());
    let source = "ordinary source\n";
    std::fs::write(p.root.join("current.txt"), source).unwrap();
    let entry = analyze("current.txt", &hash(source), source).unwrap();
    let files = vec![entry; 100_001];
    p.deadline = Some(Deadline::from_millis(60_000).unwrap());
    let value = find(&p, &files, &metadata_request("path", "unmatched")).unwrap();
    assert_eq!(value["coverage"]["status"], "partial");
    assert_eq!(
        value["coverage"]["reasons"],
        serde_json::json!(["scan_cap"])
    );
    assert_eq!(value["scanned_files"], 100_000);
    assert!(value["items"].as_array().unwrap().is_empty());
    assert!(p.check_deadline().is_ok());
}
#[test]
fn stale_symbol_and_ambiguous_names_never_select_code() {
    use pctx::search::read_selection;
    let temp = tempfile::tempdir().unwrap();
    let p = project(temp.path());
    let text = "def login():\n    return 1\ndef login():\n    return 2\n";
    std::fs::write(temp.path().join("auth.py"), text).unwrap();
    let f = analyze("auth.py", &hash(text), text).unwrap();
    assert_eq!(
        read_selection(
            &p,
            std::slice::from_ref(&f),
            Some("auth.py"),
            None,
            None,
            Some("login")
        )
        .unwrap_err()
        .code,
        "AMBIGUOUS_SYMBOL"
    );
    std::fs::write(temp.path().join("auth.py"), "def login():\n    return 3\n").unwrap();
    assert_eq!(
        read_selection(
            &p,
            std::slice::from_ref(&f),
            None,
            None,
            Some(&f.symbols[0].id),
            None
        )
        .unwrap_err()
        .code,
        "STALE_INDEX"
    );
    let lines = read_selection(&p, &[f], Some("auth.py"), Some("1:2"), None, None).unwrap();
    assert!(lines["text"].as_str().unwrap().contains("return 3"));
}
#[test]
fn cached_policy_change_hides_metadata_and_boolean_remains_explicit() {
    use pctx::search::{FindRequest, find};
    let temp = tempfile::tempdir().unwrap();
    let mut p = project(temp.path());
    let text = "def auth():\n    return 1\n";
    std::fs::write(temp.path().join("auth.py"), text).unwrap();
    let files = vec![analyze("auth.py", &hash(text), text).unwrap()];
    let mut req = FindRequest {
        query: None,
        boolean_query: Some("auth AND NOT legacy".into()),
        kind: "all".into(),
        regex: false,
        snippet_lines: None,
        limit: 20,
        scopes: vec![],
        freshness: "matched".into(),
        language: None,
        explain: true,
    };
    let value = find(&p, &files, &req).unwrap();
    assert_eq!(value["items"].as_array().unwrap().len(), 1);
    assert!(value["items"][0].get("text").is_none());
    req.boolean_query = None;
    req.query = Some("auth AND NOT legacy".into());
    assert!(
        find(&p, &files, &req).unwrap()["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    req.query = Some("auth".into());
    p.config.policy.exclude.push("auth.py".into());
    assert!(
        find(&p, &files, &req).unwrap()["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn metadata_request(kind: &str, query: &str) -> pctx::search::FindRequest {
    pctx::search::FindRequest {
        query: Some(query.into()),
        boolean_query: None,
        kind: kind.into(),
        regex: false,
        snippet_lines: None,
        limit: 20,
        scopes: vec![],
        freshness: "matched".into(),
        language: None,
        explain: false,
    }
}

#[test]
fn metadata_non_candidates_are_not_opened_as_source() {
    use pctx::search::find;
    for (kind, candidate_path, candidate, other_path, other) in [
        (
            "path",
            "auth.py",
            "def login():\n    pass\n",
            "legacy.py",
            "def legacy():\n    pass\n",
        ),
        (
            "symbol",
            "current.py",
            "def auth():\n    pass\n",
            "legacy.py",
            "def legacy():\n    pass\n",
        ),
        (
            "document",
            "current.md",
            "# auth\n",
            "legacy.md",
            "# legacy\n",
        ),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let p = project(temp.path());
        std::fs::write(temp.path().join(candidate_path), candidate).unwrap();
        // A real read of this authorized noncandidate would fail, so success
        // demonstrates a source-read boundary rather than just matching output.
        std::fs::write(temp.path().join(other_path), [0xff, 0xfe]).unwrap();
        let files = vec![
            analyze(other_path, &hash(other), other).unwrap(),
            analyze(candidate_path, &hash(candidate), candidate).unwrap(),
        ];
        assert_eq!(
            pctx::reader::read(&p, other_path).unwrap_err().code,
            "UNSUPPORTED_ENCODING"
        );
        let value = find(&p, &files, &metadata_request(kind, "auth")).unwrap();
        assert_eq!(value["items"].as_array().unwrap().len(), 1, "{kind}");
        assert_eq!(value["items"][0]["path"], candidate_path);
        assert_eq!(value["items"][0]["freshness"], "current");
        assert_eq!(value["coverage"]["universe"], "provided_index");
        assert_eq!(
            value["coverage"]["source_validation"],
            "matching_metadata_candidates"
        );
    }
}

#[test]
fn matched_metadata_validates_stale_candidates_without_discovering_new_names() {
    use pctx::search::{find, read_selection};
    let temp = tempfile::tempdir().unwrap();
    let p = project(temp.path());
    let old_match = "def auth():\n    return 1\n";
    let old_other = "def legacy():\n    pass\n";
    let current_match = "def renamed():\n    return 2\n";
    let current_other = "def auth():\n    pass\n";
    std::fs::write(temp.path().join("matched.py"), current_match).unwrap();
    std::fs::write(temp.path().join("other.py"), current_other).unwrap();
    let files = vec![
        analyze("matched.py", &hash(old_match), old_match).unwrap(),
        analyze("other.py", &hash(old_other), old_other).unwrap(),
        analyze("deleted.py", &hash(old_other), old_other).unwrap(),
    ];
    let value = find(&p, &files, &metadata_request("symbol", "auth")).unwrap();
    assert_eq!(value["items"].as_array().unwrap().len(), 1);
    let item = &value["items"][0];
    assert_eq!(item["path"], "matched.py");
    assert_eq!(item["freshness"], "stale");
    assert_eq!(item["file_hash"], hash(current_match));
    assert!(item["line_numbers"].as_array().unwrap().is_empty());
    assert!(item.get("symbols").is_none());
    assert_eq!(
        value["coverage"]["new_candidate_discovery"],
        "not_performed_by_find"
    );
    assert_eq!(
        read_selection(&p, &files, None, None, Some(&files[0].symbols[0].id), None)
            .unwrap_err()
            .code,
        "STALE_INDEX"
    );
    // A refreshed index, as supplied by the CLI's strict update, discovers the
    // new candidate and removes the stale match and deleted file.
    let refreshed = vec![
        analyze("matched.py", &hash(current_match), current_match).unwrap(),
        analyze("other.py", &hash(current_other), current_other).unwrap(),
    ];
    let mut strict = metadata_request("symbol", "auth");
    strict.freshness = "strict".into();
    let value = find(&p, &refreshed, &strict).unwrap();
    assert_eq!(value["items"].as_array().unwrap().len(), 1);
    assert_eq!(value["items"][0]["path"], "other.py");
    assert_eq!(value["items"][0]["freshness"], "current");
}

#[test]
fn boolean_complements_validate_candidates_without_positive_hits() {
    use pctx::search::find;
    let temp = tempfile::tempdir().unwrap();
    let p = project(temp.path());
    let old_clean = "def clean():\n    pass\n";
    let new_clean = "def clean():\n    return 2\n";
    let old_legacy = "def legacy():\n    pass\n";
    std::fs::write(temp.path().join("clean.py"), new_clean).unwrap();
    std::fs::write(temp.path().join("legacy.py"), [0xff]).unwrap();
    let files = vec![
        analyze("clean.py", &hash(old_clean), old_clean).unwrap(),
        analyze("legacy.py", &hash(old_legacy), old_legacy).unwrap(),
    ];
    let mut req = metadata_request("symbol", "unused");
    req.query = None;
    req.boolean_query = Some("auth OR NOT legacy".into());
    let value = find(&p, &files, &req).unwrap();
    assert_eq!(value["items"].as_array().unwrap().len(), 1);
    assert_eq!(value["items"][0]["path"], "clean.py");
    assert_eq!(value["items"][0]["score"], 0);
    assert_eq!(value["items"][0]["freshness"], "stale");
    // A complement-only candidate is still read, and its read errors propagate.
    std::fs::write(temp.path().join("clean.py"), [0xff]).unwrap();
    assert_eq!(
        find(&p, &files, &req).unwrap_err().code,
        "UNSUPPORTED_ENCODING"
    );
    req.boolean_query = Some("NOT legacy".into());
    assert!(find(&p, &files, &req).is_err());
}

#[test]
fn alias_and_aggregate_regex_candidates_use_current_validation() {
    use pctx::search::find;
    let temp = tempfile::tempdir().unwrap();
    let p = project(temp.path());
    std::fs::create_dir(temp.path().join(".pctx")).unwrap();
    std::fs::write(
        temp.path().join(".pctx/glossary.toml"),
        "[aliases]\nauth = [\"login\"]\n",
    )
    .unwrap();
    let old_match = "def login():\n    pass\ndef logout():\n    pass\n";
    let changed = "def login():\n    return 2\ndef logout():\n    pass\n";
    let other = "def unrelated():\n    pass\n";
    std::fs::write(temp.path().join("match.py"), changed).unwrap();
    std::fs::write(temp.path().join("other.py"), [0xff]).unwrap();
    let files = vec![
        analyze("other.py", &hash(other), other).unwrap(),
        analyze("match.py", &hash(old_match), old_match).unwrap(),
    ];
    let value = find(&p, &files, &metadata_request("symbol", "auth")).unwrap();
    assert_eq!(value["items"][0]["path"], "match.py");
    assert_eq!(value["items"][0]["freshness"], "stale");
    let mut regex = metadata_request("symbol", "^login\\nlogout$");
    regex.regex = true;
    let value = find(&p, &files, &regex).unwrap();
    assert_eq!(value["items"].as_array().unwrap().len(), 1);
    assert_eq!(value["items"][0]["file_hash"], hash(changed));
}

#[test]
fn oversized_non_candidate_is_skipped_but_matching_candidate_fails() {
    use pctx::search::find;
    let temp = tempfile::tempdir().unwrap();
    let mut p = project(temp.path());
    p.config.index.max_file_bytes = 64;
    let cached = "def legacy():\n    pass\n";
    std::fs::write(temp.path().join("legacy.py"), vec![b'x'; 65]).unwrap();
    let files = vec![analyze("legacy.py", &hash(cached), cached).unwrap()];
    let value = find(&p, &files, &metadata_request("symbol", "auth")).unwrap();
    assert!(value["items"].as_array().unwrap().is_empty());
    assert_eq!(
        find(&p, &files, &metadata_request("symbol", "legacy"))
            .unwrap_err()
            .code,
        "FILE_TOO_LARGE"
    );
}

#[test]
fn text_and_all_still_read_sources_without_metadata_hits_and_policy_hides_candidates() {
    use pctx::search::find;
    let temp = tempfile::tempdir().unwrap();
    let mut p = project(temp.path());
    let original = "def unrelated():\n    return 'auth'\n";
    std::fs::write(temp.path().join("other.py"), original).unwrap();
    let files = vec![analyze("other.py", &hash(original), original).unwrap()];
    for kind in ["text", "all"] {
        let value = find(&p, &files, &metadata_request(kind, "auth")).unwrap();
        assert_eq!(value["items"][0]["path"], "other.py");
        std::fs::write(temp.path().join("other.py"), [0xff]).unwrap();
        assert_eq!(
            find(&p, &files, &metadata_request(kind, "auth"))
                .unwrap_err()
                .code,
            "UNSUPPORTED_ENCODING"
        );
        std::fs::write(temp.path().join("other.py"), original).unwrap();
    }
    p.config.policy.exclude.push("other.py".into());
    std::fs::write(temp.path().join("other.py"), [0xff]).unwrap();
    let value = find(&p, &files, &metadata_request("symbol", "unrelated")).unwrap();
    assert!(value["items"].as_array().unwrap().is_empty());
    assert!(!serde_json::to_string(&value).unwrap().contains("other.py"));
}

#[test]
fn invalid_policy_fails_even_without_metadata_candidates() {
    let temp = tempfile::tempdir().unwrap();
    let mut p = project(temp.path());
    let source = "def auth():\n    return True\n";
    std::fs::write(temp.path().join("auth.py"), source).unwrap();
    let entry = analyze("auth.py", &hash(source), source).unwrap();
    p.config.policy.exclude.push("[".into());
    let request = metadata_request("symbol", "unmatched");
    for entries in [&[][..], std::slice::from_ref(&entry)] {
        assert_eq!(
            pctx::search::find(&p, entries, &request).unwrap_err().code,
            "INVALID_CONFIG"
        );
        assert_eq!(
            pctx::search::outline(&p, entries, "auth.py", None, "off")
                .unwrap_err()
                .code,
            "INVALID_CONFIG"
        );
    }
    assert_eq!(
        pctx::reader::inventory(&p, false).unwrap_err().code,
        "INVALID_CONFIG"
    );
}

fn indexed_project() -> (tempfile::TempDir, pctx::project::Project) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let mut p = project(&root);
    p.data_dir = temp.path().join("data");
    p.workspace_dir = p.data_dir.join("workspace");
    p.control_dir = p.data_dir.join("control");
    std::fs::create_dir_all(&p.workspace_dir).unwrap();
    std::fs::create_dir_all(&p.control_dir).unwrap();
    (temp, p)
}

#[test]
fn indexed_metadata_search_hides_unopened_historical_counts_and_propagates_generation() {
    use pctx::{search::find_indexed, storage};
    let (_temp, mut p) = indexed_project();
    for (path, text) in [
        ("auth.py", "def auth():\n    pass\n"),
        ("deleted.py", "def legacy():\n    pass\n"),
        ("excluded.py", "def auth():\n    pass\n"),
    ] {
        std::fs::write(p.root.join(path), text).unwrap();
    }
    let update = storage::update(&p).unwrap();
    std::fs::remove_file(p.root.join("deleted.py")).unwrap();
    p.config.policy.exclude.push("excluded.py".into());
    let value = find_indexed(&p, &metadata_request("symbol", "auth")).unwrap();
    assert_eq!(value["items"].as_array().unwrap().len(), 1);
    assert_eq!(value["items"][0]["path"], "auth.py");
    assert_eq!(value["generation_id"], update["generation_id"]);
    assert!(value["scanned_files"].is_null());
    assert_eq!(value["coverage"]["physical_non_candidates_checked"], false);
    assert_eq!(value["omitted_count"], 0);
    let output = value.to_string();
    assert!(!output.contains("deleted.py"));
    assert!(!output.contains("excluded.py"));
    for kind in ["text", "all"] {
        let result = find_indexed(&p, &metadata_request(kind, "auth")).unwrap();
        assert_eq!(result["items"].as_array().unwrap().len(), 1);
        assert_eq!(result["coverage"]["physical_non_candidates_checked"], false);
        assert!(result["scanned_files"].is_null());
        assert_eq!(result["verified_body_files"], 1);
    }
}

#[cfg(unix)]
#[test]
fn indexed_metadata_matches_including_complements_authorize_before_limit_even_off() {
    use pctx::{search::find_indexed, storage};
    let (temp, p) = indexed_project();
    for (path, text) in [
        ("a.py", "def other():\n    pass\n"),
        ("b.py", "def auth_helper():\n    pass\n"),
        ("z.py", "def auth():\n    pass\n"),
    ] {
        std::fs::write(p.root.join(path), text).unwrap();
    }
    storage::update(&p).unwrap();
    let outside = temp.path().join("outside.py");
    std::fs::write(&outside, "def other():\n    pass\n").unwrap();
    std::fs::remove_file(p.root.join("a.py")).unwrap();
    std::os::unix::fs::symlink(&outside, p.root.join("a.py")).unwrap();
    let mut req = metadata_request("symbol", "auth");
    req.limit = 1;
    req.freshness = "off".into();
    req.query = None;
    req.boolean_query = Some("auth OR NOT legacy".into());
    let result = find_indexed(&p, &req).unwrap();
    assert_eq!(result["items"][0]["path"], "z.py");
    assert_eq!(result["items"][0]["freshness"], "unchecked");
    assert_eq!(result["omitted_count"], 1);
    assert!(!result.to_string().contains("a.py"));
    // The second ranked candidate must still be read before limit truncation.
    std::fs::write(p.root.join("b.py"), [0xff]).unwrap();
    req.freshness = "matched".into();
    assert_eq!(
        find_indexed(&p, &req).unwrap_err().code,
        "UNSUPPORTED_ENCODING"
    );
}

#[test]
fn indexed_metadata_staleness_and_body_only_discovery_keep_their_boundaries() {
    use pctx::{domain::hash, search::find_indexed, storage};
    let (_temp, p) = indexed_project();
    std::fs::write(p.root.join("match.py"), "def auth():\n    pass\n").unwrap();
    std::fs::write(p.root.join("other.py"), "def other():\n    pass\n").unwrap();
    storage::update(&p).unwrap();
    let new_match = "def renamed():\n    pass\n";
    std::fs::write(p.root.join("match.py"), new_match).unwrap();
    std::fs::write(p.root.join("other.py"), "def other():\n    return 'auth'\n").unwrap();
    let result = find_indexed(&p, &metadata_request("symbol", "auth")).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert_eq!(result["items"][0]["path"], "match.py");
    assert_eq!(result["items"][0]["freshness"], "stale");
    assert_eq!(result["items"][0]["file_hash"], hash(new_match));
    assert!(
        result["items"][0]["line_numbers"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for kind in ["text", "all"] {
        let result = find_indexed(&p, &metadata_request(kind, "auth")).unwrap();
        assert!(
            result["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["path"] == "other.py")
        );
    }
}

#[test]
fn indexed_metadata_invalid_policy_and_foreign_workspace_fail_closed() {
    use pctx::{search::find_indexed, storage};
    let (_temp, mut p) = indexed_project();
    std::fs::write(p.root.join("auth.py"), "def auth():\n    pass\n").unwrap();
    storage::update(&p).unwrap();
    p.config.policy.exclude.push("[".into());
    assert_eq!(
        find_indexed(&p, &metadata_request("symbol", "missing"))
            .unwrap_err()
            .code,
        "INVALID_CONFIG"
    );
    p.config.policy.exclude.clear();
    p.workspace_id = "foreign-workspace".into();
    assert_eq!(
        find_indexed(&p, &metadata_request("symbol", "auth"))
            .unwrap_err()
            .code,
        "NOT_INITIALIZED"
    );
}

#[cfg(unix)]
#[test]
fn indexed_metadata_empty_generation_rejects_replaced_root_before_search() {
    use pctx::{search::find_indexed, storage};
    let (temp, p) = indexed_project();
    storage::update(&p).unwrap();
    std::fs::rename(&p.root, temp.path().join("original")).unwrap();
    std::fs::create_dir(&p.root).unwrap();
    for kind in ["symbol", "text", "all"] {
        assert_eq!(
            find_indexed(&p, &metadata_request(kind, "missing"))
                .unwrap_err()
                .code,
            "POLICY_DENIED"
        );
    }
}

#[test]
fn indexed_body_search_filters_current_policy_scope_language_and_deleted_rows_without_counts() {
    use pctx::{search::find_indexed, storage};
    let (_temp, mut p) = indexed_project();
    std::fs::create_dir(p.root.join("src")).unwrap();
    std::fs::create_dir(p.root.join("elsewhere")).unwrap();
    let current = "def unrelated():\n    return 'needle'\n";
    for (path, text) in [
        ("src/current.py", current),
        ("src/deleted.py", current),
        ("src/private.py", current),
        (
            "src/other.js",
            "function unrelated() { return 'needle'; }\n",
        ),
        ("elsewhere/other.py", current),
    ] {
        std::fs::write(p.root.join(path), text).unwrap();
    }
    let indexed = storage::update(&p).unwrap();
    std::fs::remove_file(p.root.join("src/deleted.py")).unwrap();
    p.config.policy.exclude.push("src/private.py".into());
    // Real verified reads would fail. Scope/language and current policy must
    // exclude these before reading, regardless of their old matching bodies.
    for path in ["src/private.py", "src/other.js", "elsewhere/other.py"] {
        std::fs::write(p.root.join(path), [0xff]).unwrap();
    }
    for kind in ["text", "all"] {
        let mut req = metadata_request(kind, "needle");
        req.scopes = vec!["src".into()];
        req.language = Some("python".into());
        let value = find_indexed(&p, &req).unwrap();
        assert_eq!(value["generation_id"], indexed["generation_id"]);
        assert_eq!(value["items"].as_array().unwrap().len(), 1);
        assert_eq!(value["items"][0]["path"], "src/current.py");
        assert_eq!(value["items"][0]["file_hash"], hash(current));
        assert_eq!(value["items"][0]["line_numbers"], serde_json::json!([2]));
        assert_eq!(value["verified_body_files"], 1);
        assert!(value["scanned_files"].is_null());
        assert_eq!(value["coverage"]["physical_non_candidates_checked"], false);
        assert_eq!(
            value["coverage"]["body_candidate_universe"],
            "policy_eligible_indexed_files_in_requested_scope_language"
        );
        for hidden in ["deleted.py", "private.py", "other.js", "elsewhere"] {
            assert!(!value.to_string().contains(hidden), "{value}");
        }
    }
    // The generic snapshot still removes physically missing entries for its
    // other consumers, while retaining the authorized out-of-scope rows.
    let (_, generic) = storage::snapshot(&p).unwrap();
    assert_eq!(generic.len(), 3);
    assert!(
        !generic
            .iter()
            .any(|f| f.path == "src/deleted.py" || f.path == "src/private.py")
    );
}

#[test]
fn indexed_body_search_observes_new_text_and_boolean_complements_with_actual_hashes() {
    use pctx::{search::find_indexed, storage};
    let (_temp, p) = indexed_project();
    for path in ["changed.py", "clean.py", "banned.py"] {
        std::fs::write(p.root.join(path), "def unrelated():\n    pass\n").unwrap();
    }
    let indexed = storage::update(&p).unwrap();
    let changed = "def renamed():\n    return 'needle'\n";
    let clean = "def unrelated():\n    return 2\n";
    let banned = "def unrelated():\n    return 'banned'\n";
    for (path, text) in [
        ("changed.py", changed),
        ("clean.py", clean),
        ("banned.py", banned),
    ] {
        std::fs::write(p.root.join(path), text).unwrap();
    }
    for kind in ["text", "all"] {
        let mut req = metadata_request(kind, "needle");
        req.query = None;
        req.boolean_query = Some("needle OR NOT banned".into());
        let value = find_indexed(&p, &req).unwrap();
        assert_eq!(value["generation_id"], indexed["generation_id"]);
        assert_eq!(value["verified_body_files"], 3);
        assert_eq!(value["items"].as_array().unwrap().len(), 2);
        assert_eq!(value["items"][0]["path"], "changed.py");
        assert_eq!(value["items"][0]["file_hash"], hash(changed));
        assert_eq!(value["items"][0]["line_numbers"], serde_json::json!([2]));
        assert_eq!(value["items"][0]["freshness"], "stale");
        assert_eq!(value["items"][1]["path"], "clean.py");
        assert_eq!(value["items"][1]["file_hash"], hash(clean));
        assert_eq!(value["items"][1]["score"], 0);
        assert_eq!(value["items"][1]["match_count"], 0);
        assert!(!value.to_string().contains("banned.py"));
    }
}

#[test]
fn indexed_body_search_validates_nonmatching_later_sources_before_limit_even_off() {
    use pctx::{search::find_indexed, storage};
    let (_temp, p) = indexed_project();
    std::fs::write(
        p.root.join("a.py"),
        "def unrelated():\n    return 'needle'\n",
    )
    .unwrap();
    std::fs::write(p.root.join("z.py"), "def later():\n    pass\n").unwrap();
    storage::update(&p).unwrap();
    std::fs::write(p.root.join("z.py"), [0xff]).unwrap();
    for kind in ["text", "all"] {
        for freshness in ["matched", "off"] {
            let mut req = metadata_request(kind, "needle");
            req.limit = 1;
            req.freshness = freshness.into();
            let error = find_indexed(&p, &req).unwrap_err();
            assert_eq!(error.code, "UNSUPPORTED_ENCODING");
            assert_eq!(error.exit, 3);
        }
    }
}

#[cfg(unix)]
#[test]
fn indexed_body_search_does_not_follow_a_matching_or_complement_symlink() {
    use pctx::{search::find_indexed, storage};
    let (temp, p) = indexed_project();
    let current = "def unrelated():\n    return 'needle'\n";
    for path in ["a.py", "z.py"] {
        std::fs::write(p.root.join(path), current).unwrap();
    }
    storage::update(&p).unwrap();
    let outside = temp.path().join("outside.py");
    std::fs::write(&outside, [0xff]).unwrap();
    std::fs::remove_file(p.root.join("a.py")).unwrap();
    std::os::unix::fs::symlink(&outside, p.root.join("a.py")).unwrap();
    for kind in ["text", "all"] {
        let mut req = metadata_request(kind, "needle");
        req.query = None;
        req.boolean_query = Some("needle OR NOT banned".into());
        let value = find_indexed(&p, &req).unwrap();
        assert_eq!(value["items"].as_array().unwrap().len(), 1);
        assert_eq!(value["items"][0]["path"], "z.py");
        assert_eq!(value["verified_body_files"], 1);
        assert!(!value.to_string().contains("a.py"));
    }
    let (_, generic) = storage::snapshot(&p).unwrap();
    assert_eq!(generic.len(), 1);
    assert_eq!(generic[0].path, "z.py");
}

#[test]
fn indexed_body_search_preserves_invalid_policy_workspace_and_shared_lock_deadlines() {
    use fs2::FileExt;
    use pctx::{deadline::Deadline, search::find_indexed, storage};
    let (_temp, mut p) = indexed_project();
    std::fs::write(
        p.root.join("current.py"),
        "def unrelated():\n    return 'needle'\n",
    )
    .unwrap();
    let indexed = storage::update(&p).unwrap();
    let mut invalid = p.clone();
    invalid.config.policy.exclude.push("[".into());
    let mut foreign = p.clone();
    foreign.workspace_id = "foreign".into();
    for kind in ["text", "all"] {
        assert_eq!(
            find_indexed(&invalid, &metadata_request(kind, "absent"))
                .unwrap_err()
                .code,
            "INVALID_CONFIG"
        );
        assert_eq!(
            find_indexed(&foreign, &metadata_request(kind, "needle"))
                .unwrap_err()
                .code,
            "NOT_INITIALIZED"
        );
    }
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(p.workspace_dir.join("writer.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    for kind in ["text", "all"] {
        let deadline = Deadline::from_millis(60).unwrap();
        p.deadline = Some(deadline);
        let error = find_indexed(&p, &metadata_request(kind, "needle")).unwrap_err();
        assert_eq!(error.code, "TIMEOUT");
        assert_eq!(error.exit, 7);
        assert!(
            deadline.check().is_err(),
            "The snapshot renewed its deadline"
        );
    }
    FileExt::unlock(&lock).unwrap();
    p.deadline = None;
    let (generation, _) = storage::snapshot(&p).unwrap();
    assert_eq!(generation.as_deref(), indexed["generation_id"].as_str());
}

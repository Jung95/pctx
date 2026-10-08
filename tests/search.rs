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

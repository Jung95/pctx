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

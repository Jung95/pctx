//! Versioned structural metadata and bounded lexical search; source is never persisted.
use crate::{
    deadline::Deadline,
    domain::{Error, FileEntry, Result, Symbol, hash},
    project::Project,
    reader,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tree_sitter::{Node, ParseOptions, Parser, Tree};

fn invalid(message: &str) -> Error {
    Error::new("INVALID_ARGUMENT", message, 2)
}
fn language(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "py" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "ts" => "typescript",
        "tsx" => "tsx",
        "md" | "markdown" => "markdown",
        _ => "text",
    }
}
// Named grammar fields are passed together to keep source positions explicit.
#[allow(clippy::too_many_arguments)]
fn make_symbol(
    path: &str,
    file_hash: &str,
    name: String,
    kind: &str,
    start: usize,
    end: usize,
    first: usize,
    last: usize,
    parent: Option<&Symbol>,
) -> Symbol {
    Symbol {
        id: format!(
            "SYM-{}",
            hash(format!("{path}\0{file_hash}\0{start}\0{end}\0{kind}"))
        ),
        path: path.into(),
        file_hash: file_hash.into(),
        qualified_name: parent
            .map(|p| format!("{}.{}", p.qualified_name, name))
            .unwrap_or(name.clone()),
        name,
        kind: kind.into(),
        start_byte: start,
        end_byte: end,
        start_line: first,
        end_line: last,
        parent_symbol_id: parent.map(|p| p.id.clone()),
    }
}
fn walk(
    node: Node<'_>,
    text: &str,
    path: &str,
    file_hash: &str,
    parent: Option<&Symbol>,
    out: &mut Vec<Symbol>,
    deadline: Option<Deadline>,
) -> Result<()> {
    check_parse_deadline(deadline)?;
    let kind = match node.kind() {
        "function_definition"
        | "function_declaration"
        | "generator_function_declaration"
        | "function_signature" => Some(if parent.is_some_and(|p| p.kind == "class") {
            "method"
        } else {
            "function"
        }),
        "class_definition" | "class_declaration" | "abstract_class_declaration" => Some("class"),
        "method_definition" | "method_signature" => Some("method"),
        "interface_declaration" => Some("interface"),
        "type_alias_declaration" => Some("type"),
        "enum_declaration" => Some("enum"),
        "variable_declarator"
            if node.child_by_field_name("value").is_some_and(|n| {
                matches!(
                    n.kind(),
                    "arrow_function" | "function_expression" | "generator_function"
                )
            }) =>
        {
            Some("function")
        }
        _ => None,
    };
    let symbol = kind.map(|k| {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(text.as_bytes()).ok())
            .unwrap_or("")
            .to_owned();
        let end_line = if node.end_position().column == 0
            && node.end_position().row > node.start_position().row
        {
            node.end_position().row
        } else {
            node.end_position().row + 1
        };
        make_symbol(
            path,
            file_hash,
            name,
            k,
            node.start_byte(),
            node.end_byte(),
            node.start_position().row + 1,
            end_line,
            parent,
        )
    });
    if let Some(s) = &symbol {
        out.push(s.clone());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(
            child,
            text,
            path,
            file_hash,
            symbol.as_ref().or(parent),
            out,
            deadline,
        )?;
    }
    Ok(())
}
fn check_parse_deadline(deadline: Option<Deadline>) -> Result<()> {
    deadline.map(|d| d.check()).unwrap_or(Ok(()))
}
fn parse_tree(parser: &mut Parser, text: &str, deadline: Option<Deadline>) -> Result<Tree> {
    check_parse_deadline(deadline)?;
    let mut progress = |_: &tree_sitter::ParseState| deadline.is_some_and(|d| d.check().is_err());
    let mut input =
        |offset: usize, _: tree_sitter::Point| text.as_bytes().get(offset..).unwrap_or(&[]);
    let tree = if deadline.is_some() {
        parser.parse_with_options(
            &mut input,
            None,
            Some(ParseOptions::new().progress_callback(&mut progress)),
        )
    } else {
        parser.parse(text, None)
    };
    check_parse_deadline(deadline)?;
    tree.ok_or_else(|| Error::new("PARSE_FAILED", "Parser did not produce a tree", 3))
}
/// Library search entry points share a ten-second default, never a fresh budget
/// for nested phases. Application-supplied deadlines remain authoritative.
fn query_project(p: &Project) -> Result<Project> {
    let mut p = p.clone();
    if p.deadline.is_none() {
        p.deadline = Some(Deadline::from_millis(10_000)?);
    }
    p.check_deadline()?;
    Ok(p)
}
pub fn analyze(path: &str, file_hash: &str, text: &str) -> Result<FileEntry> {
    analyze_with_deadline(path, file_hash, text, None)
}
pub fn analyze_with_deadline(
    path: &str,
    file_hash: &str,
    text: &str,
    deadline: Option<Deadline>,
) -> Result<FileEntry> {
    check_parse_deadline(deadline)?;
    let lang = language(path);
    let mut symbols = Vec::new();
    let mut status = "complete";
    if lang == "markdown" {
        let mut offset = 0;
        let mut headings: Vec<(usize, Symbol)> = Vec::new();
        let mut fence: Option<char> = None;
        for (line, part) in text.split_inclusive('\n').enumerate() {
            check_parse_deadline(deadline)?;
            let trimmed = part.trim_end_matches(['\r', '\n']);
            let leading = trimmed.trim_start();
            if leading.starts_with("```") || leading.starts_with("~~~") {
                let c = leading.chars().next().unwrap();
                if fence == Some(c) {
                    fence = None;
                } else if fence.is_none() {
                    fence = Some(c);
                }
                offset += part.len();
                continue;
            }
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            if fence.is_none()
                && (1..=6).contains(&level)
                && trimmed.as_bytes().get(level) == Some(&b' ')
            {
                while headings.last().is_some_and(|(d, _)| *d >= level) {
                    let (_, mut s) = headings.pop().unwrap();
                    s.end_byte = offset;
                    s.end_line = line;
                    symbols.push(s);
                }
                let name = trimmed[level..]
                    .trim()
                    .trim_end_matches('#')
                    .trim()
                    .to_string();
                let s = make_symbol(
                    path,
                    file_hash,
                    name,
                    "heading",
                    offset,
                    text.len(),
                    line + 1,
                    text.lines().count(),
                    headings.last().map(|(_, s)| s),
                );
                headings.push((level, s));
            }
            offset += part.len();
        }
        for (_, s) in headings {
            symbols.push(s);
        }
        // Re-key headings after their final section spans are known.
        let ids: BTreeMap<_, _> = symbols
            .iter()
            .map(|s| {
                (
                    s.id.clone(),
                    format!(
                        "SYM-{}",
                        hash(format!(
                            "{path}\0{file_hash}\0{}\0{}\0heading",
                            s.start_byte, s.end_byte
                        ))
                    ),
                )
            })
            .collect();
        for s in &mut symbols {
            s.id = ids[&s.id].clone();
            s.parent_symbol_id = s
                .parent_symbol_id
                .as_ref()
                .and_then(|id| ids.get(id).cloned());
        }
    } else if lang != "text" {
        let grammar = match lang {
            "python" => tree_sitter_python::LANGUAGE.into(),
            "javascript" | "jsx" => tree_sitter_javascript::LANGUAGE.into(),
            "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            _ => tree_sitter_typescript::LANGUAGE_TSX.into(),
        };
        let mut parser = Parser::new();
        parser
            .set_language(&grammar)
            .map_err(|_| invalid("Grammar initialization failed"))?;
        let tree = parse_tree(&mut parser, text, deadline)?;
        if tree.root_node().has_error() || text.contains("<<<<<<<") {
            status = "partial";
        }
        walk(
            tree.root_node(),
            text,
            path,
            file_hash,
            None,
            &mut symbols,
            deadline,
        )?;
    } else {
        status = "unsupported";
    }
    symbols.sort_by_key(|s| (s.start_byte, s.end_byte));
    check_parse_deadline(deadline)?;
    Ok(FileEntry {
        path: path.into(),
        file_hash: file_hash.into(),
        size_bytes: text.len() as u64,
        language: lang.into(),
        parse_status: status.into(),
        symbols,
    })
}

#[derive(clap::Args, Debug, Clone)]
pub struct FindRequest {
    #[arg(conflicts_with = "boolean_query")]
    pub query: Option<String>,
    #[arg(long = "query", conflicts_with = "query")]
    pub boolean_query: Option<String>,
    #[arg(long,default_value="all",value_parser=["path","symbol","document","text","all"])]
    pub kind: String,
    #[arg(long)]
    pub regex: bool,
    #[arg(long)]
    pub snippet_lines: Option<usize>,
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    #[arg(long = "scope")]
    pub scopes: Vec<String>,
    #[arg(long,default_value="matched",value_parser=["off","matched","strict"])]
    pub freshness: String,
    #[arg(long)]
    pub language: Option<String>,
    #[arg(long)]
    pub explain: bool,
}
#[derive(Clone, Debug)]
enum Expr {
    Term(String),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}
impl Expr {
    fn matches(&self, test: &impl Fn(&str) -> bool) -> bool {
        match self {
            Self::Term(t) => test(t),
            Self::Not(e) => !e.matches(test),
            Self::And(a, b) => a.matches(test) && b.matches(test),
            Self::Or(a, b) => a.matches(test) || b.matches(test),
        }
    }
    fn terms(&self, negative: bool, out: &mut Vec<String>) {
        match self {
            Self::Term(t) if !negative => out.push(t.clone()),
            Self::Term(_) => {}
            Self::Not(e) => e.terms(!negative, out),
            Self::And(a, b) | Self::Or(a, b) => {
                a.terms(negative, out);
                b.terms(negative, out);
            }
        }
    }
}
fn parse_boolean(input: &str) -> Result<Expr> {
    if input.len() > 4096 {
        return Err(invalid("Query exceeds 4096 bytes"));
    }
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if matches!(c, '(' | ')') {
            tokens.push(c.to_string());
            continue;
        }
        let mut t = String::new();
        if c == '"' {
            let mut closed = false;
            for x in chars.by_ref() {
                if x == '"' {
                    closed = true;
                    break;
                }
                t.push(x);
            }
            if !closed || t.is_empty() {
                return Err(invalid("Invalid quoted phrase"));
            }
            tokens.push(format!("\"{t}"));
        } else {
            t.push(c);
            while chars
                .peek()
                .is_some_and(|x| !x.is_whitespace() && !matches!(x, '(' | ')'))
            {
                t.push(chars.next().unwrap());
            }
            tokens.push(t);
        }
        if tokens.len() > 128 {
            return Err(invalid("Query complexity exceeds limit"));
        }
    }
    struct P {
        ts: Vec<String>,
        pos: usize,
        nodes: usize,
    }
    impl P {
        fn eat(&mut self, t: &str) -> bool {
            if self.ts.get(self.pos).is_some_and(|x| x == t) {
                self.pos += 1;
                true
            } else {
                false
            }
        }
        fn add(&mut self, e: Expr) -> Result<Expr> {
            self.nodes += 1;
            if self.nodes > 64 {
                Err(invalid("Query exceeds 64 AST nodes"))
            } else {
                Ok(e)
            }
        }
        fn atom(&mut self, depth: usize) -> Result<Expr> {
            if depth > 8 {
                return Err(invalid("Query exceeds depth 8"));
            }
            if self.eat("NOT") {
                let e = self.atom(depth + 1)?;
                return self.add(Expr::Not(Box::new(e)));
            }
            if self.eat("(") {
                let e = self.or(depth + 1)?;
                if !self.eat(")") {
                    return Err(invalid("Missing closing parenthesis"));
                }
                return Ok(e);
            }
            let t = self
                .ts
                .get(self.pos)
                .cloned()
                .ok_or_else(|| invalid("Expected search term"))?;
            if matches!(t.as_str(), "AND" | "OR" | ")") {
                return Err(invalid("Expected search term"));
            }
            self.pos += 1;
            self.add(Expr::Term(t.trim_start_matches('"').into()))
        }
        fn and(&mut self, d: usize) -> Result<Expr> {
            let mut e = self.atom(d)?;
            while self.eat("AND") {
                let r = self.atom(d)?;
                e = self.add(Expr::And(Box::new(e), Box::new(r)))?;
            }
            Ok(e)
        }
        fn or(&mut self, d: usize) -> Result<Expr> {
            let mut e = self.and(d)?;
            while self.eat("OR") {
                let r = self.and(d)?;
                e = self.add(Expr::Or(Box::new(e), Box::new(r)))?;
            }
            Ok(e)
        }
    }
    let mut p = P {
        ts: tokens,
        pos: 0,
        nodes: 0,
    };
    let e = p.or(0)?;
    if p.pos != p.ts.len() {
        return Err(invalid("Unexpected token; use explicit AND or OR"));
    }
    let mut terms = Vec::new();
    e.terms(false, &mut terms);
    if terms.is_empty() {
        return Err(invalid("Query requires a positive search term"));
    }
    Ok(e)
}
fn literal(hay: &str, needle: &str) -> bool {
    if needle.chars().any(char::is_uppercase) {
        hay.contains(needle)
    } else {
        hay.to_lowercase().contains(&needle.to_lowercase())
    }
}
fn tokens(name: &str) -> Vec<String> {
    let mut result = String::new();
    let mut prev = false;
    for c in name.chars() {
        if c.is_uppercase() && prev {
            result.push(' ');
        }
        if c.is_alphanumeric() {
            result.push(c);
            prev = c.is_lowercase();
        } else {
            result.push(' ');
            prev = false;
        }
    }
    result.split_whitespace().map(str::to_owned).collect()
}
fn scope(path: &str, scopes: &[String]) -> bool {
    scopes.is_empty()
        || scopes
            .iter()
            .any(|s| path == s || path.starts_with(&format!("{}/", s.trim_end_matches('/'))))
}
type AliasExpansion = (BTreeMap<String, Vec<String>>, Vec<Value>);
fn aliases(p: &Project, terms: &[String]) -> Result<AliasExpansion> {
    let mut map = BTreeMap::new();
    let mut expanded = Vec::new();
    let path = p.root.join(".pctx/glossary.toml");
    if !path.exists() {
        return Ok((map, expanded));
    }
    let source = reader::read(p, ".pctx/glossary.toml")?;
    let doc: toml::Value = toml::from_str(&source.text)
        .map_err(|_| Error::new("INVALID_CONFIG", "Invalid glossary TOML", 2))?;
    if let Some(table) = doc.get("aliases").and_then(toml::Value::as_table) {
        for term in terms {
            for (phrase, values) in table {
                if literal(term, phrase) {
                    let values = values
                        .as_array()
                        .ok_or_else(|| Error::new("INVALID_CONFIG", "Alias must be an array", 2))?;
                    for v in values {
                        let v = v
                            .as_str()
                            .ok_or_else(|| Error::new("INVALID_CONFIG", "Alias must be text", 2))?;
                        if expanded.len() == 16 {
                            break;
                        }
                        map.entry(term.clone())
                            .or_insert_with(Vec::new)
                            .push(v.to_string());
                        expanded
                            .push(json!({"term":v,"phrase":phrase,"source":".pctx/glossary.toml"}));
                    }
                }
            }
        }
    }
    Ok((map, expanded))
}
/// Search one pinned index generation. Strict refresh and its partial-coverage
/// propagation remain the calling application's responsibility, as for `find`.
/// Metadata-only kinds defer physical access checks until the exact expression
/// selects a candidate. Text/all defer physical admission until scope/language
/// filtering, then retain authorization and verified reads of every candidate.
pub fn find_indexed(p: &Project, req: &FindRequest) -> Result<Value> {
    let bounded = query_project(p)?;
    let p = &bounded;
    let metadata_only = matches!(req.kind.as_str(), "path" | "symbol" | "document");
    let body_search = matches!(req.kind.as_str(), "text" | "all");
    let (generation, files) = if metadata_only {
        crate::storage::metadata_search_snapshot(p)?
    } else if body_search {
        crate::storage::body_search_snapshot(p)?
    } else {
        crate::storage::snapshot(p)?
    };
    let mut value = find(p, &files, req)?;
    if value["coverage"]["reasons"]
        .as_array()
        .is_some_and(|reasons| reasons.iter().any(|r| r == "timeout"))
    {
        reader::validate_partial_root(p)?;
    } else {
        reader::validate_root(p)?;
    }
    value["generation_id"] = json!(generation);
    if metadata_only || body_search {
        // Unopened noncandidates can be deleted, inaccessible or links. Their
        // historical indexed count is not a count of authorized current files.
        value["scanned_files"] = Value::Null;
        value["coverage"]["physical_non_candidates_checked"] = json!(false);
    } else {
        value["coverage"]["physical_non_candidates_checked"] = json!(true);
    }
    if body_search {
        value["coverage"]["body_candidate_universe"] =
            json!("policy_eligible_indexed_files_in_requested_scope_language");
    }
    Ok(value)
}

pub fn find(p: &Project, files: &[FileEntry], req: &FindRequest) -> Result<Value> {
    let bounded = query_project(p)?;
    let p = &bounded;
    reader::validate_policy(p)?;
    reader::validate_root(p)?;
    if req.regex && req.boolean_query.is_some() {
        return Err(invalid("Regex and Boolean modes cannot be combined"));
    }
    if req.limit > 1000 || req.snippet_lines.is_some_and(|n| n > 80) {
        return Err(invalid("Limit exceeds supported bound"));
    }
    let expr = if let Some(q) = &req.boolean_query {
        parse_boolean(q)?
    } else {
        let q = req
            .query
            .as_ref()
            .ok_or_else(|| invalid("Provide QUERY or --query expression"))?;
        if q.is_empty() || q.len() > 4096 {
            return Err(invalid("Query must contain 1..4096 bytes"));
        }
        Expr::Term(q.clone())
    };
    let mut positive = Vec::new();
    expr.terms(false, &mut positive);
    let (alias, expanded) = aliases(p, &positive)?;
    let regex = if req.regex {
        Some(
            regex::RegexBuilder::new(req.query.as_deref().unwrap())
                .size_limit(1 << 20)
                .dfa_size_limit(1 << 20)
                .case_insensitive(!req.query.as_ref().unwrap().chars().any(char::is_uppercase))
                .build()
                .map_err(|_| invalid("Unsupported or oversized regex"))?,
        )
    } else {
        None
    };
    let matches = |hay: &str, term: &str| {
        regex.as_ref().map(|r| r.is_match(hay)).unwrap_or_else(|| {
            literal(hay, term)
                || alias
                    .get(term)
                    .is_some_and(|ts| ts.iter().any(|t| literal(hay, t)))
        })
    };
    let weighted = |hay: &str, term: &str, base: i64| {
        if regex
            .as_ref()
            .map(|r| r.is_match(hay))
            .unwrap_or_else(|| literal(hay, term))
        {
            base
        } else if alias
            .get(term)
            .is_some_and(|ts| ts.iter().any(|t| literal(hay, t)))
        {
            base * 4 / 5
        } else {
            0
        }
    };
    let mut results = Vec::new();
    let mut reasons = Vec::new();
    let mut scanned = 0;
    let mut verified_body_files = 0;
    'scan: for f in files {
        if let Err(e) = p.check_deadline() {
            if results.is_empty() {
                return Err(e);
            }
            reasons.push("timeout");
            break;
        }
        if !scope(&f.path, &req.scopes) || req.language.as_ref().is_some_and(|l| l != &f.language) {
            continue;
        }
        if let Err(e) = reader::policy_allows(p, &f.path) {
            if matches!(e.code.as_str(), "POLICY_DENIED" | "PATH_OUTSIDE_ROOT") {
                continue;
            }
            if e.code == "TIMEOUT" && !results.is_empty() {
                reasons.push("timeout");
                break;
            }
            return Err(e);
        }
        if scanned >= 100000 {
            reasons.push("scan_cap");
            break;
        }
        scanned += 1;
        // Evaluate the whole cached corpus, including Boolean complements, before
        // opening metadata-search candidates. Positive terms alone are not a filter.
        let metadata_corpus = match req.kind.as_str() {
            "path" => Some(f.path.clone()),
            "symbol" | "document" => Some(
                f.symbols
                    .iter()
                    .filter(|s| (s.kind == "heading") == (req.kind == "document"))
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            _ => None,
        };
        if metadata_corpus
            .as_ref()
            .is_some_and(|corpus| !expr.matches(&|term| matches(corpus, term)))
        {
            continue;
        }
        // Policy filters index metadata first; physical path checks happen before
        // any candidate is emitted, including freshness=off and negative matches.
        if let Err(e) = reader::authorize(p, &f.path) {
            if matches!(
                e.code.as_str(),
                "TIMEOUT" | "INVALID_CONFIG" | "POLICY_UNAVAILABLE"
            ) {
                if e.code != "TIMEOUT" || results.is_empty() {
                    return Err(e);
                }
                reasons.push("timeout");
                break;
            }
            continue;
        }
        let body = if matches!(req.kind.as_str(), "text" | "all") || req.freshness != "off" {
            match reader::read(p, &f.path) {
                Ok(body) => Some(body),
                Err(e) if e.code == "TIMEOUT" && !results.is_empty() => {
                    reasons.push("timeout");
                    break;
                }
                Err(e) => return Err(e),
            }
        } else {
            None
        };
        if body.is_some() && matches!(req.kind.as_str(), "text" | "all") {
            verified_body_files += 1;
        }
        let freshness = if req.freshness == "off" {
            "unchecked"
        } else if body.as_ref().is_some_and(|v| v.hash == f.file_hash) {
            "current"
        } else {
            "stale"
        };
        let mut signals = Vec::new();
        let mut score = 0i64;
        let mut lines = Vec::new();
        let mut count = 0usize;
        // Text and combined searches require current source even if metadata has
        // no positive hit: the body may match, or negate a Boolean expression.
        if metadata_corpus.is_none() {
            let corpus = if req.kind == "text" {
                body.as_ref().map(|b| b.text.clone()).unwrap_or_default()
            } else {
                format!(
                    "{}\n{}\n{}",
                    f.path,
                    f.symbols
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    body.as_ref().map(|b| b.text.as_str()).unwrap_or("")
                )
            };
            if !expr.matches(&|term| matches(&corpus, term)) {
                continue;
            }
        }
        for term in &positive {
            if let Err(e) = p.check_deadline() {
                if results.is_empty() {
                    return Err(e);
                }
                reasons.push("timeout");
                break 'scan;
            }
            if matches!(req.kind.as_str(), "path" | "all") && matches(&f.path, term) {
                score = score.max(weighted(
                    &f.path,
                    term,
                    if f.path.split('/').any(|x| x == term) {
                        60
                    } else {
                        50
                    },
                ));
                signals.push("path_match");
            }
            if matches!(req.kind.as_str(), "symbol" | "document" | "all") {
                for s in &f.symbols {
                    if (req.kind == "symbol" && s.kind == "heading")
                        || (req.kind == "document" && s.kind != "heading")
                    {
                        continue;
                    }
                    if matches(&s.name, term) {
                        let exact = if term.chars().any(char::is_uppercase) {
                            s.name == *term
                        } else {
                            s.name.to_lowercase() == term.to_lowercase()
                        };
                        score = score.max(weighted(
                            &s.name,
                            term,
                            if s.kind == "heading" {
                                40
                            } else if exact {
                                80
                            } else if tokens(&s.name).iter().any(|t| literal(t, term)) {
                                50
                            } else {
                                25
                            },
                        ));
                        signals.push("name_match");
                    }
                }
            }
        }
        if let Some(b) = &body
            && matches!(req.kind.as_str(), "text" | "all")
        {
            for (i, line) in b.text.lines().enumerate() {
                if let Err(e) = p.check_deadline() {
                    if results.is_empty() {
                        return Err(e);
                    }
                    reasons.push("timeout");
                    break 'scan;
                }
                if positive.iter().any(|t| matches(line, t)) {
                    count += 1;
                    lines.push(i + 1);
                }
            }
            if count > 0 {
                score = score.max(
                    positive
                        .iter()
                        .map(|t| weighted(&b.text, t, 25))
                        .max()
                        .unwrap_or(0),
                );
                signals.push("body_literal");
            }
        }
        signals.sort();
        signals.dedup();
        let mut item = json!({"path":f.path,"file_hash":body.as_ref().map(|b|b.hash.as_str()).unwrap_or(&f.file_hash),"freshness":freshness,"evidence_status":"observed","score":score,"match_methods":signals,"line_numbers":lines,"match_count":count});
        if req.explain {
            item["reason_codes"] = json!(signals);
            item["scorer_version"] = json!("lexical-v1");
        }
        if let (Some(n), Some(b)) = (req.snippet_lines, &body)
            && n > 0
        {
            let ls: Vec<_> = b.text.lines().collect();
            let mut ranges = Vec::new();
            for line in &lines {
                if let Err(e) = p.check_deadline() {
                    if results.is_empty() {
                        return Err(e);
                    }
                    reasons.push("timeout");
                    break 'scan;
                }
                let first = line.saturating_sub(n + 1);
                let last = (*line + n).min(ls.len());
                let start = b
                    .text
                    .split_inclusive('\n')
                    .take(first)
                    .map(str::len)
                    .sum::<usize>();
                let end = b
                    .text
                    .split_inclusive('\n')
                    .take(last)
                    .map(str::len)
                    .sum::<usize>();
                let (text, masked) = reader::redact_span(&b.text, start, end);
                ranges.push(
                    json!({"start_line":first+1,"end_line":last,"text":text,"redacted":masked}),
                );
                if ranges.len() >= 20 {
                    break;
                }
            }
            item["snippets"] = json!(ranges);
        }
        if let Err(e) = p.check_deadline() {
            if results.is_empty() {
                return Err(e);
            }
            reasons.push("timeout");
            break;
        }
        results.push(item);
    }
    if p.check_deadline().is_err() && !reasons.contains(&"timeout") {
        if results.is_empty() {
            p.check_deadline()?;
        }
        reasons.push("timeout");
    }
    results.sort_by(|a, b| {
        b["score"]
            .as_i64()
            .cmp(&a["score"].as_i64())
            .then_with(|| a["path"].as_str().cmp(&b["path"].as_str()))
    });
    let omitted = results.len().saturating_sub(req.limit);
    results.truncate(req.limit);
    if let Err(e) = p.check_deadline() {
        if results.is_empty() {
            return Err(e);
        }
        if !reasons.contains(&"timeout") {
            reasons.push("timeout");
        }
    }
    // Already verified candidates may survive expiry, but the root authority
    // must still match. This narrow identity gate opens no new source and does
    // not restart or clear the shared request deadline.
    if reasons.contains(&"timeout") {
        reader::validate_partial_root(p)?;
    } else {
        reader::validate_root(p)?;
    }
    let source_validation =
        if req.freshness == "off" && !matches!(req.kind.as_str(), "text" | "all") {
            "none"
        } else if matches!(req.kind.as_str(), "path" | "symbol" | "document") {
            "matching_metadata_candidates"
        } else {
            "in_scope_files"
        };
    Ok(json!({
        "items": results,
        "expanded_terms": expanded,
        "coverage": {
            "status": if reasons.is_empty() { "complete" } else { "partial" },
            "reasons": reasons,
            "universe": "provided_index",
            "source_validation": source_validation,
            "new_candidate_discovery": "not_performed_by_find"
        },
        "omitted_count": if reasons.is_empty() { json!(omitted) } else { Value::Null },
        "scanned_files": scanned,
        "verified_body_files": if matches!(req.kind.as_str(), "text" | "all") { json!(verified_body_files) } else { Value::Null }
    }))
}

pub fn outline(
    p: &Project,
    files: &[FileEntry],
    path: &str,
    depth: Option<usize>,
    freshness: &str,
) -> Result<Value> {
    let bounded = query_project(p)?;
    let p = &bounded;
    reader::validate_policy(p)?;
    reader::validate_root(p)?;
    let mut results = Vec::new();
    for f in files {
        p.check_deadline()?;
        if !scope(&f.path, &[path.into()]) {
            continue;
        }
        if let Err(e) = reader::authorize(p, &f.path) {
            if matches!(
                e.code.as_str(),
                "TIMEOUT" | "INVALID_CONFIG" | "POLICY_UNAVAILABLE"
            ) {
                return Err(e);
            }
            continue;
        }
        let current = if freshness == "off" {
            "unchecked"
        } else {
            let v = reader::read(p, &f.path)?;
            if v.hash != f.file_hash {
                return Err(Error::new(
                    "STALE_INDEX",
                    "Outline file changed; update the index or use strict freshness",
                    4,
                ));
            }
            "current"
        };
        let symbols: Vec<_> = f
            .symbols
            .iter()
            .filter(|s| {
                let mut d = 1;
                let mut parent = s.parent_symbol_id.as_ref();
                while let Some(id) = parent {
                    d += 1;
                    parent = f
                        .symbols
                        .iter()
                        .find(|x| &x.id == id)
                        .and_then(|x| x.parent_symbol_id.as_ref());
                }
                depth.is_none_or(|n| d <= n)
            })
            .collect();
        results.push(json!({"path":f.path,"file_hash":f.file_hash,"language":f.language,"parse_status":f.parse_status,"freshness":current,"evidence_status":"observed","symbols":symbols,"coverage":{"status":if f.parse_status=="unsupported"{"unsupported"}else if f.parse_status=="partial"{"partial"}else{"complete"}}}));
    }
    p.check_deadline()?;
    Ok(json!({"files":results}))
}
pub fn read_selection(
    p: &Project,
    files: &[FileEntry],
    path: Option<&str>,
    lines: Option<&str>,
    symbol: Option<&str>,
    symbol_name: Option<&str>,
) -> Result<Value> {
    let bounded = query_project(p)?;
    let p = &bounded;
    reader::validate_policy(p)?;
    reader::validate_root(p)?;
    if symbol.is_some() && symbol_name.is_some()
        || lines.is_some() && (symbol.is_some() || symbol_name.is_some())
    {
        return Err(invalid("Choose lines, symbol ID, or symbol name"));
    }
    let selected = if symbol.is_some() || symbol_name.is_some() {
        let mut candidates = Vec::new();
        for f in files {
            p.check_deadline()?;
            if path.is_some_and(|path| f.path != path) {
                continue;
            }
            if let Err(e) = reader::authorize(p, &f.path) {
                if matches!(
                    e.code.as_str(),
                    "TIMEOUT" | "INVALID_CONFIG" | "POLICY_UNAVAILABLE"
                ) {
                    return Err(e);
                }
                continue;
            }
            for s in &f.symbols {
                p.check_deadline()?;
                if symbol.is_some_and(|id| s.id == id)
                    || symbol_name.is_some_and(|n| s.name == n || s.qualified_name == n)
                {
                    candidates.push(s);
                }
            }
        }
        if symbol_name.is_some() && path.is_none() {
            return Err(invalid("Symbol name lookup requires --path"));
        }
        if candidates.len() > 1 {
            return Err(Error::new(
                "AMBIGUOUS_SYMBOL",
                format!(
                    "Multiple candidates: {}",
                    candidates
                        .iter()
                        .map(|s| format!("{}:{}:{}", s.id, s.path, s.start_line))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                2,
            ));
        }
        Some(*candidates.first().ok_or_else(|| {
            Error::new(
                if symbol.is_some() {
                    "STALE_INDEX"
                } else {
                    "SYMBOL_NOT_FOUND"
                },
                "Exact symbol version was not found",
                4,
            )
        })?)
    } else {
        None
    };
    let path = selected
        .map(|s| s.path.as_str())
        .or(path)
        .ok_or_else(|| invalid("Provide a path or symbol ID"))?;
    let f = reader::read(p, path)?;
    if selected.is_some_and(|s| s.file_hash != f.hash) {
        return Err(Error::new("STALE_INDEX", "Symbol source hash changed", 4));
    }
    let total = f.text.lines().count();
    let (start, end, start_byte, end_byte) = if let Some(s) = selected {
        (s.start_line, s.end_line, s.start_byte, s.end_byte)
    } else {
        let (a, b) = if let Some(range) = lines {
            let (a, b) = range
                .split_once(':')
                .ok_or_else(|| invalid("Lines must be A:B"))?;
            (
                a.parse::<usize>()
                    .map_err(|_| invalid("Invalid first line"))?,
                b.parse::<usize>()
                    .map_err(|_| invalid("Invalid last line"))?,
            )
        } else {
            (1, 80.min(total.max(1)))
        };
        if a == 0 || b < a || a > total.max(1) {
            return Err(invalid("Invalid line range"));
        }
        let mut offsets: Vec<_> = f.text.match_indices('\n').map(|(i, _)| i + 1).collect();
        offsets.insert(0, 0);
        (
            a,
            b.min(total),
            *offsets.get(a - 1).unwrap_or(&0),
            *offsets.get(b).unwrap_or(&f.text.len()),
        )
    };
    let source = f
        .text
        .get(start_byte..end_byte)
        .ok_or_else(|| Error::new("STALE_INDEX", "Invalid UTF-8 symbol bounds", 4))?;
    let limited: String = source.split_inclusive('\n').take(80).collect();
    let mut limited = limited;
    while limited.len() > 65536 {
        let mut cut = 65536;
        while !limited.is_char_boundary(cut) {
            cut -= 1;
        }
        limited.truncate(cut);
    }
    let truncated = limited.len() < source.len() || (selected.is_none() && end < total);
    let actual_end = start_byte + limited.len();
    let (text, masked) = reader::redact_span(&f.text, start_byte, actual_end);
    p.check_deadline()?;
    Ok(
        json!({"path":path,"file_hash":f.hash,"symbol_id":selected.map(|s|&s.id),"range":{"start_byte":start_byte,"end_byte":actual_end,"start_line":start,"end_line":end.min(start+limited.lines().count().saturating_sub(1))},"text":text,"redacted":masked,"freshness":"current","evidence_status":"observed","completeness":if truncated{"partial"}else{"complete"},"truncated":truncated,"extractor":"tree-sitter/line-reader","extractor_version":"1"}),
    )
}

#[derive(clap::Args, Debug, Clone)]
pub struct StructureRequest {
    #[arg(long,value_parser=["python","javascript","jsx","typescript","tsx"])]
    pub language: String,
    #[arg(long,default_value="function",value_parser=["function","method","class","interface","type","enum"])]
    pub kind: String,
    #[arg(long,value_parser=["async"])]
    pub modifier: Option<String>,
    #[arg(long = "scope")]
    pub scopes: Vec<String>,
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    #[arg(long,default_value="matched",value_parser=["matched","strict"])]
    pub freshness: String,
}
/// Limited structural predicates use grammar tokens, never guessed source patterns.
pub fn query_structure(p: &Project, files: &[FileEntry], req: &StructureRequest) -> Result<Value> {
    let bounded = query_project(p)?;
    let p = &bounded;
    reader::validate_policy(p)?;
    reader::validate_root(p)?;
    if req.limit > 1000 {
        return Err(invalid("Limit exceeds supported bound"));
    }
    if matches!(req.kind.as_str(), "interface" | "type" | "enum")
        && !matches!(req.language.as_str(), "typescript" | "tsx")
    {
        return Err(Error::new(
            "CAPABILITY_UNAVAILABLE",
            "This language does not support the requested declaration kind",
            6,
        ));
    }
    if req.modifier.is_some() && !matches!(req.kind.as_str(), "function" | "method") {
        return Err(invalid(
            "Async predicate applies only to functions and methods",
        ));
    }
    let mut results = Vec::new();
    for f in files {
        p.check_deadline()?;
        if f.language != req.language || !scope(&f.path, &req.scopes) {
            continue;
        }
        if let Err(e) = reader::authorize(p, &f.path) {
            if matches!(
                e.code.as_str(),
                "TIMEOUT" | "INVALID_CONFIG" | "POLICY_UNAVAILABLE"
            ) {
                return Err(e);
            }
            continue;
        }
        let current = reader::read(p, &f.path)?;
        if current.hash != f.file_hash {
            return Err(Error::new(
                "STALE_INDEX",
                "Structure query source changed",
                4,
            ));
        }
        let grammar = match req.language.as_str() {
            "python" => tree_sitter_python::LANGUAGE.into(),
            "javascript" | "jsx" => tree_sitter_javascript::LANGUAGE.into(),
            "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
            _ => return Err(invalid("Unsupported structure language")),
        };
        let mut parser = Parser::new();
        parser
            .set_language(&grammar)
            .map_err(|_| invalid("Grammar initialization failed"))?;
        let tree = parse_tree(&mut parser, &current.text, p.deadline)?;
        for s in &f.symbols {
            p.check_deadline()?;
            if s.kind != req.kind {
                continue;
            }
            if req.modifier.is_some() {
                let Some(mut node) = tree
                    .root_node()
                    .descendant_for_byte_range(s.start_byte, s.end_byte)
                else {
                    continue;
                };
                if node.kind() == "variable_declarator"
                    && let Some(value) = node.child_by_field_name("value")
                {
                    node = value;
                }
                let mut cursor = node.walk();
                if !node.children(&mut cursor).any(|n| n.kind() == "async") {
                    continue;
                }
            }
            results.push(json!({"symbol":s,"freshness":"current","evidence_status":"observed","extractor":"tree-sitter","extractor_version":"1","parse_status":f.parse_status}));
        }
    }
    results.sort_by(|a, b| {
        a["symbol"]["path"]
            .as_str()
            .cmp(&b["symbol"]["path"].as_str())
            .then_with(|| {
                a["symbol"]["start_byte"]
                    .as_u64()
                    .cmp(&b["symbol"]["start_byte"].as_u64())
            })
    });
    p.check_deadline()?;
    let omitted = results.len().saturating_sub(req.limit);
    results.truncate(req.limit);
    Ok(
        json!({"items":results,"omitted_count":omitted,"capabilities":{"language":req.language,"predicates":["kind","async"],"semantic_resolution":false}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boolean_precedence_and_bounds() {
        let e = parse_boolean("login OR auth AND NOT legacy").unwrap();
        assert!(e.matches(&|t| t == "login" || t == "legacy"));
        assert!(!e.matches(&|t| t == "auth" || t == "legacy"));
        assert!(e.matches(&|t| t == "auth"));
        assert!(parse_boolean("NOT legacy").is_err());
        assert!(parse_boolean("login auth").is_err());
        assert!(parse_boolean("(((((((((login)))))))))").is_err());
        assert!(parse_boolean(&vec!["login"; 34].join(" OR ")).is_err());
        assert!(
            parse_boolean("\"session expired\" AND NOT legacy")
                .unwrap()
                .matches(&|t| t == "session expired")
        );
    }
    #[test]
    fn literal_smart_case() {
        assert!(literal("Authentication", "auth"));
        assert!(!literal("authentication", "Auth"));
    }
}

use clap::{Parser, Subcommand, ValueEnum};
use pctx::adapter;
use pctx::inventory;
use pctx::runner;
use pctx::schedule;
use pctx::{
    broker, context,
    domain::{self, Error, Result},
    extract, filters, graph, operations, output, pack,
    project::{self, Project},
    quota, search, session, storage, work,
};
use serde_json::{Value, json};
use std::path::PathBuf;
#[derive(Debug, Clone, ValueEnum)]
enum Format {
    Compact,
    Json,
    Markdown,
    Ndjson,
}
#[derive(Parser, Debug)]
#[command(
    name = "pctx",
    version,
    about = "Portable project context and local work coordination"
)]
struct Cli {
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[arg(long, global = true, value_enum, default_value = "compact")]
    format: Format,
    #[arg(long, global = true)]
    output: Option<PathBuf>,
    #[arg(long, global = true)]
    no_color: bool,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand, Debug)]
enum Command {
    #[command(flatten)]
    Work(work::WorkCommand),
    #[command(flatten)]
    Operations(operations::OperationCommand),
    Quota {
        #[command(subcommand)]
        command: quota::QuotaCommand,
    },
    Pack {
        #[command(subcommand)]
        command: pack::PackCommand,
    },
    Filter {
        #[command(subcommand)]
        command: filters::FilterCommand,
    },
    Session {
        #[command(subcommand)]
        command: session::SessionCommand,
    },
    Context {
        #[command(subcommand)]
        command: session::ContextCommand,
    },
    Adapter {
        #[command(subcommand)]
        command: adapter::AdapterCommand,
    },
    Schedule {
        #[command(subcommand)]
        command: schedule::ScheduleCommand,
    },
    Inventory {
        #[command(subcommand)]
        command: inventory::InventoryCommand,
    },
    Resource {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    Job {
        #[command(subcommand)]
        command: JobCommand,
    },
    Runner {
        #[command(subcommand)]
        command: runner::RunnerCommand,
    },
    Run(output::RunRequest),
    Output {
        #[command(subcommand)]
        command: output::OutputCommand,
    },
    Trust {
        #[command(subcommand)]
        command: output::TrustCommand,
    },
    Savings {
        #[command(subcommand)]
        command: SavingsCommand,
    },
    Repo {
        #[command(subcommand)]
        command: broker::RepoCommand,
    },
    Cache {
        #[command(subcommand)]
        command: broker::CacheCommand,
    },
    Board {
        #[arg(long)]
        watch: bool,
    },
    Activity {
        #[arg(long, default_value_t = 0)]
        since_seq: i64,
        #[arg(long)]
        follow: bool,
    },
    /// Create project configuration and a local workspace binding (write).
    Init,
    /// Inspect current project and capabilities (read).
    Status {
        #[arg(long)]
        capabilities: bool,
    },
    /// Diagnose configuration and databases without changing them (read).
    Doctor,
    /// Publish or clean workspace-local index generations (write).
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },
    /// Search metadata and current file text (read; strict may update index).
    Find(search::FindRequest),
    #[command(flatten)]
    Graph(graph::GraphCommand),
    Query(search::StructureRequest),
    Extract(extract::ExtractRequest),
    /// Read verified syntax metadata (read).
    Outline {
        path: String,
        #[arg(long)]
        depth: Option<usize>,
        #[arg(long,default_value="matched",value_parser=["off","matched","strict"])]
        freshness: String,
    },
    /// Read current lines or a hash-verified symbol (read).
    Read {
        path: Option<String>,
        #[arg(long)]
        lines: Option<String>,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long)]
        symbol_name: Option<String>,
        #[arg(long = "path")]
        named_path: Option<String>,
        #[arg(long)]
        require_complete: bool,
    },
    /// Construct context within an exact serialized byte budget (write index).
    Build(context::BuildRequest),
    /// Save or inspect immutable source hash manifests.
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommand,
    },
    /// Compare a saved manifest with current allowed files (read).
    Changes {
        #[arg(long)]
        since: String,
    },
    /// Store or validate user-authored handoffs.
    Handoff {
        #[command(subcommand)]
        command: HandoffCommand,
    },
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Self::Work(command) => match command {
                work::WorkCommand::Task { .. } => "task",
                work::WorkCommand::Agent { .. } => "agent",
                work::WorkCommand::Check { .. } => "check",
                work::WorkCommand::Control { .. } => "control",
            },
            Self::Operations(command) => match command {
                operations::OperationCommand::Role { .. } => "role",
                operations::OperationCommand::Policy { .. } => "policy",
                operations::OperationCommand::Decision { .. } => "decision",
                operations::OperationCommand::Owner { .. } => "owner",
                operations::OperationCommand::Message { .. } => "message",
                operations::OperationCommand::Inbox { .. } => "inbox",
            },
            Self::Quota { .. } => "quota",
            Self::Pack { .. } => "pack",
            Self::Filter { .. } => "filter",
            Self::Session { .. } => "session",
            Self::Context { .. } => "context",
            Self::Adapter { .. } => "adapter",
            Self::Schedule { .. } => "schedule",
            Self::Inventory { .. } => "inventory",
            Self::Resource { .. } => "resource",
            Self::Job { .. } => "job",
            Self::Runner { .. } => "runner",
            Self::Run(_) => "run",
            Self::Output { .. } => "output",
            Self::Trust { .. } => "trust",
            Self::Savings { .. } => "savings",
            Self::Repo { .. } => "repo",
            Self::Cache { .. } => "cache",
            Self::Board { .. } => "board",
            Self::Activity { .. } => "activity",
            Self::Init => "init",
            Self::Status { .. } => "status",
            Self::Doctor => "doctor",
            Self::Index { .. } => "index",
            Self::Find(_) => "find",
            Self::Graph(command) => match command {
                graph::GraphCommand::Trace { .. } => "trace",
                graph::GraphCommand::Impact { .. } => "impact",
                graph::GraphCommand::Refs { .. } => "refs",
            },
            Self::Query(_) => "query",
            Self::Extract(_) => "extract",
            Self::Outline { .. } => "outline",
            Self::Read { .. } => "read",
            Self::Build(_) => "build",
            Self::Checkpoint { .. } => "checkpoint",
            Self::Changes { .. } => "changes",
            Self::Handoff { .. } => "handoff",
        }
    }
}
#[derive(Subcommand, Debug)]
enum ResourceCommand {
    Status {
        #[arg(long, default_value="current", value_parser=["current"])]
        host: String,
    },
}
#[derive(Subcommand, Debug)]
enum JobCommand {
    Cancel { job: String },
}
#[derive(Subcommand, Debug)]
enum SavingsCommand {
    Report,
    Opportunities,
}
#[derive(Subcommand, Debug)]
enum IndexCommand {
    Update {
        #[arg(long)]
        verify_content: bool,
    },
    Rebuild,
    Gc {
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
}
#[derive(Subcommand, Debug)]
enum CheckpointCommand {
    Create {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        pin: bool,
        #[arg(long = "scope")]
        scopes: Vec<String>,
    },
    List,
    Delete {
        id: String,
    },
}
#[derive(Subcommand, Debug)]
enum HandoffCommand {
    Create {
        #[arg(long)]
        from_file: PathBuf,
        #[arg(long)]
        name: String,
    },
    Update {
        name: String,
        #[arg(long)]
        from_file: PathBuf,
    },
    Show {
        name: String,
        #[arg(long)]
        validate: bool,
    },
}
fn encoded(value: &mut Value, format: &Format, exit: &mut i32) -> Vec<u8> {
    let format = match format {
        Format::Compact => pctx::render::Format::Compact,
        Format::Json | Format::Ndjson => pctx::render::Format::Json,
        Format::Markdown => pctx::render::Format::Markdown,
    };
    match pctx::render::render(value, format) {
        Ok(bytes) => bytes,
        Err(error) => {
            *exit = error.exit;
            *value = domain::envelope("render", None, Value::Null);
            value["status"] = json!("error");
            value["errors"] = json!([error]);
            let mut bytes = value.to_string().into_bytes();
            bytes.push(b'\n');
            bytes
        }
    }
}
fn execute(cli: &Cli) -> Result<(String, Project, Value)> {
    if matches!(cli.format, Format::Markdown) && !pctx::render::supports(cli.command.name()) {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Markdown supports build, outline, read and handoff",
            2,
        ));
    }
    let root = project::detect_root(cli.root.as_deref())?;
    if matches!(cli.command, Command::Init) {
        let p = Project::init(&root)?;
        return Ok((
            "init".into(),
            p.clone(),
            json!({"initialized":true,"project_id":p.project_id,"workspace_id":p.workspace_id,"coordination_id":p.coordination_id}),
        ));
    }
    let p = Project::open(&root)?;
    let (name, data) = match &cli.command {
        Command::Init => unreachable!(),
        Command::Operations(c) => ("operations", operations::execute(&p, c)?),
        Command::Quota { command } => ("quota", quota::execute(&p, command)?),
        Command::Pack { command } => ("pack", pack::execute(&p, command)?),
        Command::Filter { command } => ("filter", filters::execute(&p, command)?),
        Command::Work(c) => ("work", work::execute(&p, c)?),
        Command::Session { command } => ("session", session::session(&p, command)?),
        Command::Context { command } => ("context", session::context(&p, command)?),
        Command::Adapter { command } => ("adapter", adapter::execute(&p, command)?),
        Command::Inventory { command } => ("inventory", inventory::execute(&p, command)?),
        Command::Schedule { command } => ("schedule", schedule::execute(&p, command)?),
        Command::Resource {
            command: ResourceCommand::Status { .. },
        } => (
            "resource",
            runner::execute(&p, &runner::RunnerCommand::ResourceStatus)?,
        ),
        Command::Job {
            command: JobCommand::Cancel { job },
        } => (
            "job",
            runner::execute(&p, &runner::RunnerCommand::JobCancel { job: job.clone() })?,
        ),
        Command::Runner { command } => ("runner", runner::execute(&p, command)?),
        Command::Run(r) => ("run", output::run(&p, r)?),
        Command::Output { command } => ("output", output::output(&p, command)?),
        Command::Trust { command } => ("trust", output::trust(&p, command)?),
        Command::Savings { .. } => ("savings", output::savings(&p)?),
        Command::Repo { command } => ("repo status", broker::repo(&p, command)?),
        Command::Cache { .. } => ("cache stats", broker::stats(&p)?),
        Command::Board { .. } => ("board", work::board(&p)?),
        Command::Activity { since_seq, .. } => ("activity", work::activity(&p, *since_seq)?),
        Command::Status { capabilities } => {
            let snapshot = storage::snapshot(&p).ok();
            (
                "status",
                json!({"generation_id":snapshot.as_ref().and_then(|s|s.0.clone()),"indexed_files":snapshot.map(|s|s.1.len()),"capabilities":if *capabilities {json!({"file_read":"implemented","syntax_outline":"implemented","lexical_search":"implemented","byte_context":"partial","work_control":"partial","graph":"static_imports","pack":"source_plans_and_integrity","tokenizer":"not_registered","claude_adapter":"planned","session_receipts":"partial","masked_output":"partial"})}else{Value::Null},"local_storage":{"index":p.index_db(),"control":p.control_db()},"sqlite_version":rusqlite::version()}),
            )
        }
        Command::Doctor => {
            let mut databases = Vec::new();
            for (kind, path) in [("index", p.index_db()), ("control", p.control_db())] {
                if path.exists() {
                    let d = rusqlite::Connection::open_with_flags(
                        path,
                        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                    )?;
                    let check: String = d.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
                    let version: i64 = d.pragma_query_value(None, "user_version", |r| r.get(0))?;
                    databases.push(json!({"kind":kind,"integrity":check,"schema":version}));
                } else {
                    databases.push(json!({"kind":kind,"status":"not_created"}));
                }
            }
            (
                "doctor",
                json!({"read_only":true,"databases":databases,"sqlite_version":rusqlite::version(),"policy_hash":p.policy_hash()}),
            )
        }
        Command::Index { command } => match command {
            IndexCommand::Update { .. } => ("index update", storage::update(&p)?),
            IndexCommand::Rebuild => ("index rebuild", storage::rebuild(&p)?),
            IndexCommand::Gc { apply, .. } => ("index gc", storage::gc(&p, *apply)?),
        },
        Command::Find(r) => {
            if r.freshness == "strict" {
                storage::update(&p)?;
            }
            let (_, files) = storage::snapshot(&p)?;
            ("find", search::find(&p, &files, r)?)
        }
        Command::Extract(r) => ("extract", extract::extract(&p, r)?),
        Command::Graph(r) => ("graph", graph::execute(&p, r)?),
        Command::Query(r) => {
            if r.freshness == "strict" {
                storage::update(&p)?;
            }
            let (_, files) = storage::snapshot(&p)?;
            ("query", search::query_structure(&p, &files, r)?)
        }
        Command::Outline {
            path,
            depth,
            freshness,
        } => {
            if freshness == "strict" {
                storage::update(&p)?;
            }
            let (_, files) = storage::snapshot(&p)?;
            (
                "outline",
                search::outline(&p, &files, path, *depth, freshness)?,
            )
        }
        Command::Read {
            path,
            lines,
            symbol,
            symbol_name,
            named_path,
            require_complete,
        } => {
            let files = if symbol.is_some() || symbol_name.is_some() {
                storage::snapshot(&p)?.1
            } else {
                vec![]
            };
            let data = search::read_selection(
                &p,
                &files,
                path.as_deref().or(named_path.as_deref()),
                lines.as_deref(),
                symbol.as_deref(),
                symbol_name.as_deref(),
            )?;
            if *require_complete && data["completeness"] == "partial" {
                return Err(Error::new(
                    "PARTIAL_RESULT",
                    "Read body exceeds requested completeness limits",
                    3,
                ));
            }
            ("read", data)
        }
        Command::Build(r) => ("build", context::build(&p, r)?),
        Command::Checkpoint { command } => match command {
            CheckpointCommand::Create { name, pin, scopes } => (
                "checkpoint create",
                storage::checkpoint_scoped(&p, name.as_deref(), *pin, scopes)?,
            ),
            CheckpointCommand::List => ("checkpoint list", storage::checkpoint_list(&p)?),
            CheckpointCommand::Delete { id } => {
                ("checkpoint delete", storage::checkpoint_delete(&p, id)?)
            }
        },
        Command::Changes { since } => ("changes", storage::changes(&p, since)?),
        Command::Handoff { command } => match command {
            HandoffCommand::Create { from_file, name } => (
                "handoff create",
                storage::handoff_create(&p, name, from_file, false)?,
            ),
            HandoffCommand::Update { name, from_file } => (
                "handoff update",
                storage::handoff_create(&p, name, from_file, true)?,
            ),
            HandoffCommand::Show { name, validate } => {
                ("handoff show", storage::handoff_show(&p, name, *validate)?)
            }
        },
    };
    Ok((name.into(), p, data))
}
fn stream(cli: &Cli) -> Result<()> {
    if cli.output.is_some() {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Streaming requires stdout; redirect explicitly",
            2,
        ));
    }
    let p = Project::open(&project::detect_root(cli.root.as_deref())?)?;
    let ndjson = matches!(cli.format, Format::Ndjson);
    match &cli.command {
        Command::Board { watch } => pctx::watch::run(&p, true, 0, *watch, ndjson),
        Command::Activity { since_seq, follow } => {
            pctx::watch::run(&p, false, *since_seq, *follow, ndjson)
        }
        _ => Err(Error::new(
            "INVALID_ARGUMENT",
            "NDJSON supports board and activity",
            2,
        )),
    }
}

fn main() {
    let raw = std::env::args().collect::<Vec<_>>();
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            if e.use_stderr() && raw.iter().any(|s| s == "json" || s == "--format=json") {
                let err = Error::new(
                    "INVALID_ARGUMENT",
                    "Invalid command or option; see pctx --help",
                    2,
                );
                let mut out = domain::envelope("arguments", None, Value::Null);
                out["status"] = json!("error");
                out["errors"] = json!([err]);
                println!("{}", out);
                std::process::exit(2);
            }
            e.exit()
        }
    };
    let follows = matches!(
        &cli.command,
        Command::Board { watch: true } | Command::Activity { follow: true, .. }
    );
    if follows && matches!(cli.format, Format::Json) {
        let e = Error::new(
            "INVALID_ARGUMENT",
            "JSON watch is unavailable; use compact or ndjson",
            2,
        );
        let mut response = domain::envelope("arguments", None, Value::Null);
        response["status"] = json!("error");
        response["errors"] = json!([e]);
        println!("{}", response);
        std::process::exit(2);
    }
    if follows || matches!(cli.format, Format::Ndjson) {
        if let Err(e) = stream(&cli) {
            let response = json!({"schema_version":"1.0","event_namespace":"work","event_seq":null,"type":"error","data":e});
            use std::io::Write;
            let _ = writeln!(std::io::stderr(), "{}", response);
            std::process::exit(e.exit);
        }
        return;
    }
    let (mut response, mut exit) = match execute(&cli) {
        Ok((name, p, data)) => {
            let mut out = domain::envelope(&name, Some(&p), data);
            let freshness = match &cli.command {
                Command::Find(r) => Some(r.freshness.as_str()),
                Command::Outline { freshness, .. } => Some(freshness.as_str()),
                Command::Build(_) => Some("strict"),
                _ => None,
            };
            if let Some(mode) = freshness {
                out["validation"]["mode"] = json!(mode);
                if mode == "matched" || mode == "off" {
                    out["coverage"] = json!({"status":"partial","reasons":["candidate_universe_not_revalidated"]});
                }
            }
            if let Some(g) = out["data"]["generation_id"].as_str() {
                out["generation_id"] = json!(g);
            }
            let partial = out["data"]["coverage"] == "partial"
                || out["data"]["coverage"]["status"] == "partial"
                || out["data"]["completeness"] == "partial"
                || out["data"]["files"]
                    .as_array()
                    .is_some_and(|fs| fs.iter().any(|f| f["coverage"]["status"] == "partial"));
            let exit = if partial {
                out["status"] = json!("partial");
                out["coverage"] = json!({"status":"partial","reasons":["request_incomplete"]});
                3
            } else {
                0
            };
            (out, exit)
        }
        Err(e) => {
            let exit = e.exit;
            let mut out = domain::envelope(cli.command.name(), None, Value::Null);
            out["status"] = json!("error");
            out["coverage"] = json!({"status":"partial","reasons":[e.code.clone()]});
            out["errors"] = json!([e]);
            (out, exit)
        }
    };
    if matches!(&cli.command, Command::Read { .. }) {
        while serde_json::to_vec(&response).unwrap().len() + 1 > 65536 {
            let Some(text) = response["data"]["text"].as_str() else {
                break;
            };
            let mut cut = text.len() / 2;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            response["data"]["text"] = json!(&text[..cut]);
            response["data"]["completeness"] = json!("partial");
            response["data"]["truncated"] = json!(true);
            response["data"]["returned_excerpt"] = json!(true);
            response["status"] = json!("partial");
            exit = 3;
        }
        if response["data"]["truncated"] == true {
            response["truncation"] = json!({"truncated":true,"reasons":["body_limit"]});
            response["coverage"] = json!({"status":"partial","reasons":["body_limit"]});
        }
    }
    if response["data"]["hook_transport"] == true
        && response["data"]["hook_output"].is_object()
        && exit == 0
    {
        println!("{}", response["data"]["hook_output"]);
        return;
    }
    let render_format = if response["errors"].as_array().is_some_and(|errors| {
        errors
            .iter()
            .any(|error| error["code"] == "BUDGET_TOO_SMALL")
    }) || matches!(cli.format, Format::Markdown)
        && !pctx::render::supports(cli.command.name())
    {
        Format::Json
    } else {
        cli.format.clone()
    };
    let bytes = encoded(&mut response, &render_format, &mut exit);
    // Budget always applies to the format actually emitted, including the newline.
    let limit = match &cli.command {
        Command::Build(r) => Some(r.budget_bytes),
        Command::Context {
            command: session::ContextCommand::Get { budget_bytes, .. },
        } => Some(*budget_bytes),
        Command::Run(r) => Some(r.budget_bytes),
        Command::Runner {
            command:
                runner::RunnerCommand::CheckRun { budget_bytes, .. }
                | runner::RunnerCommand::HelperRequest { budget_bytes, .. },
        } => Some(*budget_bytes),
        Command::Work(work::WorkCommand::Check {
            command: work::CheckCommand::Run { budget_bytes, .. },
        }) => Some(*budget_bytes),
        Command::Extract(r) => Some(r.budget_bytes),
        Command::Read { .. } => Some(65536),
        _ => None,
    };
    let mut bytes = bytes;
    let mut execution_pointer = if response["data"]["execution"].is_object() {
        "/data/execution"
    } else {
        "/data"
    };
    if response
        .pointer(execution_pointer)
        .is_some_and(|v| v["output_id"].is_string())
        && let Some(limit) = limit
    {
        let records_pointer = format!("{execution_pointer}/records");
        while bytes.len() > limit
            && response
                .pointer(&records_pointer)
                .unwrap()
                .as_array()
                .is_some_and(|r| !r.is_empty())
        {
            let execution = response.pointer_mut(execution_pointer).unwrap();
            execution["records"].as_array_mut().unwrap().pop();
            let count = execution["records_omitted"].as_u64().unwrap_or(0) + 1;
            execution["records_omitted"] = json!(count);
            execution["records_included"] = json!(execution["records"].as_array().unwrap().len());
            bytes = encoded(&mut response, &render_format, &mut exit);
        }
    }
    if let Some(limit) = limit
        && bytes.len() > limit
    {
        let e = Error::new(
            "BUDGET_TOO_SMALL",
            "Rendered context exceeds requested budget; use JSON or increase budget",
            8,
        );
        // Keep the durable reread handle and original execution outcome even when
        // presentation cannot fit. The wrapper failure never replaces child truth.
        let mut proof = serde_json::Map::new();
        if let Some(execution) = response.pointer(execution_pointer)
            && execution["output_id"].is_string()
        {
            for key in [
                "output_id",
                "query_ref",
                "spawned",
                "child_exit_code",
                "signal",
                "execution_status",
                "delivery_kind",
            ] {
                if let Some(value) = execution.get(key) {
                    proof.insert(key.into(), value.clone());
                }
            }
        }
        response = domain::envelope(cli.command.name(), None, Value::Object(proof));
        execution_pointer = "/data";
        response["status"] = json!("error");
        response["errors"] = json!([e]);
        // The complete JSON error is the smallest reversible fallback. Budgets
        // smaller than that envelope cannot hold a valid error document.
        bytes = encoded(&mut response, &Format::Json, &mut exit);
        exit = 8;
    }
    if let Command::Run(r) = &cli.command
        && response["status"] != "error"
        && response["data"]["spawned"] == true
        && r.exit_policy == "child"
    {
        exit = response["data"]["child_exit_code"]
            .as_i64()
            .map(|v| v as i32)
            .or_else(|| response["data"]["signal"].as_i64().map(|s| 128 + s as i32))
            .unwrap_or(exit);
    }
    let delivered = if let Some(output) = &cli.output {
        match project::atomic_write(output, &bytes, false) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("{e}");
                exit = e.exit;
                false
            }
        }
    } else {
        use std::io::Write;
        match std::io::stdout().write_all(&bytes) {
            Ok(()) => true,
            Err(_) => {
                exit = 7;
                false
            }
        }
    };
    if delivered
        && let Some(output_id) = response
            .pointer(execution_pointer)
            .and_then(|v| v["output_id"].as_str())
        && let Ok(root) = project::detect_root(cli.root.as_deref())
        && let Ok(p) = Project::open(&root)
    {
        let kind = if response["data"]["delivery_kind"] == "retrieval" {
            "retrieval"
        } else {
            "compact"
        };
        let _ = output::record_delivery(&p, output_id, bytes.len() as u64, kind);
    }
    std::process::exit(exit)
}

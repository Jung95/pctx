mod cli_help;

use clap::{ColorChoice, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
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
    /// One cooperative budget for a finite query, including refresh and output preparation.
    #[arg(long, global = true)]
    timeout_ms: Option<u64>,
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
    /// Read verified syntax metadata (read; strict may update index).
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
// A ready generation may still omit sources. Strict lookup must carry that
// refresh uncertainty rather than treating the available metadata as exhaustive.
fn with_refresh_coverage(mut value: Value, refresh: Option<Value>) -> Value {
    if let Some(refresh) = refresh.filter(|refresh| refresh["coverage"] == "partial") {
        let mut reasons: Vec<_> = refresh["skipped"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|skip| skip["reason"].as_str())
            .map(str::to_owned)
            .collect();
        reasons.sort();
        reasons.dedup();
        if !value["coverage"].is_object() {
            value["coverage"] = json!({});
        }
        value["coverage"]["status"] = json!("partial");
        value["coverage"]["refresh_reasons"] = json!(reasons);
        value["coverage"]["omitted_count"] = Value::Null;
        value["omitted_count"] = Value::Null;
        value["refresh"] = json!({"coverage":"partial", "generation_id":refresh["generation_id"],
            "skipped_count":refresh["skipped"].as_array().map(Vec::len), "reasons":reasons});
    }
    value
}
fn encoded(
    value: &mut Value,
    format: &Format,
    exit: &mut i32,
    deadline: Option<pctx::deadline::Deadline>,
) -> Vec<u8> {
    let format = match format {
        Format::Compact => pctx::render::Format::Compact,
        Format::Json | Format::Ndjson => pctx::render::Format::Json,
        Format::Markdown => pctx::render::Format::Markdown,
    };
    match pctx::render::render(value, format) {
        Ok(bytes) => bytes,
        Err(error) => {
            let error = request_error(deadline, error);
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
fn query_deadline(cli: &Cli) -> Result<Option<pctx::deadline::Deadline>> {
    let default = match &cli.command {
        Command::Index { .. } | Command::Build(_) | Command::Checkpoint { .. } => Some(120_000),
        Command::Work(work::WorkCommand::Task {
            command:
                work::TaskCommand::List
                | work::TaskCommand::Show { .. }
                | work::TaskCommand::Complete { dry_run: true, .. },
        })
        | Command::Work(work::WorkCommand::Agent {
            command: work::AgentCommand::List | work::AgentCommand::Show { .. },
        })
        | Command::Work(work::WorkCommand::Check {
            command:
                work::CheckCommand::List { .. }
                | work::CheckCommand::Show { .. }
                | work::CheckCommand::Plan { .. },
        })
        | Command::Quota {
            command:
                quota::QuotaCommand::Report { .. }
                | quota::QuotaCommand::Plan { .. }
                | quota::QuotaCommand::Reconcile { .. },
        } => Some(10_000),
        Command::Schedule {
            command:
                schedule::ScheduleCommand::List { .. }
                | schedule::ScheduleCommand::Plan { .. }
                | schedule::ScheduleCommand::Inspect { .. },
        }
        | Command::Filter {
            command:
                filters::FilterCommand::Validate { .. }
                | filters::FilterCommand::Apply { .. }
                | filters::FilterCommand::Explain { .. },
        }
        | Command::Resource {
            command: ResourceCommand::Status { .. },
        }
        | Command::Runner {
            command:
                runner::RunnerCommand::CheckPlan { .. }
                | runner::RunnerCommand::ResourceStatus
                | runner::RunnerCommand::HelperStatus { .. },
        }
        | Command::Trust {
            command: output::TrustCommand::Plan { .. },
        }
        | Command::Adapter {
            command:
                adapter::AdapterCommand::Claude {
                    command:
                        adapter::ClaudeCommand::Doctor
                        | adapter::ClaudeCommand::Verify
                        | adapter::ClaudeCommand::ProtocolFixture { .. },
                },
        }
        | Command::Inventory { .. }
        | Command::Find(_)
        | Command::Query(_)
        | Command::Extract(_)
        | Command::Graph(_)
        | Command::Outline { .. }
        | Command::Read { .. }
        | Command::Changes { .. }
        | Command::Status { .. }
        | Command::Doctor
        | Command::Repo { .. }
        | Command::Cache { .. }
        | Command::Output { .. }
        | Command::Savings { .. }
        | Command::Session { .. }
        | Command::Context { .. }
        | Command::Pack { .. }
        | Command::Operations(_)
        | Command::Handoff {
            command: HandoffCommand::Show { .. },
        }
        | Command::Board { watch: false }
        | Command::Activity { follow: false, .. } => Some(10_000),
        _ => None,
    };
    match (cli.timeout_ms, default) {
        (Some(_), None) => Err(Error::new(
            "INVALID_ARGUMENT",
            "--timeout-ms requires a supported finite query; child execution uses its execution timeout",
            2,
        )),
        (specified, Some(default)) => {
            pctx::deadline::Deadline::from_millis(specified.unwrap_or(default)).map(Some)
        }
        (None, None) => Ok(None),
    }
}
fn request_error(deadline: Option<pctx::deadline::Deadline>, error: Error) -> Error {
    deadline
        .and_then(|deadline| deadline.check().err())
        .unwrap_or(error)
}
fn execute(
    cli: &Cli,
    deadline: Option<pctx::deadline::Deadline>,
) -> Result<(String, Project, Value)> {
    if matches!(cli.format, Format::Markdown) && !pctx::render::supports(cli.command.name()) {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Markdown supports build, outline, read and handoff",
            2,
        ));
    }
    let root = project::detect_root_with_deadline(cli.root.as_deref(), deadline)?;
    if matches!(cli.command, Command::Init) {
        let p = Project::init(&root)?;
        return Ok((
            "init".into(),
            p.clone(),
            json!({"initialized":true,"project_id":p.project_id,"workspace_id":p.workspace_id,"coordination_id":p.coordination_id}),
        ));
    }
    let p = Project::open_with_deadline(&root, deadline)?;
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
            let snapshot = match storage::snapshot(&p) {
                Ok(snapshot) => Some(snapshot),
                Err(e) if e.code == "TIMEOUT" => return Err(e),
                Err(_) => None,
            };
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
                    if let Some(deadline) = p.deadline {
                        d.progress_handler(1000, Some(move || deadline.check().is_err()))?;
                    }
                    d.busy_timeout(
                        p.remaining(std::time::Duration::from_secs(5))?
                            .saturating_add(std::time::Duration::from_nanos(999_999)),
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
            let refresh = if r.freshness == "strict" {
                Some(storage::update(&p)?)
            } else {
                None
            };
            (
                "find",
                with_refresh_coverage(search::find_indexed(&p, r)?, refresh),
            )
        }
        Command::Extract(r) => ("extract", extract::extract(&p, r)?),
        Command::Graph(r) => ("graph", graph::execute(&p, r)?),
        Command::Query(r) => {
            let refresh = if r.freshness == "strict" {
                Some(storage::update(&p)?)
            } else {
                None
            };
            let (_, files) = storage::snapshot(&p)?;
            (
                "query",
                with_refresh_coverage(search::query_structure(&p, &files, r)?, refresh),
            )
        }
        Command::Outline {
            path,
            depth,
            freshness,
        } => {
            let refresh = if freshness == "strict" {
                Some(storage::update(&p)?)
            } else {
                None
            };
            let (_, files) = storage::snapshot(&p)?;
            (
                "outline",
                with_refresh_coverage(
                    search::outline(&p, &files, path, *depth, freshness)?,
                    refresh,
                ),
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
        Command::Build(r) => (
            "build",
            context::build_with_format(
                &p,
                r,
                match cli.format {
                    Format::Markdown => pctx::render::Format::Markdown,
                    Format::Compact => pctx::render::Format::Compact,
                    Format::Json | Format::Ndjson => pctx::render::Format::Json,
                },
            )?,
        ),
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
fn stream(cli: &Cli, deadline: Option<pctx::deadline::Deadline>) -> Result<()> {
    if cli.output.is_some() {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "Streaming requires stdout; redirect explicitly",
            2,
        ));
    }
    if !matches!(
        &cli.command,
        Command::Board { .. } | Command::Activity { .. }
    ) {
        return Err(Error::new(
            "INVALID_ARGUMENT",
            "NDJSON supports board and activity",
            2,
        ));
    }
    let root = project::detect_root_with_deadline(cli.root.as_deref(), deadline)?;
    let p = Project::open_with_deadline(&root, deadline)?;
    let ndjson = matches!(cli.format, Format::Ndjson);
    let result = match &cli.command {
        Command::Board { watch } => pctx::watch::run(&p, true, 0, *watch, ndjson),
        Command::Activity { since_seq, follow } => {
            pctx::watch::run(&p, false, *since_seq, *follow, ndjson)
        }
        _ => Err(Error::new(
            "INVALID_ARGUMENT",
            "NDJSON supports board and activity",
            2,
        )),
    };
    p.check_deadline()?;
    result
}

// A capacity guard is argument validation, not a context selection failure.
// The bound is derived from a real reversible JSON error document and reserves the
// longest current RFC3339 nanosecond timestamp. Never open a project to compute it.
fn output_budget(command: &Command) -> Option<usize> {
    match command {
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
    }
}
fn minimum_budget_error(command: &str, minimum: usize) -> Value {
    let mut response = domain::envelope(command, None, json!({"minimum_budget_bytes":minimum}));
    response["status"] = json!("error");
    response["coverage"] = json!({"status":"partial","reasons":["INVALID_ARGUMENT"]});
    response["errors"] = json!([Error::new(
        "INVALID_ARGUMENT",
        "Byte budget is below minimum JSON error envelope",
        2
    )]);
    response
}
fn minimum_error_budget(command: &str) -> Result<usize> {
    let mut minimum = 0;
    for _ in 0..3 {
        let mut response = minimum_budget_error(command, minimum);
        response["validation"]["checked_at"] = json!("2000-01-01T00:00:00.123456789+00:00");
        let measured = pctx::render::render(&response, pctx::render::Format::Json)?.len();
        if measured == minimum {
            return Ok(minimum);
        }
        minimum = measured;
    }
    Err(Error::new(
        "INVALID_ARGUMENT",
        "Cannot establish minimum JSON error budget",
        2,
    ))
}

fn main() {
    #[cfg(windows)]
    {
        // Fixed private entry precedes project loading and JSON rendering: the
        // registered keeper uses only its bounded owned-pipe protocol.
        let args: Vec<_> = std::env::args_os().collect();
        if args.len() == 2 && args[1] == "__pctx-windows-guardian-v1" {
            let status = if pctx::windows_guardian::run_from_stdio().is_ok() {
                0
            } else {
                7
            };
            std::process::exit(status);
        }
    }
    let raw = std::env::args_os().collect::<Vec<_>>();
    let options = raw.iter().take_while(|s| *s != "--").collect::<Vec<_>>();
    let json_requested = options.iter().any(|s| **s == "--format=json")
        || options
            .windows(2)
            .any(|pair| pair[0] == "--format" && pair[1] == "json");
    let no_color = options.iter().any(|s| **s == "--no-color");
    let mut command = cli_help::annotate(Cli::command());
    if no_color || json_requested {
        command = command.color(ColorChoice::Never);
    }
    let parsed = command
        .try_get_matches_from(raw.iter().cloned())
        .and_then(|matches| Cli::from_arg_matches(&matches));
    let cli = match parsed {
        Ok(c) => c,
        Err(e) => {
            if e.use_stderr() && json_requested {
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
            if e.use_stderr() {
                // Parser diagnostics also cross the shared secret/control boundary.
                let (message, _) = pctx::reader::redact(&e.to_string());
                let message = message
                    .chars()
                    .flat_map(|c| {
                        if c.is_control() && !matches!(c, '\n' | '\t') {
                            c.escape_unicode().collect::<Vec<_>>()
                        } else {
                            vec![c]
                        }
                    })
                    .collect::<String>();
                eprintln!("{message}");
                std::process::exit(e.exit_code());
            }
            e.exit()
        }
    };
    // Pure argument checks precede project discovery, refresh and output paths.
    let preflight = match &cli.command {
        Command::Find(request) => search::validate_find_request(request),
        Command::Query(request) => search::validate_structure_request(request),
        Command::Inventory {
            command:
                inventory::InventoryCommand::Scan {
                    max_files,
                    max_bytes,
                    ..
                },
        } => inventory::validate_limits(*max_files, *max_bytes),
        _ => Ok(()),
    };
    if let Err(error) = preflight {
        let exit = error.exit;
        let mut response = domain::envelope(cli.command.name(), None, Value::Null);
        response["status"] = json!("error");
        response["errors"] = json!([error]);
        println!("{}", response);
        std::process::exit(exit);
    }
    if let Some(limit) = output_budget(&cli.command) {
        match minimum_error_budget(cli.command.name()) {
            Ok(minimum) if limit < minimum => {
                let response = minimum_budget_error(cli.command.name(), minimum);
                // Invalid capacity is always one JSON error on stdout. --output
                // must not turn rejected arguments into a filesystem write.
                if let Ok(bytes) = pctx::render::render(&response, pctx::render::Format::Json) {
                    use std::io::Write;
                    let _ = std::io::stdout().write_all(&bytes);
                }
                std::process::exit(2);
            }
            Err(e) => {
                let mut response = domain::envelope(cli.command.name(), None, Value::Null);
                response["status"] = json!("error");
                response["errors"] = json!([e]);
                println!("{}", response);
                std::process::exit(2);
            }
            _ => {}
        }
    }
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
    let deadline = match query_deadline(&cli) {
        Ok(deadline) => deadline,
        Err(e) => {
            let mut response = domain::envelope("arguments", None, Value::Null);
            response["status"] = json!("error");
            response["errors"] = json!([e]);
            println!("{}", response);
            std::process::exit(2);
        }
    };
    if follows || matches!(cli.format, Format::Ndjson) {
        if let Err(e) = stream(&cli, deadline) {
            let e = request_error(deadline, e);
            let response = json!({"schema_version":"1.0","event_namespace":"work","event_seq":null,"type":"error","data":e});
            use std::io::Write;
            let _ = writeln!(std::io::stderr(), "{}", response);
            std::process::exit(e.exit);
        }
        return;
    }
    let mut opened_project = None;
    let (mut response, mut exit) = match execute(&cli, deadline) {
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
            opened_project = Some(p);
            (out, exit)
        }
        Err(e) => {
            let e = request_error(deadline, e);
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
    let mut bytes = encoded(&mut response, &render_format, &mut exit, deadline);
    let usable_timeout_partial = matches!(&cli.command, Command::Find(_))
        && response["data"]["coverage"]["reasons"]
            .as_array()
            .is_some_and(|reasons| reasons.iter().any(|reason| reason == "timeout"))
        && response["data"]["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty());
    if response["status"] != "error"
        && !usable_timeout_partial
        && let Some(deadline) = deadline
        && let Err(e) = deadline.check()
    {
        response = domain::envelope(cli.command.name(), None, Value::Null);
        response["status"] = json!("error");
        response["coverage"] = json!({"status":"partial","reasons":["TIMEOUT"]});
        response["errors"] = json!([e]);
        exit = 7;
        bytes = encoded(&mut response, &Format::Json, &mut exit, Some(deadline));
    }
    // Budget always applies to the format actually emitted, including the newline.
    let limit = output_budget(&cli.command);
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
            bytes = encoded(&mut response, &render_format, &mut exit, deadline);
        }
    }
    if let Some(limit) = limit
        && bytes.len() > limit
    {
        let e = Error::new("BUDGET_TOO_SMALL", "Output exceeds byte budget", 8);
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
        bytes = encoded(&mut response, &Format::Json, &mut exit, deadline);
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
    // Clap propagates the shared --output argument to the global field. For
    // Pack Create it names the artifact; its response still belongs on stdout.
    let response_output = match &cli.command {
        Command::Pack {
            command: pack::PackCommand::Create { .. },
        } => None,
        _ => cli.output.as_ref(),
    };
    let delivered = if let Some(output) = response_output {
        match project::atomic_write(output, &bytes, false) {
            Ok(()) => true,
            Err(e) => {
                let e = request_error(deadline, e);
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
        && let Some(p) = opened_project.as_ref()
    {
        let kind = if response["data"]["delivery_kind"] == "retrieval" {
            "retrieval"
        } else {
            "compact"
        };
        if let Err(error) = output::record_delivery(p, output_id, bytes.len() as u64, kind) {
            // The completed write and its exit outcome stand. Accounting has no
            // renewed budget or inferred provider/receipt observation.
            let error = request_error(deadline, error);
            let diagnostic = json!({"code":"OUTPUT_MEASUREMENT_UNRECORDED",
                "message":"Output was written; byte accounting could not be confirmed",
                "reason":error.code,"delivery_written":true,"measurement_recorded":"unknown"});
            eprintln!("{diagnostic}");
        }
    }
    std::process::exit(exit)
}

#[cfg(test)]
mod finite_route_tests {
    use super::*;
    #[test]
    fn finite_work_quota_adapter_reads_do_not_time_execution() {
        for values in [
            vec!["task", "list"],
            vec!["task", "show", "T001"],
            vec!["task", "complete", "T001", "--dry-run"],
            vec!["agent", "list"],
            vec!["agent", "show", "A001"],
            vec!["check", "list"],
            vec!["check", "show", "C001"],
            vec!["check", "plan", "--task-id", "T001", "--key", "test"],
            vec!["adapter", "claude", "doctor"],
            vec!["adapter", "claude", "verify"],
            vec![
                "adapter",
                "claude",
                "protocol-fixture",
                "--from-file",
                "fixture.json",
            ],
            vec!["quota", "report"],
            vec!["quota", "plan", "--pool", "local"],
            vec!["quota", "reconcile", "--pool", "local"],
            vec!["resource", "status"],
            vec!["runner", "resource-status"],
            vec!["runner", "check-plan", "--task-id", "T001", "--key", "test"],
            vec!["runner", "helper-status", "H001"],
            vec!["trust", "plan", "--", "/bin/sh"],
            vec!["filter", "validate", ".pctx/filters/sample.toml"],
            vec![
                "filter",
                "apply",
                "--filter",
                "sample",
                "--input",
                "-",
                "--child-exit",
                "1",
            ],
            vec!["filter", "explain", "--", "fixture"],
            vec!["inventory", "scan"],
            vec!["inventory", "profile", "--path", "profile.json"],
            vec!["inventory", "audit", "--registry", "registry.json"],
            vec!["schedule", "list"],
            vec!["schedule", "plan", "--namespace", "local", "digest"],
            vec![
                "schedule",
                "inspect",
                "--namespace",
                "local",
                "digest",
                "--observe-native",
            ],
        ] {
            let mut argv = vec!["pctx", "--timeout-ms", "100"];
            argv.extend(values.clone());
            let cli = Cli::try_parse_from(argv).unwrap();
            let deadline = query_deadline(&cli).unwrap().unwrap();
            assert!(
                deadline.remaining().unwrap() <= std::time::Duration::from_millis(100),
                "{values:?}"
            );
        }
        for values in [
            vec!["task", "complete", "T001"],
            vec!["task", "cancel", "T001", "--reason", "fixture"],
            vec!["adapter", "claude", "plan", "--agent", "A001"],
            vec![
                "adapter",
                "claude",
                "install",
                "--plan",
                "plan",
                "--expect-hash",
                "hash",
            ],
            vec!["quota", "release", "Q001"],
            vec!["check", "run", "test", "--task-id", "T001", "--run", "R001"],
            vec!["board", "--watch"],
            vec!["activity", "--follow"],
            vec!["runner", "trust", "--key", "test", "--expect-hash", "hash"],
            vec![
                "runner",
                "check-run",
                "--task-id",
                "T001",
                "--key",
                "test",
                "--run",
                "R001",
            ],
            vec![
                "runner",
                "helper-request",
                "--task-id",
                "T001",
                "--key",
                "test",
                "--run",
                "R001",
            ],
            vec!["runner", "helper-cancel", "H001"],
            vec!["runner", "helper-release", "H001", "--evidence", "O001"],
            vec!["runner", "job-cancel", "J001"],
            vec![
                "runner",
                "bridge-guardian",
                "--fd",
                "3",
                "--lock-path",
                "lock",
            ],
            vec!["trust", "add", "--expect-hash", "hash", "--", "/bin/sh"],
            vec![
                "filter",
                "test",
                ".pctx/filters/sample.toml",
                "--fixtures",
                "fixtures",
            ],
            vec!["filter", "activate", "sample", "--expect-hash", "hash"],
            vec![
                "schedule",
                "add",
                "--from-file",
                "definition",
                "--idempotency-key",
                "key",
            ],
            vec![
                "schedule",
                "install",
                "--from-file",
                "plan",
                "--expect-hash",
                "hash",
            ],
            vec!["schedule", "tick"],
            vec!["schedule", "run-loop"],
            vec!["schedule", "reconcile"],
        ] {
            let mut argv = vec!["pctx"];
            argv.extend(values.clone());
            assert!(
                query_deadline(&Cli::try_parse_from(&argv).unwrap())
                    .unwrap()
                    .is_none(),
                "{values:?}"
            );
            argv.splice(1..1, ["--timeout-ms", "100"]);
            assert_eq!(
                query_deadline(&Cli::try_parse_from(argv).unwrap())
                    .unwrap_err()
                    .code,
                "INVALID_ARGUMENT",
                "{values:?}"
            );
        }
    }
}

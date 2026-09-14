mod compose;
mod compose_reindex;
mod deployment_context;
mod deployment_diagram;
mod input;
mod output;
mod snapshot_config;
mod sql;
mod sql_link_context;
mod sql_link_diagram;
mod sql_links;

use std::{
    io::{IsTerminal, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Parser, Subcommand};
use graph_application::{
    AnalysisRepository, ArtifactObservationRepository, ArtifactRepository, EvidenceRepository,
    RpcLaunchQueryRepository, TaskService,
};
use graph_protocol::{Event, Lease, SCHEMA_VERSION, TaskSpec};
use graph_store::Store;

#[derive(Debug, Parser)]
#[command(
    name = "project-graph-agent",
    version,
    about = "Project graph harness: durable task control plane"
)]
struct Cli {
    /// SQLite state database. CLI values take precedence over the default.
    #[arg(long, default_value = "harness.db")]
    database: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Args)]
struct ComposeReindexArgs {
    /// Registered citation selecting project/graph/path; its old hash is not a constraint.
    id: String,
    task_spec: PathBuf,
    root: PathBuf,
    /// Caller-declared run label, not proof that a registered analyzer ran.
    #[arg(long)]
    analysis_run: String,
    #[arg(long)]
    expected_generation: u64,
    #[arg(long, default_value_t = 1048576, value_parser = clap::value_parser!(u32).range(1..=1048576))]
    max_source_bytes: u32,
    #[arg(long, default_value_t = 262144, value_parser = clap::value_parser!(u32).range(0..=16777216))]
    max_output_bytes: u32,
}

impl ComposeReindexArgs {
    fn options(&self) -> compose_reindex::ReindexOptions<'_> {
        compose_reindex::ReindexOptions {
            id: &self.id,
            task_spec: &self.task_spec,
            root: &self.root,
            analysis_run: &self.analysis_run,
            expected: self.expected_generation,
            max_source_bytes: self.max_source_bytes,
            max_output_bytes: self.max_output_bytes,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Export a declared architecture IR bundle with a separate historical evidence manifest.
    /// Does not render or verify Git links. Overview only; DB open may migrate.
    DeploymentDiagram {
        id: String,
        task_spec: PathBuf,
        #[arg(long)]
        node: String,
        #[arg(long,default_value="both",value_parser=["incoming","outgoing","both"])]
        direction: String,
        #[arg(long,default_value_t=2,value_parser=clap::value_parser!(u32).range(0..=16))]
        depth: u32,
        #[arg(long,default_value_t=12,value_parser=clap::value_parser!(u32).range(1..=12))]
        max_nodes: u32,
        #[arg(long,default_value_t=24,value_parser=clap::value_parser!(u32).range(0..=24))]
        max_edges: u32,
        #[arg(long,default_value_t=65536,value_parser=clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Read a bounded declared neighborhood with citations, without reading source bodies.
    /// Historical graph only; missing/invalidated owner or seed is an error. DB open may migrate.
    DeploymentContext {
        id: String,
        task_spec: PathBuf,
        #[arg(long)]
        node: String,
        #[arg(long,default_value="both",value_parser=["incoming","outgoing","both"])]
        direction: String,
        #[arg(long,default_value_t=2,value_parser=clap::value_parser!(u32).range(0..=16))]
        depth: u32,
        #[arg(long,default_value_t=40,value_parser=clap::value_parser!(u32).range(1..=1000))]
        max_nodes: u32,
        #[arg(long,default_value_t=80,value_parser=clap::value_parser!(u32).range(0..=5000))]
        max_edges: u32,
        #[arg(long,default_value_t=32768,value_parser=clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Capture current Compose bytes and replace graph, or retract it on source/parser failure.
    /// Invalid source emits an invalidation report and exits nonzero. Binding is caller supplied.
    ReindexCompose(ComposeReindexArgs),
    /// Opt-in sequential polling of one Compose owner, not a recursive project watcher.
    /// Source errors invalidate and permit recovery; CAS/DB/output errors stop. Query is historical.
    WatchCompose {
        #[command(flatten)]
        args: ComposeReindexArgs,
        #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(10..=60000))]
        interval_ms: u64,
        /// Omit to poll until interrupted. Each cycle emits one bounded JSON line.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=1000000))]
        max_cycles: Option<u32>,
    },
    /// Publish declared Compose graph after verifying source bytes; never launches Docker.
    /// Task/root binding is caller supplied, not worker authorization. On output failure
    /// the commit may already exist: inspect deployment generation before retrying.
    PublishCompose {
        id: String,
        task_spec: PathBuf,
        root: PathBuf,
        /// Strict CAS; zero means no prior graph owner. Required even for retries.
        #[arg(long)]
        expected_generation: u64,
        #[arg(long, default_value_t = 1048576, value_parser = clap::value_parser!(u32).range(1..=1048576))]
        max_source_bytes: u32,
        /// Receipt bytes including newline; rejection occurs before graph mutation.
        #[arg(long, default_value_t = 262144, value_parser = clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Read historical declared graph using a citation's snapshot/path owner, not its content hash.
    /// No source verification; standard DB open may initialize/migrate.
    Deployment {
        id: String,
        task_spec: PathBuf,
        #[arg(long, default_value_t = 262144, value_parser = clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Invalidate a Compose owner via a recorded citation even if its source was deleted.
    /// Explicit CAS mutation, not source verification. Inspect generation after output failure.
    InvalidateCompose {
        id: String,
        task_spec: PathBuf,
        #[arg(long)]
        expected_generation: u64,
        #[arg(long, default_value_t = 262144, value_parser = clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Analyze a recorded full-file SQL citation using SQLite syntax, without executing SQL.
    /// No graph publication; standard DB open may initialize/migrate. Root binding is caller supplied.
    /// Parser spans are hints, not full statement ranges; byte caps are not I/O deadlines.
    AnalyzeSql {
        id: String,
        task_spec: PathBuf,
        root: PathBuf,
        #[arg(long, default_value_t = 131072, value_parser = clap::value_parser!(u32).range(1..=131072))]
        max_source_bytes: u32,
        #[arg(long, default_value_t = 65536, value_parser = clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Re-verify CodeGraph file-read candidates and persist a bounded historical code-to-SQL candidate graph.
    PublishSqlLinks {
        /// Registered full-file code citation selected within TASK_SPEC's snapshot.
        id: String,
        task_spec: PathBuf,
        /// JSON emitted by `codegraph file-reads`; it is re-verified, never trusted as evidence.
        report: PathBuf,
        root: PathBuf,
        #[arg(long)]
        analysis_run: String,
        #[arg(long)]
        expected_generation: u64,
        #[arg(long, default_value_t = 1048576, value_parser = clap::value_parser!(u32).range(1..=1048576))]
        max_source_bytes: u32,
        #[arg(long, default_value_t = 262144, value_parser = clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Read a historical SQL-link owner without re-verifying current source bytes.
    SqlLinks {
        id: String,
        task_spec: PathBuf,
        #[arg(long, default_value_t=262144, value_parser=clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Read one historical code-to-SQL candidate with citations and syntax observations.
    /// Does not read SQL bodies or assert runtime, semantic, table, or DB-instance facts.
    SqlLinkContext {
        id: String,
        task_spec: PathBuf,
        #[arg(long)]
        link: String,
        #[arg(long, default_value_t=32768, value_parser=clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Render one static code-to-SQL file-read candidate as neutral architecture IR.
    /// The dashed connection is not runtime traffic, a database instance, or table identity.
    SqlLinkDiagram {
        id: String,
        task_spec: PathBuf,
        #[arg(long)]
        link: String,
        #[arg(long, default_value_t=32768, value_parser=clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// CAS-invalidate a SQL-link owner selected by a historical code citation.
    InvalidateSqlLinks {
        id: String,
        task_spec: PathBuf,
        #[arg(long)]
        expected_generation: u64,
        #[arg(long, default_value_t=262144, value_parser=clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Analyze a recorded full-file Compose citation into source-backed candidates.
    /// No graph publication; standard DB open may initialize/migrate. Root binding is caller supplied.
    AnalyzeCompose {
        id: String,
        task_spec: PathBuf,
        /// Host-selected directory for the task snapshot; never launches Docker.
        root: PathBuf,
        #[arg(long, default_value_t = 1048576, value_parser = clap::value_parser!(u32).range(1..=1048576))]
        max_source_bytes: u32,
        /// Maximum serialized JSON bytes including newline; budget rejection emits no JSON.
        #[arg(long, default_value_t = 262144, value_parser = clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Verify surviving RPC output bytes; no repair, retry permission or process control.
    /// Standard DB open may initialize/migrate. Byte caps do not impose I/O deadlines.
    VerifyRpcOutputs {
        /// Exact caller-supplied launch JSON expectation, not authorization.
        launch_spec: PathBuf,
        /// Existing private CAS root bound to the execution snapshot.
        root: PathBuf,
        #[arg(long, default_value_t = 67108864, value_parser = clap::value_parser!(u64).range(0..=67108864))]
        max_total_bytes: u64,
        /// Maximum serialized JSON bytes including newline.
        #[arg(long, default_value_t = 65536, value_parser = clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Inspect a historical RPC launch; no ledger mutation by query; standard DB open may initialize/migrate.
    /// A missing launch does not prove an absent process or authorize retry.
    /// Spawn observations and PIDs are historical reports, not liveness or kill authority.
    RpcLaunch {
        id: String,
        /// Complete caller-supplied task JSON filter; not authorization.
        task_spec: PathBuf,
        /// Maximum serialized JSON bytes including newline (up to 16 MiB).
        #[arg(long, default_value_t = 65536, value_parser = clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Read a ledger task snapshot; does not grant a lease or prove agent liveness.
    Task {
        id: String,
        /// Filter by a task JSON's snapshot; caller supplied, not authorization.
        #[arg(long)]
        scope: Option<PathBuf>,
        /// Maximum serialized JSON bytes including newline (up to 16 MiB).
        #[arg(long, default_value_t = 65536, value_parser = clap::value_parser!(u32).range(0..=16777216))]
        max_output_bytes: u32,
    },
    /// Create or validate the durable task database.
    Init,
    /// Register immutable analysis metadata; does not execute or verify an analyzer.
    RecordAnalysisRun { run: PathBuf },
    /// Read registered analysis metadata within a task's snapshot; not execution proof.
    AnalysisRun { id: String, task_spec: PathBuf },
    /// Register artifact metadata for an existing run; does not ingest or verify bytes.
    RecordArtifact { artifact: PathBuf },
    /// Ingest piped stdin bytes using a descriptor; does not verify protection or execution.
    IngestArtifact {
        artifact: PathBuf,
        /// Existing private host-selected per-snapshot blob directory.
        root: PathBuf,
        #[arg(long, default_value_t = 64 * 1024 * 1024)]
        max_bytes: u64,
    },
    /// Read artifact metadata within a task's snapshot; does not verify bytes or protection.
    Artifact { id: String, task_spec: PathBuf },
    /// Read a historical observation; never verifies current blob integrity.
    ArtifactObservation { id: String, task_spec: PathBuf },
    /// Check stored blob bytes against metadata; does not verify protection or execution.
    VerifyArtifact {
        id: String,
        task_spec: PathBuf,
        /// Host-selected per-snapshot directory containing SHA-256-named blobs.
        root: PathBuf,
        #[arg(long, default_value_t = 64 * 1024 * 1024)]
        max_bytes: u64,
        /// Persist a successful check under a new immutable observation ID.
        #[arg(long)]
        observation_id: Option<String>,
    },
    /// Record an unverified source citation from versioned JSON.
    RecordEvidence { evidence: PathBuf },
    /// Read a source citation using the snapshot scope in a task JSON file.
    Evidence { id: String, task_spec: PathBuf },
    /// Check citation hash/lines against an explicitly bound root and return a bounded source slice.
    VerifyEvidence {
        id: String,
        task_spec: PathBuf,
        /// Trusted host-selected directory for the task snapshot.
        root: PathBuf,
        /// Optional strict host snapshot configuration. Without this file the
        /// legacy caller-supplied verification path is preserved.
        #[arg(long)]
        snapshot_config: Option<PathBuf>,
        #[arg(long, default_value_t = 8 * 1024 * 1024)]
        max_source_bytes: usize,
        #[arg(long, default_value_t = 32 * 1024)]
        max_slice_bytes: usize,
    },
    /// Enqueue a versioned task request from a JSON file.
    Enqueue { spec: PathBuf },
    /// Acquire a fenced lease for a ready task.
    Lease {
        task: String,
        owner: String,
        #[arg(long, default_value_t = 60_000)]
        duration_ms: i64,
    },
    /// Submit a candidate artifact under a previously issued lease.
    Submit { lease: PathBuf, artifact: String },
    /// Cancel a task in the ledger; this does not acknowledge process termination.
    Cancel {
        task: String,
        /// Audit identity only, not an authentication or capability grant.
        #[arg(long)]
        requested_by: String,
        #[arg(long)]
        reason: String,
    },
    /// Disabled legacy command: caller text is not independent verification.
    Integrate { task: String, verification: String },
    /// Stream durable task events as newline-delimited JSON.
    Events {
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut tasks = TaskService::new(Store::open(cli.database)?);
    let now_ms = current_time_ms()?;

    match cli.command {
        Command::DeploymentDiagram {
            id,
            task_spec,
            node,
            direction,
            depth,
            max_nodes,
            max_edges,
            max_output_bytes,
        } => {
            use graph_application::deployment_context::Direction;
            let direction = match direction.as_str() {
                "incoming" => Direction::Incoming,
                "outgoing" => Direction::Outgoing,
                _ => Direction::Both,
            };
            let bytes = deployment_diagram::query(
                &tasks.into_inner(),
                &deployment_context::ContextOptions {
                    id: &id,
                    task_spec: &task_spec,
                    node: &node,
                    direction,
                    depth,
                    max_nodes: max_nodes as usize,
                    max_edges: max_edges as usize,
                    max_output_bytes: max_output_bytes as usize,
                },
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::DeploymentContext {
            id,
            task_spec,
            node,
            direction,
            depth,
            max_nodes,
            max_edges,
            max_output_bytes,
        } => {
            use graph_application::deployment_context::Direction;
            let direction = match direction.as_str() {
                "incoming" => Direction::Incoming,
                "outgoing" => Direction::Outgoing,
                _ => Direction::Both,
            };
            let bytes = deployment_context::query(
                &tasks.into_inner(),
                &deployment_context::ContextOptions {
                    id: &id,
                    task_spec: &task_spec,
                    node: &node,
                    direction,
                    depth,
                    max_nodes: max_nodes as usize,
                    max_edges: max_edges as usize,
                    max_output_bytes: max_output_bytes as usize,
                },
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::ReindexCompose(args) => {
            let (bytes, valid) =
                compose_reindex::reindex(&mut tasks.into_inner(), &args.options())?;
            std::io::stdout().lock().write_all(&bytes)?;
            if !valid {
                return Err(std::io::Error::other(
                    "Compose source invalid; selected graph owner is invalidated",
                )
                .into());
            }
        }
        Command::WatchCompose {
            args,
            interval_ms,
            max_cycles,
        } => {
            compose_reindex::watch(
                &mut tasks.into_inner(),
                &args.options(),
                interval_ms,
                max_cycles,
                &mut std::io::stdout().lock(),
            )?;
        }
        Command::PublishCompose {
            id,
            task_spec,
            root,
            expected_generation,
            max_source_bytes,
            max_output_bytes,
        } => {
            let bytes = compose::publish(
                &mut tasks.into_inner(),
                &id,
                &task_spec,
                &root,
                expected_generation,
                max_source_bytes,
                max_output_bytes,
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::Deployment {
            id,
            task_spec,
            max_output_bytes,
        } => {
            let bytes = compose::query(&tasks.into_inner(), &id, &task_spec, max_output_bytes)?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::InvalidateCompose {
            id,
            task_spec,
            expected_generation,
            max_output_bytes,
        } => {
            let bytes = compose::invalidate(
                &mut tasks.into_inner(),
                &id,
                &task_spec,
                expected_generation,
                max_output_bytes,
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::AnalyzeSql {
            id,
            task_spec,
            root,
            max_source_bytes,
            max_output_bytes,
        } => {
            let bytes = sql::analyze(
                &tasks.into_inner(),
                &id,
                &task_spec,
                &root,
                max_source_bytes,
                max_output_bytes,
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::PublishSqlLinks {
            id,
            task_spec,
            report,
            root,
            analysis_run,
            expected_generation,
            max_source_bytes,
            max_output_bytes,
        } => {
            let bytes = sql_links::publish(
                &mut tasks.into_inner(),
                &id,
                &task_spec,
                &report,
                &root,
                &analysis_run,
                expected_generation,
                max_source_bytes,
                max_output_bytes,
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::SqlLinks {
            id,
            task_spec,
            max_output_bytes,
        } => {
            let bytes = sql_links::query(&tasks.into_inner(), &id, &task_spec, max_output_bytes)?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::SqlLinkContext {
            id,
            task_spec,
            link,
            max_output_bytes,
        } => {
            let bytes = sql_link_context::query(
                &tasks.into_inner(),
                &id,
                &task_spec,
                &link,
                max_output_bytes,
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::SqlLinkDiagram {
            id,
            task_spec,
            link,
            max_output_bytes,
        } => {
            let bytes = sql_link_diagram::query(
                &tasks.into_inner(),
                &id,
                &task_spec,
                &link,
                max_output_bytes,
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::InvalidateSqlLinks {
            id,
            task_spec,
            expected_generation,
            max_output_bytes,
        } => {
            let bytes = sql_links::invalidate(
                &mut tasks.into_inner(),
                &id,
                &task_spec,
                expected_generation,
                max_output_bytes,
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::AnalyzeCompose {
            id,
            task_spec,
            root,
            max_source_bytes,
            max_output_bytes,
        } => {
            let bytes = compose::analyze(
                &tasks.into_inner(),
                &id,
                &task_spec,
                &root,
                max_source_bytes,
                max_output_bytes,
            )?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::VerifyRpcOutputs {
            launch_spec,
            root,
            max_total_bytes,
            max_output_bytes,
        } => {
            let invalid = || std::io::Error::other("invalid RPC launch input");
            let raw = input::read_json_bytes(launch_spec).map_err(|_| invalid())?;
            let wire: graph_protocol::RpcLaunchSpec =
                serde_json::from_slice(&raw).map_err(|_| invalid())?;
            let expected = wire.try_into_domain().map_err(|_| invalid())?;
            let reader = graph_source::DirectoryArtifacts::open(
                root,
                expected.execution_snapshot().clone(),
                expected.task().graph_version().into(),
            )
            .map_err(|_| std::io::Error::other("RPC output root unavailable"))?;
            let store = tasks.into_inner();
            let observation =
                graph_application::verify_rpc_outputs(&store, &reader, &expected, max_total_bytes)?;
            let stream = |value: Option<&graph_application::VerifiedArtifactContent>| {
                value.map(|v| {
                    serde_json::json!({
                        "content_verified": true,
                        "byte_length": v.artifact().byte_length(),
                        "sha256": v.artifact().content_sha256(),
                    })
                })
            };
            let receipt = observation.receipt();
            let result = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "stdout": stream(observation.stdout()), "stderr": stream(observation.stderr()),
                "completion": graph_protocol::execution::ExecutionCompletion::from(receipt.completion()),
                "uncertain_request_count": receipt.pending().len(),
                "observation_only": true, "execution_authority_granted": false,
                "retry_authorized": false, "process_liveness": "unknown",
                "protection_verified": false, "expectation_source": "caller_supplied",
            });
            let bytes = output::json_line(&result, max_output_bytes as usize)?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::RpcLaunch {
            id,
            task_spec,
            max_output_bytes,
        } => {
            // Serde diagnostics can echo untrusted values or field names. Keep this
            // inspection command's parse/validation errors independent of payloads.
            let invalid_task = || {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "invalid rpc-launch task specification",
                )
            };
            let wire: TaskSpec = serde_json::from_slice(&input::read_json_bytes(task_spec)?)
                .map_err(|_| invalid_task())?;
            let task = wire.try_into_domain().map_err(|_| invalid_task())?;
            let store = tasks.into_inner();
            let snapshot = store.rpc_launch_snapshot(&id, &task)?;
            let result = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "launch": snapshot.as_ref().map(|snapshot| {
                    let spec = snapshot.spec();
                    serde_json::json!({
                        "id": spec.id(),
                        "host_id": spec.host_id(),
                        "connection_epoch": spec.connection().epoch(),
                        "claimed_at_ms": snapshot.claimed_at_ms(),
                        "claim_state": if snapshot.claimed_at_ms().is_some() {
                            "recorded"
                        } else {
                            "not_recorded"
                        },
                        "spawn_observation": snapshot.spawn_observation().map(|observation| serde_json::json!({
                            "observed_at_ms": observation.observed_at_ms(),
                            "disposition": observation.disposition().as_str(),
                            "process_id": observation.process_id(),
                        })),
                        "terminal_receipt": snapshot.terminal_receipt().map(|receipt| serde_json::json!({
                            "finished_at_ms": receipt.finished_at_ms(),
                            "supervised_elapsed_ms": receipt.supervised_elapsed_ms(),
                            "completion": graph_protocol::execution::ExecutionCompletion::from(receipt.completion()),
                            "uncertain_request_count": receipt.pending().len(),
                        })),
                    })
                }),
                "observation_only": true,
                "execution_authority_granted": false,
                "process_liveness": "unknown",
                "retry_authorized": false,
                "snapshot_filter": "caller_supplied",
            });
            let bytes = output::json_line(&result, max_output_bytes as usize)?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::Task {
            id,
            scope,
            max_output_bytes,
        } => {
            let scope = scope
                .map(|path| -> Result<_, Box<dyn std::error::Error>> {
                    let wire: TaskSpec = serde_json::from_slice(&input::read_json_bytes(path)?)?;
                    Ok(wire.try_into_domain()?)
                })
                .transpose()?;
            let snapshot = match &scope {
                Some(scope) => tasks.task_snapshot_in_scope(&id, scope)?,
                None => tasks.task_snapshot(&id)?,
            };
            let result = serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "task": snapshot.map(|s| serde_json::json!({
                    "spec": TaskSpec::from(&s.spec),
                    "state": s.state.as_db(),
                })),
                "observation_only": true,
                "execution_authority_granted": false,
                "agent_liveness": "unknown",
                "snapshot_filter": if scope.is_some() { "caller_supplied" } else { "none" },
            });
            let bytes = output::json_line(&result, max_output_bytes as usize)?;
            std::io::stdout().lock().write_all(&bytes)?;
        }
        Command::Init => println!("{{\"protocol_schema_version\":{SCHEMA_VERSION}}}"),
        Command::RecordAnalysisRun { run } => {
            let wire: graph_protocol::AnalysisRun =
                serde_json::from_slice(&input::read_json_bytes(run)?)?;
            let run = wire.try_into_domain()?;
            let inserted = tasks.into_inner().record_analysis_run(&run)?;
            println!(
                "{}",
                serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "id": run.id(),
                    "inserted": inserted,
                    "execution_verified": false,
                })
            );
        }
        Command::AnalysisRun { id, task_spec } => {
            let wire: TaskSpec = serde_json::from_slice(&input::read_json_bytes(task_spec)?)?;
            let task = wire.try_into_domain()?;
            let run = tasks
                .into_inner()
                .analysis_run(&id, task.project(), task.graph_version())?;
            println!(
                "{}",
                serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "run": run.as_ref().map(graph_protocol::AnalysisRun::from),
                    "execution_verified": false,
                })
            );
        }
        Command::RecordArtifact { artifact } => {
            let wire: graph_protocol::Artifact =
                serde_json::from_slice(&input::read_json_bytes(artifact)?)?;
            let artifact = wire.try_into_domain()?;
            let inserted = tasks.into_inner().record_artifact(&artifact)?;
            println!(
                "{}",
                serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "id": artifact.id(), "inserted": inserted,
                    "content_verified": false, "protection_verified": false,
                    "execution_verified": false,
                })
            );
        }
        Command::IngestArtifact {
            artifact,
            root,
            max_bytes,
        } => {
            let stdin = std::io::stdin();
            if stdin.is_terminal() {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput,
                    "ingest-artifact requires piped or redirected stdin, not an interactive terminal").into());
            }
            let wire: graph_protocol::Artifact =
                serde_json::from_slice(&input::read_json_bytes(artifact)?)?;
            let artifact = wire.try_into_domain()?;
            let writer = graph_source::DirectoryArtifacts::open(
                root,
                artifact.project().clone(),
                artifact.graph_version().into(),
            )?;
            let result = graph_application::ingest_artifact(
                &mut tasks.into_inner(),
                &writer,
                &artifact,
                &mut stdin.lock(),
                max_bytes,
            )?;
            println!(
                "{}",
                serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "artifact": graph_protocol::Artifact::from(result.content.artifact()),
                    "metadata_inserted": result.metadata_inserted,
                    "blob_inserted": result.blob_inserted,
                    "content_verified": true,
                    "protection_verified": false,
                    "execution_verified": false,
                    "snapshot_binding": "caller_supplied",
                    "content_is_untrusted": true,
                    "receipt_persisted": false,
                })
            );
        }
        Command::Artifact { id, task_spec } => {
            let wire: TaskSpec = serde_json::from_slice(&input::read_json_bytes(task_spec)?)?;
            let task = wire.try_into_domain()?;
            let artifact =
                tasks
                    .into_inner()
                    .artifact(&id, task.project(), task.graph_version())?;
            println!(
                "{}",
                serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "artifact": artifact.as_ref().map(graph_protocol::Artifact::from),
                    "content_verified": false, "protection_verified": false,
                    "execution_verified": false,
                })
            );
        }
        Command::ArtifactObservation { id, task_spec } => {
            let wire: TaskSpec = serde_json::from_slice(&input::read_json_bytes(task_spec)?)?;
            let task = wire.try_into_domain()?;
            let observation = tasks.into_inner().artifact_observation(
                &id,
                task.project(),
                task.graph_version(),
            )?;
            println!(
                "{}",
                serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "observation": observation.map(|o| serde_json::json!({
                        "id": o.id,
                        "artifact": graph_protocol::Artifact::from(&o.artifact),
                        "observed_at_ms": o.observed_at_ms,
                        "verifier_version": o.verifier_version,
                    })),
                    "historical_only": true,
                    "content_verified": false,
                    "protection_verified": false,
                    "execution_verified": false,
                    "snapshot_binding": "caller_supplied",
                    "content_is_untrusted": true,
                })
            );
        }
        Command::VerifyArtifact {
            id,
            task_spec,
            root,
            max_bytes,
            observation_id,
        } => {
            if observation_id
                .as_ref()
                .is_some_and(|id| id.trim().is_empty())
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "observation id must not be blank",
                )
                .into());
            }
            let wire: TaskSpec = serde_json::from_slice(&input::read_json_bytes(task_spec)?)?;
            let task = wire.try_into_domain()?;
            let mut store = tasks.into_inner();
            let artifact = store
                .artifact(&id, task.project(), task.graph_version())?
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "artifact not found for this snapshot",
                    )
                })?;
            let reader = graph_source::DirectoryArtifacts::open(
                root,
                task.project().clone(),
                task.graph_version().into(),
            )?;
            let checked = graph_application::verify_artifact(&reader, &artifact, max_bytes)?;
            if let Some(id) = &observation_id {
                store.record_artifact_observation(id, &checked, current_time_ms()?)?;
            }
            println!(
                "{}",
                serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "artifact": graph_protocol::Artifact::from(checked.artifact()),
                    "content_verified": true,
                    "protection_verified": false,
                    "execution_verified": false,
                    "snapshot_binding": "caller_supplied",
                    "content_is_untrusted": true,
                    "receipt_persisted": observation_id.is_some(),
                })
            );
        }
        Command::RecordEvidence { evidence } => {
            let wire: graph_protocol::SourceEvidence =
                serde_json::from_slice(&input::read_json_bytes(evidence)?)?;
            let evidence = wire.try_into_domain()?;
            let inserted = tasks.into_inner().record_source(&evidence)?;
            println!(
                "{}",
                serde_json::json!({"inserted": inserted, "verified": false})
            );
        }
        Command::Evidence { id, task_spec } => {
            let wire: TaskSpec = serde_json::from_slice(&input::read_json_bytes(task_spec)?)?;
            let task = wire.try_into_domain()?;
            let evidence =
                tasks
                    .into_inner()
                    .source_evidence(&id, task.project(), task.graph_version())?;
            println!(
                "{}",
                serde_json::to_string(
                    &evidence.as_ref().map(graph_protocol::SourceEvidence::from)
                )?
            );
        }
        Command::VerifyEvidence {
            id,
            task_spec,
            root,
            snapshot_config,
            max_source_bytes,
            max_slice_bytes,
        } => {
            let limits = graph_application::SourceLimits::new(max_source_bytes, max_slice_bytes)?;
            let wire: TaskSpec = serde_json::from_slice(&input::read_json_bytes(task_spec)?)?;
            let task = wire.try_into_domain()?;
            let store = tasks.into_inner();
            let evidence = store
                .source_evidence(&id, task.project(), task.graph_version())?
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "evidence not found for this snapshot",
                    )
                })?;
            if let Some(config_path) = snapshot_config {
                // Materialization itself reads source bytes. Admit the exact
                // registered run before any source-root access, then retain
                // the bound verifier's independent check after acquisition.
                let run =
                    graph_application::AnalysisRepository::source_analysis_run(&store, &evidence)?
                        .ok_or_else(|| {
                            std::io::Error::new(
                                std::io::ErrorKind::NotFound,
                                "analysis run unavailable before source materialization",
                            )
                        })?;
                if run.id() != evidence.analysis_run()
                    || run.project() != evidence.project()
                    || run.graph_version() != evidence.graph_version()
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "analysis run mismatch before source materialization",
                    )
                    .into());
                }
                let config = snapshot_config::load(&config_path)?;
                let configured_root = config.root.canonicalize()?;
                let requested_root = root.canonicalize()?;
                if configured_root != requested_root {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "snapshot config root does not match command root",
                    )
                    .into());
                }
                let authority = graph_source::GitSnapshotAuthority::with_command_timeout(
                    configured_root,
                    config.repository_id,
                    config.worktree_id,
                    config.config_hash,
                    config.ignore_policy_version,
                    config.max_total_bytes,
                    config.command_timeout,
                )?;
                let materialized = authority.materialize_source_snapshot(
                    &requested_root,
                    task.project(),
                    task.graph_version(),
                )?;
                let checked = graph_application::verify_bound_source_identity(
                    materialized.snapshot(),
                    &store,
                    materialized.source(),
                    &evidence,
                    limits,
                )?;
                println!(
                    "{}",
                    serde_json::json!({
                        "schema_version": SCHEMA_VERSION,
                        "evidence": graph_protocol::SourceEvidence::from(checked.source().evidence()),
                        "source": checked.source().text(),
                        "source_is_untrusted": true,
                        "content_hash_and_lines_verified": true,
                        "snapshot_binding": "git_authority",
                        "snapshot_binding_id": checked.snapshot().binding_id(),
                        "analysis_run_verified": true,
                        "analysis_run": checked.analysis_run().id(),
                        "relationship_verified": false,
                    })
                );
            } else {
                let reader = graph_source::DirectorySource::open(
                    root,
                    task.project().clone(),
                    task.graph_version().into(),
                )?;
                let checked = graph_application::verify_source(&reader, &evidence, limits)?;
                println!(
                    "{}",
                    serde_json::json!({
                        "schema_version": SCHEMA_VERSION,
                        "evidence": graph_protocol::SourceEvidence::from(checked.evidence()),
                        "source": checked.text(),
                        "source_is_untrusted": true,
                        "content_hash_and_lines_verified": true,
                        "snapshot_binding": "caller_supplied",
                        "analysis_run_verified": false,
                        "relationship_verified": false,
                    })
                );
            }
        }
        Command::Enqueue { spec } => {
            let wire: TaskSpec = serde_json::from_slice(&input::read_json_bytes(spec)?)?;
            println!(
                "{}",
                serde_json::json!({"inserted": tasks.enqueue(wire.try_into_domain()?, now_ms)?})
            );
        }
        Command::Lease {
            task,
            owner,
            duration_ms,
        } => {
            let lease = tasks.lease(&task, &owner, now_ms, duration_ms)?;
            println!("{}", serde_json::to_string(&Lease::from(&lease))?);
        }
        Command::Submit { lease, artifact } => {
            let wire: Lease = serde_json::from_slice(&input::read_json_bytes(lease)?)?;
            tasks.submit(wire.try_into_domain()?, &artifact, now_ms)?;
            println!("{}", serde_json::json!({"state": "submitted"}));
        }
        Command::Cancel {
            task,
            requested_by,
            reason,
        } => {
            let changed = tasks.cancel(&task, &requested_by, &reason, now_ms)?;
            println!(
                "{}",
                serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "task_id": task,
                    "state": "cancelled",
                    "changed": changed,
                    "process_termination_confirmed": false,
                })
            );
        }
        Command::Integrate { task, verification } => {
            tasks.integrate(&task, &verification, now_ms)?;
            println!("{}", serde_json::json!({"state": "integrated"}));
        }
        Command::Events { after, limit } => {
            for event in tasks.events(after, limit)? {
                println!("{}", serde_json::to_string(&Event::from(event))?);
            }
        }
    }
    Ok(())
}

fn current_time_ms() -> Result<i64, Box<dyn std::error::Error>> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(i64::try_from(elapsed.as_millis())?)
}

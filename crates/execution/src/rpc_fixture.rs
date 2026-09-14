use crate::{
    AttachmentFailure, ConnectionInputError, ConnectionSetup, ConnectionSupervisor, FixtureError,
    OutputLimits, SupervisionLimits,
};
use graph_application::{
    RpcLaunchRepository, RpcSpawnObservationRepository, environment_fingerprint,
};
use graph_domain::{CheckCommand, RpcLaunchSpec, RpcSpawnDisposition, RpcSpawnObservation};
use std::os::unix::process::CommandExt;
use std::{
    collections::BTreeMap,
    io,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// Embedding-owned allowlist for ONE immutable trusted fixture launch.
/// Never build this from a requested wire descriptor. Not a production approval
/// resolver, sandbox, account capability or verified project snapshot.
pub struct TrustedRpcFixtureHost {
    allowed: RpcLaunchSpec,
    executable: PathBuf,
    root: PathBuf,
    environment: BTreeMap<String, String>,
    sandbox: Option<SandboxLaunch>,
}

struct SandboxLaunch {
    runtime: crate::SandboxRuntime,
    egress: crate::SandboxEgressPolicy,
}

/// Binding created only by the trusted fixture host, not from stored PID claims.
#[must_use = "poll and finish, or transfer the owned supervisor for cleanup"]
pub struct LaunchedRpc {
    spawn: RpcSpawnObservation,
    supervisor: ConnectionSupervisor,
}
impl LaunchedRpc {
    pub fn spawn_observation(&self) -> &RpcSpawnObservation {
        &self.spawn
    }
    pub fn core(&self) -> &graph_protocol::connection::Connection {
        self.supervisor.core()
    }
    pub fn input_progress(&self) -> Option<crate::InputProgress> {
        self.supervisor.input_progress()
    }
    pub fn poll(&mut self, cancelled: bool, accept_events: bool) -> crate::SupervisorTick {
        self.supervisor.poll(cancelled, accept_events)
    }
    pub fn prepare_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<graph_protocol::rpc::RequestId, crate::ConnectionIoError> {
        self.supervisor.prepare_request(method, params)
    }
    /// Explicitly discard provenance binding, retaining process ownership.
    pub fn into_supervisor(self) -> ConnectionSupervisor {
        self.supervisor
    }
    pub fn finish(self) -> Result<BoundRpcTerminal, Self> {
        match self.supervisor.finish() {
            Ok(result) => Ok(BoundRpcTerminal {
                spawn: self.spawn,
                terminal: result.into_terminal(),
            }),
            Err(supervisor) => Err(Self {
                spawn: self.spawn,
                supervisor,
            }),
        }
    }
}

/// Exact host-created spawn and terminal result from the same owned supervisor.
/// Not a production approval/containment capability or verified output receipt.
#[must_use = "retain and reconcile any unreaped child"]
pub struct BoundRpcTerminal {
    spawn: RpcSpawnObservation,
    terminal: crate::TerminalConnection,
}
impl BoundRpcTerminal {
    pub fn spawn_observation(&self) -> &RpcSpawnObservation {
        &self.spawn
    }
    pub fn terminal(&self) -> &crate::TerminalConnection {
        &self.terminal
    }
    /// Transfers raw evidence and any unreaped Child; caller still owns cleanup.
    pub fn into_parts(self) -> (RpcSpawnObservation, crate::TerminalConnection) {
        (self.spawn, self.terminal)
    }
}

#[must_use = "launch errors may retain a child; inspect, supervise and reap it"]
pub enum RpcFixtureLaunchError<E> {
    Rejected,
    Preflight(FixtureError),
    Setup(ConnectionInputError),
    Cancelled {
        claimed: bool,
    },
    Expired {
        claimed: bool,
    },
    Clock {
        claimed: bool,
        source: io::Error,
    },
    Ledger {
        claiming: bool,
        source: E,
    },
    AlreadyClaimed,
    Spawn(io::Error),
    /// Still owns the spawned child. Caller must retain and reap it.
    Attachment(AttachmentFailure),
    /// Recording may have committed. Keep the exact child and reap it; do not retry spawn.
    Observation {
        failure: RpcSpawnRecordFailure<E>,
        child: Option<Child>,
        spawn_error: Option<io::Error>,
    },
}

pub enum RpcSpawnRecordError<E> {
    Clock(io::Error),
    Domain(graph_domain::DomainError),
    Ledger(E),
}
pub struct RpcSpawnRecordFailure<E> {
    pub disposition: RpcSpawnDisposition,
    /// Retain this exact report for reconciliation; do not regenerate its timestamp
    /// or rerun the launch when a repository reports an uncertain commit.
    pub observation: Option<RpcSpawnObservation>,
    pub error: RpcSpawnRecordError<E>,
}

fn record_spawn<R: RpcSpawnObservationRepository>(
    repository: &mut R,
    launch: &RpcLaunchSpec,
    disposition: RpcSpawnDisposition,
    process_id: Option<u32>,
) -> Result<RpcSpawnObservation, RpcSpawnRecordFailure<R::Error>> {
    let at = now_ms().map_err(|error| RpcSpawnRecordFailure {
        disposition,
        observation: None,
        error: RpcSpawnRecordError::Clock(error),
    })?;
    let observation = RpcSpawnObservation::new(launch.clone(), at, disposition, process_id)
        .map_err(|error| RpcSpawnRecordFailure {
            disposition,
            observation: None,
            error: RpcSpawnRecordError::Domain(error),
        })?;
    match repository.record_rpc_spawn_observation(&observation) {
        Ok(_) => Ok(observation),
        Err(error) => Err(RpcSpawnRecordFailure {
            disposition,
            observation: Some(observation),
            error: RpcSpawnRecordError::Ledger(error),
        }),
    }
}
impl<E> std::fmt::Debug for RpcFixtureLaunchError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not leak argv/env/paths through error formatting; typed causes remain accessible.
        f.write_str(match self {
            Self::Rejected => "Rejected",
            Self::Preflight(_) => "Preflight",
            Self::Setup(_) => "Setup",
            Self::Cancelled { .. } => "Cancelled",
            Self::Expired { .. } => "Expired",
            Self::Clock { .. } => "Clock",
            Self::Ledger { .. } => "Ledger",
            Self::AlreadyClaimed => "AlreadyClaimed",
            Self::Spawn(_) => "Spawn",
            Self::Attachment(_) => "Attachment",
            Self::Observation { .. } => "Observation",
        })
    }
}

fn now_ms() -> io::Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    i64::try_from(duration.as_millis()).map_err(io::Error::other)
}

impl TrustedRpcFixtureHost {
    pub fn new(
        allowed: RpcLaunchSpec,
        executable: PathBuf,
        root: PathBuf,
        environment: BTreeMap<String, String>,
    ) -> Self {
        Self {
            allowed,
            executable,
            root,
            environment,
            sandbox: None,
        }
    }

    /// Build a trusted fixture host whose child is launched through a probed
    /// sandbox backend. The direct `new` constructor remains an explicit
    /// trusted-host fallback; this mode is opt-in and fail-closed.
    pub fn new_sandboxed(
        allowed: RpcLaunchSpec,
        executable: PathBuf,
        root: PathBuf,
        environment: BTreeMap<String, String>,
        runtime: crate::SandboxRuntime,
        egress: crate::SandboxEgressPolicy,
    ) -> Self {
        Self {
            allowed,
            executable,
            root,
            environment,
            sandbox: Some(SandboxLaunch { runtime, egress }),
        }
    }

    /// Add an immutable sandbox plan to an existing trusted fixture host.
    #[must_use]
    pub fn with_sandbox(
        mut self,
        runtime: crate::SandboxRuntime,
        egress: crate::SandboxEgressPolicy,
    ) -> Self {
        self.sandbox = Some(SandboxLaunch { runtime, egress });
        self
    }

    fn sandbox_plan(
        &self,
        process: &graph_domain::RpcProcessSpec,
    ) -> Result<Option<crate::SandboxPlan>, FixtureError> {
        let Some(sandbox) = &self.sandbox else {
            return Ok(None);
        };
        // RpcProcessSpec and CheckCommand intentionally share descriptor
        // validation, but their stdin semantics differ. This temporary command
        // is only the sandbox path/cwd/argv input; RPC stdin is attached later
        // from the exact Child by ConnectionSetup.
        let command = CheckCommand::new(
            process.program().into(),
            process.args().to_vec(),
            process.cwd().into(),
            process.executable_sha256().into(),
            process.environment_sha256().into(),
            process.timeout_ms(),
            process.cleanup_timeout_ms(),
            process.stdout_max_bytes(),
            process.stderr_max_bytes(),
        )
        .map_err(|_| FixtureError::Preflight)?;
        sandbox
            .runtime
            .plan(
                &command,
                &self.executable,
                &self.root,
                &self.environment,
                &sandbox.egress,
            )
            .map(Some)
            .map_err(FixtureError::Sandbox)
    }

    /// Trusted owned fixtures only. No implicit retry or claim reset on any error.
    /// Ledger errors may have committed; reconcile instead of assuming no claim.
    pub fn launch<R>(
        &self,
        requested: &RpcLaunchSpec,
        repository: &mut R,
        cancelled: &AtomicBool,
    ) -> Result<ConnectionSupervisor, RpcFixtureLaunchError<<R as RpcLaunchRepository>::Error>>
    where
        R: RpcLaunchRepository
            + RpcSpawnObservationRepository<Error = <R as RpcLaunchRepository>::Error>,
    {
        self.launch_bound(requested, repository, cancelled)
            .map(LaunchedRpc::into_supervisor)
    }

    /// Preserve the exact successful spawn observation through supervision.
    pub fn launch_bound<R>(
        &self,
        requested: &RpcLaunchSpec,
        repository: &mut R,
        cancelled: &AtomicBool,
    ) -> Result<LaunchedRpc, RpcFixtureLaunchError<<R as RpcLaunchRepository>::Error>>
    where
        R: RpcLaunchRepository
            + RpcSpawnObservationRepository<Error = <R as RpcLaunchRepository>::Error>,
    {
        use RpcFixtureLaunchError as Failure;
        if requested != &self.allowed {
            return Err(Failure::Rejected);
        }
        let check_control = |claimed| {
            if cancelled.load(Ordering::Acquire) {
                return Err(Failure::Cancelled { claimed });
            }
            let now = now_ms().map_err(|source| Failure::Clock { claimed, source })?;
            if now >= requested.origin_lease().expires_at_ms() {
                return Err(Failure::Expired { claimed });
            }
            Ok(now)
        };
        let _ = check_control(false)?;
        let process = requested.process();
        let connection = requested.connection();
        let output = OutputLimits::new(process.stdout_max_bytes(), process.stderr_max_bytes())
            .map_err(|error| Failure::Setup(error.into()))?;
        let limits = SupervisionLimits::new(process.timeout_ms(), process.cleanup_timeout_ms())
            .map_err(|error| Failure::Setup(error.into()))?;
        let setup = ConnectionSetup::new(
            connection.epoch(),
            connection.max_pending() as usize,
            connection.max_frame() as usize,
            connection.client_name(),
            connection.client_version(),
            connection.experimental(),
            output,
            limits,
        )
        .map_err(Failure::Setup)?;
        let preflight = || -> Result<PathBuf, FixtureError> {
            if !self.executable.is_absolute()
                || self.executable.to_str() != Some(process.program())
                || environment_fingerprint(&self.environment)
                    .map_err(|_| FixtureError::Preflight)?
                    != process.environment_sha256()
                || crate::linux::digest_executable(&self.executable)? != process.executable_sha256()
            {
                return Err(FixtureError::Preflight);
            }
            let root = self.root.canonicalize()?;
            let cwd = root.join(process.cwd()).canonicalize()?;
            if !root.is_dir() || !cwd.is_dir() || !cwd.starts_with(&root) {
                return Err(FixtureError::Preflight);
            }
            Ok(cwd)
        };
        let cwd = preflight().map_err(Failure::Preflight)?;
        let sandbox_plan = self.sandbox_plan(process).map_err(Failure::Preflight)?;
        let now = check_control(false)?;
        repository
            .register_rpc_launch(requested, now)
            .map_err(|source| Failure::Ledger {
                claiming: false,
                source,
            })?;
        let now = check_control(false)?;
        if !repository
            .claim_rpc_launch(requested, now)
            .map_err(|source| Failure::Ledger {
                claiming: true,
                source,
            })?
        {
            return Err(Failure::AlreadyClaimed);
        }
        if let Err(error) = check_control(true) {
            let disposition = match &error {
                Failure::Cancelled { .. } => RpcSpawnDisposition::CancelledBeforeSpawn,
                Failure::Expired { .. } => RpcSpawnDisposition::ExpiredBeforeSpawn,
                _ => return Err(error),
            };
            if let Err(failure) = record_spawn(repository, requested, disposition, None) {
                return Err(Failure::Observation {
                    failure,
                    child: None,
                    spawn_error: None,
                });
            }
            return Err(error);
        }
        let mut command = if let Some(plan) = &sandbox_plan {
            let mut command = Command::new(plan.backend());
            command.args(plan.args());
            command
        } else {
            let mut command = Command::new(&self.executable);
            command.args(process.args());
            command
        };
        command
            .current_dir(cwd)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        if sandbox_plan.is_none() {
            command.envs(&self.environment);
        }
        let child = command.spawn();
        match child {
            Ok(child) => {
                match record_spawn(
                    repository,
                    requested,
                    RpcSpawnDisposition::Spawned,
                    Some(child.id()),
                ) {
                    Ok(spawn) => setup
                        .attach(child)
                        .map(|supervisor| LaunchedRpc { spawn, supervisor })
                        .map_err(Failure::Attachment),
                    Err(failure) => Err(Failure::Observation {
                        failure,
                        child: Some(child),
                        spawn_error: None,
                    }),
                }
            }
            Err(error) => {
                match record_spawn(
                    repository,
                    requested,
                    RpcSpawnDisposition::SpawnFailed,
                    None,
                ) {
                    Ok(_) => Err(Failure::Spawn(error)),
                    Err(failure) => Err(Failure::Observation {
                        failure,
                        child: None,
                        spawn_error: Some(error),
                    }),
                }
            }
        }
    }
}

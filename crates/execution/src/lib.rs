//! Initial trusted-fixture adapter, not an approved general-purpose execution host.
use graph_domain::{CheckCommand, execution::ExecutionCompletion};
use std::{collections::BTreeMap, io, path::Path, process::Child, sync::atomic::AtomicBool};

#[cfg(target_os = "linux")]
mod input;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use input::{InputProgress, NonblockingInput};
#[cfg(target_os = "linux")]
mod connection_input;
#[cfg(target_os = "linux")]
pub use connection_input::{ConnectionInput, ConnectionInputError, InputEvent};
#[cfg(target_os = "linux")]
mod attachment;
#[cfg(target_os = "linux")]
pub use attachment::{AttachmentError, AttachmentFailure, ConnectionSetup};
#[cfg(target_os = "linux")]
mod connection_io;
#[cfg(target_os = "linux")]
pub use connection_io::{ConnectionIo, ConnectionIoError, IoEvent, OutputLimits, OutputSnapshot};
#[cfg(target_os = "linux")]
mod supervisor;
#[cfg(target_os = "linux")]
pub use supervisor::{
    ConnectionSupervisor, SupervisedConnection, SupervisionLimits, SupervisorTick,
    TerminalConnection,
};
mod output;
#[cfg(target_os = "linux")]
mod process_scope;
#[cfg(target_os = "linux")]
mod rpc_fixture;
#[cfg(target_os = "linux")]
mod rpc_output;
mod sandbox;
pub use output::{OutputPublication, OutputPublicationError, publish_fixture_outputs};
#[cfg(target_os = "linux")]
pub use rpc_fixture::{
    BoundRpcTerminal, LaunchedRpc, RpcFixtureLaunchError, RpcSpawnRecordError,
    RpcSpawnRecordFailure, TrustedRpcFixtureHost,
};
#[cfg(target_os = "linux")]
pub use rpc_output::{
    PreparedRpcReceipt, RpcPublicationError, RpcReceiptPublication, reopen_rpc_journal,
    replay_rpc_journal,
};
pub use sandbox::{
    SandboxCapabilities, SandboxEgressDisposition, SandboxEgressPolicy, SandboxError, SandboxPlan,
    SandboxRuntime,
};

pub struct FixtureRun {
    pub completion: ExecutionCompletion,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub elapsed_ms: u64,
    /// Caller owns any unresolved direct child and must supervise/reap it.
    /// Never detach or infer that a cleanup deadline means it has terminated.
    pub unreaped_child: Option<Child>,
}
impl std::fmt::Debug for FixtureRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FixtureRun")
            .field("completion", &self.completion)
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .field("elapsed_ms", &self.elapsed_ms)
            .field("has_unreaped_child", &self.unreaped_child.is_some())
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    #[error("trusted fixture preflight rejected command, environment, path or limits")]
    Preflight,
    #[error("fixture preflight I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("trusted fixture executor currently supports Linux only")]
    Unsupported,
    #[error("sandbox admission failed")]
    Sandbox(#[from] SandboxError),
}

/// Host-owned immutable fixtures only. No shell/PATH lookup, approval, sandbox,
/// account access, plan/claim consumption or verified ExecutionReceipt creation.
/// Preflight hashes are observations, not race-proof executable identity.
pub fn run_trusted_fixture(
    command: &CheckCommand,
    executable: &Path,
    root: &Path,
    environment: &BTreeMap<String, String>,
    cancelled: &AtomicBool,
) -> Result<FixtureRun, FixtureError> {
    #[cfg(target_os = "linux")]
    {
        linux::run(command, executable, root, environment, cancelled)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (command, executable, root, environment, cancelled);
        Err(FixtureError::Unsupported)
    }
}

/// Execute one trusted fixture through a probed sandbox backend. This remains
/// a raw host result: no lease/approval, graph write, authenticated receipt or
/// complete containment proof is created here.
pub fn run_trusted_fixture_in_sandbox(
    command: &CheckCommand,
    executable: &Path,
    root: &Path,
    environment: &BTreeMap<String, String>,
    egress: &SandboxEgressPolicy,
    runtime: &SandboxRuntime,
    cancelled: &AtomicBool,
) -> Result<FixtureRun, FixtureError> {
    #[cfg(target_os = "linux")]
    {
        linux::run_sandboxed(
            command,
            executable,
            root,
            environment,
            egress,
            runtime,
            cancelled,
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (
            command,
            executable,
            root,
            environment,
            egress,
            runtime,
            cancelled,
        );
        Err(FixtureError::Unsupported)
    }
}

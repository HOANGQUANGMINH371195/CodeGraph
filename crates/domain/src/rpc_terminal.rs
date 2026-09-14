use crate::execution::{
    ChildCompletion, ExecutionCompletion, ScopeCleanup, StopReason, StreamCompletion,
};
use crate::{AnalysisRun, Artifact, DomainError, RpcSpawnDisposition, RpcSpawnObservation};

/// Unresolved terminal correlation, never proof of dispatch or permission to retry.
#[derive(Clone, PartialEq, Eq)]
pub struct UncertainRpc {
    id: i64,
    method: String,
}
impl std::fmt::Debug for UncertainRpc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UncertainRpc")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
impl UncertainRpc {
    /// Creates an unresolved RPC correlation marker.
    ///
    /// # Errors
    /// Returns an error when the identifier is nonpositive, the method is blank,
    /// or the method exceeds the bounded metadata length.
    pub fn new(id: i64, method: String) -> Result<Self, DomainError> {
        // Match methods accepted by protocol correlation; do not lose metadata
        // merely because an accepted method includes a control character.
        if id <= 0 || method.trim().is_empty() || method.len() > 256 {
            return Err(DomainError::Invalid("invalid unresolved RPC identity"));
        }
        Ok(Self { id, method })
    }
    #[must_use]
    pub fn id(&self) -> i64 {
        self.id
    }
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }
}

/// Last frame (possibly a notification), not an identified request or peer ACK.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RpcInputProgress {
    total: u64,
    written: u64,
    failed: bool,
}
impl RpcInputProgress {
    /// Records bounded input byte counters.
    ///
    /// # Errors
    /// Returns an error when written bytes exceed total or total exceeds the
    /// signed ledger range.
    pub fn new(total: u64, written: u64, failed: bool) -> Result<Self, DomainError> {
        if written > total || total > i64::MAX as u64 {
            return Err(DomainError::Invalid("invalid RPC input counters"));
        }
        Ok(Self {
            total,
            written,
            failed,
        })
    }
    #[must_use]
    pub fn total(&self) -> u64 {
        self.total
    }
    #[must_use]
    pub fn written(&self) -> u64 {
        self.written
    }
    #[must_use]
    pub fn failed(&self) -> bool {
        self.failed
    }
}

/// Structurally validated historical claims. No authenticated host authority,
/// verified artifact bytes, task success, liveness or retry capability.
#[derive(Clone, PartialEq, Eq)]
pub struct RpcTerminalReceipt {
    spawn: RpcSpawnObservation,
    output_run: AnalysisRun,
    finished_at_ms: i64,
    supervised_elapsed_ms: u64,
    stdout: Option<Artifact>,
    stderr: Option<Artifact>,
    completion: ExecutionCompletion,
    pending: Vec<UncertainRpc>,
    input_progress: Option<RpcInputProgress>,
}
impl std::fmt::Debug for RpcTerminalReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RpcTerminalReceipt { .. }")
    }
}
impl RpcTerminalReceipt {
    /// Constructs a structurally validated terminal receipt.
    ///
    /// # Errors
    /// Returns an error when spawn, output-run, artifact, completion, timing, or
    /// pending-correlation fields violate their cross-field invariants.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spawn: RpcSpawnObservation,
        output_run: AnalysisRun,
        finished_at_ms: i64,
        supervised_elapsed_ms: u64,
        stdout: Option<Artifact>,
        stderr: Option<Artifact>,
        completion: ExecutionCompletion,
        pending: Vec<UncertainRpc>,
        input_progress: Option<RpcInputProgress>,
    ) -> Result<Self, DomainError> {
        let launch = spawn.launch();
        if spawn.disposition() != RpcSpawnDisposition::Spawned
            || completion.child == ChildCompletion::NotSpawned
            || completion.reason == StopReason::SpawnFailed
            || (completion.reason == StopReason::Exited
                && !matches!(completion.child, ChildCompletion::Reaped { .. }))
            || (completion.child == ChildCompletion::Unreaped
                && completion.cleanup == ScopeCleanup::Complete)
        {
            return Err(DomainError::Invalid(
                "inconsistent terminal RPC process state",
            ));
        }
        // Wall time may move backwards. Elapsed starts at supervisor attachment.
        if finished_at_ms < 0 || supervised_elapsed_ms > i64::MAX as u64 {
            return Err(DomainError::Invalid(
                "RPC terminal timing exceeds ledger range",
            ));
        }
        if output_run.project() != launch.execution_snapshot()
            || output_run.graph_version() != launch.task().graph_version()
        {
            return Err(DomainError::Invalid("RPC output run does not match launch"));
        }
        for (artifact, state, kind, cap) in [
            (
                &stdout,
                completion.stdout,
                "stdout",
                launch.process().stdout_max_bytes(),
            ),
            (
                &stderr,
                completion.stderr,
                "stderr",
                launch.process().stderr_max_bytes(),
            ),
        ] {
            if state == StreamCompletion::Complete && artifact.is_none() {
                return Err(DomainError::Invalid(
                    "complete RPC stream requires artifact",
                ));
            }
            if let Some(output) = artifact {
                if output.project() != launch.execution_snapshot()
                    || output.graph_version() != launch.task().graph_version()
                    || output.analysis_run() != output_run.id()
                    || output.kind() != kind
                    || output.byte_length() > cap
                {
                    return Err(DomainError::Invalid(
                        "RPC output artifact does not match launch",
                    ));
                }
            }
        }
        if stdout
            .as_ref()
            .zip(stderr.as_ref())
            .is_some_and(|(out, err)| out.id() == err.id())
        {
            return Err(DomainError::Invalid(
                "RPC stream artifact identities must differ",
            ));
        }
        if pending.len() > launch.connection().max_pending() as usize
            || pending.windows(2).any(|pair| pair[0].id() >= pair[1].id())
        {
            return Err(DomainError::Invalid(
                "RPC pending entries exceed cap or are not strictly ordered",
            ));
        }
        if input_progress
            .is_some_and(|progress| progress.total() > u64::from(launch.connection().max_frame()))
        {
            return Err(DomainError::Invalid("RPC input counters exceed frame cap"));
        }
        Ok(Self {
            spawn,
            output_run,
            finished_at_ms,
            supervised_elapsed_ms,
            stdout,
            stderr,
            completion,
            pending,
            input_progress,
        })
    }
    pub fn spawn(&self) -> &RpcSpawnObservation {
        &self.spawn
    }
    pub fn output_run(&self) -> &AnalysisRun {
        &self.output_run
    }
    pub fn finished_at_ms(&self) -> i64 {
        self.finished_at_ms
    }
    pub fn supervised_elapsed_ms(&self) -> u64 {
        self.supervised_elapsed_ms
    }
    pub fn stdout(&self) -> Option<&Artifact> {
        self.stdout.as_ref()
    }
    pub fn stderr(&self) -> Option<&Artifact> {
        self.stderr.as_ref()
    }
    pub fn completion(&self) -> &ExecutionCompletion {
        &self.completion
    }
    pub fn pending(&self) -> &[UncertainRpc] {
        &self.pending
    }
    pub fn input_progress(&self) -> Option<RpcInputProgress> {
        self.input_progress
    }
}

#[cfg(test)]
#[path = "rpc_terminal_tests.rs"]
mod tests;

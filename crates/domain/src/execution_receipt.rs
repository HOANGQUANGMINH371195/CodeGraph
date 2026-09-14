use crate::execution::{ExecutionCompletion, StreamCompletion};
use crate::{Artifact, CheckRunBinding, DomainError, ProjectRef};

/// Structurally checked producer claims, never authenticated execution authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionReceipt {
    binding: CheckRunBinding,
    host_id: String,
    execution_snapshot: ProjectRef,
    started_at_ms: i64,
    finished_at_ms: i64,
    elapsed_ms: u64,
    stdout: Option<Artifact>,
    stderr: Option<Artifact>,
    completion: ExecutionCompletion,
}
impl ExecutionReceipt {
    /// Constructs a structurally checked execution receipt.
    ///
    /// # Errors
    /// Returns an error when the target binding, timing, stream completion,
    /// artifact linkage, byte budgets, or stream identities are inconsistent.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        binding: CheckRunBinding,
        host_id: String,
        execution_snapshot: ProjectRef,
        started_at_ms: i64,
        finished_at_ms: i64,
        elapsed_ms: u64,
        stdout: Option<Artifact>,
        stderr: Option<Artifact>,
        completion: ExecutionCompletion,
    ) -> Result<Self, DomainError> {
        crate::execution_plan::validate_target(&binding, &host_id, &execution_snapshot)?;
        // Wall time can move backwards; monotonic elapsed is a separate claim.
        if started_at_ms < 0 || finished_at_ms < 0 || elapsed_ms > i64::MAX as u64 {
            return Err(DomainError::Invalid(
                "execution timing exceeds ledger range",
            ));
        }
        for (artifact, state, kind, budget) in [
            (
                &stdout,
                completion.stdout,
                "stdout",
                binding.command().stdout_max_bytes(),
            ),
            (
                &stderr,
                completion.stderr,
                "stderr",
                binding.command().stderr_max_bytes(),
            ),
        ] {
            if state == StreamCompletion::Complete && artifact.is_none() {
                return Err(DomainError::Invalid(
                    "complete stream requires retained artifact",
                ));
            }
            if let Some(output) = artifact {
                if output.project() != &execution_snapshot
                    || output.graph_version() != binding.task().graph_version()
                    || output.analysis_run() != binding.run_id()
                    || output.kind() != kind
                    || output.byte_length() > budget
                    || output.id() == binding.candidate().id()
                {
                    return Err(DomainError::Invalid(
                        "output artifact does not match execution",
                    ));
                }
            }
        }
        if let (Some(out), Some(err)) = (&stdout, &stderr) {
            if out.id() == err.id() {
                return Err(DomainError::Invalid(
                    "stream artifact identities must differ",
                ));
            }
        }
        Ok(Self {
            binding,
            host_id,
            execution_snapshot,
            started_at_ms,
            finished_at_ms,
            elapsed_ms,
            stdout,
            stderr,
            completion,
        })
    }
    #[must_use]
    pub fn binding(&self) -> &CheckRunBinding {
        &self.binding
    }
    #[must_use]
    pub fn host_id(&self) -> &String {
        &self.host_id
    }
    #[must_use]
    pub fn execution_snapshot(&self) -> &ProjectRef {
        &self.execution_snapshot
    }
    #[must_use]
    pub fn started_at_ms(&self) -> i64 {
        self.started_at_ms
    }
    #[must_use]
    pub fn finished_at_ms(&self) -> i64 {
        self.finished_at_ms
    }
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }
    #[must_use]
    pub fn stdout(&self) -> &Option<Artifact> {
        &self.stdout
    }
    #[must_use]
    pub fn stderr(&self) -> &Option<Artifact> {
        &self.stderr
    }
    #[must_use]
    pub fn completion(&self) -> &ExecutionCompletion {
        &self.completion
    }
}

use crate::{Artifact, CheckCommand, DomainError, Lease, RequiredChecks, TaskSpec, validate_text};

/// Structural provenance, NOT a current lease or execution/integration grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckRunBinding {
    run_id: String,
    task: TaskSpec,
    origin_lease: Lease,
    submission_sequence: i64,
    candidate: Artifact,
    policy: RequiredChecks,
    check_name: String,
    command: CheckCommand,
}
impl CheckRunBinding {
    /// Binds a required check to a submitted candidate and its origin lease.
    ///
    /// # Errors
    /// Returns an error for invalid run/check text, a lease for another task,
    /// nonpositive lease expiry or submission sequence, a candidate from another
    /// project/graph snapshot, or a check absent from the required policy.
    /// This does not test whether the lease is currently valid.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: String,
        task: TaskSpec,
        origin_lease: Lease,
        submission_sequence: i64,
        candidate: Artifact,
        policy: RequiredChecks,
        check_name: String,
        command: CheckCommand,
    ) -> Result<Self, DomainError> {
        validate_text("check run id", &run_id)?;
        validate_text("check name", &check_name)?;
        if origin_lease.task_id() != task.id()
            || origin_lease.expires_at_ms() <= 0
            || submission_sequence <= 0
        {
            return Err(DomainError::Invalid(
                "invalid check submission/lease binding",
            ));
        }
        if candidate.project() != task.project()
            || candidate.graph_version() != task.graph_version()
        {
            return Err(DomainError::Invalid(
                "candidate does not match task snapshot",
            ));
        }
        if !policy.names().any(|name| name == check_name) {
            return Err(DomainError::Invalid("check is absent from required policy"));
        }
        Ok(Self {
            run_id,
            task,
            origin_lease,
            submission_sequence,
            candidate,
            policy,
            check_name,
            command,
        })
    }
    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    #[must_use]
    pub fn task(&self) -> &TaskSpec {
        &self.task
    }
    #[must_use]
    pub fn origin_lease(&self) -> &Lease {
        &self.origin_lease
    }
    #[must_use]
    pub fn submission_sequence(&self) -> i64 {
        self.submission_sequence
    }
    #[must_use]
    pub fn candidate(&self) -> &Artifact {
        &self.candidate
    }
    #[must_use]
    pub fn policy(&self) -> &RequiredChecks {
        &self.policy
    }
    #[must_use]
    pub fn check_name(&self) -> &str {
        &self.check_name
    }
    #[must_use]
    pub fn command(&self) -> &CheckCommand {
        &self.command
    }
}

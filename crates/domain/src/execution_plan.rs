use crate::{CheckRunBinding, DomainError, ExecutionReceipt, ProjectRef, validate_text};

/// Intended execution target, not an authenticated launch permission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionPlan {
    binding: CheckRunBinding,
    host_id: String,
    execution_snapshot: ProjectRef,
}
impl ExecutionPlan {
    /// Binds a check run to a host and execution snapshot.
    ///
    /// # Errors
    /// Returns an error when host identity or snapshot fields do not satisfy
    /// the binding invariants.
    pub fn new(
        binding: CheckRunBinding,
        host_id: String,
        execution_snapshot: ProjectRef,
    ) -> Result<Self, DomainError> {
        validate_target(&binding, &host_id, &execution_snapshot)?;
        Ok(Self {
            binding,
            host_id,
            execution_snapshot,
        })
    }
    #[must_use]
    pub fn binding(&self) -> &CheckRunBinding {
        &self.binding
    }
    #[must_use]
    pub fn host_id(&self) -> &str {
        &self.host_id
    }
    #[must_use]
    pub fn execution_snapshot(&self) -> &ProjectRef {
        &self.execution_snapshot
    }
    #[must_use]
    pub fn matches_receipt(&self, receipt: &ExecutionReceipt) -> bool {
        receipt.binding() == &self.binding
            && receipt.host_id() == &self.host_id
            && receipt.execution_snapshot() == &self.execution_snapshot
    }
}

pub(crate) fn validate_target(
    binding: &CheckRunBinding,
    host: &str,
    snapshot: &ProjectRef,
) -> Result<(), DomainError> {
    validate_text("execution host id", host)?;
    snapshot.validate()?;
    let source = binding.task().project();
    if snapshot.repository_id != source.repository_id
        || snapshot.config_hash != source.config_hash
        || snapshot.ignore_policy_version != source.ignore_policy_version
    {
        return Err(DomainError::Invalid(
            "execution snapshot differs from task repository policy",
        ));
    }
    Ok(())
}

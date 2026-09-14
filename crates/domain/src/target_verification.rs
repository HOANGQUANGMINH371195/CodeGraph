use crate::{DomainError, ProjectRef, TaskSpec, validate_text};

/// A verifier's observation that the integration target still is the task's
/// exact source snapshot. Constructing this value does not authenticate the
/// verifier; that is the application port's responsibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetHeadVerification {
    task: TaskSpec,
    observed_target: ProjectRef,
    observed_at_ms: i64,
    verifier_version: String,
}

impl TargetHeadVerification {
    pub fn new(
        task: TaskSpec,
        observed_target: ProjectRef,
        observed_at_ms: i64,
        verifier_version: String,
    ) -> Result<Self, DomainError> {
        task.validate()?;
        observed_target.validate()?;
        if observed_at_ms < 0 {
            return Err(DomainError::Invalid("target verification time is negative"));
        }
        validate_text("target verifier version", &verifier_version)?;
        if observed_target != *task.project() {
            return Err(DomainError::Invalid(
                "observed target differs from task source snapshot",
            ));
        }
        Ok(Self {
            task,
            observed_target,
            observed_at_ms,
            verifier_version,
        })
    }

    pub fn task(&self) -> &TaskSpec {
        &self.task
    }
    pub fn observed_target(&self) -> &ProjectRef {
        &self.observed_target
    }
    pub fn observed_at_ms(&self) -> i64 {
        self.observed_at_ms
    }
    pub fn verifier_version(&self) -> &str {
        &self.verifier_version
    }
}

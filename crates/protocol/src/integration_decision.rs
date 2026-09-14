use crate::{
    Artifact, ExecutionReceipt, ProjectRef, ProtocolError, RequiredChecks, SCHEMA_VERSION,
    TaskSpec, require_current_schema,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationDecision {
    pub schema_version: u32,
    pub task: TaskSpec,
    pub candidate: Artifact,
    pub policy: RequiredChecks,
    pub receipts: Vec<ExecutionReceipt>,
    pub accepted_at_ms: i64,
    pub verifier_version: String,
}

impl IntegrationDecision {
    pub fn try_into_domain(self) -> Result<graph_domain::IntegrationDecision, ProtocolError> {
        require_current_schema(self.schema_version)?;
        graph_domain::IntegrationDecision::new(
            self.task.try_into_domain()?,
            self.candidate.try_into_domain()?,
            self.policy.try_into_domain()?,
            self.receipts
                .into_iter()
                .map(ExecutionReceipt::try_into_domain)
                .collect::<Result<Vec<_>, _>>()?,
            self.accepted_at_ms,
            self.verifier_version,
        )
        .map_err(Into::into)
    }
}
impl From<&graph_domain::IntegrationDecision> for IntegrationDecision {
    fn from(value: &graph_domain::IntegrationDecision) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            task: value.task().into(),
            candidate: value.candidate().into(),
            policy: value.policy().into(),
            receipts: value
                .receipts()
                .iter()
                .map(ExecutionReceipt::from)
                .collect(),
            accepted_at_ms: value.accepted_at_ms(),
            verifier_version: value.verifier_version().into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetHeadVerification {
    pub schema_version: u32,
    pub task: TaskSpec,
    pub observed_target: ProjectRef,
    pub observed_at_ms: i64,
    pub verifier_version: String,
}
impl TargetHeadVerification {
    pub fn try_into_domain(self) -> Result<graph_domain::TargetHeadVerification, ProtocolError> {
        require_current_schema(self.schema_version)?;
        graph_domain::TargetHeadVerification::new(
            self.task.try_into_domain()?,
            self.observed_target.into(),
            self.observed_at_ms,
            self.verifier_version,
        )
        .map_err(Into::into)
    }
}
impl From<&graph_domain::TargetHeadVerification> for TargetHeadVerification {
    fn from(value: &graph_domain::TargetHeadVerification) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            task: value.task().into(),
            observed_target: value.observed_target().into(),
            observed_at_ms: value.observed_at_ms(),
            verifier_version: value.verifier_version().into(),
        }
    }
}

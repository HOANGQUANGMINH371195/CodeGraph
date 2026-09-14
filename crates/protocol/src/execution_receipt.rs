use crate::execution::ExecutionCompletion;
use crate::{
    Artifact, CheckRunBinding, ProjectRef, ProtocolError, SCHEMA_VERSION, require_current_schema,
};
use serde::{Deserialize, Serialize};

/// Untrusted wire receipt. Parsing cannot attest to a host or actual execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReceipt {
    pub schema_version: u32,
    pub binding: CheckRunBinding,
    pub host_id: String,
    pub execution_snapshot: ProjectRef,
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
    pub elapsed_ms: u64,
    #[serde(deserialize_with = "required_output")]
    pub stdout: Option<Artifact>,
    #[serde(deserialize_with = "required_output")]
    pub stderr: Option<Artifact>,
    pub completion: ExecutionCompletion,
}
// Require explicit null rather than silently defaulting a missing output field.
fn required_output<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Artifact>, D::Error> {
    Option::<Artifact>::deserialize(d)
}
impl ExecutionReceipt {
    pub fn try_into_domain(self) -> Result<graph_domain::ExecutionReceipt, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(graph_domain::ExecutionReceipt::new(
            self.binding.try_into_domain()?,
            self.host_id,
            self.execution_snapshot.into(),
            self.started_at_ms,
            self.finished_at_ms,
            self.elapsed_ms,
            self.stdout.map(Artifact::try_into_domain).transpose()?,
            self.stderr.map(Artifact::try_into_domain).transpose()?,
            self.completion.try_into_domain()?,
        )?)
    }
}
impl From<&graph_domain::ExecutionReceipt> for ExecutionReceipt {
    fn from(value: &graph_domain::ExecutionReceipt) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            binding: value.binding().into(),
            host_id: value.host_id().clone(),
            execution_snapshot: value.execution_snapshot().into(),
            started_at_ms: value.started_at_ms(),
            finished_at_ms: value.finished_at_ms(),
            elapsed_ms: value.elapsed_ms(),
            stdout: value.stdout().as_ref().map(Artifact::from),
            stderr: value.stderr().as_ref().map(Artifact::from),
            completion: value.completion().into(),
        }
    }
}

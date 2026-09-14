use crate::{CheckRunBinding, ProjectRef, ProtocolError, SCHEMA_VERSION, require_current_schema};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPlan {
    pub schema_version: u32,
    pub binding: CheckRunBinding,
    pub host_id: String,
    pub execution_snapshot: ProjectRef,
}
impl ExecutionPlan {
    pub fn try_into_domain(self) -> Result<graph_domain::ExecutionPlan, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(graph_domain::ExecutionPlan::new(
            self.binding.try_into_domain()?,
            self.host_id,
            self.execution_snapshot.into(),
        )?)
    }
}
impl From<&graph_domain::ExecutionPlan> for ExecutionPlan {
    fn from(value: &graph_domain::ExecutionPlan) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            binding: value.binding().into(),
            host_id: value.host_id().into(),
            execution_snapshot: value.execution_snapshot().into(),
        }
    }
}

use crate::{ProjectRef, ProtocolError, SCHEMA_VERSION, require_current_schema};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisRun {
    pub schema_version: u32,
    pub id: String,
    pub project: ProjectRef,
    pub graph_version: String,
    pub analyzer: String,
    pub analyzer_version: String,
    pub configuration_sha256: String,
    pub input_manifest_sha256: String,
}

impl AnalysisRun {
    pub fn try_into_domain(self) -> Result<graph_domain::AnalysisRun, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(graph_domain::AnalysisRun::new(
            self.id,
            self.project.into(),
            self.graph_version,
            self.analyzer,
            self.analyzer_version,
            self.configuration_sha256,
            self.input_manifest_sha256,
        )?)
    }
}

impl From<&graph_domain::AnalysisRun> for AnalysisRun {
    fn from(run: &graph_domain::AnalysisRun) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: run.id().into(),
            project: run.project().into(),
            graph_version: run.graph_version().into(),
            analyzer: run.analyzer().into(),
            analyzer_version: run.analyzer_version().into(),
            configuration_sha256: run.configuration_sha256().into(),
            input_manifest_sha256: run.input_manifest_sha256().into(),
        }
    }
}

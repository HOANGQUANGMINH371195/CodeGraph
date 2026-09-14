use crate::{
    AnalysisRun, Artifact, ProtocolError, RpcSpawnObservation, SCHEMA_VERSION,
    execution::ExecutionCompletion, require_current_schema,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UncertainRpc {
    pub id: i64,
    pub method: String,
    pub uncertain: bool,
}
impl std::fmt::Debug for UncertainRpc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UncertainRpc { .. }")
    }
}
impl UncertainRpc {
    pub fn try_into_domain(self) -> Result<graph_domain::UncertainRpc, ProtocolError> {
        if !self.uncertain {
            return Err(
                graph_domain::DomainError::Invalid("terminal RPC must remain uncertain").into(),
            );
        }
        Ok(graph_domain::UncertainRpc::new(self.id, self.method)?)
    }
}
impl From<&graph_domain::UncertainRpc> for UncertainRpc {
    fn from(value: &graph_domain::UncertainRpc) -> Self {
        Self {
            id: value.id(),
            method: value.method().into(),
            uncertain: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcInputProgress {
    pub total: u64,
    pub written: u64,
    pub failed: bool,
}
impl RpcInputProgress {
    pub fn try_into_domain(self) -> Result<graph_domain::RpcInputProgress, ProtocolError> {
        Ok(graph_domain::RpcInputProgress::new(
            self.total,
            self.written,
            self.failed,
        )?)
    }
}
impl From<graph_domain::RpcInputProgress> for RpcInputProgress {
    fn from(value: graph_domain::RpcInputProgress) -> Self {
        Self {
            total: value.total(),
            written: value.written(),
            failed: value.failed(),
        }
    }
}

/// Untrusted report; callers bound input bytes before parsing and sanitize errors.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcTerminalReceipt {
    pub schema_version: u32,
    pub spawn: RpcSpawnObservation,
    pub output_run: AnalysisRun,
    pub finished_at_ms: i64,
    pub supervised_elapsed_ms: u64,
    #[serde(deserialize_with = "required_output")]
    pub stdout: Option<Artifact>,
    #[serde(deserialize_with = "required_output")]
    pub stderr: Option<Artifact>,
    pub completion: ExecutionCompletion,
    pub pending: Vec<UncertainRpc>,
    #[serde(deserialize_with = "required_progress")]
    pub input_progress: Option<RpcInputProgress>,
}
impl std::fmt::Debug for RpcTerminalReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RpcTerminalReceipt { .. }")
    }
}
fn required_output<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Artifact>, D::Error> {
    Option::<Artifact>::deserialize(d)
}
fn required_progress<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<RpcInputProgress>, D::Error> {
    Option::<RpcInputProgress>::deserialize(d)
}
impl RpcTerminalReceipt {
    pub fn try_into_domain(self) -> Result<graph_domain::RpcTerminalReceipt, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(graph_domain::RpcTerminalReceipt::new(
            self.spawn.try_into_domain()?,
            self.output_run.try_into_domain()?,
            self.finished_at_ms,
            self.supervised_elapsed_ms,
            self.stdout.map(Artifact::try_into_domain).transpose()?,
            self.stderr.map(Artifact::try_into_domain).transpose()?,
            self.completion.try_into_domain()?,
            self.pending
                .into_iter()
                .map(UncertainRpc::try_into_domain)
                .collect::<Result<_, _>>()?,
            self.input_progress
                .map(RpcInputProgress::try_into_domain)
                .transpose()?,
        )?)
    }
}
impl From<&graph_domain::RpcTerminalReceipt> for RpcTerminalReceipt {
    fn from(value: &graph_domain::RpcTerminalReceipt) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            spawn: value.spawn().into(),
            output_run: value.output_run().into(),
            finished_at_ms: value.finished_at_ms(),
            supervised_elapsed_ms: value.supervised_elapsed_ms(),
            stdout: value.stdout().map(Artifact::from),
            stderr: value.stderr().map(Artifact::from),
            completion: value.completion().into(),
            pending: value.pending().iter().map(UncertainRpc::from).collect(),
            input_progress: value.input_progress().map(RpcInputProgress::from),
        }
    }
}

#[cfg(test)]
#[path = "rpc_terminal_tests.rs"]
mod tests;

//! Versioned JSON contracts at the process boundary. Domain objects never need
//! to deserialize untrusted wire data directly.

use graph_domain::{
    self as domain, Lease as DomainLease, ProjectRef as DomainProjectRef, TaskEvent, TaskId,
    TaskSpec as DomainTaskSpec, WorkerId,
};
use serde::{Deserialize, Serialize};

mod schema;
pub use schema::{
    ExtensionFields, SCHEMA_VERSION, SchemaCompatibility, require_current_schema,
    schema_compatibility,
};
mod analysis;
pub mod archify;
mod artifact;
mod check_run;
mod checks;
mod command;
pub mod deployment;
pub mod rpc_journal;
mod rpc_launch;
mod rpc_spawn;
mod rpc_terminal;
pub use rpc_launch::{RpcConnectionSpec, RpcLaunchSpec, RpcProcessSpec, RpcTransport};
pub use rpc_spawn::RpcSpawnObservation;
pub use rpc_terminal::{RpcInputProgress, RpcTerminalReceipt, UncertainRpc};
mod execution_plan;
mod execution_receipt;
mod fact;
mod graph_version;
pub use check_run::CheckRunBinding;
pub use checks::RequiredChecks;
pub use command::CheckCommand;
pub use execution_plan::ExecutionPlan;
pub use execution_receipt::ExecutionReceipt;
pub use fact::{ClaimDecision, EvidenceRef, FactAssertion, Producer, VerificationReceipt};
pub use graph_version::{
    GraphDelta, GraphSnapshotCache, GraphVersion, SnapshotFreshness, SnapshotStaleReason,
};
mod integration_decision;
pub use integration_decision::{IntegrationDecision, TargetHeadVerification};
pub mod connection;
pub mod context;
pub mod context_pack;
pub mod correlation;
pub mod execution;
pub mod file_reads;
pub mod framing;
pub mod handshake;
pub mod native;
pub mod output;
pub mod rpc;
pub mod sql;
pub mod sql_link;
pub use analysis::AnalysisRun;
pub use artifact::{Artifact, ArtifactProtection, ArtifactRetention};

/// Untrusted source citation. The hash and run ID are claims until verified.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEvidence {
    pub schema_version: u32,
    pub id: String,
    pub project: ProjectRef,
    pub graph_version: String,
    pub path: String,
    pub content_sha256: String,
    pub start_line: u32,
    pub end_line: u32,
    pub analysis_run: String,
}

impl SourceEvidence {
    pub fn try_into_domain(self) -> Result<domain::SourceEvidence, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(domain::SourceEvidence::new(
            self.id,
            self.project.into(),
            self.graph_version,
            self.path,
            self.content_sha256,
            self.start_line,
            self.end_line,
            self.analysis_run,
        )?)
    }
}

impl From<&domain::SourceEvidence> for SourceEvidence {
    fn from(evidence: &domain::SourceEvidence) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: evidence.id().into(),
            project: evidence.project().into(),
            graph_version: evidence.graph_version().into(),
            path: evidence.path().into(),
            content_sha256: evidence.content_sha256().into(),
            start_line: evidence.start_line(),
            end_line: evidence.end_line(),
            analysis_run: evidence.analysis_run().into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectRef {
    pub repository_id: String,
    pub worktree_id: String,
    pub git_head: String,
    pub working_tree_fingerprint: String,
    pub config_hash: String,
    pub ignore_policy_version: String,
}

impl From<&DomainProjectRef> for ProjectRef {
    fn from(project: &DomainProjectRef) -> Self {
        Self {
            repository_id: project.repository_id.clone(),
            worktree_id: project.worktree_id.clone(),
            git_head: project.git_head.clone(),
            working_tree_fingerprint: project.working_tree_fingerprint.clone(),
            config_hash: project.config_hash.clone(),
            ignore_policy_version: project.ignore_policy_version.clone(),
        }
    }
}

impl From<ProjectRef> for DomainProjectRef {
    fn from(project: ProjectRef) -> Self {
        Self {
            repository_id: project.repository_id,
            worktree_id: project.worktree_id,
            git_head: project.git_head,
            working_tree_fingerprint: project.working_tree_fingerprint,
            config_hash: project.config_hash,
            ignore_policy_version: project.ignore_policy_version,
        }
    }
}

/// An untrusted task request. Convert it with `try_into_domain` before using
/// it for scheduling or persistence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub schema_version: u32,
    pub id: String,
    pub project: ProjectRef,
    pub graph_version: String,
    pub role: String,
    pub account_lane: String,
    pub scope: Vec<String>,
    pub dependencies: Vec<String>,
    pub context_ref: String,
    pub expected_artifacts: Vec<String>,
    pub token_budget: u64,
}

impl TaskSpec {
    pub fn try_into_domain(self) -> Result<DomainTaskSpec, ProtocolError> {
        require_current_schema(self.schema_version)?;
        let id = TaskId::new(self.id)?;
        let dependencies = self
            .dependencies
            .into_iter()
            .map(TaskId::new)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(DomainTaskSpec::new(
            id,
            self.project.into(),
            self.graph_version,
            self.role,
            self.account_lane,
            self.scope,
            dependencies,
            self.context_ref,
            self.expected_artifacts,
            self.token_budget,
        )?)
    }
}

impl From<&DomainTaskSpec> for TaskSpec {
    fn from(task: &DomainTaskSpec) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: task.id().as_str().into(),
            project: task.project().into(),
            graph_version: task.graph_version().into(),
            role: task.role().into(),
            account_lane: task.account_lane().into(),
            scope: task.scope().to_vec(),
            dependencies: task
                .dependencies()
                .iter()
                .map(ToString::to_string)
                .collect(),
            context_ref: task.context_ref().into(),
            expected_artifacts: task.expected_artifacts().to_vec(),
            token_budget: task.token_budget(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lease {
    pub task_id: String,
    pub owner: String,
    pub fencing_token: i64,
    pub expires_at_ms: i64,
}

impl Lease {
    pub fn try_into_domain(self) -> Result<DomainLease, ProtocolError> {
        Ok(DomainLease::issue(
            TaskId::new(self.task_id)?,
            WorkerId::new(self.owner)?,
            self.fencing_token,
            self.expires_at_ms,
        )?)
    }
}

impl From<&DomainLease> for Lease {
    fn from(lease: &DomainLease) -> Self {
        Self {
            task_id: lease.task_id().as_str().into(),
            owner: lease.owner().as_str().into(),
            fencing_token: lease.fencing_token(),
            expires_at_ms: lease.expires_at_ms(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Event {
    pub sequence: i64,
    pub task_id: String,
    pub kind: String,
    pub at_ms: i64,
    pub payload: String,
}

impl From<TaskEvent> for Event {
    fn from(event: TaskEvent) -> Self {
        Self {
            sequence: event.sequence,
            task_id: event.task_id.to_string(),
            kind: event.kind.as_str().into(),
            at_ms: event.at_ms,
            payload: event.payload,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("unsupported protocol schema version {0}")]
    UnsupportedSchema(u32),
    #[error("unsupported context schema version {0}")]
    UnsupportedContextSchema(String),
    #[error("unsupported context-pack schema version {0}")]
    UnsupportedContextPackSchema(String),
    #[error("invalid schema extension fields: {0}")]
    InvalidSchemaExtensions(&'static str),
    #[error("invalid context envelope: {0}")]
    InvalidContext(String),
    #[error("invalid context pack: {0}")]
    InvalidContextPack(String),
    #[error("invalid fact assertion: {0}")]
    InvalidFact(&'static str),
    #[error(transparent)]
    Domain(#[from] domain::DomainError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_future_wire_schema_before_domain_conversion() {
        let request = TaskSpec {
            schema_version: 2,
            id: "task".into(),
            project: ProjectRef {
                repository_id: "repo".into(),
                worktree_id: "main".into(),
                git_head: "a".into(),
                working_tree_fingerprint: "clean".into(),
                config_hash: "c".into(),
                ignore_policy_version: "1".into(),
            },
            graph_version: "g".into(),
            role: "role".into(),
            account_lane: "lane".into(),
            scope: vec!["src".into()],
            dependencies: vec![],
            context_ref: "ctx".into(),
            expected_artifacts: vec!["patch".into()],
            token_budget: 1,
        };
        assert!(matches!(
            request.try_into_domain(),
            Err(ProtocolError::UnsupportedSchema(2))
        ));
    }
}

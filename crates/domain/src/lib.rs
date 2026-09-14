//! Pure domain rules for the graph harness. This crate deliberately has no
//! `SQLite`, process, network, or CLI dependency.

use std::collections::HashSet;

use thiserror::Error;

mod analysis;
mod artifact;
mod check_run;
mod checks;
mod command;
mod context_pack;
mod rpc_launch;
mod rpc_spawn;
mod rpc_terminal;
pub use rpc_launch::{RpcConnectionSpec, RpcLaunchSpec, RpcProcessSpec};
pub use rpc_spawn::{RpcSpawnDisposition, RpcSpawnObservation};
pub use rpc_terminal::{RpcInputProgress, RpcTerminalReceipt, UncertainRpc};
mod execution_plan;
mod execution_receipt;
mod fact;
mod graph_version;
mod identity;
mod integration_decision;
mod target_verification;
pub use check_run::CheckRunBinding;
pub use command::CheckCommand;
pub use context_pack::{
    Capability, CapabilityGrant, ClaimBundle, ContextOperation, ContextPack, Handoff,
    ScopeSelector, UnknownClaim, UntrustedContentLabel, UntrustedContentOrigin,
};
pub use execution_plan::ExecutionPlan;
pub use execution_receipt::ExecutionReceipt;
pub use graph_version::{
    GraphDelta, GraphSnapshotCache, GraphVersion, SnapshotFreshness, SnapshotStaleReason,
};
pub use identity::{
    EDGE_IDENTITY_VERSION, EdgeId, EdgeIdentityInput, IdentityNamespace, NODE_IDENTITY_VERSION,
    NodeId, NodeIdentityInput,
};
pub use integration_decision::IntegrationDecision;
pub use target_verification::TargetHeadVerification;
pub mod deployment;
mod evidence;
pub mod execution;
pub mod sql_link;
pub use analysis::AnalysisRun;
pub use artifact::{Artifact, ArtifactProtection, ArtifactRetention};
pub use checks::{CheckIssue, CheckObservation, CheckOutcome, RequiredChecks};
pub use evidence::SourceEvidence;
pub use fact::{
    AcceptedFact, AssertionKind, ClaimDecision, ClaimState, EvidenceKind, EvidenceRef,
    FactAssertion, GraphWriter, Producer, VerificationReceipt,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        validate_text("task id", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerId(String);

impl WorkerId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        validate_text("worker id", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectRef {
    pub repository_id: String,
    pub worktree_id: String,
    pub git_head: String,
    pub working_tree_fingerprint: String,
    pub config_hash: String,
    pub ignore_policy_version: String,
}

impl ProjectRef {
    pub fn validate(&self) -> Result<(), DomainError> {
        for (name, value) in [
            ("repository id", &self.repository_id),
            ("worktree id", &self.worktree_id),
            ("git head", &self.git_head),
            ("working tree fingerprint", &self.working_tree_fingerprint),
            ("config hash", &self.config_hash),
            ("ignore policy version", &self.ignore_policy_version),
        ] {
            validate_text(name, value)?;
        }
        Ok(())
    }
}

/// The immutable work contract. A task's identity is its `TaskId`, while the
/// rest is a snapshot that must remain stable for idempotent enqueue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSpec {
    id: TaskId,
    project: ProjectRef,
    graph_version: String,
    role: String,
    account_lane: String,
    scope: Vec<String>,
    dependencies: Vec<TaskId>,
    context_ref: String,
    expected_artifacts: Vec<String>,
    token_budget: u64,
}

impl TaskSpec {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TaskId,
        project: ProjectRef,
        graph_version: String,
        role: String,
        account_lane: String,
        scope: Vec<String>,
        dependencies: Vec<TaskId>,
        context_ref: String,
        expected_artifacts: Vec<String>,
        token_budget: u64,
    ) -> Result<Self, DomainError> {
        let task = Self {
            id,
            project,
            graph_version,
            role,
            account_lane,
            scope,
            dependencies,
            context_ref,
            expected_artifacts,
            token_budget,
        };
        task.validate()?;
        Ok(task)
    }

    pub fn id(&self) -> &TaskId {
        &self.id
    }
    pub fn project(&self) -> &ProjectRef {
        &self.project
    }
    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }
    pub fn role(&self) -> &str {
        &self.role
    }
    pub fn account_lane(&self) -> &str {
        &self.account_lane
    }
    pub fn scope(&self) -> &[String] {
        &self.scope
    }
    pub fn dependencies(&self) -> &[TaskId] {
        &self.dependencies
    }
    pub fn context_ref(&self) -> &str {
        &self.context_ref
    }
    pub fn expected_artifacts(&self) -> &[String] {
        &self.expected_artifacts
    }
    pub fn token_budget(&self) -> u64 {
        self.token_budget
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        self.project.validate()?;
        for (name, value) in [
            ("graph version", &self.graph_version),
            ("role", &self.role),
            ("account lane", &self.account_lane),
            ("context reference", &self.context_ref),
        ] {
            validate_text(name, value)?;
        }
        if self.scope.is_empty() {
            return Err(DomainError::Missing("scope"));
        }
        if self.expected_artifacts.is_empty() {
            return Err(DomainError::Missing("expected artifacts"));
        }
        if self.token_budget == 0 {
            return Err(DomainError::Invalid("token budget must be positive"));
        }
        validate_text_list("scope", &self.scope)?;
        validate_text_list("expected artifact", &self.expected_artifacts)?;
        let mut dependencies = HashSet::new();
        for dependency in &self.dependencies {
            if dependency == &self.id {
                return Err(DomainError::Invalid("a task cannot depend on itself"));
            }
            if !dependencies.insert(dependency) {
                return Err(DomainError::Invalid("duplicate dependency"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Queued,
    Leased,
    Submitted,
    Integrated,
    Rejected,
    Cancelled,
}

impl TaskState {
    pub fn can_cancel(self) -> bool {
        matches!(self, Self::Queued | Self::Leased | Self::Submitted)
    }

    pub fn as_db(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Leased => "leased",
            Self::Submitted => "submitted",
            Self::Integrated => "integrated",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DomainError> {
        match value {
            "queued" => Ok(Self::Queued),
            "leased" => Ok(Self::Leased),
            "submitted" => Ok(Self::Submitted),
            "integrated" => Ok(Self::Integrated),
            "rejected" => Ok(Self::Rejected),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(DomainError::Invalid("unknown task state in persistence")),
        }
    }

    pub fn satisfies_dependency(self) -> bool {
        self == Self::Integrated
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    task_id: TaskId,
    owner: WorkerId,
    fencing_token: i64,
    expires_at_ms: i64,
}

impl Lease {
    pub fn issue(
        task_id: TaskId,
        owner: WorkerId,
        fencing_token: i64,
        expires_at_ms: i64,
    ) -> Result<Self, DomainError> {
        if fencing_token <= 0 {
            return Err(DomainError::Invalid("fencing token must be positive"));
        }
        Ok(Self {
            task_id,
            owner,
            fencing_token,
            expires_at_ms,
        })
    }

    pub fn task_id(&self) -> &TaskId {
        &self.task_id
    }
    pub fn owner(&self) -> &WorkerId {
        &self.owner
    }
    pub fn fencing_token(&self) -> i64 {
        self.fencing_token
    }
    pub fn expires_at_ms(&self) -> i64 {
        self.expires_at_ms
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    RpcLaunchRegistered,
    RpcLaunchClaimed,
    RpcSpawnObserved,
    RpcTerminalRecorded,
    ExecutionLaunchClaimed,
    CheckPolicyRegistered,
    Queued,
    Leased,
    Submitted,
    Integrated,
    Rejected,
    Cancelled,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RpcLaunchRegistered => "rpc_launch_registered",
            Self::RpcLaunchClaimed => "rpc_launch_claimed",
            Self::RpcSpawnObserved => "rpc_spawn_observed",
            Self::RpcTerminalRecorded => "rpc_terminal_recorded",
            Self::CheckPolicyRegistered => "check_policy_registered",
            Self::ExecutionLaunchClaimed => "execution_launch_claimed",
            Self::Queued => "queued",
            Self::Leased => "leased",
            Self::Submitted => "submitted",
            Self::Integrated => "integrated",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value {
            "rpc_launch_registered" => Ok(Self::RpcLaunchRegistered),
            "rpc_launch_claimed" => Ok(Self::RpcLaunchClaimed),
            "rpc_spawn_observed" => Ok(Self::RpcSpawnObserved),
            "rpc_terminal_recorded" => Ok(Self::RpcTerminalRecorded),
            "check_policy_registered" => Ok(Self::CheckPolicyRegistered),
            "execution_launch_claimed" => Ok(Self::ExecutionLaunchClaimed),
            "queued" => Ok(Self::Queued),
            "leased" => Ok(Self::Leased),
            "submitted" => Ok(Self::Submitted),
            "integrated" => Ok(Self::Integrated),
            "rejected" => Ok(Self::Rejected),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(DomainError::Invalid("unknown event kind in persistence")),
        }
    }
}

impl std::str::FromStr for EventKind {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_str(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskEvent {
    pub sequence: i64,
    pub task_id: TaskId,
    pub kind: EventKind,
    pub at_ms: i64,
    pub payload: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("{0} is required")]
    Missing(&'static str),
    #[error("invalid domain value: {0}")]
    Invalid(&'static str),
}

fn validate_text(name: &'static str, value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        Err(DomainError::Missing(name))
    } else {
        Ok(())
    }
}

fn validate_text_list(name: &'static str, values: &[String]) -> Result<(), DomainError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        Err(DomainError::Invalid(name))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: "abc".into(),
            working_tree_fingerprint: "clean".into(),
            config_hash: "cfg".into(),
            ignore_policy_version: "1".into(),
        }
    }

    #[test]
    fn rejects_self_and_duplicate_dependencies() {
        let id = TaskId::new("task").unwrap();
        let self_dependency = TaskSpec::new(
            id.clone(),
            project(),
            "g".into(),
            "role".into(),
            "lane".into(),
            vec!["src".into()],
            vec![id.clone()],
            "context".into(),
            vec!["patch".into()],
            1,
        );
        assert!(matches!(
            self_dependency,
            Err(DomainError::Invalid("a task cannot depend on itself"))
        ));
        let dependency = TaskId::new("parent").unwrap();
        let duplicate = TaskSpec::new(
            id,
            project(),
            "g".into(),
            "role".into(),
            "lane".into(),
            vec!["src".into()],
            vec![dependency.clone(), dependency],
            "context".into(),
            vec!["patch".into()],
            1,
        );
        assert!(matches!(
            duplicate,
            Err(DomainError::Invalid("duplicate dependency"))
        ));
    }
}

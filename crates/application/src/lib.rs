//! Application use cases. Adapters implement `TaskRepository`; this crate owns
//! neither SQLite nor the command-line interface.

use std::error::Error;

use graph_domain::{DomainError, Lease, TaskEvent, TaskId, TaskSpec, WorkerId};
use thiserror::Error;

mod admission;
mod check_binding;
pub mod deployment_context;
mod receipt_output;
pub mod sql_link_context;
pub use check_binding::{CheckBindingError, reconcile_check_binding};
pub use receipt_output::{ReceiptOutputError, ReceiptOutputObservation, verify_receipt_outputs};
mod integration_verification;
pub use integration_verification::{
    IntegrationVerificationError, TargetHeadVerifier, VerifiedIntegration, verify_integration,
};
mod fingerprint;
mod identity;
pub use fingerprint::{command_fingerprint, environment_fingerprint};
pub use identity::{derive_edge_id, derive_node_id, sha256_fingerprint};
mod artifact;
mod fact;
pub use admission::{AdmissionError, SwarmAdmission};
mod rpc_output;
mod rpc_query;
pub use rpc_output::{RpcOutputError, RpcOutputObservation, verify_rpc_outputs};
mod source;
mod submission;
pub use artifact::{
    ARTIFACT_CONTENT_VERIFIER_VERSION, ArtifactContentObservation, ArtifactIngestion,
    ArtifactIngestionError, ArtifactObservationRepository, ArtifactReader,
    ArtifactVerificationError, ArtifactWriter, VerifiedArtifactContent, ingest_artifact,
    verify_artifact,
};
pub use fact::{FactAssertionRepository, FactWriteError, accept_fact, record_fact_candidate};
pub use rpc_query::{RpcLaunchLedgerSnapshot, RpcLaunchQueryRepository};
pub use source::{
    BoundSourceIdentityVerificationError, SourceIdentityVerificationError, SourceLimits,
    SourceReader, SourceSlice, SourceSnapshotAuthority, SourceSnapshotBinding,
    SourceVerificationError, VerifiedSourceIdentity, capture_source, verify_bound_source_identity,
    verify_source, verify_source_identity,
};
pub use submission::{SubmissionContentError, VerifiedSubmittedContent, verify_submitted_content};

/// Durable RPC descriptors/one-shot markers; not permission to launch or relaunch.
/// The host supplies its own clock; timestamps are not authenticated by this port.
/// One launch origin attempt may own a connection serving separately authorized RPCs.
pub trait RpcLaunchRepository {
    type Error: Error + Send + Sync + 'static;
    /// Registers the complete description against the current origin lease.
    /// Exact replay is historical false, not refreshed approval.
    fn register_rpc_launch(
        &mut self,
        spec: &graph_domain::RpcLaunchSpec,
        now_ms: i64,
    ) -> Result<bool, Self::Error>;
    fn rpc_launch(
        &self,
        id: &str,
        task: &TaskSpec,
    ) -> Result<Option<graph_domain::RpcLaunchSpec>, Self::Error>;
    /// One committed marker per launch origin attempt; no reset after expiry.
    /// True is accounting only: host approval/preflight are still required.
    /// Neither true nor false proves whether a process was spawned or died.
    fn claim_rpc_launch(
        &mut self,
        spec: &graph_domain::RpcLaunchSpec,
        now_ms: i64,
    ) -> Result<bool, Self::Error>;
}

/// Immutable host-reported spawn outcome, not authenticated process or RPC proof.
pub trait RpcSpawnObservationRepository {
    type Error: Error + Send + Sync + 'static;
    /// Requires an exact registered, consumed launch. Late diagnostics are allowed
    /// after cancellation/expiry; exact replay is false, differing data conflicts.
    fn record_rpc_spawn_observation(
        &mut self,
        observation: &graph_domain::RpcSpawnObservation,
    ) -> Result<bool, Self::Error>;
    /// Full launch filter, with claim/descriptor linkage checked in one snapshot.
    fn rpc_spawn_observation(
        &self,
        launch: &graph_domain::RpcLaunchSpec,
    ) -> Result<Option<graph_domain::RpcSpawnObservation>, Self::Error>;
}

/// Immutable terminal claims, not verified output bytes or retry authority.
pub trait RpcTerminalReceiptRepository {
    type Error: Error + Send + Sync + 'static;
    /// Requires exact stored spawn/run/artifacts; permits late historical reports.
    /// True means inserted; identical replay false, differing content conflicts.
    fn record_rpc_terminal_receipt(
        &mut self,
        receipt: &graph_domain::RpcTerminalReceipt,
    ) -> Result<bool, Self::Error>;
    fn rpc_terminal_receipt(
        &self,
        launch: &graph_domain::RpcLaunchSpec,
    ) -> Result<Option<graph_domain::RpcTerminalReceipt>, Self::Error>;
}

/// Immutable intended execution target; not permission to launch or relaunch.
pub trait ExecutionLaunchRepository {
    type Error: Error + Send + Sync + 'static;
    /// True only for the first committed claim. False is never launch permission.
    /// At-most-one ledger claim, not authenticated or exactly-once OS execution.
    /// No reset on timeout/restart: reconcile uncertain process state separately.
    fn claim_execution_launch(
        &mut self,
        plan: &graph_domain::ExecutionPlan,
        now_ms: i64,
    ) -> Result<bool, Self::Error>;
}

/// Immutable intended execution target; not permission to launch or relaunch.
pub trait ExecutionPlanRepository {
    type Error: Error + Send + Sync + 'static;
    /// First insert checks submitted ledger. Replay is historical, never relaunch authority.
    fn register_execution_plan(
        &mut self,
        plan: &graph_domain::ExecutionPlan,
    ) -> Result<bool, Self::Error>;
    fn execution_plan(
        &self,
        run_id: &str,
        task: &TaskSpec,
    ) -> Result<Option<graph_domain::ExecutionPlan>, Self::Error>;
}

/// Historical receipt claims, not verified execution or pre-launch admission.
pub trait ExecutionReceiptRepository {
    type Error: Error + Send + Sync + 'static;
    /// Exact replay returns false; reusing a run ID with different data fails.
    /// Late diagnostics are allowed after cancellation, without changing state.
    fn record_execution_receipt(
        &mut self,
        receipt: &graph_domain::ExecutionReceipt,
    ) -> Result<bool, Self::Error>;
    fn execution_receipt(
        &self,
        run_id: &str,
        task: &TaskSpec,
    ) -> Result<Option<graph_domain::ExecutionReceipt>, Self::Error>;
}

/// Evidence is immutable. Reads require the complete requested snapshot;
/// callers must not use a citation from another worktree or graph version.
pub trait EvidenceRepository {
    type Error: Error + Send + Sync + 'static;

    fn record_source(
        &mut self,
        evidence: &graph_domain::SourceEvidence,
    ) -> Result<bool, Self::Error>;
    fn source_evidence(
        &self,
        id: &str,
        project: &graph_domain::ProjectRef,
        graph_version: &str,
    ) -> Result<Option<graph_domain::SourceEvidence>, Self::Error>;
}

/// Historical declared graph state, never a fresh source/runtime capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentSnapshot {
    pub generation: u64,
    /// None is an explicit invalidation, distinct from an empty valid graph.
    pub graph: Option<graph_domain::deployment::DeploymentGraph>,
}

/// Strict generation compare-and-swap, scoped to one source and adapter.
pub trait DeploymentRepository {
    type Error: Error + Send + Sync + 'static;
    fn replace_deployment(
        &mut self,
        graph: &graph_domain::deployment::DeploymentGraph,
        expected_generation: u64,
    ) -> Result<u64, Self::Error>;
    fn invalidate_deployment(
        &mut self,
        scope: &graph_domain::deployment::DeploymentScope,
        expected_generation: u64,
    ) -> Result<u64, Self::Error>;
    fn deployment(
        &self,
        scope: &graph_domain::deployment::DeploymentScope,
    ) -> Result<Option<DeploymentSnapshot>, Self::Error>;
}

/// Historical code-to-SQL candidate state. `None` is an explicit invalidation,
/// distinct from a verified empty candidate replacement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlLinkSnapshot {
    pub generation: u64,
    pub graph: Option<graph_domain::sql_link::SqlLinkGraph>,
}

/// Strict generation-CAS for host-verified code-to-SQL candidates. This port
/// does not attest execution, physical database/table resolution, semantics or
/// authorization to read the candidate files.
pub trait SqlLinkRepository {
    type Error: Error + Send + Sync + 'static;
    fn replace_sql_links(
        &mut self,
        graph: &graph_domain::sql_link::SqlLinkGraph,
        expected_generation: u64,
    ) -> Result<u64, Self::Error>;
    fn invalidate_sql_links(
        &mut self,
        scope: &graph_domain::sql_link::SqlLinkScope,
        expected_generation: u64,
    ) -> Result<u64, Self::Error>;
    fn sql_links(
        &self,
        scope: &graph_domain::sql_link::SqlLinkScope,
    ) -> Result<Option<SqlLinkSnapshot>, Self::Error>;
}

/// Registered provenance is not proof that an analyzer ran successfully.
pub trait AnalysisRepository {
    type Error: Error + Send + Sync + 'static;
    fn record_analysis_run(&mut self, run: &graph_domain::AnalysisRun)
    -> Result<bool, Self::Error>;
    fn analysis_run(
        &self,
        id: &str,
        project: &graph_domain::ProjectRef,
        graph_version: &str,
    ) -> Result<Option<graph_domain::AnalysisRun>, Self::Error>;
    /// A missing/mismatched registration remains unknown, never fabricated.
    fn source_analysis_run(
        &self,
        evidence: &graph_domain::SourceEvidence,
    ) -> Result<Option<graph_domain::AnalysisRun>, Self::Error> {
        self.analysis_run(
            evidence.analysis_run(),
            evidence.project(),
            evidence.graph_version(),
        )
    }
}

/// Immutable artifact metadata; registration requires a matching analysis run.
/// This port neither verifies blob bytes nor grants deletion/read authority.
pub trait ArtifactRepository {
    type Error: Error + Send + Sync + 'static;
    fn record_artifact(&mut self, artifact: &graph_domain::Artifact) -> Result<bool, Self::Error>;
    fn artifact(
        &self,
        id: &str,
        project: &graph_domain::ProjectRef,
        graph_version: &str,
    ) -> Result<Option<graph_domain::Artifact>, Self::Error>;
}

/// Point-in-time ledger view, not a lease, permission grant or live-agent status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSnapshot {
    pub spec: TaskSpec,
    pub state: graph_domain::TaskState,
}

pub trait TaskQueryRepository {
    type Error: Error + Send + Sync + 'static;
    fn task_snapshot(&self, id: &TaskId) -> Result<Option<TaskSnapshot>, Self::Error>;
}

/// Ledger observation, not a lease or verified artifact capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmittedCandidate {
    pub spec: TaskSpec,
    pub owner: WorkerId,
    pub fencing_token: i64,
    pub submission_sequence: i64,
    pub submitted_at_ms: i64,
    pub artifact: String,
}

/// Host-only access; verifier must bind complete scope and artifact content.
pub trait SubmissionQueryRepository {
    type Error: Error + Send + Sync + 'static;
    fn submitted_candidate(&self, id: &TaskId) -> Result<Option<SubmittedCandidate>, Self::Error>;
}

/// Host-only immutable policy binding. Not an execution or integration grant.
pub trait CheckPolicyRepository {
    type Error: Error + Send + Sync + 'static;
    /// First registration precedes any lease. Identical replays are harmless.
    fn register_check_policy(
        &mut self,
        task: &TaskSpec,
        policy: &graph_domain::RequiredChecks,
        now_ms: i64,
    ) -> Result<bool, Self::Error>;
    fn check_policy(
        &self,
        task: &TaskSpec,
    ) -> Result<Option<graph_domain::RequiredChecks>, Self::Error>;
}

/// The only persistence port permitted to transition a submitted task to
/// integrated. Implementations must revalidate all durable task/candidate/
/// policy/plan/receipt identities and insert the decision, event and outbox in
/// one transaction. Target verification is supplied by an authenticated host
/// adapter and is persisted for audit, but its value alone is not a credential.
pub trait VerifiedIntegrationRepository {
    type Error: Error + Send + Sync + 'static;
    /// Exact decision + target replay is false. A differing replay conflicts;
    /// a cancellation race can produce only one terminal transition.
    fn integrate_verified(
        &mut self,
        decision: &graph_domain::IntegrationDecision,
        target: &graph_domain::TargetHeadVerification,
    ) -> Result<bool, Self::Error>;
}

pub trait TaskRepository {
    type Error: Error + Send + Sync + 'static;

    fn enqueue(&mut self, task: &TaskSpec, now_ms: i64) -> Result<bool, Self::Error>;
    fn lease(
        &mut self,
        task_id: &TaskId,
        owner: &WorkerId,
        now_ms: i64,
        duration_ms: i64,
    ) -> Result<Lease, Self::Error>;
    fn submit(&mut self, lease: &Lease, artifact: &str, now_ms: i64) -> Result<(), Self::Error>;
    /// Coordinator action, not a worker lease operation. The execution host
    /// must separately reconcile cancellation with any live process.
    fn cancel(
        &mut self,
        task_id: &TaskId,
        requested_by: &WorkerId,
        reason: &str,
        now_ms: i64,
    ) -> Result<bool, Self::Error>;
    /// Legacy compatibility surface. The production store rejects caller text;
    /// a future verified integration port must bind candidate, checks and snapshot.
    fn integrate(
        &mut self,
        task_id: &TaskId,
        verification: &str,
        now_ms: i64,
    ) -> Result<(), Self::Error>;
    fn events(&self, after: i64, limit: u32) -> Result<Vec<TaskEvent>, Self::Error>;
}

/// Native dispatcher must use this policy-bound path, not the legacy ID-only lease.
/// Host supplies authenticated task/worker scope; this port does not launch work.
pub trait PolicyLeaseRepository {
    type Error: Error + Send + Sync + 'static;
    fn lease_with_policy(
        &mut self,
        task: &TaskSpec,
        policy: &graph_domain::RequiredChecks,
        owner: &WorkerId,
        now_ms: i64,
        duration_ms: i64,
    ) -> Result<Lease, Self::Error>;
}

pub struct TaskService<R> {
    repository: R,
}

impl<R> TaskService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
    pub fn into_inner(self) -> R {
        self.repository
    }
}

impl<R: TaskQueryRepository> TaskService<R> {
    /// Caller-selected snapshot filter, not caller authentication. The host must
    /// bind scope before exposing this view to a child; do not expose the raw DB.
    pub fn task_snapshot_in_scope(
        &self,
        id: &str,
        scope: &TaskSpec,
    ) -> Result<Option<TaskSnapshot>, ServiceError<R::Error>> {
        scope.validate()?;
        Ok(self.task_snapshot(id)?.filter(|snapshot| {
            snapshot.spec.project() == scope.project()
                && snapshot.spec.graph_version() == scope.graph_version()
        }))
    }

    pub fn task_snapshot(&self, id: &str) -> Result<Option<TaskSnapshot>, ServiceError<R::Error>> {
        let id = TaskId::new(id)?;
        self.repository
            .task_snapshot(&id)
            .map_err(ServiceError::Repository)
    }
}

impl<R: TaskRepository> TaskService<R> {
    pub fn enqueue(&mut self, task: TaskSpec, now_ms: i64) -> Result<bool, ServiceError<R::Error>> {
        task.validate()?;
        Ok(self
            .repository
            .enqueue(&task, now_ms)
            .map_err(ServiceError::Repository)?)
    }

    pub fn lease(
        &mut self,
        task: &str,
        owner: &str,
        now_ms: i64,
        duration_ms: i64,
    ) -> Result<Lease, ServiceError<R::Error>> {
        if duration_ms <= 0 {
            return Err(DomainError::Invalid("lease duration must be positive").into());
        }
        let task_id = TaskId::new(task)?;
        let owner = WorkerId::new(owner)?;
        Ok(self
            .repository
            .lease(&task_id, &owner, now_ms, duration_ms)
            .map_err(ServiceError::Repository)?)
    }

    pub fn submit(
        &mut self,
        lease: Lease,
        artifact: &str,
        now_ms: i64,
    ) -> Result<(), ServiceError<R::Error>> {
        if artifact.trim().is_empty() {
            return Err(DomainError::Missing("artifact").into());
        }
        Ok(self
            .repository
            .submit(&lease, artifact, now_ms)
            .map_err(ServiceError::Repository)?)
    }

    pub fn cancel(
        &mut self,
        task: &str,
        requested_by: &str,
        reason: &str,
        now_ms: i64,
    ) -> Result<bool, ServiceError<R::Error>> {
        if reason.trim().is_empty() {
            return Err(DomainError::Missing("cancellation reason").into());
        }
        let task_id = TaskId::new(task)?;
        let actor = WorkerId::new(requested_by)?;
        self.repository
            .cancel(&task_id, &actor, reason, now_ms)
            .map_err(ServiceError::Repository)
    }

    pub fn integrate(
        &mut self,
        task: &str,
        verification: &str,
        now_ms: i64,
    ) -> Result<(), ServiceError<R::Error>> {
        if verification.trim().is_empty() {
            return Err(DomainError::Missing("verification evidence").into());
        }
        let task_id = TaskId::new(task)?;
        Ok(self
            .repository
            .integrate(&task_id, verification, now_ms)
            .map_err(ServiceError::Repository)?)
    }

    pub fn events(&self, after: i64, limit: u32) -> Result<Vec<TaskEvent>, ServiceError<R::Error>> {
        Ok(self
            .repository
            .events(after, limit)
            .map_err(ServiceError::Repository)?)
    }
}

impl<R: VerifiedIntegrationRepository> TaskService<R> {
    /// Commits only a `VerifiedIntegration` produced by `verify_integration`.
    /// Callers cannot supply a task ID, string, or receipt list to bypass the
    /// candidate/output/target verifier boundary.
    pub fn integrate_verified(
        &mut self,
        verified: VerifiedIntegration,
    ) -> Result<bool, ServiceError<R::Error>> {
        self.repository
            .integrate_verified(verified.decision(), verified.target())
            .map_err(ServiceError::Repository)
    }
}

#[derive(Debug, Error)]
pub enum ServiceError<E: Error + Send + Sync + 'static> {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repository(E),
}

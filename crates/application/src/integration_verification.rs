use std::collections::HashSet;

use crate::{
    ArtifactReader, ArtifactRepository, CheckPolicyRepository, ExecutionPlanRepository,
    ExecutionReceiptRepository, ReceiptOutputError, SubmissionContentError,
    SubmissionQueryRepository, verify_receipt_outputs, verify_submitted_content,
};
use graph_domain::{CheckObservation, IntegrationDecision, TargetHeadVerification, TaskSpec};

/// Host boundary for checking the current integration target. The adapter is
/// responsible for authenticating its host and resolving the actual target;
/// a `ProjectRef` supplied by a worker is never an acceptable substitute.
pub trait TargetHeadVerifier {
    type Error: std::error::Error + Send + Sync + 'static;

    fn verify_target_head(&self, task: &TaskSpec) -> Result<TargetHeadVerification, Self::Error>;
}

#[derive(Debug)]
pub struct VerifiedIntegration {
    decision: IntegrationDecision,
    target: TargetHeadVerification,
}
impl VerifiedIntegration {
    pub fn decision(&self) -> &IntegrationDecision {
        &self.decision
    }
    pub fn target(&self) -> &TargetHeadVerification {
        &self.target
    }
    pub fn into_parts(self) -> (IntegrationDecision, TargetHeadVerification) {
        (self.decision, self.target)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IntegrationVerificationError<
    R: std::error::Error + Send + Sync + 'static,
    T: std::error::Error + Send + Sync + 'static,
> {
    #[error("integration receipt set is empty or contains an invalid run id")]
    InvalidReceiptSet,
    #[error("integration receipt run ids are not unique")]
    DuplicateReceiptRun,
    #[error("integration receipt outputs exceed the total byte budget")]
    OutputTooLarge,
    #[error("integration ledger read failed: {0}")]
    Repository(#[source] R),
    #[error("an expected execution receipt or its plan is unavailable")]
    Unavailable,
    #[error("submitted candidate content verification failed: {0}")]
    Submission(#[source] SubmissionContentError<R>),
    #[error("receipt output verification failed: {0}")]
    Receipt(#[source] ReceiptOutputError<R>),
    #[error("target-head verification failed: {0}")]
    Target(#[source] T),
    #[error("target verifier returned a decision for a different task")]
    TargetScopeMismatch,
    #[error("submitted candidate changed during integration verification")]
    CandidateChanged,
    #[error("complete integration decision is structurally invalid: {0}")]
    Decision(#[from] graph_domain::DomainError),
}

/// Read-only, bounded pre-commit verification. It verifies receipt output bytes
/// against host-registered execution plans and verifies the target through the
/// explicit host port. It intentionally does not mutate task state: callers
/// must hand the resulting decision to one atomic repository transition.
///
/// The store must re-check its ledger preconditions at commit. This function's
/// content/target observations are point-in-time and cannot turn a worker claim
/// into host authentication by themselves.
pub fn verify_integration<R, B, T>(
    repository: &R,
    reader: &B,
    target_verifier: &T,
    task: &TaskSpec,
    receipt_run_ids: &[String],
    content_max_bytes: u64,
    output_max_bytes: u64,
    verifier_version: String,
) -> Result<
    VerifiedIntegration,
    IntegrationVerificationError<<R as SubmissionQueryRepository>::Error, T::Error>,
>
where
    R: SubmissionQueryRepository
        + CheckPolicyRepository<Error = <R as SubmissionQueryRepository>::Error>
        + ArtifactRepository<Error = <R as SubmissionQueryRepository>::Error>
        + ExecutionReceiptRepository<Error = <R as SubmissionQueryRepository>::Error>
        + ExecutionPlanRepository<Error = <R as SubmissionQueryRepository>::Error>,
    B: ArtifactReader,
    T: TargetHeadVerifier,
{
    if receipt_run_ids.is_empty() || receipt_run_ids.iter().any(|id| id.trim().is_empty()) {
        return Err(IntegrationVerificationError::InvalidReceiptSet);
    }
    let mut seen = HashSet::with_capacity(receipt_run_ids.len());
    if receipt_run_ids.iter().any(|id| !seen.insert(id.as_str())) {
        return Err(IntegrationVerificationError::DuplicateReceiptRun);
    }

    let initial = verify_submitted_content(repository, reader, task, content_max_bytes)
        .map_err(IntegrationVerificationError::Submission)?;
    let policy = repository
        .check_policy(task)
        .map_err(IntegrationVerificationError::Repository)?
        .ok_or(IntegrationVerificationError::Unavailable)?;

    let mut receipts = Vec::with_capacity(receipt_run_ids.len());
    let mut remaining_output_bytes = output_max_bytes;
    for run_id in receipt_run_ids {
        let plan = repository
            .execution_plan(run_id, task)
            .map_err(IntegrationVerificationError::Repository)?
            .ok_or(IntegrationVerificationError::Unavailable)?;
        let receipt = repository
            .execution_receipt(run_id, task)
            .map_err(IntegrationVerificationError::Repository)?
            .ok_or(IntegrationVerificationError::Unavailable)?;
        let next_remaining = [receipt.stdout().as_ref(), receipt.stderr().as_ref()]
            .into_iter()
            .flatten()
            .try_fold(remaining_output_bytes, |remaining, artifact| {
                remaining.checked_sub(artifact.byte_length())
            })
            .ok_or(IntegrationVerificationError::OutputTooLarge)?;
        verify_receipt_outputs(
            repository,
            reader,
            &receipt,
            plan.binding(),
            plan.host_id(),
            plan.execution_snapshot(),
            remaining_output_bytes,
        )
        .map_err(IntegrationVerificationError::Receipt)?;
        remaining_output_bytes = next_remaining;
        receipts.push(receipt);
    }

    let observations: Vec<_> = receipts
        .iter()
        .map(|receipt| CheckObservation {
            name: receipt.binding().check_name().to_owned(),
            candidate: receipt.binding().candidate().clone(),
            outcome: receipt.completion().reported_check_outcome(),
            evidence_ref: receipt.binding().run_id().to_owned(),
        })
        .collect();
    if !policy
        .evaluate(initial.content().artifact(), &observations)
        .is_empty()
    {
        return Err(IntegrationVerificationError::Decision(
            graph_domain::DomainError::Invalid(
                "integration receipts do not exactly satisfy required checks",
            ),
        ));
    }

    let target = target_verifier
        .verify_target_head(task)
        .map_err(IntegrationVerificationError::Target)?;
    if target.task() != task {
        return Err(IntegrationVerificationError::TargetScopeMismatch);
    }
    let final_content = verify_submitted_content(repository, reader, task, content_max_bytes)
        .map_err(IntegrationVerificationError::Submission)?;
    if final_content.candidate() != initial.candidate() {
        return Err(IntegrationVerificationError::CandidateChanged);
    }
    let decision = IntegrationDecision::new(
        task.clone(),
        initial.content().artifact().clone(),
        policy,
        receipts,
        target.observed_at_ms(),
        verifier_version,
    )
    .map_err(IntegrationVerificationError::Decision)?;
    Ok(VerifiedIntegration { decision, target })
}

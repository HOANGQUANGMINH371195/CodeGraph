use crate::{
    ArtifactReader, ArtifactRepository, ArtifactVerificationError, CheckBindingError,
    CheckPolicyRepository, SubmissionQueryRepository, VerifiedArtifactContent,
    reconcile_check_binding, verify_artifact,
};
use graph_domain::{CheckRunBinding, ExecutionReceipt, ProjectRef};

/// Bytes observed now. Host, process status and snapshot remain producer claims.
/// This result is not a verified check, candidate verification or integration grant.
#[derive(Debug)]
pub struct ReceiptOutputObservation {
    receipt: ExecutionReceipt,
    stdout: Option<VerifiedArtifactContent>,
    stderr: Option<VerifiedArtifactContent>,
}
impl ReceiptOutputObservation {
    pub fn receipt(&self) -> &ExecutionReceipt {
        &self.receipt
    }
    pub fn stdout(&self) -> Option<&VerifiedArtifactContent> {
        self.stdout.as_ref()
    }
    pub fn stderr(&self) -> Option<&VerifiedArtifactContent> {
        self.stderr.as_ref()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiptOutputError<E: std::error::Error + Send + Sync + 'static> {
    #[error("receipt differs from host-selected binding, host or execution snapshot")]
    ExpectationMismatch,
    #[error("receipt outputs exceed host total byte budget")]
    TooLarge,
    #[error("check binding reconciliation failed: {0}")]
    Binding(#[source] CheckBindingError<E>),
    #[error("output repository failed: {0}")]
    Repository(#[source] E),
    #[error("output descriptor is not exactly registered")]
    OutputMismatch,
    #[error("output content verification failed: {0}")]
    Content(#[source] ArtifactVerificationError),
    #[error("submission changed while reading outputs")]
    Changed,
}

/// Expected values must come from the host, not be copied from untrusted input.
/// Output budget excludes at most one sentinel byte per present stream. Blocking
/// reader time must be bounded by the supervisor. No writes or post-return lock.
#[allow(clippy::too_many_arguments)]
pub fn verify_receipt_outputs<R, B>(
    repository: &R,
    reader: &B,
    receipt: &ExecutionReceipt,
    expected: &CheckRunBinding,
    expected_host: &str,
    expected_snapshot: &ProjectRef,
    max_total_bytes: u64,
) -> Result<ReceiptOutputObservation, ReceiptOutputError<<R as SubmissionQueryRepository>::Error>>
where
    R: SubmissionQueryRepository
        + CheckPolicyRepository<Error = <R as SubmissionQueryRepository>::Error>
        + ArtifactRepository<Error = <R as SubmissionQueryRepository>::Error>,
    B: ArtifactReader,
{
    if receipt.binding() != expected
        || receipt.host_id() != expected_host
        || receipt.execution_snapshot() != expected_snapshot
    {
        return Err(ReceiptOutputError::ExpectationMismatch);
    }
    let outputs = [receipt.stdout().as_ref(), receipt.stderr().as_ref()];
    let total = outputs
        .iter()
        .flatten()
        .try_fold(0_u64, |sum, artifact| {
            sum.checked_add(artifact.byte_length())
        })
        .ok_or(ReceiptOutputError::TooLarge)?;
    if total > max_total_bytes {
        return Err(ReceiptOutputError::TooLarge);
    }
    let before =
        reconcile_check_binding(repository, expected).map_err(ReceiptOutputError::Binding)?;
    // Validate both registrations before opening either stream.
    for artifact in outputs.into_iter().flatten() {
        if repository
            .artifact(
                artifact.id(),
                expected_snapshot,
                expected.task().graph_version(),
            )
            .map_err(ReceiptOutputError::Repository)?
            .as_ref()
            != Some(artifact)
        {
            return Err(ReceiptOutputError::OutputMismatch);
        }
    }
    let stdout = receipt
        .stdout()
        .as_ref()
        .map(|artifact| verify_artifact(reader, artifact, max_total_bytes))
        .transpose()
        .map_err(ReceiptOutputError::Content)?;
    let stderr = receipt
        .stderr()
        .as_ref()
        .map(|artifact| verify_artifact(reader, artifact, max_total_bytes))
        .transpose()
        .map_err(ReceiptOutputError::Content)?;
    if repository
        .submitted_candidate(expected.task().id())
        .map_err(ReceiptOutputError::Repository)?
        .as_ref()
        != Some(&before)
    {
        return Err(ReceiptOutputError::Changed);
    }
    Ok(ReceiptOutputObservation {
        receipt: receipt.clone(),
        stdout,
        stderr,
    })
}

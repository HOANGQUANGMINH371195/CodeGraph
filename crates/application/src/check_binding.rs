use crate::{
    ArtifactRepository, CheckPolicyRepository, SubmissionQueryRepository, SubmittedCandidate,
};
use graph_domain::CheckRunBinding;

#[derive(Debug, thiserror::Error)]
pub enum CheckBindingError<E: std::error::Error + Send + Sync + 'static> {
    #[error("check binding repository failed: {0}")]
    Repository(#[source] E),
    #[error("submitted candidate is unavailable")]
    Unavailable,
    #[error("binding does not match the current submitted candidate")]
    SubmissionMismatch,
    #[error("binding policy is missing or differs from registered policy")]
    PolicyMismatch,
    #[error("binding artifact is missing or differs from registered descriptor")]
    CandidateMismatch,
    #[error("submission changed during reconciliation")]
    Changed,
}

/// Metadata observation only: no command approval, original-expiry attestation,
/// blob read, execution grant, or protection against races after return.
pub fn reconcile_check_binding<R>(
    repository: &R,
    binding: &CheckRunBinding,
) -> Result<SubmittedCandidate, CheckBindingError<<R as SubmissionQueryRepository>::Error>>
where
    R: SubmissionQueryRepository
        + CheckPolicyRepository<Error = <R as SubmissionQueryRepository>::Error>
        + ArtifactRepository<Error = <R as SubmissionQueryRepository>::Error>,
{
    let submitted = repository
        .submitted_candidate(binding.task().id())
        .map_err(CheckBindingError::Repository)?
        .ok_or(CheckBindingError::Unavailable)?;
    if submitted.spec != *binding.task()
        || submitted.owner != *binding.origin_lease().owner()
        || submitted.fencing_token != binding.origin_lease().fencing_token()
        || submitted.submission_sequence != binding.submission_sequence()
        || submitted.artifact != binding.candidate().id()
        || submitted.submitted_at_ms < 0
    {
        return Err(CheckBindingError::SubmissionMismatch);
    }
    if repository
        .check_policy(binding.task())
        .map_err(CheckBindingError::Repository)?
        .as_ref()
        != Some(binding.policy())
    {
        return Err(CheckBindingError::PolicyMismatch);
    }
    if repository
        .artifact(
            binding.candidate().id(),
            binding.task().project(),
            binding.task().graph_version(),
        )
        .map_err(CheckBindingError::Repository)?
        .as_ref()
        != Some(binding.candidate())
    {
        return Err(CheckBindingError::CandidateMismatch);
    }
    if repository
        .submitted_candidate(binding.task().id())
        .map_err(CheckBindingError::Repository)?
        .as_ref()
        != Some(&submitted)
    {
        return Err(CheckBindingError::Changed);
    }
    Ok(submitted)
}

use crate::{
    ArtifactReader, ArtifactRepository, ArtifactVerificationError, SubmissionQueryRepository,
    SubmittedCandidate, VerifiedArtifactContent, verify_artifact,
};
use graph_domain::TaskSpec;

/// Point-in-time content observation, NOT integration authority. The host must
/// still verify checks/target and atomically revalidate the candidate at commit.
#[derive(Debug)]
pub struct VerifiedSubmittedContent {
    candidate: SubmittedCandidate,
    content: VerifiedArtifactContent,
}

impl VerifiedSubmittedContent {
    pub fn candidate(&self) -> &SubmittedCandidate {
        &self.candidate
    }

    pub fn content(&self) -> &VerifiedArtifactContent {
        &self.content
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SubmissionContentError<E: std::error::Error + Send + Sync + 'static> {
    #[error("submission repository failed: {0}")]
    Repository(#[source] E),
    #[error("no submitted candidate")]
    Unavailable,
    #[error("candidate does not match expected task or has invalid binding")]
    ScopeMismatch,
    #[error("submitted artifact is not registered in the expected snapshot")]
    ArtifactMissing,
    #[error("submission changed during content verification")]
    Changed,
    #[error("submitted content verification failed: {0}")]
    Content(#[source] ArtifactVerificationError),
}

/// Read-only verification. Expected scope is host supplied, not authentication.
/// Rechecking detects changes during reads, not races after this function returns.
pub fn verify_submitted_content<R, B>(
    repository: &R,
    reader: &B,
    expected: &TaskSpec,
    max_bytes: u64,
) -> Result<VerifiedSubmittedContent, SubmissionContentError<<R as SubmissionQueryRepository>::Error>>
where
    R: SubmissionQueryRepository
        + ArtifactRepository<Error = <R as SubmissionQueryRepository>::Error>,
    B: ArtifactReader,
{
    let candidate = repository
        .submitted_candidate(expected.id())
        .map_err(SubmissionContentError::Repository)?
        .ok_or(SubmissionContentError::Unavailable)?;
    if candidate.spec != *expected
        || candidate.fencing_token <= 0
        || candidate.submission_sequence <= 0
        || candidate.submitted_at_ms < 0
        || candidate.artifact.trim().is_empty()
    {
        return Err(SubmissionContentError::ScopeMismatch);
    }
    let artifact = repository
        .artifact(
            &candidate.artifact,
            expected.project(),
            expected.graph_version(),
        )
        .map_err(SubmissionContentError::Repository)?
        .ok_or(SubmissionContentError::ArtifactMissing)?;
    // Independently check the returned descriptor, even if an adapter ignores filters.
    if artifact.id() != candidate.artifact
        || artifact.project() != expected.project()
        || artifact.graph_version() != expected.graph_version()
    {
        return Err(SubmissionContentError::ScopeMismatch);
    }
    let content =
        verify_artifact(reader, &artifact, max_bytes).map_err(SubmissionContentError::Content)?;
    if repository
        .submitted_candidate(expected.id())
        .map_err(SubmissionContentError::Repository)?
        .as_ref()
        != Some(&candidate)
    {
        return Err(SubmissionContentError::Changed);
    }
    Ok(VerifiedSubmittedContent { candidate, content })
}

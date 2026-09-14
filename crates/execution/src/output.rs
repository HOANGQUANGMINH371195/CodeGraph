use crate::FixtureRun;
use graph_application::{
    ArtifactIngestion, ArtifactIngestionError, ArtifactRepository, ArtifactWriter, ingest_artifact,
};
use graph_domain::{Artifact, ArtifactProtection};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct OutputPublication {
    pub stdout: ArtifactIngestion,
    pub stderr: ArtifactIngestion,
}

#[derive(Debug, thiserror::Error)]
pub enum OutputPublicationError<E: std::error::Error + Send + Sync + 'static> {
    #[error("output descriptors or byte budget rejected before publication")]
    Preflight,
    #[error("stdout publication failed: {0}")]
    Stdout(#[source] ArtifactIngestionError<E>),
    #[error("stdout published; stderr publication failed; retry same descriptors: {source}")]
    Stderr {
        stdout: ArtifactIngestion,
        #[source]
        source: ArtifactIngestionError<E>,
    },
}

/// Publish retained bytes without changing completion or consuming child ownership.
/// Descriptors are host-selected claims: matching content does not attest run,
/// snapshot, command, redaction, cleanup, launch approval or receipt authority.
/// This is replayable metadata-first ingestion, not an atomic two-stream commit.
pub fn publish_fixture_outputs<R: ArtifactRepository, W: ArtifactWriter>(
    run: &FixtureRun,
    stdout: &Artifact,
    stderr: &Artifact,
    repository: &mut R,
    writer: &W,
    max_total_bytes: u64,
) -> Result<OutputPublication, OutputPublicationError<R::Error>> {
    if stdout.id() == stderr.id()
        || stdout.project() != stderr.project()
        || stdout.graph_version() != stderr.graph_version()
        || stdout.analysis_run() != stderr.analysis_run()
        || stdout
            .byte_length()
            .checked_add(stderr.byte_length())
            .is_none_or(|n| n > max_total_bytes)
    {
        return Err(OutputPublicationError::Preflight);
    }
    // Validate both streams before the first metadata/blob side effect.
    for (artifact, bytes, kind) in [
        (stdout, &run.stdout, "stdout"),
        (stderr, &run.stderr, "stderr"),
    ] {
        if artifact.kind() != kind
            || artifact.declared_protection() != ArtifactProtection::Unreviewed
            || artifact.byte_length() != bytes.len() as u64
            || artifact.content_sha256() != format!("{:x}", Sha256::digest(bytes))
        {
            return Err(OutputPublicationError::Preflight);
        }
    }
    let stdout = ingest_artifact(
        repository,
        writer,
        stdout,
        &mut run.stdout.as_slice(),
        stdout.byte_length(),
    )
    .map_err(OutputPublicationError::Stdout)?;
    match ingest_artifact(
        repository,
        writer,
        stderr,
        &mut run.stderr.as_slice(),
        stderr.byte_length(),
    ) {
        Ok(stderr) => Ok(OutputPublication { stdout, stderr }),
        Err(source) => Err(OutputPublicationError::Stderr { stdout, source }),
    }
}

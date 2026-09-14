use graph_domain::Artifact;
use sha2::{Digest, Sha256};
use std::io::{self, Read};

/// Adapter must enforce host-selected snapshot and expose only regular blobs.
pub trait ArtifactReader {
    fn open_artifact(&self, artifact: &Artifact) -> io::Result<Box<dyn Read>>;
}

pub const ARTIFACT_CONTENT_VERIFIER_VERSION: &str = "sha256-length-v1";

/// Historical ledger data, NOT a fresh VerifiedArtifactContent capability.
/// Time/root binding are host supplied; no assertion of present blob integrity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactContentObservation {
    pub id: String,
    pub artifact: Artifact,
    pub observed_at_ms: i64,
    pub verifier_version: String,
}

pub trait ArtifactObservationRepository {
    type Error: std::error::Error + Send + Sync + 'static;
    /// Only a successful verifier result may be written via this port.
    /// Exact id/time/content replay is idempotent; new observations need new IDs.
    fn record_artifact_observation(
        &mut self,
        id: &str,
        content: &VerifiedArtifactContent,
        observed_at_ms: i64,
    ) -> Result<bool, Self::Error>;
    fn artifact_observation(
        &self,
        id: &str,
        project: &graph_domain::ProjectRef,
        graph_version: &str,
    ) -> Result<Option<ArtifactContentObservation>, Self::Error>;
}

/// Publication must not replace an existing blob. The host owns root selection.
pub trait ArtifactWriter: ArtifactReader {
    fn write_artifact(
        &self,
        artifact: &Artifact,
        input: &mut dyn Read,
        max_bytes: u64,
    ) -> Result<bool, ArtifactVerificationError>;
}

#[derive(Debug)]
pub struct ArtifactIngestion {
    pub metadata_inserted: bool,
    pub blob_inserted: bool,
    pub content: VerifiedArtifactContent,
}

#[derive(Debug, thiserror::Error)]
pub enum ArtifactIngestionError<E: std::error::Error + Send + Sync + 'static> {
    #[error("artifact exceeds the configured byte budget; nothing registered")]
    TooLarge,
    #[error("artifact registration failed before blob write: {0}")]
    Registration(#[source] E),
    #[error(
        "artifact metadata is registered but blob ingestion/verification failed; retry the same descriptor: {source}"
    )]
    Content {
        metadata_inserted: bool,
        #[source]
        source: ArtifactVerificationError,
    },
}

/// Metadata-first, replayable orchestration, not a cross-resource transaction.
/// Registration enforces run linkage and conflicts before consuming input.
/// A failed write leaves unverified metadata; retry the identical descriptor.
/// Success re-reads published bytes, independently of writer success claims.
pub fn ingest_artifact<R: crate::ArtifactRepository, W: ArtifactWriter>(
    repository: &mut R,
    writer: &W,
    artifact: &Artifact,
    input: &mut dyn Read,
    max_bytes: u64,
) -> Result<ArtifactIngestion, ArtifactIngestionError<R::Error>> {
    if artifact.byte_length() > max_bytes {
        return Err(ArtifactIngestionError::TooLarge);
    }
    let metadata_inserted = repository
        .record_artifact(artifact)
        .map_err(ArtifactIngestionError::Registration)?;
    let content_error = |source| ArtifactIngestionError::Content {
        metadata_inserted,
        source,
    };
    let blob_inserted = writer
        .write_artifact(artifact, input, max_bytes)
        .map_err(content_error)?;
    let content = verify_artifact(writer, artifact, max_bytes).map_err(content_error)?;
    Ok(ArtifactIngestion {
        metadata_inserted,
        blob_inserted,
        content,
    })
}

/// Receipt for bytes read now, not continued filesystem immutability, redaction,
/// execution attestation or a semantic acceptance decision. Contains no blob bytes.
#[derive(Debug)]
pub struct VerifiedArtifactContent {
    artifact: Artifact,
}
impl VerifiedArtifactContent {
    pub fn artifact(&self) -> &Artifact {
        &self.artifact
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ArtifactVerificationError {
    #[error("artifact exceeds the configured byte budget")]
    TooLarge,
    #[error("artifact byte length differs from its descriptor")]
    LengthMismatch,
    #[error("artifact SHA-256 differs from its descriptor")]
    HashMismatch,
    #[error("artifact read failed: {0}")]
    Read(#[from] io::Error),
}

/// Streams at most declared length + one sentinel byte using fixed memory.
/// A byte budget does not bound blocking-reader time; the execution supervisor
/// must enforce deadlines/cancellation for adapters that may block.
pub fn verify_artifact<R: ArtifactReader>(
    reader: &R,
    artifact: &Artifact,
    max_bytes: u64,
) -> Result<VerifiedArtifactContent, ArtifactVerificationError> {
    if artifact.byte_length() > max_bytes {
        return Err(ArtifactVerificationError::TooLarge);
    }
    let mut input = reader
        .open_artifact(artifact)?
        .take(artifact.byte_length() + 1);
    let mut digest = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        let count = match input.read(&mut buffer) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            other => other?,
        };
        if count == 0 {
            break;
        }
        length += count as u64;
        if length > artifact.byte_length() {
            return Err(ArtifactVerificationError::LengthMismatch);
        }
        digest.update(&buffer[..count]);
    }
    if length != artifact.byte_length() {
        return Err(ArtifactVerificationError::LengthMismatch);
    }
    if format!("{:x}", digest.finalize()) != artifact.content_sha256() {
        return Err(ArtifactVerificationError::HashMismatch);
    }
    Ok(VerifiedArtifactContent {
        artifact: artifact.clone(),
    })
}

#[cfg(test)]
mod ingestion_tests {
    use super::*;
    use graph_domain::{ArtifactProtection, ArtifactRetention, ProjectRef};

    #[test]
    fn writer_success_is_independently_checked_before_returning_receipt() {
        struct Repository;
        impl crate::ArtifactRepository for Repository {
            type Error = io::Error;
            fn record_artifact(&mut self, _: &Artifact) -> io::Result<bool> {
                Ok(true)
            }
            fn artifact(&self, _: &str, _: &ProjectRef, _: &str) -> io::Result<Option<Artifact>> {
                Ok(None)
            }
        }
        struct LyingWriter;
        impl ArtifactReader for LyingWriter {
            fn open_artifact(&self, _: &Artifact) -> io::Result<Box<dyn Read>> {
                Ok(Box::new(io::Cursor::new(b"bad")))
            }
        }
        impl ArtifactWriter for LyingWriter {
            fn write_artifact(
                &self,
                _: &Artifact,
                _: &mut dyn Read,
                _: u64,
            ) -> Result<bool, ArtifactVerificationError> {
                Ok(true)
            }
        }
        let artifact = Artifact::new(
            "a1".into(),
            ProjectRef {
                repository_id: "repo".into(),
                worktree_id: "w".into(),
                git_head: "h".into(),
                working_tree_fingerprint: "s".into(),
                config_hash: "c".into(),
                ignore_policy_version: "1".into(),
            },
            "g1".into(),
            "r1".into(),
            format!("{:x}", Sha256::digest(b"abc")),
            3,
            "stdout".into(),
            ArtifactRetention::Evidence,
            ArtifactProtection::Unreviewed,
        )
        .unwrap();
        assert!(matches!(
            ingest_artifact(
                &mut Repository,
                &LyingWriter,
                &artifact,
                &mut b"abc".as_slice(),
                3
            ),
            Err(ArtifactIngestionError::Content {
                metadata_inserted: true,
                source: ArtifactVerificationError::HashMismatch
            })
        ));
    }
}

//! Filesystem source adapter. A directory capability bounds all path lookups.
//! Root-to-snapshot binding is supplied by the trusted host; it is not inferred
//! from an agent-provided path or from a claim's hash.

use std::{
    io::{self, Read},
    path::Path,
};

use cap_std::fs::{Dir, OpenOptions};
use graph_application::SourceReader;
use graph_domain::{DomainError, ProjectRef, SourceEvidence, TargetHeadVerification, TaskSpec};
mod artifact;
pub use artifact::DirectoryArtifacts;
mod snapshot;
pub use snapshot::{GitSnapshotAuthority, GitSnapshotAuthorityError, MaterializedSource};

/// Host-owned target verifier backed by the same Git authority as source
/// materialization. The task's ProjectRef is compared, never used to select
/// the repository or fingerprint policy.
pub struct GitTargetHeadVerifier {
    authority: GitSnapshotAuthority,
    verifier_version: String,
}

impl GitTargetHeadVerifier {
    pub fn new(
        authority: GitSnapshotAuthority,
        verifier_version: String,
    ) -> Result<Self, GitSnapshotAuthorityError> {
        if verifier_version.trim().is_empty() || verifier_version.chars().any(char::is_control) {
            return Err(GitSnapshotAuthorityError::InvalidMetadata(
                "invalid target verifier metadata",
            ));
        }
        Ok(Self {
            authority,
            verifier_version,
        })
    }

    pub fn authority(&self) -> &GitSnapshotAuthority {
        &self.authority
    }
}

impl graph_application::TargetHeadVerifier for GitTargetHeadVerifier {
    type Error = GitSnapshotAuthorityError;

    fn verify_target_head(&self, task: &TaskSpec) -> Result<TargetHeadVerification, Self::Error> {
        let observed = self.authority.current_project()?;
        if &observed != task.project() {
            return Err(GitSnapshotAuthorityError::ProjectMismatch);
        }
        let observed_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .ok_or(GitSnapshotAuthorityError::InvalidMetadata(
                "target clock unavailable",
            ))?;
        TargetHeadVerification::new(
            task.clone(),
            observed,
            observed_at_ms,
            self.verifier_version.clone(),
        )
        .map_err(|_| GitSnapshotAuthorityError::InvalidMetadata("invalid target observation"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceReadError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("source citation does not match the configured project and graph snapshot")]
    ScopeMismatch,
    #[error("source must be a regular file")]
    NotRegularFile,
    #[error("source file exceeds the read byte budget")]
    TooLarge,
    #[error("source filesystem access failed: {0}")]
    Io(#[from] io::Error),
}

pub struct DirectorySource {
    directory: Dir,
    project: ProjectRef,
    graph_version: String,
}

impl DirectorySource {
    /// The host explicitly grants access to this root and supplies its snapshot
    /// binding. This constructor does not verify a whole-tree Git fingerprint.
    pub fn open(
        root: impl AsRef<Path>,
        project: ProjectRef,
        graph_version: String,
    ) -> Result<Self, SourceReadError> {
        project.validate()?;
        if graph_version.trim().is_empty() {
            return Err(DomainError::Missing("graph version").into());
        }
        Ok(Self {
            directory: Dir::open_ambient_dir(root, cap_std::ambient_authority())?,
            project,
            graph_version,
        })
    }
}

impl SourceReader for DirectorySource {
    type Error = SourceReadError;

    fn read_source(&self, evidence: &SourceEvidence, limit: usize) -> Result<Vec<u8>, Self::Error> {
        if evidence.project() != &self.project || evidence.graph_version() != self.graph_version {
            return Err(SourceReadError::ScopeMismatch);
        }
        if limit == 0 || limit > 64 * 1024 * 1024 {
            return Err(DomainError::Invalid("source read limit must be 1 byte to 64 MiB").into());
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            // Opening a FIFO must not block before we can inspect its type.
            options.custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32);
        }
        let file = self.directory.open_with(evidence.path(), &options)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(SourceReadError::NotRegularFile);
        }
        if metadata.len() > limit as u64 {
            return Err(SourceReadError::TooLarge);
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(SourceReadError::TooLarge);
        }
        Ok(bytes)
    }
}

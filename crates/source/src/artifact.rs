use cap_std::fs::{Dir, OpenOptions};
use graph_application::{
    ArtifactReader, ArtifactVerificationError, ArtifactWriter, verify_artifact,
};
use graph_domain::{Artifact, ProjectRef};
use std::{
    io::{self, Read, Seek, SeekFrom},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

/// Blob directory: filenames are lowercase SHA-256 values, never
/// arbitrary wire paths. The host must select a private per-snapshot root.
pub struct DirectoryArtifacts {
    directory: Dir,
    project: ProjectRef,
    graph_version: String,
}
impl DirectoryArtifacts {
    /// Publish verified bytes without replacing an existing blob. Returns false
    /// for an already present, verified blob. Root must be private to the host;
    /// this is not protection against another process mutating that directory.
    /// No ledger entry or redaction/execution attestation is created.
    pub fn ingest(
        &self,
        artifact: &Artifact,
        input: impl Read,
        max_bytes: u64,
    ) -> Result<bool, ArtifactVerificationError> {
        if artifact.project() != &self.project || artifact.graph_version() != self.graph_version {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "artifact snapshot mismatch",
            )
            .into());
        }
        if artifact.byte_length() > max_bytes {
            return Err(ArtifactVerificationError::TooLarge);
        }
        let (name, mut file) = self.create_staging_file()?;
        let mut staging = StagingCleanup {
            directory: &self.directory,
            name,
            armed: true,
        };
        let length = io::copy(&mut input.take(artifact.byte_length() + 1), &mut file)?;
        if length != artifact.byte_length() {
            return Err(ArtifactVerificationError::LengthMismatch);
        }
        file.seek(SeekFrom::Start(0))?;
        verify_artifact(&StagedReader(&file), artifact, max_bytes)?;
        file.sync_all()?;
        let inserted = match self.directory.hard_link(
            &staging.name,
            &self.directory,
            artifact.content_sha256(),
        ) {
            Ok(()) => true,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                // Never repair/overwrite an existing bad blob implicitly.
                verify_artifact(self, artifact, max_bytes)?;
                false
            }
            Err(error) => return Err(error.into()),
        };
        drop(file);
        self.directory.remove_file(&staging.name)?;
        staging.armed = false;
        // A failure here may follow successful publication; caller can retry
        // safely. No claim of cross-platform crash durability without testing.
        // Dir may hold an O_PATH handle on Linux, which cannot be fsynced.
        // Reopen the same capability root for reading to sync its entries.
        self.directory.open(".")?.sync_all()?;
        Ok(inserted)
    }

    fn create_staging_file(&self) -> io::Result<(String, cap_std::fs::File)> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..64 {
            let name = format!(
                ".ingest-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            let mut options = OpenOptions::new();
            options.read(true).write(true).create_new(true);
            #[cfg(unix)]
            {
                use cap_std::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match self.directory.open_with(&name, &options) {
                Ok(file) => return Ok((name, file)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "staging filename collision budget exhausted",
        ))
    }

    pub fn open(
        root: impl AsRef<Path>,
        project: ProjectRef,
        graph_version: String,
    ) -> io::Result<Self> {
        project
            .validate()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        if graph_version.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "graph version is required",
            ));
        }
        Ok(Self {
            directory: Dir::open_ambient_dir(root, cap_std::ambient_authority())?,
            project,
            graph_version,
        })
    }
}

struct StagedReader<'a>(&'a cap_std::fs::File);
impl ArtifactReader for StagedReader<'_> {
    fn open_artifact(&self, _: &Artifact) -> io::Result<Box<dyn Read>> {
        Ok(Box::new(self.0.try_clone()?))
    }
}

/// Only removes the exclusive staging file owned by this operation. Abrupt
/// process death can leave staging files; recovery/GC must reconcile them.
struct StagingCleanup<'a> {
    directory: &'a Dir,
    name: String,
    armed: bool,
}
impl Drop for StagingCleanup<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.directory.remove_file(&self.name);
        }
    }
}
impl ArtifactReader for DirectoryArtifacts {
    fn open_artifact(&self, artifact: &Artifact) -> io::Result<Box<dyn Read>> {
        if artifact.project() != &self.project || artifact.graph_version() != self.graph_version {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "artifact snapshot mismatch",
            ));
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32);
        }
        let file = self
            .directory
            .open_with(artifact.content_sha256(), &options)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "artifact must be a regular file",
            ));
        }
        Ok(Box::new(file))
    }
}

impl ArtifactWriter for DirectoryArtifacts {
    fn write_artifact(
        &self,
        artifact: &Artifact,
        input: &mut dyn Read,
        max_bytes: u64,
    ) -> Result<bool, ArtifactVerificationError> {
        self.ingest(artifact, input, max_bytes)
    }
}

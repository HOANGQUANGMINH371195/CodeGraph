use crate::{DomainError, ProjectRef, validate_text};

/// Requested lifecycle class; actual deletion requires ledger reference checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactRetention {
    Ephemeral,
    Evidence,
    UserPinned,
}

/// Producer-declared protection, not verified redaction or encryption evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactProtection {
    Unreviewed,
    Redacted,
    Encrypted,
    RedactedAndEncrypted,
}

/// Immutable metadata for exact stored bytes. No host path or deletion authority.
/// A valid descriptor does not prove that bytes exist or an analyzer executed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    id: String,
    project: ProjectRef,
    graph_version: String,
    analysis_run: String,
    content_sha256: String,
    byte_length: u64,
    kind: String,
    retention: ArtifactRetention,
    declared_protection: ArtifactProtection,
}

impl Artifact {
    /// Constructs an artifact descriptor without reading or authenticating its bytes.
    ///
    /// # Errors
    /// Returns an error for invalid project or descriptor text, a digest that is
    /// not 64 lowercase hexadecimal characters, or a length exceeding `i64::MAX`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        project: ProjectRef,
        graph_version: String,
        analysis_run: String,
        content_sha256: String,
        byte_length: u64,
        kind: String,
        retention: ArtifactRetention,
        declared_protection: ArtifactProtection,
    ) -> Result<Self, DomainError> {
        project.validate()?;
        for (name, value) in [
            ("artifact id", &id),
            ("graph version", &graph_version),
            ("analysis run", &analysis_run),
            ("artifact kind", &kind),
        ] {
            validate_text(name, value)?;
        }
        if content_sha256.len() != 64
            || !content_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(DomainError::Invalid(
                "artifact hash must be lowercase SHA-256 hex",
            ));
        }
        // SQLite INTEGER is signed; avoid lossy conversion at the store boundary.
        if byte_length > i64::MAX as u64 {
            return Err(DomainError::Invalid("artifact length exceeds ledger range"));
        }
        Ok(Self {
            id,
            project,
            graph_version,
            analysis_run,
            content_sha256,
            byte_length,
            kind,
            retention,
            declared_protection,
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn project(&self) -> &ProjectRef {
        &self.project
    }
    #[must_use]
    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }
    #[must_use]
    pub fn analysis_run(&self) -> &str {
        &self.analysis_run
    }
    #[must_use]
    pub fn content_sha256(&self) -> &str {
        &self.content_sha256
    }
    #[must_use]
    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
    #[must_use]
    pub fn retention(&self) -> ArtifactRetention {
        self.retention
    }
    #[must_use]
    pub fn declared_protection(&self) -> ArtifactProtection {
        self.declared_protection
    }
}

use crate::{DomainError, ProjectRef, validate_text};

/// Immutable provenance descriptor. Registration is not execution attestation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisRun {
    id: String,
    project: ProjectRef,
    graph_version: String,
    analyzer: String,
    analyzer_version: String,
    configuration_sha256: String,
    input_manifest_sha256: String,
}

impl AnalysisRun {
    #[allow(clippy::too_many_arguments)]
    /// Constructs a validated immutable analysis registration.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::Invalid`] when textual identity fields are empty
    /// or contain control characters, or when either digest is not lowercase
    /// SHA-256 hexadecimal.
    pub fn new(
        id: String,
        project: ProjectRef,
        graph_version: String,
        analyzer: String,
        analyzer_version: String,
        configuration_sha256: String,
        input_manifest_sha256: String,
    ) -> Result<Self, DomainError> {
        project.validate()?;
        for (name, value) in [
            ("analysis run", &id),
            ("graph version", &graph_version),
            ("analyzer", &analyzer),
            ("analyzer version", &analyzer_version),
        ] {
            validate_text(name, value)?;
        }
        for hash in [&configuration_sha256, &input_manifest_sha256] {
            if hash.len() != 64
                || !hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(DomainError::Invalid(
                    "analysis digest must be lowercase SHA-256 hex",
                ));
            }
        }
        Ok(Self {
            id,
            project,
            graph_version,
            analyzer,
            analyzer_version,
            configuration_sha256,
            input_manifest_sha256,
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
    pub fn analyzer(&self) -> &str {
        &self.analyzer
    }
    #[must_use]
    pub fn analyzer_version(&self) -> &str {
        &self.analyzer_version
    }
    #[must_use]
    pub fn configuration_sha256(&self) -> &str {
        &self.configuration_sha256
    }
    #[must_use]
    pub fn input_manifest_sha256(&self) -> &str {
        &self.input_manifest_sha256
    }
}

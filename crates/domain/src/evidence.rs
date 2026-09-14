use crate::{DomainError, ProjectRef, validate_text};

/// An immutable source citation. Recording this value does not verify that
/// its bytes exist or that a claimed relationship follows from them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEvidence {
    id: String,
    project: ProjectRef,
    graph_version: String,
    path: String,
    content_sha256: String,
    start_line: u32,
    end_line: u32,
    analysis_run: String,
}

impl SourceEvidence {
    /// Lines are inclusive and one-based; the hash covers the entire source
    /// file at this snapshot, not just the cited slice.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        project: ProjectRef,
        graph_version: String,
        path: String,
        content_sha256: String,
        start_line: u32,
        end_line: u32,
        analysis_run: String,
    ) -> Result<Self, DomainError> {
        project.validate()?;
        for (name, value) in [
            ("evidence id", &id),
            ("graph version", &graph_version),
            ("source path", &path),
            ("analysis run", &analysis_run),
        ] {
            validate_text(name, value)?;
        }
        // A portable repository-relative logical path, never a host path.
        // Filesystem adapters must additionally reject symlink escapes.
        if path.contains(['\\', ':', '\0'])
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(DomainError::Invalid(
                "source path must be repository relative",
            ));
        }
        if content_sha256.len() != 64
            || !content_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(DomainError::Invalid(
                "source hash must be lowercase SHA-256 hex",
            ));
        }
        if start_line == 0 || end_line < start_line {
            return Err(DomainError::Invalid("source line range"));
        }
        Ok(Self {
            id,
            project,
            graph_version,
            path,
            content_sha256,
            start_line,
            end_line,
            analysis_run,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn project(&self) -> &ProjectRef {
        &self.project
    }
    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn content_sha256(&self) -> &str {
        &self.content_sha256
    }
    pub fn start_line(&self) -> u32 {
        self.start_line
    }
    pub fn end_line(&self) -> u32 {
        self.end_line
    }
    pub fn analysis_run(&self) -> &str {
        &self.analysis_run
    }
}

//! Structurally validated code-to-SQL candidate batches. These records prove
//! neither that a code path executes nor that SQL relation syntax resolves to a
//! physical table or database instance.

use std::collections::{HashMap, HashSet};

use crate::{DomainError, ProjectRef, SourceEvidence};

const MAX_LINKS: usize = 10_000;
const MAX_STATEMENTS_PER_LINK: usize = 256;

/// Replacement owner for candidate links originating in one code file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlLinkScope {
    adapter: String,
    project: ProjectRef,
    graph_version: String,
    code_path: String,
}

impl SqlLinkScope {
    pub fn new(adapter: String, code: &SourceEvidence) -> Result<Self, DomainError> {
        text(&adapter, 128)?;
        full_file(code, "SQL-link code source")?;
        Ok(Self {
            adapter,
            project: code.project().clone(),
            graph_version: code.graph_version().into(),
            code_path: code.path().into(),
        })
    }

    #[must_use]
    pub fn adapter(&self) -> &str {
        &self.adapter
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
    pub fn code_path(&self) -> &str {
        &self.code_path
    }
}

/// Public offset unit. JavaScript/TypeScript extractor locations are UTF-16
/// code units, zero-based and half-open; byte offsets are intentionally not
/// accepted here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CoordinateEncoding {
    Utf16CodeUnit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SqlOperation {
    Query,
    Insert,
    Update,
    Delete,
    CreateTable,
    Begin,
    Commit,
    Rollback,
    Other,
}

/// Offline SQL syntax observation. `relations` are parser names, not catalog
/// identities or authorization/runtime facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlStatement {
    pub ordinal: u16,
    pub operation: SqlOperation,
    pub relations: Vec<String>,
}

/// One independently captured target and the extractor's source extent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlLink {
    pub id: String,
    pub target: SourceEvidence,
    pub candidate_path: String,
    pub start: u32,
    pub end: u32,
    pub coordinate_encoding: CoordinateEncoding,
    pub statements: Vec<SqlStatement>,
}

/// Immutable candidate replacement unit. Accepted persistence means only that
/// host-verified code/target bytes and bounded syntax observations were stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlLinkGraph {
    adapter: String,
    adapter_version: String,
    code: SourceEvidence,
    links: Vec<SqlLink>,
}

impl SqlLinkGraph {
    pub fn new(
        adapter: String,
        adapter_version: String,
        code: SourceEvidence,
        links: Vec<SqlLink>,
    ) -> Result<Self, DomainError> {
        text(&adapter, 128)?;
        text(&adapter_version, 128)?;
        full_file(&code, "SQL-link code source")?;
        if links.len() > MAX_LINKS {
            return Err(DomainError::Invalid("SQL-link graph exceeds link budget"));
        }
        let mut ids = HashSet::with_capacity(links.len());
        let mut citations = HashMap::with_capacity(links.len());
        for link in &links {
            text(&link.id, 512)?;
            if !ids.insert(link.id.as_str()) {
                return Err(DomainError::Invalid("duplicate SQL-link identity"));
            }
            if link.start >= link.end {
                return Err(DomainError::Invalid("invalid SQL-link UTF-16 extent"));
            }
            if link.coordinate_encoding != CoordinateEncoding::Utf16CodeUnit {
                return Err(DomainError::Invalid(
                    "unsupported SQL-link coordinate encoding",
                ));
            }
            if link.candidate_path != link.target.path() {
                return Err(DomainError::Invalid(
                    "SQL-link candidate path differs from target evidence",
                ));
            }
            full_file(&link.target, "SQL-link target source")?;
            if link.target.project() != code.project()
                || link.target.graph_version() != code.graph_version()
            {
                return Err(DomainError::Invalid(
                    "SQL-link target differs from code snapshot scope",
                ));
            }
            if let Some(existing) = citations.insert(link.target.id(), &link.target)
                && existing != &link.target
            {
                return Err(DomainError::Invalid(
                    "SQL-link target evidence identity conflicts",
                ));
            }
            if link.statements.len() > MAX_STATEMENTS_PER_LINK {
                return Err(DomainError::Invalid(
                    "SQL-link target exceeds statement budget",
                ));
            }
            for (expected, statement) in link.statements.iter().enumerate() {
                if usize::from(statement.ordinal) != expected {
                    return Err(DomainError::Invalid(
                        "SQL-link statement ordinal is not contiguous",
                    ));
                }
                if statement.relations.len() > 256 {
                    return Err(DomainError::Invalid(
                        "SQL-link statement exceeds relation budget",
                    ));
                }
                for relation in &statement.relations {
                    text(relation, 1024)?;
                }
            }
        }
        Ok(Self {
            adapter,
            adapter_version,
            code,
            links,
        })
    }

    #[must_use]
    pub fn scope(&self) -> SqlLinkScope {
        SqlLinkScope {
            adapter: self.adapter.clone(),
            project: self.code.project().clone(),
            graph_version: self.code.graph_version().into(),
            code_path: self.code.path().into(),
        }
    }
    #[must_use]
    pub fn adapter(&self) -> &str {
        &self.adapter
    }
    #[must_use]
    pub fn adapter_version(&self) -> &str {
        &self.adapter_version
    }
    #[must_use]
    pub fn code(&self) -> &SourceEvidence {
        &self.code
    }
    #[must_use]
    pub fn links(&self) -> &[SqlLink] {
        &self.links
    }
}

fn full_file(evidence: &SourceEvidence, label: &'static str) -> Result<(), DomainError> {
    if evidence.start_line() != 1 {
        return Err(DomainError::Invalid(label));
    }
    Ok(())
}

fn text(value: &str, max_bytes: usize) -> Result<(), DomainError> {
    if value.trim().is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(DomainError::Invalid("invalid or oversized SQL-link text"));
    }
    Ok(())
}

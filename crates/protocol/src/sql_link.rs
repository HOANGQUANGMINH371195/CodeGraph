//! Historical code-to-SQL candidate context. This is an output projection, not
//! a runtime data-flow, table identity, database instance, or authorization claim.

use serde::Serialize;

use crate::SourceEvidence;
use graph_domain::sql_link as domain;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Serialize)]
pub struct SqlStatement {
    pub ordinal: u16,
    pub operation: SqlOperation,
    /// Parser relation spelling only; not a physical database-table identity.
    pub relations: Vec<String>,
}

impl From<&domain::SqlStatement> for SqlStatement {
    fn from(statement: &domain::SqlStatement) -> Self {
        Self {
            ordinal: statement.ordinal,
            operation: match statement.operation {
                domain::SqlOperation::Query => SqlOperation::Query,
                domain::SqlOperation::Insert => SqlOperation::Insert,
                domain::SqlOperation::Update => SqlOperation::Update,
                domain::SqlOperation::Delete => SqlOperation::Delete,
                domain::SqlOperation::CreateTable => SqlOperation::CreateTable,
                domain::SqlOperation::Begin => SqlOperation::Begin,
                domain::SqlOperation::Commit => SqlOperation::Commit,
                domain::SqlOperation::Rollback => SqlOperation::Rollback,
                domain::SqlOperation::Other => SqlOperation::Other,
            },
            relations: statement.relations.clone(),
        }
    }
}

/// One bounded historical candidate. Neither source body is serialized.
#[derive(Debug, Serialize)]
pub struct SqlLinkContext {
    pub schema_version: u32,
    pub kind: &'static str,
    pub generation: u64,
    pub historical: bool,
    pub candidate_only: bool,
    pub relationship_verified: bool,
    pub runtime_verified: bool,
    pub semantic_verified: bool,
    pub source_bytes_verified: bool,
    pub snapshot_binding: &'static str,
    pub seed: String,
    pub code_evidence: SourceEvidence,
    pub target_evidence: SourceEvidence,
    pub candidate_path: String,
    pub coordinate_encoding: &'static str,
    pub start: u32,
    pub end: u32,
    pub statements: Vec<SqlStatement>,
    pub total_links: usize,
    pub omitted_links: usize,
    pub total_statements: usize,
    pub omitted_statements: usize,
}

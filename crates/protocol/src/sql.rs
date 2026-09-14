//! Output-only syntax observations, never graph publication capabilities.

use crate::SourceEvidence;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
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
pub struct ParserSpan {
    pub start_line: u64,
    pub start_column: u64,
    pub end_line: u64,
    pub end_column: u64,
}

#[derive(Debug, Serialize)]
pub struct Statement {
    pub ordinal: usize,
    pub operation: Operation,
    pub relations: Vec<String>,
    pub parser_span: Option<ParserSpan>,
}

#[derive(Debug, Serialize)]
pub struct Analysis {
    pub schema_version: u32,
    pub kind: &'static str,
    pub dialect: &'static str,
    pub candidate_only: bool,
    pub persisted: bool,
    pub source_is_untrusted: bool,
    pub content_hash_and_lines_verified: bool,
    pub snapshot_binding: &'static str,
    pub analysis_run_verified: bool,
    pub semantic_verified: bool,
    pub relationship_verified: bool,
    pub relation_semantics: &'static str,
    pub span_coverage: &'static str,
    pub evidence: SourceEvidence,
    pub statements: Vec<Statement>,
}

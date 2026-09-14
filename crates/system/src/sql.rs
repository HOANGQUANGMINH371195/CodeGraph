//! Bounded SQLite-dialect syntax observations. Never executes SQL or resolves a catalog.

use std::ops::ControlFlow;

use sha2::{Digest, Sha256};
use sqlparser::{
    ast::{Spanned, Statement, visit_relations},
    dialect::SQLiteDialect,
    parser::Parser,
    tokenizer::Tokenizer,
};

const MAX_BYTES: usize = 128 * 1024;
const MAX_TOKENS: usize = 2048;
const MAX_STATEMENTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SqlError {
    #[error("SQL source budget exceeded")]
    SourceLimit,
    #[error("SQL token budget exceeded")]
    TokenLimit,
    #[error("SQL statement budget exceeded")]
    StatementLimit,
    #[error("SQL syntax unsupported, invalid, or exceeds parser recursion limit")]
    Syntax,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Upstream parser coordinates, not a guaranteed full statement extent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlSpan {
    pub start_line: u64,
    pub start_column: u64,
    pub end_line: u64,
    pub end_column: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlStatement {
    /// Zero-based position in the parsed statement sequence.
    pub ordinal: usize,
    pub operation: SqlOperation,
    /// Named relation syntax in visitation order; may include CTE names or views.
    /// Neither access roles nor physical table/catalog identity are resolved.
    pub relations: Vec<String>,
    pub parser_span: Option<SqlSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlSyntaxReport {
    pub source_sha256: String,
    pub statements: Vec<SqlStatement>,
    pub semantic_verified: bool,
}

/// Parse offline source with a fixed `SQLite` dialect and resource limits.
///
/// Syntax acceptance is not `SQLite` execution validity. Visitor coverage and
/// source spans come from the pinned parser; source hash + ordinal are the
/// observation anchor, not a repository/revision verification capability.
/// No source bodies, literal values or upstream error snippets are returned.
///
/// # Errors
/// Returns a typed limit or syntax error without returning a partial report.
pub fn analyze_sqlite(source: &str) -> Result<SqlSyntaxReport, SqlError> {
    if source.len() > MAX_BYTES {
        return Err(SqlError::SourceLimit);
    }
    let dialect = SQLiteDialect {};
    let tokens = Tokenizer::new(&dialect, source)
        .tokenize_with_location()
        .map_err(|_| SqlError::Syntax)?;
    // Tokenization allocates under the source-byte cap. This second limit
    // bounds AST size, including long flat expressions and their destruction.
    if tokens.len() > MAX_TOKENS {
        return Err(SqlError::TokenLimit);
    }
    let statements = Parser::new(&dialect)
        .with_recursion_limit(32)
        .with_tokens_with_locations(tokens)
        .parse_statements()
        .map_err(|_| SqlError::Syntax)?;
    if statements.len() > MAX_STATEMENTS {
        return Err(SqlError::StatementLimit);
    }
    let observations = statements
        .iter()
        .enumerate()
        .map(|(ordinal, statement)| {
            let operation = match statement {
                Statement::Query(_) => SqlOperation::Query,
                Statement::Insert(_) => SqlOperation::Insert,
                Statement::Update(_) => SqlOperation::Update,
                Statement::Delete(_) => SqlOperation::Delete,
                Statement::CreateTable(_) => SqlOperation::CreateTable,
                Statement::StartTransaction { .. } => SqlOperation::Begin,
                Statement::Commit { .. } => SqlOperation::Commit,
                Statement::Rollback { .. } => SqlOperation::Rollback,
                _ => SqlOperation::Other,
            };
            let mut relations = Vec::new();
            let _: ControlFlow<()> = visit_relations(statement, |name| {
                relations.push(name.to_string());
                ControlFlow::Continue(())
            });
            let span = statement.span();
            let parser_span = (span.start.line > 0 && span.end.line > 0).then_some(SqlSpan {
                start_line: span.start.line,
                start_column: span.start.column,
                end_line: span.end.line,
                end_column: span.end.column,
            });
            SqlStatement {
                ordinal,
                operation,
                relations,
                parser_span,
            }
        })
        .collect();
    Ok(SqlSyntaxReport {
        source_sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
        statements: observations,
        semantic_verified: false,
    })
}

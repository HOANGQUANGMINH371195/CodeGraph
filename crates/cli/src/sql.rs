//! Citation-scoped offline SQL analysis, with bounded output and no publication.

use graph_application::{EvidenceRepository, SourceLimits, verify_source};
use graph_protocol::{SCHEMA_VERSION, SourceEvidence, TaskSpec, sql as wire};
use graph_store::Store;
use graph_system::{SqlOperation, analyze_sqlite};
use std::path::Path;

pub fn analyze(
    store: &Store,
    id: &str,
    task_spec: &Path,
    root: &Path,
    max_source_bytes: u32,
    max_output_bytes: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let invalid = || std::io::Error::other("invalid SQL analysis task input");
    let raw = crate::input::read_json_bytes(task_spec)?;
    let task: TaskSpec = serde_json::from_slice(&raw).map_err(|_| invalid())?;
    let task = task.try_into_domain().map_err(|_| invalid())?;
    let evidence = store
        .source_evidence(id, task.project(), task.graph_version())?
        .ok_or_else(|| std::io::Error::other("evidence not found for this snapshot"))?;
    let reader = graph_source::DirectorySource::open(
        root,
        evidence.project().clone(),
        evidence.graph_version().into(),
    )?;
    let limits = SourceLimits::new(max_source_bytes as usize, max_source_bytes as usize)?;
    let source = verify_source(&reader, &evidence, limits)?;
    if evidence.start_line() != 1 {
        return Err(std::io::Error::other("SQL analysis requires a full-file citation").into());
    }
    let report = analyze_sqlite(source.text())?;
    if report.source_sha256 != evidence.content_sha256() {
        return Err(
            std::io::Error::other("SQL analysis requires complete cited source bytes").into(),
        );
    }
    let output = wire::Analysis {
        schema_version: SCHEMA_VERSION,
        kind: "sql_analysis",
        dialect: "sqlite",
        candidate_only: true,
        persisted: false,
        source_is_untrusted: true,
        content_hash_and_lines_verified: true,
        snapshot_binding: "caller_supplied",
        analysis_run_verified: false,
        semantic_verified: false,
        relationship_verified: false,
        relation_semantics: "syntactic_unresolved",
        span_coverage: "parser_hint_not_full_statement",
        evidence: SourceEvidence::from(&evidence),
        statements: report
            .statements
            .into_iter()
            .map(|statement| wire::Statement {
                ordinal: statement.ordinal,
                operation: match statement.operation {
                    SqlOperation::Query => wire::Operation::Query,
                    SqlOperation::Insert => wire::Operation::Insert,
                    SqlOperation::Update => wire::Operation::Update,
                    SqlOperation::Delete => wire::Operation::Delete,
                    SqlOperation::CreateTable => wire::Operation::CreateTable,
                    SqlOperation::Begin => wire::Operation::Begin,
                    SqlOperation::Commit => wire::Operation::Commit,
                    SqlOperation::Rollback => wire::Operation::Rollback,
                    SqlOperation::Other => wire::Operation::Other,
                },
                relations: statement.relations,
                parser_span: statement.parser_span.map(|span| wire::ParserSpan {
                    start_line: span.start_line,
                    start_column: span.start_column,
                    end_line: span.end_line,
                    end_column: span.end_column,
                }),
            })
            .collect(),
    };
    Ok(crate::output::json_line(
        &output,
        max_output_bytes as usize,
    )?)
}

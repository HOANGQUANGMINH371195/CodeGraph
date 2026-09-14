//! Historical SQL-link context projection. No source I/O and no mutation.

use std::{error::Error, path::Path};

use graph_application::{EvidenceRepository, SqlLinkRepository, sql_link_context::select};
use graph_domain::sql_link::CoordinateEncoding;
use graph_protocol::{SCHEMA_VERSION, SourceEvidence, TaskSpec, sql_link as wire};
use graph_store::Store;

pub fn query(
    store: &Store,
    id: &str,
    task_path: &Path,
    link_id: &str,
    cap: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = report(store, id, task_path, link_id)?;
    Ok(crate::output::json_line(&output, cap as usize)?)
}

/// Shared historical projection. Callers own final serialization and its cap.
pub fn report(
    store: &Store,
    id: &str,
    task_path: &Path,
    link_id: &str,
) -> Result<wire::SqlLinkContext, Box<dyn Error>> {
    let task: TaskSpec = serde_json::from_slice(&crate::input::read_json_bytes(task_path)?)?;
    let task = task.try_into_domain()?;
    let evidence = store
        .source_evidence(id, task.project(), task.graph_version())?
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "code evidence not found")
        })?;
    let scope =
        graph_domain::sql_link::SqlLinkScope::new("codegraph-file-reads".into(), &evidence)?;
    let snapshot = store
        .sql_links(&scope)?
        .ok_or_else(|| std::io::Error::other("SQL-link owner has no graph"))?;
    let graph = snapshot
        .graph
        .ok_or_else(|| std::io::Error::other("SQL-link owner is invalidated"))?;
    let link = select(&graph, link_id)?;
    let total_statements = graph.links().iter().map(|item| item.statements.len()).sum();
    Ok(wire::SqlLinkContext {
        schema_version: SCHEMA_VERSION,
        kind: "sql_link_context",
        generation: snapshot.generation,
        historical: true,
        candidate_only: true,
        relationship_verified: false,
        runtime_verified: false,
        semantic_verified: false,
        source_bytes_verified: false,
        snapshot_binding: "caller_supplied",
        seed: link.id.clone(),
        code_evidence: SourceEvidence::from(graph.code()),
        target_evidence: SourceEvidence::from(&link.target),
        candidate_path: link.candidate_path.clone(),
        coordinate_encoding: match link.coordinate_encoding {
            CoordinateEncoding::Utf16CodeUnit => "utf16_code_unit",
        },
        start: link.start,
        end: link.end,
        statements: link
            .statements
            .iter()
            .map(wire::SqlStatement::from)
            .collect(),
        total_links: graph.links().len(),
        omitted_links: graph.links().len() - 1,
        total_statements,
        omitted_statements: total_statements - link.statements.len(),
    })
}

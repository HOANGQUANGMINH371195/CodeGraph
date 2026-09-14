use std::{collections::HashSet, path::Path};

use graph_application::{
    EvidenceRepository, SourceLimits, SqlLinkRepository, capture_source, verify_source,
};
use graph_domain::{
    SourceEvidence,
    sql_link::{CoordinateEncoding, SqlLink, SqlLinkGraph, SqlOperation, SqlStatement},
};
use graph_protocol::{SCHEMA_VERSION, TaskSpec, file_reads::Report};
use graph_store::Store;
use graph_system::{SqlOperation as ParsedOperation, analyze_sqlite};

pub fn publish(
    store: &mut Store,
    id: &str,
    task_spec: &Path,
    report: &Path,
    root: &Path,
    run: &str,
    expected: u64,
    max_source: u32,
    max_output: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let task: TaskSpec = serde_json::from_slice(&crate::input::read_json_bytes(task_spec)?)?;
    let task = task.try_into_domain()?;
    let code = store
        .source_evidence(id, task.project(), task.graph_version())?
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "code evidence not found")
        })?;
    let reader = graph_source::DirectorySource::open(
        root,
        task.project().clone(),
        task.graph_version().into(),
    )?;
    let limits = SourceLimits::new(max_source as usize, max_source as usize)?;
    let code = verify_source(&reader, &code, limits)?;
    if code.evidence().start_line() != 1 {
        return Err(std::io::Error::other("SQL-link code source must be full-file").into());
    }
    let report: Report = serde_json::from_slice(&crate::input::read_json_bytes(report)?)?;
    let source = report
        .source
        .as_ref()
        .ok_or_else(|| std::io::Error::other("file-read report missing source"))?;
    if !report.valid_candidate_transport()
        || source.path != code.evidence().path()
        || source.sha256 != code.evidence().content_sha256()
        || source.byte_length != code.text().len() as u64
        || source.line_count != code.evidence().end_line()
    {
        return Err(
            std::io::Error::other("file-read report does not match verified code source").into(),
        );
    }
    let extents = report.candidate_read_extents()?;
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    for target in &report.targets {
        if target.status != "captured-candidate"
            || target.runtime_verified
            || target.atomic_snapshot_verified
            || target.race_free_containment_verified
            || !target.containment_checks_passed
            || !target.requires_stable_filesystem
        {
            return Err(
                std::io::Error::other("file-read target is not a captured candidate").into(),
            );
        }
        let captured = target
            .target
            .as_ref()
            .ok_or_else(|| std::io::Error::other("file-read target missing capture"))?;
        let captured_source = target
            .source
            .as_ref()
            .ok_or_else(|| std::io::Error::other("file-read target missing source capture"))?;
        if captured_source.path != source.path
            || captured_source.sha256 != source.sha256
            || captured_source.byte_length != source.byte_length
            || captured_source.line_count != source.line_count
        {
            return Err(std::io::Error::other(
                "file-read target source differs from report source",
            )
            .into());
        }
        if !seen.insert(target.candidate_index) {
            return Err(
                std::io::Error::other("file-read target candidate index is duplicated").into(),
            );
        }
        let extent = *extents
            .get(target.candidate_index as usize)
            .ok_or_else(|| std::io::Error::other("file-read target candidate index missing"))?;
        if !utf16_boundary(code.text(), extent.start) || !utf16_boundary(code.text(), extent.end) {
            return Err(
                std::io::Error::other("file-read extent splits a UTF-16 surrogate pair").into(),
            );
        }
        let locator = SourceEvidence::new(
            format!("file-read-locator:{}", target.candidate_index),
            task.project().clone(),
            task.graph_version().into(),
            captured.path.clone(),
            captured.sha256.clone(),
            1,
            1,
            run.into(),
        )?;
        let target_source = capture_source(&reader, &locator, run, limits)?;
        if target_source.evidence().content_sha256() != captured.sha256
            || target_source.evidence().path() != captured.path
            || target_source.text().len() as u64 != captured.byte_length
            || target_source.evidence().end_line() != captured.line_count
        {
            return Err(std::io::Error::other(
                "captured SQL target differs from host verification",
            )
            .into());
        }
        let syntax = analyze_sqlite(target_source.text())?;
        links.push(SqlLink {
            id: format!("file-read:{}:{}", target.candidate_index, captured.path),
            target: target_source.evidence().clone(),
            candidate_path: captured.path.clone(),
            start: extent.start,
            end: extent.end,
            coordinate_encoding: CoordinateEncoding::Utf16CodeUnit,
            statements: syntax.statements.into_iter().map(statement).collect(),
        });
    }
    if links.len() != extents.len() {
        return Err(std::io::Error::other(
            "file-read report candidates and captured targets are not one-to-one",
        )
        .into());
    }
    let graph = SqlLinkGraph::new(
        "codegraph-file-reads".into(),
        "1".into(),
        code.evidence().clone(),
        links,
    )?;
    let output = crate::output::json_line(
        &serde_json::json!({"schema_version":SCHEMA_VERSION,"kind":"sql_link_mutation","operation":"publish","generation":expected.checked_add(1).ok_or_else(||std::io::Error::other("generation exhausted"))?,"persisted":true,"candidate_only":true,"relationship_verified":false,"runtime_verified":false,"semantic_verified":false,"source_bytes_verified_during_operation":true,"snapshot_binding":"caller_supplied"}),
        max_output as usize,
    )?;
    store.replace_sql_links(&graph, expected)?;
    Ok(output)
}

fn utf16_boundary(text: &str, offset: u32) -> bool {
    let mut n = 0_u32;
    if offset == 0 {
        return true;
    }
    for ch in text.chars() {
        n += ch.len_utf16() as u32;
        if n == offset {
            return true;
        }
        if n > offset {
            return false;
        }
    }
    false
}

fn scope(
    store: &Store,
    id: &str,
    task_path: &Path,
) -> Result<graph_domain::sql_link::SqlLinkScope, Box<dyn std::error::Error>> {
    let task: TaskSpec = serde_json::from_slice(&crate::input::read_json_bytes(task_path)?)?;
    let task = task.try_into_domain()?;
    let evidence = store
        .source_evidence(id, task.project(), task.graph_version())?
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "code evidence not found")
        })?;
    Ok(graph_domain::sql_link::SqlLinkScope::new(
        "codegraph-file-reads".into(),
        &evidence,
    )?)
}

pub fn query(
    store: &Store,
    id: &str,
    task: &Path,
    cap: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let scope = scope(store, id, task)?;
    let snapshot = store.sql_links(&scope)?;
    let value = snapshot.map(|s| serde_json::json!({"schema_version":SCHEMA_VERSION,"kind":"sql_link_snapshot","generation":s.generation,"historical":true,"candidate_only":true,"relationship_verified":false,"runtime_verified":false,"semantic_verified":false,"source_bytes_verified":false,"snapshot_binding":"caller_supplied","graph":s.graph.map(|g|serde_json::json!({"adapter":g.adapter(),"adapter_version":g.adapter_version(),"code_evidence":graph_protocol::SourceEvidence::from(g.code()),"link_count":g.links().len()}))}));
    Ok(crate::output::json_line(&value, cap as usize)?)
}

pub fn invalidate(
    store: &mut Store,
    id: &str,
    task: &Path,
    expected: u64,
    cap: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let scope = scope(store, id, task)?;
    let next = expected
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("generation exhausted"))?;
    let bytes = crate::output::json_line(
        &serde_json::json!({"schema_version":SCHEMA_VERSION,"kind":"sql_link_mutation","operation":"invalidate","generation":next,"persisted":true,"candidate_only":true,"relationship_verified":false,"runtime_verified":false,"semantic_verified":false,"source_bytes_verified_during_operation":false,"snapshot_binding":"caller_supplied"}),
        cap as usize,
    )?;
    store.invalidate_sql_links(&scope, expected)?;
    Ok(bytes)
}
fn statement(value: graph_system::SqlStatement) -> SqlStatement {
    SqlStatement {
        ordinal: u16::try_from(value.ordinal).unwrap_or(u16::MAX),
        operation: match value.operation {
            ParsedOperation::Query => SqlOperation::Query,
            ParsedOperation::Insert => SqlOperation::Insert,
            ParsedOperation::Update => SqlOperation::Update,
            ParsedOperation::Delete => SqlOperation::Delete,
            ParsedOperation::CreateTable => SqlOperation::CreateTable,
            ParsedOperation::Begin => SqlOperation::Begin,
            ParsedOperation::Commit => SqlOperation::Commit,
            ParsedOperation::Rollback => SqlOperation::Rollback,
            ParsedOperation::Other => SqlOperation::Other,
        },
        relations: value.relations,
    }
}

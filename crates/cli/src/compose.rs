//! Compose candidate query composition: stored citation -> verified bytes ->
//! offline declarations -> bounded wire output. Explicit publication uses CAS;
//! query reports remain historical, never current source verification.

use std::path::Path;

use graph_application::{DeploymentRepository, EvidenceRepository, SourceLimits, verify_source};
use graph_domain::deployment::DeploymentScope;
use graph_protocol::{SCHEMA_VERSION, SourceEvidence, TaskSpec, deployment as wire};
use graph_store::Store;
use graph_system::{DeploymentGraph, analyze_compose_graph};

pub fn analyze(
    store: &Store,
    id: &str,
    task_spec: &Path,
    root: &Path,
    max_source_bytes: u32,
    max_output_bytes: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let projection = verified_graph(store, id, task_spec, root, max_source_bytes)?;
    let output = candidate(&projection);
    Ok(crate::output::json_line(
        &output,
        max_output_bytes as usize,
    )?)
}

pub(crate) fn selected_evidence(
    store: &Store,
    id: &str,
    task_spec: &Path,
) -> Result<graph_domain::SourceEvidence, Box<dyn std::error::Error>> {
    let invalid = || std::io::Error::other("invalid deployment task input");
    let raw = crate::input::read_json_bytes(task_spec)?;
    let task: TaskSpec = serde_json::from_slice(&raw).map_err(|_| invalid())?;
    let task = task.try_into_domain().map_err(|_| invalid())?;
    Ok(store
        .source_evidence(id, task.project(), task.graph_version())?
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "evidence not found for this snapshot",
            )
        })?)
}

fn verified_graph(
    store: &Store,
    id: &str,
    task_spec: &Path,
    root: &Path,
    max_source_bytes: u32,
) -> Result<DeploymentGraph, Box<dyn std::error::Error>> {
    let evidence = selected_evidence(store, id, task_spec)?;
    let reader = graph_source::DirectorySource::open(
        root,
        evidence.project().clone(),
        evidence.graph_version().into(),
    )?;
    let limits = SourceLimits::new(max_source_bytes as usize, max_source_bytes as usize)?;
    let source = verify_source(&reader, &evidence, limits)?;
    Ok(analyze_compose_graph(&source)?)
}

fn mutation_receipt(
    expected: u64,
    publish: bool,
    max_output_bytes: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let generation = expected
        .checked_add(1)
        .filter(|n| i64::try_from(*n).is_ok())
        .ok_or_else(|| std::io::Error::other("deployment generation exhausted"))?;
    Ok(crate::output::json_line(
        &wire::DeploymentMutation {
            schema_version: SCHEMA_VERSION,
            kind: "deployment_mutation",
            operation: if publish { "publish" } else { "invalidate" },
            generation,
            persisted: true,
            source_bytes_verified_during_operation: publish,
            snapshot_binding: "caller_supplied",
            relationship_verified: false,
        },
        max_output_bytes as usize,
    )?)
}

pub fn publish(
    store: &mut Store,
    id: &str,
    task_spec: &Path,
    root: &Path,
    expected: u64,
    max_source_bytes: u32,
    max_output_bytes: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let graph = verified_graph(store, id, task_spec, root, max_source_bytes)?;
    // Encoding/budget failure must occur before graph mutation. The concrete
    // Store returns exactly expected+1 or errors; no query race after commit.
    let output = mutation_receipt(expected, true, max_output_bytes)?;
    store
        .replace_deployment(&graph, expected)
        .map_err(mutation_error)?;
    Ok(output)
}

pub fn invalidate(
    store: &mut Store,
    id: &str,
    task_spec: &Path,
    expected: u64,
    max_output_bytes: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let scope = DeploymentScope::new("compose".into(), &selected_evidence(store, id, task_spec)?)?;
    let output = mutation_receipt(expected, false, max_output_bytes)?;
    store
        .invalidate_deployment(&scope, expected)
        .map_err(mutation_error)?;
    Ok(output)
}

fn mutation_error(error: graph_store::StoreError) -> Box<dyn std::error::Error> {
    match error {
        graph_store::StoreError::Conflict => std::io::Error::other(
            "deployment generation or immutable citation conflict; inspect deployment before retrying",
        ).into(),
        other => other.into(),
    }
}

pub fn query(
    store: &Store,
    id: &str,
    task_spec: &Path,
    max_output_bytes: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let scope = DeploymentScope::new("compose".into(), &selected_evidence(store, id, task_spec)?)?;
    let snapshot = store
        .deployment(&scope)?
        .map(|snapshot| wire::DeploymentSnapshot {
            schema_version: SCHEMA_VERSION,
            kind: "deployment_snapshot",
            generation: snapshot.generation,
            historical: true,
            source_bytes_verified: false,
            relationship_verified: false,
            analysis_run_verified: false,
            snapshot_binding: "caller_supplied",
            provenance: "iac_declared",
            graph: snapshot
                .graph
                .as_ref()
                .map(wire::StoredDeploymentGraph::from),
        });
    Ok(crate::output::json_line(
        &snapshot,
        max_output_bytes as usize,
    )?)
}

fn candidate(projection: &DeploymentGraph) -> wire::ComposeProjection {
    wire::ComposeProjection {
        schema_version: SCHEMA_VERSION,
        kind: "compose_projection",
        provenance: "iac_declared",
        candidate_only: true,
        persisted: false,
        source_is_untrusted: true,
        content_hash_and_lines_verified: true,
        snapshot_binding: "caller_supplied",
        analysis_run_verified: false,
        relationship_verified: false,
        evidence: SourceEvidence::from(projection.evidence()),
        nodes: projection
            .nodes()
            .iter()
            .map(wire::DeploymentNode::from)
            .collect(),
        edges: projection
            .edges()
            .iter()
            .map(wire::DeploymentEdge::from)
            .collect(),
        unknowns: projection
            .unknowns()
            .iter()
            .map(wire::DeploymentUnknown::from)
            .collect(),
    }
}

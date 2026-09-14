//! Graph-only context output. No filesystem source read or graph mutation.
use graph_application::{
    DeploymentRepository,
    deployment_context::{ContextLimits, Direction, select},
};
use graph_domain::deployment::DeploymentScope;
use graph_protocol::{SCHEMA_VERSION, SourceEvidence, deployment as wire};
use graph_store::Store;
use std::{collections::BTreeMap, error::Error, path::Path};

pub struct ContextOptions<'a> {
    pub id: &'a str,
    pub task_spec: &'a Path,
    pub node: &'a str,
    pub direction: Direction,
    pub depth: u32,
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_output_bytes: usize,
}

pub fn query(store: &Store, options: &ContextOptions<'_>) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = report(store, options)?;
    Ok(crate::output::json_line(&output, options.max_output_bytes)?)
}

/// Shared stored projection; callers bound serialization of their complete report.
pub fn report(
    store: &Store,
    options: &ContextOptions<'_>,
) -> Result<wire::DeploymentContext, Box<dyn Error>> {
    let limits = ContextLimits::new(options.depth, options.max_nodes, options.max_edges)?;
    let locator = crate::compose::selected_evidence(store, options.id, options.task_spec)?;
    let scope = DeploymentScope::new("compose".into(), &locator)?;
    let snapshot = store
        .deployment(&scope)?
        .ok_or_else(|| std::io::Error::other("deployment owner has no graph"))?;
    let graph = snapshot
        .graph
        .ok_or_else(|| std::io::Error::other("deployment owner is invalidated"))?;
    let selection = select(&graph, options.node, options.direction, limits)?;
    let mut citations = BTreeMap::new();
    citations.insert(
        graph.evidence().id().to_owned(),
        SourceEvidence::from(graph.evidence()),
    );
    let nodes = selection
        .nodes
        .iter()
        .map(|&i| {
            let node = &graph.nodes()[i];
            let full = wire::DeploymentNode::from(node);
            citations
                .entry(node.evidence.id().to_owned())
                .or_insert(full.evidence);
            wire::ContextNode {
                id: full.id,
                kind: full.kind,
                name: full.name,
                evidence_id: node.evidence.id().into(),
            }
        })
        .collect();
    let edges = selection
        .edges
        .iter()
        .map(|&i| {
            let edge = &graph.edges()[i];
            let full = wire::DeploymentEdge::from(edge);
            citations
                .entry(edge.evidence.id().to_owned())
                .or_insert(full.evidence);
            wire::ContextEdge {
                source: full.source,
                target: full.target,
                kind: full.kind,
                mount_target: full.mount_target,
                evidence_id: edge.evidence.id().into(),
            }
        })
        .collect();
    let output = wire::DeploymentContext {
        schema_version: SCHEMA_VERSION,
        kind: "deployment_context",
        generation: snapshot.generation,
        seed: options.node.into(),
        direction: match options.direction {
            Direction::Incoming => "incoming",
            Direction::Outgoing => "outgoing",
            Direction::Both => "both",
        },
        depth: options.depth,
        historical: true,
        source_is_untrusted: true,
        source_bytes_verified: false,
        analysis_run_verified: false,
        relationship_verified: false,
        provenance: "iac_declared",
        source_evidence_id: graph.evidence().id().into(),
        nodes,
        edges,
        citations: citations.into_values().collect(),
        total_nodes: graph.nodes().len(),
        total_edges: graph.edges().len(),
        omitted_nodes: graph.nodes().len() - selection.nodes.len(),
        omitted_edges: graph.edges().len() - selection.edges.len(),
        unknown_count: graph.unknowns().len(),
        depth_limited: selection.depth_limited,
        budget_limited: selection.budget_limited,
    };
    Ok(output)
}

//! Convert parser candidates into an immutable, structurally validated batch.

use graph_application::SourceSlice;
use graph_domain::deployment as domain;

use crate::{ComposeError, EdgeKind, NodeKind, analyze_compose};

/// Analyze verified full-file bytes and validate the complete declared graph.
/// Adapter version changes must accompany changes to this producer's semantics.
/// This establishes neither runtime truth nor graph-write authorization.
///
/// # Errors
/// Propagates source/parser rejection or invalid graph shape, identity or budget.
pub fn analyze_compose_graph(
    source: &SourceSlice,
) -> Result<domain::DeploymentGraph, ComposeError> {
    let projection = analyze_compose(source)?;
    Ok(domain::DeploymentGraph::new(
        "compose".into(),
        "1".into(),
        source.evidence().clone(),
        projection
            .nodes
            .into_iter()
            .map(|node| domain::Node {
                id: node.id,
                name: node.name,
                evidence: node.evidence,
                kind: match node.kind {
                    NodeKind::Service => domain::NodeKind::Service,
                    NodeKind::Volume => domain::NodeKind::Volume,
                    NodeKind::Network => domain::NodeKind::Network,
                },
            })
            .collect(),
        projection
            .edges
            .into_iter()
            .map(|edge| domain::Edge {
                source: edge.source,
                target: edge.target,
                mount_target: edge.mount_target,
                evidence: edge.evidence,
                kind: match edge.kind {
                    EdgeKind::Mounts => domain::EdgeKind::Mounts,
                    EdgeKind::DependsOn => domain::EdgeKind::DependsOn,
                    EdgeKind::AttachedTo => domain::EdgeKind::AttachedTo,
                },
            })
            .collect(),
        projection
            .unknowns
            .into_iter()
            .map(|unknown| domain::Unknown {
                reason: unknown.reason.into(),
                line: unknown.line,
            })
            .collect(),
    )?)
}

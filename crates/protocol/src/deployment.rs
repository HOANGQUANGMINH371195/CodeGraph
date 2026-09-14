//! Output-only deployment reports. Serialized fields are reports, never
//! verification capabilities or authority to publish graph facts.

use serde::Serialize;

use crate::SourceEvidence;
use graph_domain::deployment as domain;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentNodeKind {
    Service,
    Volume,
    Network,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentEdgeKind {
    Mounts,
    DependsOn,
    AttachedTo,
}

#[derive(Debug, Serialize)]
pub struct DeploymentNode {
    pub id: String,
    pub kind: DeploymentNodeKind,
    pub name: String,
    pub evidence: SourceEvidence,
}

#[derive(Debug, Serialize)]
pub struct DeploymentEdge {
    pub source: String,
    pub target: String,
    pub kind: DeploymentEdgeKind,
    pub mount_target: Option<String>,
    pub evidence: SourceEvidence,
}

#[derive(Debug, Serialize)]
pub struct DeploymentUnknown {
    pub reason: String,
    pub line: u32,
}

impl From<&domain::Node> for DeploymentNode {
    fn from(node: &domain::Node) -> Self {
        Self {
            id: node.id.clone(),
            name: node.name.clone(),
            evidence: SourceEvidence::from(&node.evidence),
            kind: match node.kind {
                domain::NodeKind::Service => DeploymentNodeKind::Service,
                domain::NodeKind::Volume => DeploymentNodeKind::Volume,
                domain::NodeKind::Network => DeploymentNodeKind::Network,
            },
        }
    }
}

impl From<&domain::Edge> for DeploymentEdge {
    fn from(edge: &domain::Edge) -> Self {
        Self {
            source: edge.source.clone(),
            target: edge.target.clone(),
            mount_target: edge.mount_target.clone(),
            evidence: SourceEvidence::from(&edge.evidence),
            kind: match edge.kind {
                domain::EdgeKind::Mounts => DeploymentEdgeKind::Mounts,
                domain::EdgeKind::DependsOn => DeploymentEdgeKind::DependsOn,
                domain::EdgeKind::AttachedTo => DeploymentEdgeKind::AttachedTo,
            },
        }
    }
}

impl From<&domain::Unknown> for DeploymentUnknown {
    fn from(unknown: &domain::Unknown) -> Self {
        Self {
            reason: unknown.reason.clone(),
            line: unknown.line,
        }
    }
}

/// A process-boundary report. Consumers must not treat these booleans or
/// citations as a durable verification token when reading them back.
#[derive(Debug, Serialize)]
pub struct ComposeProjection {
    pub schema_version: u32,
    pub kind: &'static str,
    pub provenance: &'static str,
    pub candidate_only: bool,
    pub persisted: bool,
    pub source_is_untrusted: bool,
    pub content_hash_and_lines_verified: bool,
    pub snapshot_binding: &'static str,
    pub analysis_run_verified: bool,
    pub relationship_verified: bool,
    pub evidence: SourceEvidence,
    pub nodes: Vec<DeploymentNode>,
    pub edges: Vec<DeploymentEdge>,
    pub unknowns: Vec<DeploymentUnknown>,
}

/// Stored declared facts without fabricated current source verification.
#[derive(Debug, Serialize)]
pub struct StoredDeploymentGraph {
    pub adapter: String,
    pub adapter_version: String,
    pub evidence: SourceEvidence,
    pub nodes: Vec<DeploymentNode>,
    pub edges: Vec<DeploymentEdge>,
    pub unknowns: Vec<DeploymentUnknown>,
}

impl From<&domain::DeploymentGraph> for StoredDeploymentGraph {
    fn from(graph: &domain::DeploymentGraph) -> Self {
        Self {
            adapter: graph.adapter().into(),
            adapter_version: graph.adapter_version().into(),
            evidence: SourceEvidence::from(graph.evidence()),
            nodes: graph.nodes().iter().map(DeploymentNode::from).collect(),
            edges: graph.edges().iter().map(DeploymentEdge::from).collect(),
            unknowns: graph
                .unknowns()
                .iter()
                .map(DeploymentUnknown::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DeploymentSnapshot {
    pub schema_version: u32,
    pub kind: &'static str,
    pub generation: u64,
    pub historical: bool,
    pub source_bytes_verified: bool,
    pub relationship_verified: bool,
    pub analysis_run_verified: bool,
    pub snapshot_binding: &'static str,
    pub provenance: &'static str,
    pub graph: Option<StoredDeploymentGraph>,
}

/// Emitted only after successful CAS. Output transport failure does not undo it.
#[derive(Debug, Serialize)]
pub struct DeploymentMutation {
    pub schema_version: u32,
    pub kind: &'static str,
    pub operation: &'static str,
    pub generation: u64,
    pub persisted: bool,
    pub source_bytes_verified_during_operation: bool,
    pub snapshot_binding: &'static str,
    pub relationship_verified: bool,
}

/// Reconciliation report: invalid source may have successfully retracted rows.
#[derive(Debug, Serialize)]
pub struct ComposeReindex {
    pub schema_version: u32,
    pub kind: &'static str,
    pub status: &'static str,
    pub generation: u64,
    pub source_valid: bool,
    pub changed: bool,
    pub historical: bool,
    pub snapshot_binding: &'static str,
    pub relationship_verified: bool,
    pub reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct ContextNode {
    pub id: String,
    pub kind: DeploymentNodeKind,
    pub name: String,
    pub evidence_id: String,
}

#[derive(Debug, Serialize)]
pub struct ContextEdge {
    pub source: String,
    pub target: String,
    pub kind: DeploymentEdgeKind,
    pub mount_target: Option<String>,
    pub evidence_id: String,
}

/// Bounded declared neighborhood. Omitted counts refer to the entire owner,
/// including unrelated/direction-excluded facts; unknown_count is owner-wide.
#[derive(Debug, Serialize)]
pub struct DeploymentContext {
    pub schema_version: u32,
    pub kind: &'static str,
    pub generation: u64,
    pub seed: String,
    pub direction: &'static str,
    pub depth: u32,
    pub historical: bool,
    pub source_is_untrusted: bool,
    pub source_bytes_verified: bool,
    pub analysis_run_verified: bool,
    pub relationship_verified: bool,
    pub provenance: &'static str,
    pub source_evidence_id: String,
    pub nodes: Vec<ContextNode>,
    pub edges: Vec<ContextEdge>,
    pub citations: Vec<SourceEvidence>,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub omitted_nodes: usize,
    pub omitted_edges: usize,
    pub unknown_count: usize,
    pub depth_limited: bool,
    pub budget_limited: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_kinds_distinguish_declared_dependencies_from_traffic() {
        assert_eq!(
            serde_json::to_string(&DeploymentNodeKind::Service).unwrap(),
            "\"service\""
        );
        assert_eq!(
            serde_json::to_string(&DeploymentNodeKind::Volume).unwrap(),
            "\"volume\""
        );
        assert_eq!(
            serde_json::to_string(&DeploymentNodeKind::Network).unwrap(),
            "\"network\""
        );
        assert_eq!(
            serde_json::to_string(&DeploymentEdgeKind::Mounts).unwrap(),
            "\"mounts\""
        );
        assert_eq!(
            serde_json::to_string(&DeploymentEdgeKind::DependsOn).unwrap(),
            "\"depends_on\""
        );
        assert_eq!(
            serde_json::to_string(&DeploymentEdgeKind::AttachedTo).unwrap(),
            "\"attached_to\""
        );
    }
}

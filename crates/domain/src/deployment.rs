//! Structurally validated declared graph batches. This module does not verify
//! source bytes, runtime relationships or permission to publish a graph.

use std::collections::{HashMap, HashSet};

use crate::{DomainError, ProjectRef, SourceEvidence};

/// Replacement owner. Producer version and source content may change within it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentScope {
    adapter: String,
    project: ProjectRef,
    graph_version: String,
    path: String,
}

impl DeploymentScope {
    /// Derive a portable source scope from validated evidence.
    ///
    /// # Errors
    /// Rejects empty, oversized or control-character-containing adapter names.
    pub fn new(adapter: String, source: &SourceEvidence) -> Result<Self, DomainError> {
        text(&adapter, 128)?;
        Ok(Self {
            adapter,
            project: source.project().clone(),
            graph_version: source.graph_version().into(),
            path: source.path().into(),
        })
    }

    #[must_use]
    pub fn adapter(&self) -> &str {
        &self.adapter
    }
    #[must_use]
    pub fn project(&self) -> &ProjectRef {
        &self.project
    }
    #[must_use]
    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Service,
    Volume,
    Network,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    Mounts,
    DependsOn,
    AttachedTo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub name: String,
    pub evidence: SourceEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,
    pub mount_target: Option<String>,
    pub evidence: SourceEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unknown {
    pub reason: String,
    pub line: u32,
}

/// Immutable replacement unit, owned by one adapter/version and source snapshot.
/// Empty batches intentionally represent no remaining declarations, not failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentGraph {
    adapter: String,
    adapter_version: String,
    evidence: SourceEvidence,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    unknowns: Vec<Unknown>,
}

impl DeploymentGraph {
    #[must_use]
    pub fn scope(&self) -> DeploymentScope {
        DeploymentScope {
            adapter: self.adapter.clone(),
            project: self.evidence.project().clone(),
            graph_version: self.evidence.graph_version().into(),
            path: self.evidence.path().into(),
        }
    }

    /// Validate all relationships before exposing an immutable graph batch.
    ///
    /// # Errors
    /// Rejects oversized/invalid identities, non-full-file evidence, mixed
    /// citation scopes, dangling/mistyped/duplicate edges and out-of-range lines.
    pub fn new(
        adapter: String,
        adapter_version: String,
        evidence: SourceEvidence,
        nodes: Vec<Node>,
        edges: Vec<Edge>,
        unknowns: Vec<Unknown>,
    ) -> Result<Self, DomainError> {
        text(&adapter, 128)?;
        text(&adapter_version, 128)?;
        if evidence.start_line() != 1 {
            return Err(DomainError::Invalid(
                "deployment source must start at line one",
            ));
        }
        if nodes.len() > 10_000 || edges.len() > 50_000 || unknowns.len() > 50_000 {
            return Err(DomainError::Invalid("deployment graph exceeds item budget"));
        }
        let mut citations = HashMap::new();
        check_citation(&evidence, &evidence, &mut citations)?;
        let mut identities = HashMap::with_capacity(nodes.len());
        for node in &nodes {
            text(&node.id, 512)?;
            text(&node.name, 1024)?;
            if identities.insert(node.id.as_str(), node.kind).is_some() {
                return Err(DomainError::Invalid("duplicate deployment node identity"));
            }
            check_citation(&node.evidence, &evidence, &mut citations)?;
        }
        let mut relationships = HashSet::with_capacity(edges.len());
        for edge in &edges {
            check_edge(edge, &identities)?;
            check_citation(&edge.evidence, &evidence, &mut citations)?;
            if !relationships.insert((
                edge.source.as_str(),
                edge.target.as_str(),
                edge.kind,
                edge.mount_target.as_deref(),
                edge.evidence.id(),
            )) {
                return Err(DomainError::Invalid("duplicate deployment edge identity"));
            }
        }
        for unknown in &unknowns {
            text(&unknown.reason, 512)?;
            if unknown.line == 0 || unknown.line > evidence.end_line() {
                return Err(DomainError::Invalid(
                    "deployment unknown line outside source",
                ));
            }
        }
        Ok(Self {
            adapter,
            adapter_version,
            evidence,
            nodes,
            edges,
            unknowns,
        })
    }

    #[must_use]
    pub fn adapter(&self) -> &str {
        &self.adapter
    }
    #[must_use]
    pub fn adapter_version(&self) -> &str {
        &self.adapter_version
    }
    #[must_use]
    pub fn evidence(&self) -> &SourceEvidence {
        &self.evidence
    }
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    #[must_use]
    pub fn unknowns(&self) -> &[Unknown] {
        &self.unknowns
    }
}

fn text(value: &str, max_bytes: usize) -> Result<(), DomainError> {
    if value.trim().is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(DomainError::Invalid("invalid or oversized deployment text"));
    }
    Ok(())
}

fn check_citation<'a>(
    citation: &'a SourceEvidence,
    source: &SourceEvidence,
    identities: &mut HashMap<&'a str, &'a SourceEvidence>,
) -> Result<(), DomainError> {
    if citation.project() != source.project()
        || citation.graph_version() != source.graph_version()
        || citation.path() != source.path()
        || citation.content_sha256() != source.content_sha256()
        || citation.analysis_run() != source.analysis_run()
        || citation.start_line() < source.start_line()
        || citation.end_line() > source.end_line()
    {
        return Err(DomainError::Invalid(
            "deployment citation differs from source scope",
        ));
    }
    if let Some(previous) = identities.insert(citation.id(), citation)
        && previous != citation
    {
        return Err(DomainError::Invalid(
            "deployment citation identity has conflicting content",
        ));
    }
    Ok(())
}

fn check_edge(edge: &Edge, nodes: &HashMap<&str, NodeKind>) -> Result<(), DomainError> {
    let source = nodes.get(edge.source.as_str());
    let target = nodes.get(edge.target.as_str());
    let expected_target = match edge.kind {
        EdgeKind::Mounts => NodeKind::Volume,
        EdgeKind::DependsOn => NodeKind::Service,
        EdgeKind::AttachedTo => NodeKind::Network,
    };
    if source != Some(&NodeKind::Service) || target != Some(&expected_target) {
        return Err(DomainError::Invalid(
            "deployment edge has missing or mistyped endpoints",
        ));
    }
    match (&edge.kind, &edge.mount_target) {
        (EdgeKind::Mounts, Some(target)) if target.starts_with('/') && !target.contains('$') => {
            text(target, 4096)?;
        }
        (EdgeKind::DependsOn | EdgeKind::AttachedTo, None) => {}
        _ => {
            return Err(DomainError::Invalid(
                "deployment mount target does not match edge kind",
            ));
        }
    }
    Ok(())
}

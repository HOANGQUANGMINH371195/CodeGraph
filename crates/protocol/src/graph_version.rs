//! Versioned wire contracts for graph snapshots and incremental deltas.
//!
//! These DTOs carry freshness metadata across process boundaries. They do not
//! authenticate a producer or grant GraphWriter authority.

use graph_domain as domain;
use serde::{Deserialize, Serialize};

use crate::{ProjectRef, ProtocolError, SCHEMA_VERSION, require_current_schema};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphVersion {
    pub schema_version: u32,
    pub id: String,
    pub project: ProjectRef,
    pub generation: u64,
    pub source_manifest_sha256: String,
    pub extractor_version: String,
    pub resolver_version: String,
    pub analyzer_version: String,
}

impl GraphVersion {
    pub fn try_into_domain(self) -> Result<domain::GraphVersion, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(domain::GraphVersion::new(
            self.id,
            self.project.into(),
            self.generation,
            self.source_manifest_sha256,
            self.extractor_version,
            self.resolver_version,
            self.analyzer_version,
        )?)
    }
}

impl From<&domain::GraphVersion> for GraphVersion {
    fn from(version: &domain::GraphVersion) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: version.id().into(),
            project: version.project().into(),
            generation: version.generation(),
            source_manifest_sha256: version.source_manifest_sha256().into(),
            extractor_version: version.extractor_version().into(),
            resolver_version: version.resolver_version().into(),
            analyzer_version: version.analyzer_version().into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphSnapshotCache {
    pub schema_version: u32,
    pub version: GraphVersion,
    pub observed_at_ms: u64,
    pub expires_at_ms: Option<u64>,
}

impl GraphSnapshotCache {
    pub fn try_into_domain(self) -> Result<domain::GraphSnapshotCache, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(domain::GraphSnapshotCache::new(
            self.version.try_into_domain()?,
            self.observed_at_ms,
            self.expires_at_ms,
        )?)
    }
}

impl From<&domain::GraphSnapshotCache> for GraphSnapshotCache {
    fn from(cache: &domain::GraphSnapshotCache) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            version: GraphVersion::from(cache.version()),
            observed_at_ms: cache.observed_at_ms(),
            expires_at_ms: cache.expires_at_ms(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStaleReason {
    ProjectScopeChanged,
    RevisionChanged,
    WorkingTreeChanged,
    SourceChanged,
    ConfigurationChanged,
    IgnorePolicyChanged,
    ExtractorChanged,
    ResolverChanged,
    AnalyzerChanged,
    GraphVersionChanged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotFreshness {
    Fresh,
    Stale(SnapshotStaleReason),
    Expired,
}

impl From<domain::SnapshotStaleReason> for SnapshotStaleReason {
    fn from(reason: domain::SnapshotStaleReason) -> Self {
        match reason {
            domain::SnapshotStaleReason::ProjectScopeChanged => Self::ProjectScopeChanged,
            domain::SnapshotStaleReason::RevisionChanged => Self::RevisionChanged,
            domain::SnapshotStaleReason::WorkingTreeChanged => Self::WorkingTreeChanged,
            domain::SnapshotStaleReason::SourceChanged => Self::SourceChanged,
            domain::SnapshotStaleReason::ConfigurationChanged => Self::ConfigurationChanged,
            domain::SnapshotStaleReason::IgnorePolicyChanged => Self::IgnorePolicyChanged,
            domain::SnapshotStaleReason::ExtractorChanged => Self::ExtractorChanged,
            domain::SnapshotStaleReason::ResolverChanged => Self::ResolverChanged,
            domain::SnapshotStaleReason::AnalyzerChanged => Self::AnalyzerChanged,
            domain::SnapshotStaleReason::GraphVersionChanged => Self::GraphVersionChanged,
        }
    }
}

impl From<domain::SnapshotFreshness> for SnapshotFreshness {
    fn from(freshness: domain::SnapshotFreshness) -> Self {
        match freshness {
            domain::SnapshotFreshness::Fresh => Self::Fresh,
            domain::SnapshotFreshness::Stale(reason) => Self::Stale(reason.into()),
            domain::SnapshotFreshness::Expired => Self::Expired,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphDelta {
    pub schema_version: u32,
    pub from: GraphVersion,
    pub to: GraphVersion,
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub changed_nodes: Vec<String>,
    pub added_edges: Vec<String>,
    pub removed_edges: Vec<String>,
    pub changed_edges: Vec<String>,
    pub invalidated_evidence: Vec<String>,
}

impl GraphDelta {
    pub fn try_into_domain(self) -> Result<domain::GraphDelta, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(domain::GraphDelta::new(
            self.from.try_into_domain()?,
            self.to.try_into_domain()?,
            self.added_nodes,
            self.removed_nodes,
            self.changed_nodes,
            self.added_edges,
            self.removed_edges,
            self.changed_edges,
            self.invalidated_evidence,
        )?)
    }
}

impl From<&domain::GraphDelta> for GraphDelta {
    fn from(delta: &domain::GraphDelta) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            from: GraphVersion::from(delta.from()),
            to: GraphVersion::from(delta.to()),
            added_nodes: delta.added_nodes().into(),
            removed_nodes: delta.removed_nodes().into(),
            changed_nodes: delta.changed_nodes().into(),
            added_edges: delta.added_edges().into(),
            removed_edges: delta.removed_edges().into(),
            changed_edges: delta.changed_edges().into(),
            invalidated_evidence: delta.invalidated_evidence().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(head: &str) -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: head.into(),
            working_tree_fingerprint: "dirty".into(),
            config_hash: "config".into(),
            ignore_policy_version: "ignore-v1".into(),
        }
    }

    fn version(generation: u64, head: &str) -> GraphVersion {
        GraphVersion {
            schema_version: SCHEMA_VERSION,
            id: format!("graph-{generation}"),
            project: project(head),
            generation,
            source_manifest_sha256: "a".repeat(64),
            extractor_version: "extractor-1".into(),
            resolver_version: "resolver-1".into(),
            analyzer_version: "analyzer-1".into(),
        }
    }

    #[test]
    fn graph_version_and_cache_round_trip_without_losing_snapshot_inputs() {
        let wire = GraphSnapshotCache {
            schema_version: SCHEMA_VERSION,
            version: version(1, "head"),
            observed_at_ms: 10,
            expires_at_ms: Some(20),
        };
        let domain = wire.clone().try_into_domain().unwrap();
        assert_eq!(GraphSnapshotCache::from(&domain), wire);
        assert_eq!(domain.version().project().git_head, "head");
    }

    #[test]
    fn future_nested_schema_and_mixed_delta_are_rejected() {
        let mut future = version(1, "head");
        future.schema_version = SCHEMA_VERSION + 1;
        assert!(matches!(
            future.try_into_domain(),
            Err(ProtocolError::UnsupportedSchema(2))
        ));

        let wire = GraphDelta {
            schema_version: SCHEMA_VERSION,
            from: version(1, "old"),
            to: version(2, "new"),
            added_nodes: vec!["node".into()],
            removed_nodes: vec!["node".into()],
            changed_nodes: vec![],
            added_edges: vec![],
            removed_edges: vec![],
            changed_edges: vec![],
            invalidated_evidence: vec![],
        };
        assert!(matches!(
            wire.try_into_domain(),
            Err(ProtocolError::Domain(graph_domain::DomainError::Invalid(_)))
        ));
    }
}

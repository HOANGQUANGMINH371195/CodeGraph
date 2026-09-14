use crate::{DomainError, ProjectRef, validate_text};

const MAX_VERSION_TEXT_BYTES: usize = 4 * 1024;
const MAX_DELTA_ITEMS: usize = 100_000;

/// Immutable identity of one materialized graph view.
///
/// `ProjectRef` carries the worktree inputs while this value records the
/// graph-producing implementation and the source manifest it consumed. A
/// version is therefore a snapshot label, not proof that the filesystem was
/// atomically frozen while it was built; adapters must re-check their inputs
/// before publishing facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphVersion {
    id: String,
    project: ProjectRef,
    generation: u64,
    source_manifest_sha256: String,
    extractor_version: String,
    resolver_version: String,
    analyzer_version: String,
}

impl GraphVersion {
    /// Constructs and validates an immutable graph version identity.
    ///
    /// # Errors
    /// Returns an error when project identity, version text, digest, or
    /// generation metadata is invalid or exceeds configured bounds.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        project: ProjectRef,
        generation: u64,
        source_manifest_sha256: String,
        extractor_version: String,
        resolver_version: String,
        analyzer_version: String,
    ) -> Result<Self, DomainError> {
        let version = Self {
            id,
            project,
            generation,
            source_manifest_sha256,
            extractor_version,
            resolver_version,
            analyzer_version,
        };
        version.validate()?;
        Ok(version)
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn project(&self) -> &ProjectRef {
        &self.project
    }

    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub fn source_manifest_sha256(&self) -> &str {
        &self.source_manifest_sha256
    }

    #[must_use]
    pub fn extractor_version(&self) -> &str {
        &self.extractor_version
    }

    #[must_use]
    pub fn resolver_version(&self) -> &str {
        &self.resolver_version
    }

    #[must_use]
    pub fn analyzer_version(&self) -> &str {
        &self.analyzer_version
    }

    /// The identity that may remain stable while a branch/head or file
    /// contents change. Snapshot equality must still compare all inputs.
    #[must_use]
    pub fn same_worktree(&self, other: &Self) -> bool {
        self.project.repository_id == other.project.repository_id
            && self.project.worktree_id == other.project.worktree_id
    }

    fn validate(&self) -> Result<(), DomainError> {
        self.project.validate()?;
        for (name, value) in [
            ("graph version id", &self.id),
            ("source manifest hash", &self.source_manifest_sha256),
            ("extractor version", &self.extractor_version),
            ("resolver version", &self.resolver_version),
            ("analyzer version", &self.analyzer_version),
        ] {
            validate_text(name, value)?;
            if value.len() > MAX_VERSION_TEXT_BYTES || value.chars().any(char::is_control) {
                return Err(DomainError::Invalid("graph version text"));
            }
        }
        if self.generation == 0 {
            return Err(DomainError::Invalid("graph generation must be positive"));
        }
        if !is_sha256(&self.source_manifest_sha256) {
            return Err(DomainError::Invalid(
                "source manifest must be lowercase SHA-256 hex",
            ));
        }
        Ok(())
    }
}

/// Why a cached graph can no longer be used as the current graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Freshness is explicit so callers cannot interpret an absent or old cache as
/// a current graph by accident.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotFreshness {
    Fresh,
    Stale(SnapshotStaleReason),
    Expired,
}

/// Descriptor for a cached graph snapshot. It has no authority to write graph
/// state and does not claim an atomic filesystem snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphSnapshotCache {
    version: GraphVersion,
    observed_at_ms: u64,
    expires_at_ms: Option<u64>,
}

impl GraphSnapshotCache {
    pub fn new(
        version: GraphVersion,
        observed_at_ms: u64,
        expires_at_ms: Option<u64>,
    ) -> Result<Self, DomainError> {
        if observed_at_ms == 0 {
            return Err(DomainError::Invalid(
                "snapshot observation time must be positive",
            ));
        }
        if expires_at_ms.is_some_and(|expires| expires <= observed_at_ms) {
            return Err(DomainError::Invalid(
                "snapshot expiry must be after observation",
            ));
        }
        Ok(Self {
            version,
            observed_at_ms,
            expires_at_ms,
        })
    }

    pub fn version(&self) -> &GraphVersion {
        &self.version
    }

    pub fn observed_at_ms(&self) -> u64 {
        self.observed_at_ms
    }

    pub fn expires_at_ms(&self) -> Option<u64> {
        self.expires_at_ms
    }

    /// Expiry wins over input comparison: a time-invalid cache must never be
    /// made current merely because its source inputs still match.
    pub fn freshness_against(&self, current: &GraphVersion, now_ms: u64) -> SnapshotFreshness {
        if now_ms < self.observed_at_ms
            || self.expires_at_ms.is_some_and(|expires| now_ms >= expires)
        {
            return SnapshotFreshness::Expired;
        }
        stale_reason(&self.version, current)
            .map_or(SnapshotFreshness::Fresh, SnapshotFreshness::Stale)
    }
}

/// Forward-only changes between two materialized graph versions.
///
/// Lists are required to be sorted and duplicate-free at this boundary. That
/// makes serialized deltas deterministic and leaves canonicalization to the
/// producer instead of hiding ordering bugs in a protocol adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphDelta {
    from: GraphVersion,
    to: GraphVersion,
    added_nodes: Vec<String>,
    removed_nodes: Vec<String>,
    changed_nodes: Vec<String>,
    added_edges: Vec<String>,
    removed_edges: Vec<String>,
    changed_edges: Vec<String>,
    invalidated_evidence: Vec<String>,
}

impl GraphDelta {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        from: GraphVersion,
        to: GraphVersion,
        added_nodes: Vec<String>,
        removed_nodes: Vec<String>,
        changed_nodes: Vec<String>,
        added_edges: Vec<String>,
        removed_edges: Vec<String>,
        changed_edges: Vec<String>,
        invalidated_evidence: Vec<String>,
    ) -> Result<Self, DomainError> {
        if !from.same_worktree(&to) {
            return Err(DomainError::Invalid(
                "graph delta must stay within one repository worktree",
            ));
        }
        if from.generation() >= to.generation() {
            return Err(DomainError::Invalid(
                "graph delta target generation must advance",
            ));
        }
        let delta = Self {
            from,
            to,
            added_nodes,
            removed_nodes,
            changed_nodes,
            added_edges,
            removed_edges,
            changed_edges,
            invalidated_evidence,
        };
        delta.validate_lists()?;
        Ok(delta)
    }

    pub fn from(&self) -> &GraphVersion {
        &self.from
    }

    pub fn to(&self) -> &GraphVersion {
        &self.to
    }

    pub fn added_nodes(&self) -> &[String] {
        &self.added_nodes
    }

    pub fn removed_nodes(&self) -> &[String] {
        &self.removed_nodes
    }

    pub fn changed_nodes(&self) -> &[String] {
        &self.changed_nodes
    }

    pub fn added_edges(&self) -> &[String] {
        &self.added_edges
    }

    pub fn removed_edges(&self) -> &[String] {
        &self.removed_edges
    }

    pub fn changed_edges(&self) -> &[String] {
        &self.changed_edges
    }

    pub fn invalidated_evidence(&self) -> &[String] {
        &self.invalidated_evidence
    }

    fn validate_lists(&self) -> Result<(), DomainError> {
        validate_id_lists(
            "node delta",
            [&self.added_nodes, &self.removed_nodes, &self.changed_nodes],
        )?;
        validate_id_lists(
            "edge delta",
            [&self.added_edges, &self.removed_edges, &self.changed_edges],
        )?;
        validate_id_list("invalidated evidence", &self.invalidated_evidence)?;
        ensure_disjoint(
            [&self.added_nodes, &self.removed_nodes, &self.changed_nodes],
            "node delta categories overlap",
        )?;
        ensure_disjoint(
            [&self.added_edges, &self.removed_edges, &self.changed_edges],
            "edge delta categories overlap",
        )?;
        Ok(())
    }
}

fn stale_reason(previous: &GraphVersion, current: &GraphVersion) -> Option<SnapshotStaleReason> {
    if !previous.same_worktree(current) {
        return Some(SnapshotStaleReason::ProjectScopeChanged);
    }
    if previous.project().git_head != current.project().git_head {
        return Some(SnapshotStaleReason::RevisionChanged);
    }
    if previous.project().working_tree_fingerprint != current.project().working_tree_fingerprint {
        return Some(SnapshotStaleReason::WorkingTreeChanged);
    }
    if previous.source_manifest_sha256 != current.source_manifest_sha256 {
        return Some(SnapshotStaleReason::SourceChanged);
    }
    if previous.project().config_hash != current.project().config_hash {
        return Some(SnapshotStaleReason::ConfigurationChanged);
    }
    if previous.project().ignore_policy_version != current.project().ignore_policy_version {
        return Some(SnapshotStaleReason::IgnorePolicyChanged);
    }
    if previous.extractor_version != current.extractor_version {
        return Some(SnapshotStaleReason::ExtractorChanged);
    }
    if previous.resolver_version != current.resolver_version {
        return Some(SnapshotStaleReason::ResolverChanged);
    }
    if previous.analyzer_version != current.analyzer_version {
        return Some(SnapshotStaleReason::AnalyzerChanged);
    }
    if previous.id != current.id || previous.generation != current.generation {
        return Some(SnapshotStaleReason::GraphVersionChanged);
    }
    None
}

fn validate_id_lists<const N: usize>(
    label: &'static str,
    lists: [&[String]; N],
) -> Result<(), DomainError> {
    for list in lists {
        validate_id_list(label, list)?;
    }
    Ok(())
}

fn validate_id_list(label: &'static str, ids: &[String]) -> Result<(), DomainError> {
    if ids.len() > MAX_DELTA_ITEMS {
        return Err(DomainError::Invalid("graph delta item budget"));
    }
    for id in ids {
        validate_text(label, id)?;
        if id.len() > MAX_VERSION_TEXT_BYTES || id.chars().any(char::is_control) {
            return Err(DomainError::Invalid("graph delta identifier"));
        }
    }
    if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(DomainError::Invalid(
            "graph delta identifiers must be sorted and unique",
        ));
    }
    Ok(())
}

fn ensure_disjoint<const N: usize>(
    lists: [&[String]; N],
    message: &'static str,
) -> Result<(), DomainError> {
    let mut seen = std::collections::HashSet::new();
    for list in lists {
        if list.iter().any(|id| !seen.insert(id)) {
            return Err(DomainError::Invalid(message));
        }
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(head: &str, working: &str) -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: head.into(),
            working_tree_fingerprint: working.into(),
            config_hash: "config".into(),
            ignore_policy_version: "ignore-v1".into(),
        }
    }

    fn version(generation: u64, head: &str, working: &str) -> GraphVersion {
        GraphVersion::new(
            format!("graph-{generation}"),
            project(head, working),
            generation,
            "a".repeat(64),
            "extractor-1".into(),
            "resolver-1".into(),
            "analyzer-1".into(),
        )
        .unwrap()
    }

    #[test]
    fn cache_distinguishes_source_change_and_expiry() {
        let cache = GraphSnapshotCache::new(version(1, "head", "dirty"), 10, Some(20)).unwrap();
        assert_eq!(
            cache.freshness_against(&version(1, "head", "changed"), 11),
            SnapshotFreshness::Stale(SnapshotStaleReason::WorkingTreeChanged)
        );
        assert_eq!(
            cache.freshness_against(&version(1, "head", "dirty"), 20),
            SnapshotFreshness::Expired
        );
        assert_eq!(
            cache.freshness_against(&version(1, "head", "dirty"), 9),
            SnapshotFreshness::Expired
        );
    }

    #[test]
    fn delta_requires_forward_same_worktree_and_canonical_lists() {
        let delta = GraphDelta::new(
            version(1, "old", "dirty"),
            version(2, "new", "dirty"),
            vec!["node.b".into()],
            vec!["node.a".into()],
            vec!["node.c".into()],
            vec!["edge.b".into()],
            vec![],
            vec![],
            vec!["evidence.1".into()],
        )
        .unwrap();
        assert_eq!(delta.to().generation(), 2);
        assert!(
            GraphDelta::new(
                version(2, "new", "dirty"),
                version(1, "old", "dirty"),
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
            )
            .is_err()
        );
        assert!(
            GraphDelta::new(
                version(1, "old", "dirty"),
                version(2, "new", "dirty"),
                vec!["b".into(), "a".into()],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
            )
            .is_err()
        );
    }
}

//! Versioned, bounded context sent to agents and renderers.
//!
//! This is an agent-facing projection, not graph-write authority. Every source
//! derived item references a snapshot-bound `SourceEvidence` record and source
//! text remains explicitly untrusted data.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{ExtensionFields, ProjectRef, ProtocolError, SourceEvidence};

pub const CONTEXT_SCHEMA_VERSION: &str = "project-graph/context/v1";

const MAX_QUERY_BYTES: usize = 64 * 1024;
const MAX_NODES: usize = 1_000;
const MAX_EDGES: usize = 5_000;
const MAX_SOURCE_RANGES: usize = 1_000;
const MAX_EVIDENCE: usize = 10_000;
const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DEPTH: u16 = 64;
const MAX_TEXT_BYTES: usize = 4 * 1024;
const MAX_SERIALIZED_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CHARACTERS: u64 = 16 * 1024 * 1024;
const MAX_TOKENS: u64 = 4 * 1024 * 1024;
const MAX_TOKENIZER_BYTES: usize = 128;

/// Agent-facing view requested from the context gateway.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextView {
    Orientation,
    Symbol,
    Flow,
    Impact,
    Architecture,
    Dataflow,
    Deployment,
    Runtime,
    Change,
}

/// Where a context item came from. This is provenance metadata, not an
/// acceptance decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextProvenance {
    Codegraph,
    Resolver,
    Cpg,
    Joern,
    RuntimeTrace,
    Document,
    Human,
    Heuristic,
}

/// Whether the relation or item is sufficiently resolved for this view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    Resolved,
    Partial,
    Unresolved,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    Complete,
    Partial,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnknownContext {
    pub code: String,
    pub reason: String,
    pub subject: Option<String>,
    pub path: Option<String>,
    pub line: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub status: CoverageStatus,
    pub unknowns: Vec<UnknownContext>,
}

/// The limits used to produce this envelope and the actual emitted usage.
/// Serialized bytes are the deterministic transport bound. Character counts
/// and tokenizer-specific counts are disclosed separately and do not grant
/// model entitlement or execution authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextBudget {
    pub max_nodes: u32,
    pub max_edges: u32,
    pub max_source_ranges: u32,
    pub max_source_bytes: u64,
    pub max_traversal_depth: u16,
    pub max_serialized_bytes: u64,
    pub max_characters: u64,
    pub max_tokens: Option<u64>,
    pub tokenizer: Option<String>,
    pub emitted_nodes: u32,
    pub emitted_edges: u32,
    pub emitted_source_ranges: u32,
    pub emitted_source_bytes: u64,
    pub emitted_serialized_bytes: u64,
    pub emitted_characters: u64,
    pub emitted_tokens: Option<u64>,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextNode {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub provenance: ContextProvenance,
    pub resolution: ResolutionStatus,
    pub source_derived: bool,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub provenance: ContextProvenance,
    pub resolution: ResolutionStatus,
    pub source_derived: bool,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeSlice {
    pub path: String,
    pub start: u32,
    pub end: u32,
    pub language: Option<String>,
    /// Optional verbatim source. It is data supplied to the agent, never an
    /// instruction or an authority-bearing field.
    pub content: Option<String>,
    pub evidence_ids: Vec<String>,
}

/// Snapshot-bound context for one agent turn or renderer request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextEnvelope {
    pub schema_version: String,
    pub project: ProjectRef,
    pub graph_version: String,
    pub revision: String,
    pub view: ContextView,
    pub query: String,
    /// Always true: source and external text are untrusted data.
    pub source_is_untrusted: bool,
    pub nodes: Vec<ContextNode>,
    pub edges: Vec<ContextEdge>,
    pub code_slices: Vec<CodeSlice>,
    /// The canonical evidence manifest referenced by nodes, edges and slices.
    pub evidence: Vec<SourceEvidence>,
    pub coverage: Coverage,
    pub budget: ContextBudget,
    #[serde(default)]
    pub extensions: ExtensionFields,
}

impl ContextEnvelope {
    /// Validate the wire projection before passing any of its facts to an
    /// application port. This does not accept assertions or write graph state.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.schema_version != CONTEXT_SCHEMA_VERSION {
            return Err(ProtocolError::UnsupportedContextSchema(
                self.schema_version.clone(),
            ));
        }
        let project: graph_domain::ProjectRef = self.project.clone().into();
        project.validate()?;
        bounded("graph version", &self.graph_version, MAX_TEXT_BYTES)?;
        bounded("revision", &self.revision, MAX_TEXT_BYTES)?;
        if self.revision != self.project.git_head {
            return Err(ProtocolError::InvalidContext(
                "revision differs from project git_head".into(),
            ));
        }
        bounded("query", &self.query, MAX_QUERY_BYTES)?;
        if !self.source_is_untrusted {
            return Err(ProtocolError::InvalidContext(
                "source_is_untrusted must be true".into(),
            ));
        }

        if self.nodes.len() > MAX_NODES
            || self.edges.len() > MAX_EDGES
            || self.code_slices.len() > MAX_SOURCE_RANGES
            || self.evidence.len() > MAX_EVIDENCE
        {
            return Err(ProtocolError::InvalidContext(
                "context envelope exceeds item budget".into(),
            ));
        }
        validate_budget(&self.budget)?;
        self.extensions.validate()?;

        let evidence = self.evidence_index()?;
        let mut node_ids = HashSet::with_capacity(self.nodes.len());
        for node in &self.nodes {
            bounded("context node id", &node.id, MAX_TEXT_BYTES)?;
            bounded("context node kind", &node.kind, MAX_TEXT_BYTES)?;
            bounded("context node name", &node.name, MAX_TEXT_BYTES)?;
            if !node_ids.insert(node.id.as_str()) {
                return Err(ProtocolError::InvalidContext(
                    "duplicate context node id".into(),
                ));
            }
            validate_item_evidence(
                &node.evidence_ids,
                node.source_derived,
                node.resolution,
                &evidence,
            )?;
        }

        for edge in &self.edges {
            bounded("context edge kind", &edge.kind, MAX_TEXT_BYTES)?;
            if !node_ids.contains(edge.from.as_str()) || !node_ids.contains(edge.to.as_str()) {
                return Err(ProtocolError::InvalidContext(
                    "context edge has dangling endpoint".into(),
                ));
            }
            validate_item_evidence(
                &edge.evidence_ids,
                edge.source_derived,
                edge.resolution,
                &evidence,
            )?;
        }

        for slice in &self.code_slices {
            validate_path(&slice.path)?;
            if slice.start == 0 || slice.end < slice.start {
                return Err(ProtocolError::InvalidContext(
                    "invalid code slice line range".into(),
                ));
            }
            if let Some(language) = &slice.language {
                bounded("code slice language", language, 128)?;
            }
            if let Some(content) = &slice.content {
                bounded("code slice content", content, MAX_SOURCE_BYTES as usize)?;
            }
            validate_evidence_ids(&slice.evidence_ids, true, &evidence)?;
            let covered = slice.evidence_ids.iter().any(|id| {
                evidence.get(id).is_some_and(|citation| {
                    citation.path == slice.path
                        && citation.start_line <= slice.start
                        && citation.end_line >= slice.end
                })
            });
            if !covered {
                return Err(ProtocolError::InvalidContext(
                    "code slice is not covered by same-path evidence".into(),
                ));
            }
        }

        for unknown in &self.coverage.unknowns {
            bounded("unknown code", &unknown.code, 256)?;
            bounded("unknown reason", &unknown.reason, MAX_TEXT_BYTES)?;
            if let Some(subject) = &unknown.subject {
                bounded("unknown subject", subject, MAX_TEXT_BYTES)?;
            }
            if let Some(path) = &unknown.path {
                validate_path(path)?;
            }
            if unknown.line == Some(0) {
                return Err(ProtocolError::InvalidContext(
                    "unknown context line must be one-based".into(),
                ));
            }
        }
        Ok(())
    }

    /// Serialize a validated envelope in a canonical order.
    ///
    /// Producers may discover the same graph in different traversal or
    /// insertion orders. Sorting the set-like collections here makes the
    /// agent-facing bytes depend on the graph/query/snapshot content rather
    /// than worker scheduling. This is a deterministic wire projection, not
    /// graph-write authority.
    pub fn canonical_json(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical.canonicalize_collections();
        serde_json::to_vec(&canonical)
            .map_err(|_| ProtocolError::InvalidContext("cannot serialize context envelope".into()))
    }

    fn canonicalize_collections(&mut self) {
        self.nodes
            .sort_unstable_by(|left, right| left.id.cmp(&right.id));
        for node in &mut self.nodes {
            node.evidence_ids.sort_unstable();
        }

        self.edges.sort_unstable_by(|left, right| {
            left.from
                .cmp(&right.from)
                .then(left.to.cmp(&right.to))
                .then(left.kind.cmp(&right.kind))
                .then(left.provenance.cmp(&right.provenance))
                .then(left.resolution.cmp(&right.resolution))
                .then(left.source_derived.cmp(&right.source_derived))
        });
        for edge in &mut self.edges {
            edge.evidence_ids.sort_unstable();
        }

        self.code_slices.sort_unstable_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.start.cmp(&right.start))
                .then(left.end.cmp(&right.end))
                .then(left.language.cmp(&right.language))
                .then(left.content.cmp(&right.content))
        });
        for slice in &mut self.code_slices {
            slice.evidence_ids.sort_unstable();
        }

        self.evidence
            .sort_unstable_by(|left, right| left.id.cmp(&right.id));
        self.coverage.unknowns.sort_unstable_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then(left.path.cmp(&right.path))
                .then(left.line.cmp(&right.line))
                .then(left.subject.cmp(&right.subject))
                .then(left.reason.cmp(&right.reason))
        });
    }

    fn evidence_index(&self) -> Result<HashMap<String, SourceEvidence>, ProtocolError> {
        let expected_project: graph_domain::ProjectRef = self.project.clone().into();
        let mut index = HashMap::with_capacity(self.evidence.len());
        for citation in &self.evidence {
            let domain = citation.clone().try_into_domain()?;
            if domain.project() != &expected_project || domain.graph_version() != self.graph_version
            {
                return Err(ProtocolError::InvalidContext(
                    "evidence is outside envelope snapshot".into(),
                ));
            }
            if index
                .insert(citation.id.clone(), citation.clone())
                .is_some()
            {
                return Err(ProtocolError::InvalidContext(
                    "duplicate context evidence id".into(),
                ));
            }
        }
        Ok(index)
    }
}

fn validate_evidence_ids(
    ids: &[String],
    required: bool,
    evidence: &HashMap<String, SourceEvidence>,
) -> Result<(), ProtocolError> {
    if required && ids.is_empty() {
        return Err(ProtocolError::InvalidContext(
            "source-derived context item lacks evidence".into(),
        ));
    }
    let mut seen = HashSet::with_capacity(ids.len());
    for id in ids {
        bounded("context evidence id", id, MAX_TEXT_BYTES)?;
        if !seen.insert(id) || !evidence.contains_key(id) {
            return Err(ProtocolError::InvalidContext(
                "context evidence reference is missing or duplicated".into(),
            ));
        }
    }
    Ok(())
}

fn validate_item_evidence(
    ids: &[String],
    source_derived: bool,
    resolution: ResolutionStatus,
    evidence: &HashMap<String, SourceEvidence>,
) -> Result<(), ProtocolError> {
    validate_evidence_ids(ids, source_derived, evidence)?;
    if ids.is_empty() && resolution == ResolutionStatus::Resolved {
        return Err(ProtocolError::InvalidContext(
            "resolved context item lacks evidence or explicit uncertainty".into(),
        ));
    }
    Ok(())
}

fn validate_budget(budget: &ContextBudget) -> Result<(), ProtocolError> {
    if budget.max_nodes == 0
        || usize::try_from(budget.max_nodes).unwrap_or(usize::MAX) > MAX_NODES
        || budget.max_edges == 0
        || usize::try_from(budget.max_edges).unwrap_or(usize::MAX) > MAX_EDGES
        || budget.max_source_ranges == 0
        || usize::try_from(budget.max_source_ranges).unwrap_or(usize::MAX) > MAX_SOURCE_RANGES
        || budget.max_source_bytes == 0
        || budget.max_source_bytes > MAX_SOURCE_BYTES
        || budget.max_traversal_depth > MAX_DEPTH
        || budget.max_serialized_bytes == 0
        || budget.max_serialized_bytes > MAX_SERIALIZED_BYTES
        || budget.max_characters == 0
        || budget.max_characters > MAX_CHARACTERS
        || budget.emitted_nodes > budget.max_nodes
        || budget.emitted_edges > budget.max_edges
        || budget.emitted_source_ranges > budget.max_source_ranges
        || budget.emitted_source_bytes > budget.max_source_bytes
        || budget.emitted_serialized_bytes > budget.max_serialized_bytes
        || budget.emitted_characters > budget.max_characters
    {
        return Err(ProtocolError::InvalidContext(
            "invalid or oversized context budget".into(),
        ));
    }
    match (
        budget.max_tokens,
        budget.tokenizer.as_deref(),
        budget.emitted_tokens,
    ) {
        (Some(max_tokens), Some(tokenizer), Some(emitted_tokens))
            if max_tokens > 0
                && max_tokens <= MAX_TOKENS
                && emitted_tokens <= max_tokens
                && !tokenizer.trim().is_empty()
                && tokenizer.len() <= MAX_TOKENIZER_BYTES
                && !tokenizer.chars().any(char::is_control) => {}
        (None, None, None) => {}
        _ => {
            return Err(ProtocolError::InvalidContext(
                "invalid tokenizer-aware context budget".into(),
            ));
        }
    }
    Ok(())
}

fn bounded(label: &'static str, value: &str, max_bytes: usize) -> Result<(), ProtocolError> {
    if value.trim().is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(ProtocolError::InvalidContext(format!(
            "{label} is empty, oversized or contains control characters"
        )));
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), ProtocolError> {
    bounded("context path", path, MAX_TEXT_BYTES)?;
    if path.starts_with('/')
        || path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(ProtocolError::InvalidContext(
            "context path must be repository relative".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SCHEMA_VERSION;

    fn project() -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: "a".into(),
            working_tree_fingerprint: "dirty".into(),
            config_hash: "c".into(),
            ignore_policy_version: "ignore-v1".into(),
        }
    }

    fn evidence(id: &str, path: &str, start: u32, end: u32) -> SourceEvidence {
        SourceEvidence {
            schema_version: SCHEMA_VERSION,
            id: id.into(),
            project: project(),
            graph_version: "graph-v1".into(),
            path: path.into(),
            content_sha256: "a".repeat(64),
            start_line: start,
            end_line: end,
            analysis_run: "run-1".into(),
        }
    }

    fn budget() -> ContextBudget {
        ContextBudget {
            max_nodes: 10,
            max_edges: 10,
            max_source_ranges: 10,
            max_source_bytes: 10_000,
            max_traversal_depth: 4,
            max_serialized_bytes: 20_000,
            max_characters: 16_000,
            max_tokens: Some(4_000),
            tokenizer: Some("test-tokenizer-v1".into()),
            emitted_nodes: 2,
            emitted_edges: 1,
            emitted_source_ranges: 1,
            emitted_source_bytes: 100,
            emitted_serialized_bytes: 500,
            emitted_characters: 400,
            emitted_tokens: Some(100),
            truncated: false,
        }
    }

    fn valid() -> ContextEnvelope {
        ContextEnvelope {
            schema_version: CONTEXT_SCHEMA_VERSION.into(),
            project: project(),
            graph_version: "graph-v1".into(),
            revision: "a".into(),
            view: ContextView::Flow,
            query: "how does login reach storage".into(),
            source_is_untrusted: true,
            nodes: vec![
                ContextNode {
                    id: "route.login".into(),
                    kind: "route".into(),
                    name: "POST /login".into(),
                    provenance: ContextProvenance::Codegraph,
                    resolution: ResolutionStatus::Resolved,
                    source_derived: true,
                    evidence_ids: vec!["ev-route".into()],
                },
                ContextNode {
                    id: "auth.login".into(),
                    kind: "method".into(),
                    name: "login".into(),
                    provenance: ContextProvenance::Resolver,
                    resolution: ResolutionStatus::Resolved,
                    source_derived: true,
                    evidence_ids: vec!["ev-method".into()],
                },
            ],
            edges: vec![ContextEdge {
                from: "route.login".into(),
                to: "auth.login".into(),
                kind: "dispatch".into(),
                provenance: ContextProvenance::Resolver,
                resolution: ResolutionStatus::Resolved,
                source_derived: true,
                evidence_ids: vec!["ev-route".into()],
            }],
            code_slices: vec![CodeSlice {
                path: "src/auth.ts".into(),
                start: 42,
                end: 48,
                language: Some("typescript".into()),
                content: Some("return service.login();".into()),
                evidence_ids: vec!["ev-method".into()],
            }],
            evidence: vec![
                evidence("ev-route", "src/auth.ts", 1, 80),
                evidence("ev-method", "src/auth.ts", 40, 60),
            ],
            coverage: Coverage {
                status: CoverageStatus::Partial,
                unknowns: vec![UnknownContext {
                    code: "dynamic_di".into(),
                    reason: "runtime dependency injection target is unresolved".into(),
                    subject: Some("auth.login".into()),
                    path: Some("src/auth.ts".into()),
                    line: Some(47),
                }],
            },
            budget: budget(),
            extensions: ExtensionFields::empty(),
        }
    }

    #[test]
    fn valid_envelope_round_trips_and_validates() {
        let envelope = valid();
        envelope.validate().unwrap();
        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: ContextEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, envelope);
        decoded.validate().unwrap();
    }

    #[test]
    fn budget_requires_consistent_serialized_character_and_token_metadata() {
        let mut missing_tokenizer = valid();
        missing_tokenizer.budget.tokenizer = None;
        assert!(missing_tokenizer.validate().is_err());

        let mut missing_token_measurement = valid();
        missing_token_measurement.budget.emitted_tokens = None;
        assert!(missing_token_measurement.validate().is_err());

        let mut over_serialized = valid();
        over_serialized.budget.emitted_serialized_bytes =
            over_serialized.budget.max_serialized_bytes + 1;
        assert!(over_serialized.validate().is_err());

        let mut over_characters = valid();
        over_characters.budget.emitted_characters = over_characters.budget.max_characters + 1;
        assert!(over_characters.validate().is_err());
    }

    #[test]
    fn source_derived_items_require_line_evidence() {
        let mut envelope = valid();
        envelope.nodes[0].evidence_ids.clear();
        assert!(matches!(
            envelope.validate(),
            Err(ProtocolError::InvalidContext(message))
                if message.contains("lacks evidence")
        ));
    }

    #[test]
    fn resolved_items_cannot_hide_missing_evidence_as_non_source_data() {
        let mut envelope = valid();
        envelope.edges[0].source_derived = false;
        envelope.edges[0].evidence_ids.clear();
        assert!(matches!(
            envelope.validate(),
            Err(ProtocolError::InvalidContext(message))
                if message.contains("lacks evidence")
        ));

        envelope.edges[0].resolution = ResolutionStatus::Unresolved;
        envelope.validate().unwrap();
    }

    #[test]
    fn canonical_json_is_stable_across_discovery_order() {
        let envelope = valid();
        let mut reordered = envelope.clone();
        reordered.nodes.reverse();
        reordered.evidence.reverse();
        reordered.edges[0].evidence_ids.reverse();
        assert_eq!(
            envelope.canonical_json().unwrap(),
            reordered.canonical_json().unwrap()
        );
    }

    #[test]
    fn mixed_snapshot_evidence_is_rejected() {
        let mut envelope = valid();
        envelope.evidence[1].project.worktree_id = "other".into();
        assert!(matches!(
            envelope.validate(),
            Err(ProtocolError::Domain(_)) | Err(ProtocolError::InvalidContext(_))
        ));
    }

    #[test]
    fn dangling_edges_and_uncovered_slices_are_rejected() {
        let mut dangling = valid();
        dangling.edges[0].to = "missing".into();
        assert!(matches!(
            dangling.validate(),
            Err(ProtocolError::InvalidContext(message))
                if message.contains("dangling")
        ));

        let mut uncovered = valid();
        uncovered.code_slices[0].path = "src/other.ts".into();
        assert!(matches!(
            uncovered.validate(),
            Err(ProtocolError::InvalidContext(message))
                if message.contains("not covered")
        ));
    }

    #[test]
    fn future_schema_and_trusted_source_flag_are_rejected() {
        let mut future = valid();
        future.schema_version = "project-graph/context/v2".into();
        assert!(matches!(
            future.validate(),
            Err(ProtocolError::UnsupportedContextSchema(_))
        ));

        let mut trusted = valid();
        trusted.source_is_untrusted = false;
        assert!(matches!(
            trusted.validate(),
            Err(ProtocolError::InvalidContext(message))
                if message.contains("source_is_untrusted")
        ));
    }

    #[test]
    fn unknowns_are_explicit_and_unknown_fields_are_not_accepted() {
        let envelope = valid();
        assert_eq!(envelope.coverage.status, CoverageStatus::Partial);
        assert_eq!(envelope.coverage.unknowns.len(), 1);
        let mut json = serde_json::to_value(envelope).unwrap();
        json["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ContextEnvelope>(json).is_err());
    }

    #[test]
    fn explicit_extensions_are_forward_compatible_but_untrusted() {
        let mut envelope = valid();
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("acme/context-hint".into(), serde_json::json!({"rank": 1}));
        envelope.extensions = ExtensionFields::new(fields).unwrap();
        envelope.validate().unwrap();

        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: ContextEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, envelope);
        assert_eq!(decoded.extensions.as_map()["acme/context-hint"]["rank"], 1);

        let mut legacy_json = serde_json::to_value(envelope).unwrap();
        legacy_json.as_object_mut().unwrap().remove("extensions");
        let legacy: ContextEnvelope = serde_json::from_value(legacy_json).unwrap();
        assert!(legacy.extensions.as_map().is_empty());
    }
}

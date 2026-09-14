//! Versioned context-pack contracts for isolated workers.
//!
//! A pack joins an agent-facing [`ContextEnvelope`] with the exact scope,
//! host-issued capability description and optional structured handoff that
//! produced it. The envelope is still a projection: this boundary does not
//! accept graph facts, authenticate a grant or grant GraphWriter authority.

use graph_domain as domain;
use serde::{Deserialize, Serialize};

use crate::{ExtensionFields, FactAssertion, ProjectRef, ProtocolError, context::ContextEnvelope};

pub const CONTEXT_PACK_SCHEMA_VERSION: &str = "project-graph/context-pack/v1";

/// Repository/worktree and graph snapshot selector for one pack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeSelector {
    pub project: ProjectRef,
    pub graph_version: String,
    pub paths: Vec<String>,
    pub node_ids: Vec<String>,
    pub max_depth: u16,
}

impl ScopeSelector {
    pub fn try_into_domain(self) -> Result<domain::ScopeSelector, ProtocolError> {
        Ok(domain::ScopeSelector::new(
            self.project.into(),
            self.graph_version,
            self.paths,
            self.node_ids,
            self.max_depth,
        )?)
    }
}

impl From<&domain::ScopeSelector> for ScopeSelector {
    fn from(scope: &domain::ScopeSelector) -> Self {
        Self {
            project: scope.project().into(),
            graph_version: scope.graph_version().into(),
            paths: scope.paths().into(),
            node_ids: scope.node_ids().into(),
            max_depth: scope.max_depth(),
        }
    }
}

/// Host-issued capability metadata. It is not authentication or execution
/// permission until a later runtime admission layer verifies it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityGrant {
    pub grant_id: String,
    pub issuer: String,
    pub subject: String,
    pub role: String,
    pub scope: ScopeSelector,
    pub capabilities: Vec<String>,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub parent_grant_id: Option<String>,
}

impl CapabilityGrant {
    pub fn try_into_domain(self) -> Result<domain::CapabilityGrant, ProtocolError> {
        Ok(domain::CapabilityGrant::new(
            self.grant_id,
            self.issuer,
            self.subject,
            self.role,
            self.scope.try_into_domain()?,
            self.capabilities
                .into_iter()
                .map(|value| domain::Capability::from_str(&value))
                .collect::<Result<Vec<_>, _>>()?,
            self.issued_at_ms,
            self.expires_at_ms,
            self.parent_grant_id,
        )?)
    }
}

impl From<&domain::CapabilityGrant> for CapabilityGrant {
    fn from(grant: &domain::CapabilityGrant) -> Self {
        Self {
            grant_id: grant.grant_id().into(),
            issuer: grant.issuer().into(),
            subject: grant.subject().into(),
            role: grant.role().into(),
            scope: grant.scope().into(),
            capabilities: grant
                .capabilities()
                .iter()
                .map(|capability| capability.as_str().into())
                .collect(),
            issued_at_ms: grant.issued_at_ms(),
            expires_at_ms: grant.expires_at_ms(),
            parent_grant_id: grant.parent_grant_id().map(Into::into),
        }
    }
}

/// A reasoned gap in worker understanding. Unknowns are data, not an
/// instruction to broaden the worker's scope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnknownClaim {
    pub code: String,
    pub reason: String,
    pub subject: Option<String>,
    pub path: Option<String>,
    pub line: Option<u32>,
}

impl UnknownClaim {
    fn try_into_domain(self) -> Result<domain::UnknownClaim, ProtocolError> {
        Ok(domain::UnknownClaim::new(
            self.code,
            self.reason,
            self.subject,
            self.path,
            self.line,
        )?)
    }
}

impl From<&domain::UnknownClaim> for UnknownClaim {
    fn from(unknown: &domain::UnknownClaim) -> Self {
        Self {
            code: unknown.code().into(),
            reason: unknown.reason().into(),
            subject: unknown.subject().map(Into::into),
            path: unknown.path().map(Into::into),
            line: unknown.line(),
        }
    }
}

/// Worker claims are split by support level but remain candidate-only in both
/// sections. The graph writer owns all acceptance transitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimBundle {
    pub bundle_id: String,
    pub project: ProjectRef,
    pub graph_version: String,
    pub scope: ScopeSelector,
    pub verified: Vec<FactAssertion>,
    pub heuristic: Vec<FactAssertion>,
    pub unknowns: Vec<UnknownClaim>,
}

impl ClaimBundle {
    pub fn try_into_domain(self) -> Result<domain::ClaimBundle, ProtocolError> {
        Ok(domain::ClaimBundle::new(
            self.bundle_id,
            self.project.into(),
            self.graph_version,
            self.scope.try_into_domain()?,
            self.verified
                .into_iter()
                .map(FactAssertion::try_into_domain)
                .collect::<Result<Vec<_>, _>>()?,
            self.heuristic
                .into_iter()
                .map(FactAssertion::try_into_domain)
                .collect::<Result<Vec<_>, _>>()?,
            self.unknowns
                .into_iter()
                .map(UnknownClaim::try_into_domain)
                .collect::<Result<Vec<_>, _>>()?,
        )?)
    }
}

impl From<&domain::ClaimBundle> for ClaimBundle {
    fn from(bundle: &domain::ClaimBundle) -> Self {
        Self {
            bundle_id: bundle.bundle_id().into(),
            project: bundle.project().into(),
            graph_version: bundle.graph_version().into(),
            scope: bundle.scope().into(),
            verified: bundle.verified().iter().map(Into::into).collect(),
            heuristic: bundle.heuristic().iter().map(Into::into).collect(),
            unknowns: bundle.unknowns().iter().map(Into::into).collect(),
        }
    }
}

/// Compact continuation payload for another worker. It carries a bounded
/// context reference and candidate claims, never a raw transcript or a new
/// capability/lease/worker-tree instruction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handoff {
    pub handoff_id: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub project: ProjectRef,
    pub graph_version: String,
    pub scope: ScopeSelector,
    pub context_ref: String,
    pub claims: ClaimBundle,
    pub expires_at_ms: u64,
}

impl Handoff {
    pub fn try_into_domain(self) -> Result<domain::Handoff, ProtocolError> {
        Ok(domain::Handoff::new(
            self.handoff_id,
            self.sender,
            self.recipient,
            self.project.into(),
            self.graph_version,
            self.scope.try_into_domain()?,
            self.context_ref,
            self.claims.try_into_domain()?,
            self.expires_at_ms,
        )?)
    }
}

impl From<&domain::Handoff> for Handoff {
    fn from(handoff: &domain::Handoff) -> Self {
        Self {
            handoff_id: handoff.handoff_id().into(),
            sender: handoff.sender().into(),
            recipient: handoff.recipient().map(Into::into),
            project: handoff.project().into(),
            graph_version: handoff.graph_version().into(),
            scope: handoff.scope().into(),
            context_ref: handoff.context_ref().into(),
            claims: handoff.claims().into(),
            expires_at_ms: handoff.expires_at_ms(),
        }
    }
}

/// Origins of content carried by a pack. No trusted variant exists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UntrustedContentLabel {
    pub origins: Vec<String>,
}

impl UntrustedContentLabel {
    fn try_into_domain(self) -> Result<domain::UntrustedContentLabel, ProtocolError> {
        Ok(domain::UntrustedContentLabel::new(
            self.origins
                .into_iter()
                .map(|value| domain::UntrustedContentOrigin::from_str(&value))
                .collect::<Result<Vec<_>, _>>()?,
        )?)
    }
}

impl From<&domain::UntrustedContentLabel> for UntrustedContentLabel {
    fn from(label: &domain::UntrustedContentLabel) -> Self {
        Self {
            origins: label
                .origins()
                .iter()
                .map(|origin| origin.as_str().into())
                .collect(),
        }
    }
}

/// Wire representation of a complete context pack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPack {
    pub schema_version: String,
    pub pack_id: String,
    pub operation: String,
    pub envelope: ContextEnvelope,
    pub scope: ScopeSelector,
    pub grant: CapabilityGrant,
    pub handoff: Option<Handoff>,
    pub untrusted_content: UntrustedContentLabel,
    #[serde(default)]
    pub extensions: ExtensionFields,
}

impl ContextPack {
    /// Validate every nested contract and all snapshot/scope bindings without
    /// creating graph authority. This is the protocol trust boundary.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.clone().into_domain_parts().map(|_| ())
    }

    /// Convert the metadata portion to domain types after validating the full
    /// envelope. The envelope remains available through the wire object or
    /// `try_into_domain_with_envelope`.
    pub fn try_into_domain(self) -> Result<domain::ContextPack, ProtocolError> {
        self.into_domain_parts().map(|(pack, _envelope)| pack)
    }

    /// Preserve both the validated domain metadata and the agent-facing
    /// projection at the process boundary.
    pub fn try_into_domain_with_envelope(
        self,
    ) -> Result<(domain::ContextPack, ContextEnvelope), ProtocolError> {
        let envelope = self.envelope.clone();
        let (pack, _) = self.into_domain_parts()?;
        Ok((pack, envelope))
    }

    fn into_domain_parts(self) -> Result<(domain::ContextPack, ContextEnvelope), ProtocolError> {
        if self.schema_version != CONTEXT_PACK_SCHEMA_VERSION {
            return Err(ProtocolError::UnsupportedContextPackSchema(
                self.schema_version,
            ));
        }
        self.extensions.validate()?;
        self.envelope.validate()?;

        let scope = self.scope.try_into_domain()?;
        let envelope_project: domain::ProjectRef = self.envelope.project.clone().into();
        if scope.project() != &envelope_project
            || scope.graph_version() != self.envelope.graph_version
        {
            return Err(ProtocolError::InvalidContextPack(
                "envelope and pack scope use different project or graph version".into(),
            ));
        }

        let operation = domain::ContextOperation::from_str(&self.operation)?;
        let grant = self.grant.try_into_domain()?;
        let handoff = self.handoff.map(Handoff::try_into_domain).transpose()?;
        let untrusted_content = self.untrusted_content.try_into_domain()?;
        let pack = domain::ContextPack::new(
            self.pack_id,
            operation,
            scope,
            grant,
            handoff,
            untrusted_content,
        )?;
        Ok((pack, self.envelope))
    }
}

#[cfg(test)]
fn valid_project() -> ProjectRef {
    ProjectRef {
        repository_id: "repo".into(),
        worktree_id: "main".into(),
        git_head: "head".into(),
        working_tree_fingerprint: "tree".into(),
        config_hash: "config".into(),
        ignore_policy_version: "ignore-v1".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ExtensionFields,
        context::{ContextBudget, ContextView, Coverage, CoverageStatus},
    };

    fn project() -> ProjectRef {
        valid_project()
    }

    fn scope() -> ScopeSelector {
        ScopeSelector {
            project: project(),
            graph_version: "graph-v1".into(),
            paths: vec!["src/auth".into()],
            node_ids: vec!["symbol:auth.login".into()],
            max_depth: 4,
        }
    }

    fn envelope() -> ContextEnvelope {
        ContextEnvelope {
            schema_version: crate::context::CONTEXT_SCHEMA_VERSION.into(),
            project: project(),
            graph_version: "graph-v1".into(),
            revision: "head".into(),
            view: ContextView::Flow,
            query: "how does login reach storage".into(),
            source_is_untrusted: true,
            nodes: vec![],
            edges: vec![],
            code_slices: vec![],
            evidence: vec![],
            coverage: Coverage {
                status: CoverageStatus::Partial,
                unknowns: vec![],
            },
            budget: ContextBudget {
                max_nodes: 10,
                max_edges: 10,
                max_source_ranges: 10,
                max_source_bytes: 10_000,
                max_traversal_depth: 4,
                max_serialized_bytes: 20_000,
                max_characters: 16_000,
                max_tokens: Some(4_000),
                tokenizer: Some("test-tokenizer-v1".into()),
                emitted_nodes: 0,
                emitted_edges: 0,
                emitted_source_ranges: 0,
                emitted_source_bytes: 0,
                emitted_serialized_bytes: 0,
                emitted_characters: 0,
                emitted_tokens: Some(0),
                truncated: false,
            },
            extensions: ExtensionFields::empty(),
        }
    }

    fn grant() -> CapabilityGrant {
        CapabilityGrant {
            grant_id: "grant-1".into(),
            issuer: "host".into(),
            subject: "worker-1".into(),
            role: "flow-tracer".into(),
            scope: scope(),
            capabilities: vec![
                "read_graph".into(),
                "read_source".into(),
                "submit_claims".into(),
            ],
            issued_at_ms: 10,
            expires_at_ms: 20,
            parent_grant_id: None,
        }
    }

    fn valid_pack() -> ContextPack {
        ContextPack {
            schema_version: CONTEXT_PACK_SCHEMA_VERSION.into(),
            pack_id: "pack-1".into(),
            operation: "trace".into(),
            envelope: envelope(),
            scope: scope(),
            grant: grant(),
            handoff: None,
            untrusted_content: UntrustedContentLabel {
                origins: vec!["source".into(), "tool_output".into()],
            },
            extensions: ExtensionFields::empty(),
        }
    }

    #[test]
    fn strict_pack_round_trips_and_preserves_the_envelope() {
        let wire = valid_pack();
        let first = serde_json::to_string(&wire).unwrap();
        let second = serde_json::to_string(&wire).unwrap();
        assert_eq!(first, second);

        let decoded: ContextPack = serde_json::from_str(&first).unwrap();
        let (domain, decoded_envelope) = decoded.try_into_domain_with_envelope().unwrap();
        assert_eq!(domain.pack_id(), "pack-1");
        assert_eq!(domain.scope().paths(), &["src/auth"]);
        assert_eq!(decoded_envelope, envelope());
    }

    #[test]
    fn unknown_fields_and_missing_untrusted_label_fail_closed() {
        let mut value = serde_json::to_value(valid_pack()).unwrap();
        value["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ContextPack>(value).is_err());

        let mut value = serde_json::to_value(valid_pack()).unwrap();
        value.as_object_mut().unwrap().remove("untrusted_content");
        assert!(serde_json::from_value::<ContextPack>(value).is_err());
    }

    #[test]
    fn future_pack_schema_is_rejected_before_domain_conversion() {
        let mut pack = valid_pack();
        pack.schema_version = "project-graph/context-pack/v2".into();
        assert!(matches!(
            pack.try_into_domain(),
            Err(ProtocolError::UnsupportedContextPackSchema(_))
        ));
    }

    #[test]
    fn mixed_snapshot_and_capability_duplicates_are_rejected() {
        let mut mixed = valid_pack();
        mixed.envelope.graph_version = "graph-old".into();
        assert!(matches!(
            mixed.try_into_domain(),
            Err(ProtocolError::InvalidContextPack(_))
        ));

        let mut duplicate = valid_pack();
        duplicate.grant.capabilities.push("read_graph".into());
        assert!(duplicate.try_into_domain().is_err());

        let mut widened_grant = valid_pack();
        widened_grant.grant.scope.paths = vec!["src/other".into()];
        assert!(widened_grant.try_into_domain().is_err());
    }

    #[test]
    fn accepted_claims_cannot_cross_the_pack_boundary() {
        let mut pack = valid_pack();
        pack.grant.capabilities = vec!["submit_claims".into()];
        pack.handoff = Some(Handoff {
            handoff_id: "handoff-1".into(),
            sender: "worker-1".into(),
            recipient: Some("worker-2".into()),
            project: project(),
            graph_version: "graph-v1".into(),
            scope: scope(),
            context_ref: "context-1".into(),
            claims: ClaimBundle {
                bundle_id: "bundle-1".into(),
                project: project(),
                graph_version: "graph-v1".into(),
                scope: scope(),
                verified: vec![FactAssertion {
                    schema_version: crate::SCHEMA_VERSION,
                    assertion_id: "fact-1".into(),
                    project: project(),
                    graph_version: "graph-v1".into(),
                    subject: "symbol:auth.login".into(),
                    predicate: "calls".into(),
                    object: "symbol:auth.store".into(),
                    claim_state: domain::ClaimState::Accepted.as_str().into(),
                    assertion_kind: "resolver".into(),
                    evidence: vec![crate::EvidenceRef {
                        id: "source-1".into(),
                        kind: "source".into(),
                    }],
                    producer: crate::Producer {
                        analyzer: "resolver".into(),
                        version: "v1".into(),
                        run_id: "run-1".into(),
                    },
                    decision: None,
                }],
                heuristic: vec![],
                unknowns: vec![],
            },
            expires_at_ms: 100,
        });
        assert!(matches!(
            pack.try_into_domain(),
            Err(ProtocolError::InvalidFact(_))
        ));
    }

    #[test]
    fn invalid_untrusted_origin_is_rejected() {
        let mut pack = valid_pack();
        pack.untrusted_content.origins = vec!["trusted".into()];
        assert!(pack.try_into_domain().is_err());
    }
}

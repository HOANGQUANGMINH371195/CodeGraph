//! Versioned fact-assertion contracts.
//!
//! A wire submission is deliberately candidate-only. Persisted records may
//! carry a terminal state for rehydration, but accepting one from an external
//! payload is never allowed through `try_into_domain`.

use graph_domain as domain;
use serde::{Deserialize, Serialize};

use crate::{ProjectRef, ProtocolError, SCHEMA_VERSION, require_current_schema};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub id: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    pub analyzer: String,
    pub version: String,
    pub run_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationReceipt {
    pub schema_version: u32,
    pub receipt_id: String,
    pub assertion_id: String,
    pub project: ProjectRef,
    pub graph_version: String,
    pub evidence: Vec<EvidenceRef>,
    pub verifier: String,
    pub verifier_version: String,
    pub verified_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClaimDecision {
    Accepted {
        receipt_id: String,
        verifier: String,
        verifier_version: String,
        decided_at_ms: i64,
    },
    Rejected {
        reason: String,
    },
    Superseded {
        replacement_id: String,
        reason: String,
    },
    Expired {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactAssertion {
    pub schema_version: u32,
    pub assertion_id: String,
    pub project: ProjectRef,
    pub graph_version: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub claim_state: String,
    pub assertion_kind: String,
    pub evidence: Vec<EvidenceRef>,
    pub producer: Producer,
    pub decision: Option<ClaimDecision>,
}

impl FactAssertion {
    /// Convert an external submission. The submission must be a candidate;
    /// accepted/terminal state is only created by the domain GraphWriter.
    pub fn try_into_domain(self) -> Result<domain::FactAssertion, ProtocolError> {
        require_current_schema(self.schema_version)?;
        if self.claim_state != domain::ClaimState::Candidate.as_str() || self.decision.is_some() {
            return Err(ProtocolError::InvalidFact(
                "wire fact assertions must be candidate-only",
            ));
        }
        candidate_from_wire(self)
    }

    /// Convert a trusted persisted row after the store selected it by its
    /// complete scope. This is not an acceptance path.
    pub fn try_into_persisted_domain(self) -> Result<domain::FactAssertion, ProtocolError> {
        require_current_schema(self.schema_version)?;
        let state = domain::ClaimState::from_str(&self.claim_state)?;
        let decision = self.decision.map(claim_decision_to_domain).transpose()?;
        Ok(domain::FactAssertion::from_persistence(
            self.assertion_id,
            self.project.into(),
            self.graph_version,
            self.subject,
            self.predicate,
            self.object,
            state,
            domain::AssertionKind::from_str(&self.assertion_kind)?,
            self.evidence
                .into_iter()
                .map(evidence_to_domain)
                .collect::<Result<Vec<_>, _>>()?,
            domain::Producer::new(
                self.producer.analyzer,
                self.producer.version,
                self.producer.run_id,
            )?,
            decision,
        )?)
    }
}

impl From<&domain::FactAssertion> for FactAssertion {
    fn from(value: &domain::FactAssertion) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            assertion_id: value.assertion_id().into(),
            project: value.project().into(),
            graph_version: value.graph_version().into(),
            subject: value.subject().into(),
            predicate: value.predicate().into(),
            object: value.object().into(),
            claim_state: value.claim_state().as_str().into(),
            assertion_kind: value.assertion_kind().as_str().into(),
            evidence: value
                .evidence()
                .iter()
                .map(|reference| EvidenceRef {
                    id: reference.id().into(),
                    kind: reference.kind().as_str().into(),
                })
                .collect(),
            producer: Producer {
                analyzer: value.producer().analyzer().into(),
                version: value.producer().version().into(),
                run_id: value.producer().run_id().into(),
            },
            decision: value.decision().map(claim_decision_from_domain),
        }
    }
}

impl VerificationReceipt {
    pub fn try_into_domain(self) -> Result<domain::VerificationReceipt, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(domain::VerificationReceipt::new(
            self.receipt_id,
            self.assertion_id,
            self.project.into(),
            self.graph_version,
            self.evidence
                .into_iter()
                .map(evidence_to_domain)
                .collect::<Result<Vec<_>, _>>()?,
            self.verifier,
            self.verifier_version,
            self.verified_at_ms,
        )?)
    }
}

impl From<&domain::VerificationReceipt> for VerificationReceipt {
    fn from(value: &domain::VerificationReceipt) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            receipt_id: value.receipt_id().into(),
            assertion_id: value.assertion_id().into(),
            project: value.project().into(),
            graph_version: value.graph_version().into(),
            evidence: value
                .evidence()
                .iter()
                .map(|reference| EvidenceRef {
                    id: reference.id().into(),
                    kind: reference.kind().as_str().into(),
                })
                .collect(),
            verifier: value.verifier().into(),
            verifier_version: value.verifier_version().into(),
            verified_at_ms: value.verified_at_ms(),
        }
    }
}

fn candidate_from_wire(value: FactAssertion) -> Result<domain::FactAssertion, ProtocolError> {
    Ok(domain::FactAssertion::candidate(
        value.assertion_id,
        value.project.into(),
        value.graph_version,
        value.subject,
        value.predicate,
        value.object,
        domain::AssertionKind::from_str(&value.assertion_kind)?,
        value
            .evidence
            .into_iter()
            .map(evidence_to_domain)
            .collect::<Result<Vec<_>, _>>()?,
        domain::Producer::new(
            value.producer.analyzer,
            value.producer.version,
            value.producer.run_id,
        )?,
    )?)
}

fn evidence_to_domain(value: EvidenceRef) -> Result<domain::EvidenceRef, ProtocolError> {
    Ok(domain::EvidenceRef::new(
        value.id,
        domain::EvidenceKind::from_str(&value.kind)?,
    )?)
}

fn claim_decision_from_domain(value: &domain::ClaimDecision) -> ClaimDecision {
    match value {
        domain::ClaimDecision::Accepted {
            receipt_id,
            verifier,
            verifier_version,
            decided_at_ms,
        } => ClaimDecision::Accepted {
            receipt_id: receipt_id.into(),
            verifier: verifier.into(),
            verifier_version: verifier_version.into(),
            decided_at_ms: *decided_at_ms,
        },
        domain::ClaimDecision::Rejected { reason } => ClaimDecision::Rejected {
            reason: reason.into(),
        },
        domain::ClaimDecision::Superseded {
            replacement_id,
            reason,
        } => ClaimDecision::Superseded {
            replacement_id: replacement_id.into(),
            reason: reason.into(),
        },
        domain::ClaimDecision::Expired { reason } => ClaimDecision::Expired {
            reason: reason.into(),
        },
    }
}

fn claim_decision_to_domain(value: ClaimDecision) -> Result<domain::ClaimDecision, ProtocolError> {
    Ok(match value {
        ClaimDecision::Accepted {
            receipt_id,
            verifier,
            verifier_version,
            decided_at_ms,
        } => domain::ClaimDecision::Accepted {
            receipt_id,
            verifier,
            verifier_version,
            decided_at_ms,
        },
        ClaimDecision::Rejected { reason } => domain::ClaimDecision::Rejected { reason },
        ClaimDecision::Superseded {
            replacement_id,
            reason,
        } => domain::ClaimDecision::Superseded {
            replacement_id,
            reason,
        },
        ClaimDecision::Expired { reason } => domain::ClaimDecision::Expired { reason },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_domain::{AssertionKind, EvidenceKind, EvidenceRef, Producer};

    fn project() -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "worktree".into(),
            git_head: "head".into(),
            working_tree_fingerprint: "tree".into(),
            config_hash: "config".into(),
            ignore_policy_version: "1".into(),
        }
    }

    fn candidate() -> domain::FactAssertion {
        domain::FactAssertion::candidate(
            "fact-1".into(),
            project().into(),
            "graph-1".into(),
            "a".into(),
            "calls".into(),
            "b".into(),
            AssertionKind::Parser,
            vec![EvidenceRef::new("source-1".into(), EvidenceKind::Source).unwrap()],
            Producer::new("parser".into(), "v1".into(), "run-1".into()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn candidate_round_trip_is_strict_and_accepted_wire_is_not_submission() {
        let expected = candidate();
        let wire = FactAssertion::from(&expected);
        assert_eq!(wire.clone().try_into_domain().unwrap(), expected);
        let accepted = domain::GraphWriter
            .accept(
                expected.clone(),
                domain::VerificationReceipt::new(
                    "receipt-1".into(),
                    "fact-1".into(),
                    expected.project().clone(),
                    "graph-1".into(),
                    expected.evidence().to_vec(),
                    "verifier".into(),
                    "v1".into(),
                    1,
                )
                .unwrap(),
            )
            .unwrap()
            .into_assertion();
        let accepted_wire = FactAssertion::from(&accepted);
        assert!(accepted_wire.clone().try_into_domain().is_err());
        assert_eq!(accepted_wire.try_into_persisted_domain().unwrap(), accepted);
    }

    #[test]
    fn unknown_fields_and_future_schema_are_rejected() {
        let wire = FactAssertion::from(&candidate());
        let json = serde_json::to_value(&wire).unwrap();
        let mut extra = json.clone();
        extra["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<FactAssertion>(extra).is_err());
        let mut future = wire;
        future.schema_version = SCHEMA_VERSION + 1;
        assert!(future.try_into_domain().is_err());
    }
}

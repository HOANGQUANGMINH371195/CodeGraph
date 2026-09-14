//! Durable fact assertions and verified decisions.
//!
//! Assertions and decisions are append-only rows. The accepted projection is
//! reconstructed only after the complete project/graph scope, evidence
//! registrations and verifier receipt have been checked.

use graph_application::FactAssertionRepository;
use graph_domain::{
    AcceptedFact, ClaimDecision, ClaimState, EvidenceKind, FactAssertion, GraphWriter, ProjectRef,
    VerificationReceipt,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::{Store, StoreError, domain_corruption};

impl FactAssertionRepository for Store {
    type Error = StoreError;

    fn record_fact_candidate(&mut self, candidate: &FactAssertion) -> Result<bool, StoreError> {
        if candidate.claim_state() != ClaimState::Candidate {
            return Err(StoreError::Invalid("only candidate facts may be submitted"));
        }
        candidate
            .project()
            .validate()
            .map_err(|_| StoreError::Invalid("invalid fact project scope"))?;
        let project = project_json(candidate.project())?;
        let descriptor = serde_json::to_string(&graph_protocol::FactAssertion::from(candidate))?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            include_str!("sql/insert_fact_assertion.sql"),
            params![
                candidate.assertion_id(),
                project,
                candidate.graph_version(),
                descriptor
            ],
        )?;
        if inserted == 0 {
            let equal: bool = tx.query_row(
                include_str!("sql/match_fact_assertion.sql"),
                params![
                    candidate.assertion_id(),
                    project,
                    candidate.graph_version(),
                    descriptor
                ],
                |row| row.get(0),
            )?;
            if !equal {
                return Err(StoreError::Conflict);
            }
        }
        tx.commit()?;
        Ok(inserted == 1)
    }

    fn record_accepted_fact(
        &mut self,
        accepted: &AcceptedFact,
        receipt: &VerificationReceipt,
    ) -> Result<bool, StoreError> {
        let fact = accepted.assertion();
        verify_acceptance_binding(fact, receipt)?;
        let project = project_json(fact.project())?;
        let mut candidate_wire = graph_protocol::FactAssertion::from(fact);
        candidate_wire.claim_state = ClaimState::Candidate.as_str().into();
        candidate_wire.decision = None;
        let candidate_domain = candidate_wire
            .clone()
            .try_into_domain()
            .map_err(|error| StoreError::Corrupt(error.to_string()))?;
        verify_evidence_scope(
            &self.0,
            candidate_domain.evidence(),
            &project,
            candidate_domain.graph_version(),
        )?;
        let descriptor = serde_json::to_string(&candidate_wire)?;
        let decision = fact
            .decision()
            .cloned()
            .ok_or(StoreError::Invalid("accepted fact has no decision"))?;
        if !matches!(decision, ClaimDecision::Accepted { .. }) {
            return Err(StoreError::Invalid(
                "accepted fact has a non-accepted decision",
            ));
        }
        let decision_json = serde_json::to_string(&decision_wire(&decision))?;
        let receipt_json =
            serde_json::to_string(&graph_protocol::VerificationReceipt::from(receipt))?;

        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted_fact = insert_or_match_fact(
            &tx,
            fact.assertion_id(),
            &project,
            fact.graph_version(),
            &descriptor,
        )?;
        let inserted_decision = tx.execute(
            include_str!("sql/insert_fact_decision.sql"),
            params![
                fact.assertion_id(),
                project,
                fact.graph_version(),
                decision_json,
                receipt_json
            ],
        )?;
        if inserted_decision == 0 {
            let equal: bool = tx.query_row(
                include_str!("sql/match_fact_decision.sql"),
                params![
                    fact.assertion_id(),
                    project,
                    fact.graph_version(),
                    serde_json::to_string(&decision_wire(&decision))?,
                    serde_json::to_string(&graph_protocol::VerificationReceipt::from(receipt))?
                ],
                |row| row.get(0),
            )?;
            if !equal {
                return Err(StoreError::Conflict);
            }
        }
        tx.commit()?;
        Ok(inserted_fact || inserted_decision == 1)
    }

    fn fact_assertion(
        &self,
        id: &str,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<Option<FactAssertion>, StoreError> {
        validate_lookup(id, project, graph_version)?;
        let project = project_json(project)?;
        let raw = self
            .0
            .query_row(
                include_str!("sql/select_fact_assertion.sql"),
                params![id, project, graph_version],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;
        raw.map(|(descriptor, decision, receipt)| {
            decode_fact_row(&descriptor, decision.as_deref(), receipt.as_deref())
        })
        .transpose()
    }

    fn accepted_facts(
        &self,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<Vec<FactAssertion>, StoreError> {
        project
            .validate()
            .map_err(|_| StoreError::Invalid("invalid fact project scope"))?;
        if graph_version.trim().is_empty() {
            return Err(StoreError::Invalid("fact graph version is required"));
        }
        let project_json = project_json(project)?;
        let mut statement = self.0.prepare(
            "SELECT a.descriptor,d.decision,d.receipt
             FROM fact_assertions AS a
             JOIN fact_decisions AS d
               ON d.assertion_id=a.assertion_id
              AND d.project=a.project
              AND d.graph_version=a.graph_version
             WHERE a.project=?1 AND a.graph_version=?2
             ORDER BY a.assertion_id",
        )?;
        let mut rows = statement.query(params![project_json, graph_version])?;
        let mut facts = Vec::new();
        while let Some(row) = rows.next()? {
            if facts.len() >= 10_000 {
                return Err(StoreError::Corrupt("fact projection exceeds limit".into()));
            }
            let fact = decode_fact_row(
                &row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.as_deref(),
                row.get::<_, Option<String>>(2)?.as_deref(),
            )?;
            if fact.claim_state() == ClaimState::Accepted {
                facts.push(fact);
            }
        }
        Ok(facts)
    }
}

fn project_json(project: &ProjectRef) -> Result<String, StoreError> {
    Ok(serde_json::to_string(&graph_protocol::ProjectRef::from(
        project,
    ))?)
}

fn validate_lookup(id: &str, project: &ProjectRef, graph_version: &str) -> Result<(), StoreError> {
    project
        .validate()
        .map_err(|_| StoreError::Invalid("invalid fact project scope"))?;
    if id.trim().is_empty() || graph_version.trim().is_empty() {
        return Err(StoreError::Invalid(
            "fact id and graph version are required",
        ));
    }
    Ok(())
}

fn insert_or_match_fact(
    tx: &Transaction<'_>,
    id: &str,
    project: &str,
    graph_version: &str,
    descriptor: &str,
) -> Result<bool, StoreError> {
    let inserted = tx.execute(
        include_str!("sql/insert_fact_assertion.sql"),
        params![id, project, graph_version, descriptor],
    )?;
    if inserted == 0 {
        let equal: bool = tx.query_row(
            include_str!("sql/match_fact_assertion.sql"),
            params![id, project, graph_version, descriptor],
            |row| row.get(0),
        )?;
        if !equal {
            return Err(StoreError::Conflict);
        }
    }
    Ok(inserted == 1)
}

fn verify_acceptance_binding(
    fact: &FactAssertion,
    receipt: &VerificationReceipt,
) -> Result<(), StoreError> {
    if fact.claim_state() != ClaimState::Accepted
        || receipt.assertion_id() != fact.assertion_id()
        || receipt.project() != fact.project()
        || receipt.graph_version() != fact.graph_version()
        || receipt.evidence() != fact.evidence()
    {
        return Err(StoreError::Invalid(
            "accepted fact and verification receipt do not bind",
        ));
    }
    match fact.decision() {
        Some(ClaimDecision::Accepted {
            receipt_id,
            verifier,
            verifier_version,
            decided_at_ms,
        }) if receipt_id == receipt.receipt_id()
            && verifier == receipt.verifier()
            && verifier_version == receipt.verifier_version()
            && *decided_at_ms == receipt.verified_at_ms() =>
        {
            Ok(())
        }
        _ => Err(StoreError::Invalid(
            "accepted fact decision does not bind verification receipt",
        )),
    }
}

fn verify_evidence_scope(
    connection: &Connection,
    evidence: &[graph_domain::EvidenceRef],
    project: &str,
    graph_version: &str,
) -> Result<(), StoreError> {
    for reference in evidence {
        let registered = match reference.kind() {
            EvidenceKind::Source => connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM source_evidence WHERE id=?1 AND project=?2 AND graph_version=?3)",
                params![reference.id(), project, graph_version],
                |row| row.get(0),
            )?,
            EvidenceKind::Artifact => connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM artifacts WHERE id=?1 AND project=?2 AND graph_version=?3)",
                params![reference.id(), project, graph_version],
                |row| row.get(0),
            )?,
            EvidenceKind::AnalysisRun => connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM analysis_runs WHERE id=?1 AND project=?2 AND graph_version=?3)",
                params![reference.id(), project, graph_version],
                |row| row.get(0),
            )?,
            EvidenceKind::RuntimeTrace | EvidenceKind::Document | EvidenceKind::Human => false,
        };
        if !registered {
            return Err(StoreError::Invalid(
                "fact evidence is missing or unsupported in this store",
            ));
        }
    }
    Ok(())
}

fn decision_wire(value: &ClaimDecision) -> graph_protocol::ClaimDecision {
    match value {
        ClaimDecision::Accepted {
            receipt_id,
            verifier,
            verifier_version,
            decided_at_ms,
        } => graph_protocol::ClaimDecision::Accepted {
            receipt_id: receipt_id.into(),
            verifier: verifier.into(),
            verifier_version: verifier_version.into(),
            decided_at_ms: *decided_at_ms,
        },
        ClaimDecision::Rejected { reason } => graph_protocol::ClaimDecision::Rejected {
            reason: reason.into(),
        },
        ClaimDecision::Superseded {
            replacement_id,
            reason,
        } => graph_protocol::ClaimDecision::Superseded {
            replacement_id: replacement_id.into(),
            reason: reason.into(),
        },
        ClaimDecision::Expired { reason } => graph_protocol::ClaimDecision::Expired {
            reason: reason.into(),
        },
    }
}

fn decode_fact_row(
    descriptor: &str,
    decision: Option<&str>,
    receipt: Option<&str>,
) -> Result<FactAssertion, StoreError> {
    let mut candidate: graph_protocol::FactAssertion = serde_json::from_str(descriptor)
        .map_err(|_| StoreError::Corrupt("invalid fact descriptor".into()))?;
    if candidate.claim_state != ClaimState::Candidate.as_str() || candidate.decision.is_some() {
        return Err(StoreError::Corrupt(
            "fact descriptor is not a candidate row".into(),
        ));
    }
    let candidate_domain = candidate
        .clone()
        .try_into_domain()
        .map_err(|error| StoreError::Corrupt(error.to_string()))?;
    let Some(decision) = decision else {
        if receipt.is_some() {
            return Err(StoreError::Corrupt("fact receipt without decision".into()));
        }
        return Ok(candidate_domain);
    };
    let decision: graph_protocol::ClaimDecision = serde_json::from_str(decision)
        .map_err(|_| StoreError::Corrupt("invalid fact decision".into()))?;
    let receipt =
        receipt.ok_or_else(|| StoreError::Corrupt("fact decision without receipt".into()))?;
    let receipt: graph_protocol::VerificationReceipt = serde_json::from_str(receipt)
        .map_err(|_| StoreError::Corrupt("invalid fact receipt".into()))?;
    candidate.claim_state = match &decision {
        graph_protocol::ClaimDecision::Accepted { .. } => ClaimState::Accepted.as_str(),
        graph_protocol::ClaimDecision::Rejected { .. } => ClaimState::Rejected.as_str(),
        graph_protocol::ClaimDecision::Superseded { .. } => ClaimState::Superseded.as_str(),
        graph_protocol::ClaimDecision::Expired { .. } => ClaimState::Expired.as_str(),
    }
    .into();
    candidate.decision = Some(decision);
    let fact = candidate
        .try_into_persisted_domain()
        .map_err(|error| StoreError::Corrupt(error.to_string()))?;
    let receipt = receipt
        .try_into_domain()
        .map_err(|error| StoreError::Corrupt(error.to_string()))?;
    if fact.claim_state() == ClaimState::Accepted {
        let expected = GraphWriter
            .accept(candidate_domain, receipt)
            .map_err(domain_corruption)?;
        if expected.assertion() != &fact {
            return Err(StoreError::Corrupt(
                "fact decision and receipt mismatch".into(),
            ));
        }
    }
    Ok(fact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_application::{AnalysisRepository, EvidenceRepository};
    use graph_domain::{AnalysisRun, AssertionKind, EvidenceRef, Producer, SourceEvidence};

    fn project() -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: "head".into(),
            working_tree_fingerprint: "tree".into(),
            config_hash: "config".into(),
            ignore_policy_version: "1".into(),
        }
    }

    fn run() -> AnalysisRun {
        AnalysisRun::new(
            "run-1".into(),
            project(),
            "graph-1".into(),
            "codegraph".into(),
            "v1".into(),
            "a".repeat(64),
            "b".repeat(64),
        )
        .unwrap()
    }

    fn source() -> SourceEvidence {
        SourceEvidence::new(
            "source-1".into(),
            project(),
            "graph-1".into(),
            "src/lib.rs".into(),
            "c".repeat(64),
            1,
            2,
            "run-1".into(),
        )
        .unwrap()
    }

    fn candidate(id: &str) -> FactAssertion {
        FactAssertion::candidate(
            id.into(),
            project(),
            "graph-1".into(),
            "symbol:a".into(),
            "calls".into(),
            "symbol:b".into(),
            AssertionKind::Resolver,
            vec![EvidenceRef::new("source-1".into(), EvidenceKind::Source).unwrap()],
            Producer::new("codegraph".into(), "v1".into(), "run-1".into()).unwrap(),
        )
        .unwrap()
    }

    fn receipt(fact: &FactAssertion) -> VerificationReceipt {
        VerificationReceipt::new(
            format!("receipt-{}", fact.assertion_id()),
            fact.assertion_id().into(),
            fact.project().clone(),
            fact.graph_version().into(),
            fact.evidence().to_vec(),
            "verifier".into(),
            "v1".into(),
            10,
        )
        .unwrap()
    }

    fn open_store() -> Store {
        let mut store = Store::open(":memory:").unwrap();
        store.record_analysis_run(&run()).unwrap();
        store.record_source(&source()).unwrap();
        store
    }

    #[test]
    fn candidate_replays_immutably_and_is_absent_from_accepted_view() {
        let mut store = open_store();
        let fact = candidate("fact-1");
        assert!(store.record_fact_candidate(&fact).unwrap());
        assert!(!store.record_fact_candidate(&fact).unwrap());
        assert_eq!(store.accepted_facts(&project(), "graph-1").unwrap(), vec![]);
        assert_eq!(
            store
                .fact_assertion("fact-1", &project(), "graph-1")
                .unwrap(),
            Some(fact.clone())
        );
        let mut changed = graph_protocol::FactAssertion::from(&fact);
        changed.object = "symbol:c".into();
        assert!(matches!(
            store.record_fact_candidate(&changed.try_into_domain().unwrap()),
            Err(StoreError::Conflict)
        ));
        assert!(
            store
                .0
                .execute("UPDATE fact_assertions SET descriptor='{}'", [])
                .is_err()
        );
        assert!(store.0.execute("DELETE FROM fact_assertions", []).is_err());
    }

    #[test]
    fn accepted_fact_requires_registered_scoped_evidence_and_is_replayable() {
        let mut store = open_store();
        let fact = candidate("fact-accepted");
        let receipt = receipt(&fact);
        let accepted = GraphWriter.accept(fact.clone(), receipt.clone()).unwrap();
        assert!(store.record_accepted_fact(&accepted, &receipt).unwrap());
        assert!(!store.record_accepted_fact(&accepted, &receipt).unwrap());
        let stored = store
            .fact_assertion("fact-accepted", &project(), "graph-1")
            .unwrap()
            .unwrap();
        assert_eq!(stored, accepted.assertion().clone());
        assert_eq!(
            store.accepted_facts(&project(), "graph-1").unwrap(),
            vec![stored]
        );
    }

    #[test]
    fn missing_or_foreign_evidence_cannot_create_an_accepted_projection() {
        let mut store = Store::open(":memory:").unwrap();
        let fact = candidate("fact-missing-evidence");
        let receipt = receipt(&fact);
        let accepted = GraphWriter.accept(fact, receipt.clone()).unwrap();
        assert!(matches!(
            store.record_accepted_fact(&accepted, &receipt),
            Err(StoreError::Invalid(_))
        ));
        assert_eq!(store.accepted_facts(&project(), "graph-1").unwrap(), vec![]);
    }
}

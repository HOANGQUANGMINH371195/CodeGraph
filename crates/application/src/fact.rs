//! Fact assertion application ports and the GraphWriter use case.
//!
//! The application layer coordinates the domain transition with durable
//! storage. Persistence adapters must still verify evidence scope before
//! committing the accepted projection.

use std::error::Error;

use graph_domain::{AcceptedFact, FactAssertion, GraphWriter, ProjectRef, VerificationReceipt};
use thiserror::Error as DeriveError;

/// Durable assertion ledger. Implementations must keep assertion rows and
/// decisions immutable and scope every read by the complete project snapshot.
pub trait FactAssertionRepository {
    type Error: Error + Send + Sync + 'static;

    fn record_fact_candidate(&mut self, candidate: &FactAssertion) -> Result<bool, Self::Error>;

    /// The adapter must persist the candidate and its verified decision in one
    /// transaction. The receipt is retained for audit and exact replay.
    fn record_accepted_fact(
        &mut self,
        accepted: &AcceptedFact,
        receipt: &VerificationReceipt,
    ) -> Result<bool, Self::Error>;

    fn fact_assertion(
        &self,
        id: &str,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<Option<FactAssertion>, Self::Error>;

    /// Normal graph projection: only accepted assertions are returned.
    fn accepted_facts(
        &self,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<Vec<FactAssertion>, Self::Error>;
}

#[derive(Debug, DeriveError)]
pub enum FactWriteError<E: Error + Send + Sync + 'static> {
    #[error(transparent)]
    Domain(#[from] graph_domain::DomainError),
    #[error(transparent)]
    Repository(E),
}

/// Register an untrusted producer candidate without granting it graph
/// authority. A candidate may be stored before its referenced evidence is
/// verified; only the next use case can create an accepted fact.
pub fn record_fact_candidate<R: FactAssertionRepository>(
    repository: &mut R,
    candidate: FactAssertion,
) -> Result<bool, FactWriteError<R::Error>> {
    if candidate.claim_state() != graph_domain::ClaimState::Candidate {
        return Err(FactWriteError::Domain(graph_domain::DomainError::Invalid(
            "only candidate facts may be submitted",
        )));
    }
    repository
        .record_fact_candidate(&candidate)
        .map_err(FactWriteError::Repository)
}

/// The sole application path from a candidate plus a verifier receipt to an
/// accepted fact. The returned marker is the only value accepted storage may
/// persist as an accepted projection.
pub fn accept_fact<R: FactAssertionRepository>(
    repository: &mut R,
    candidate: FactAssertion,
    receipt: VerificationReceipt,
) -> Result<AcceptedFact, FactWriteError<R::Error>> {
    let accepted = GraphWriter
        .accept(candidate, receipt.clone())
        .map_err(FactWriteError::Domain)?;
    repository
        .record_accepted_fact(&accepted, &receipt)
        .map_err(FactWriteError::Repository)?;
    Ok(accepted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_domain::{AssertionKind, EvidenceKind, EvidenceRef, Producer};

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

    fn candidate() -> FactAssertion {
        FactAssertion::candidate(
            "fact-1".into(),
            project(),
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

    fn receipt(fact: &FactAssertion) -> VerificationReceipt {
        VerificationReceipt::new(
            "receipt-1".into(),
            fact.assertion_id().into(),
            fact.project().clone(),
            fact.graph_version().into(),
            fact.evidence().to_vec(),
            "verifier".into(),
            "v1".into(),
            1,
        )
        .unwrap()
    }

    #[derive(Default)]
    struct Repository {
        accepted: Option<(AcceptedFact, VerificationReceipt)>,
    }

    impl FactAssertionRepository for Repository {
        type Error = std::io::Error;

        fn record_fact_candidate(&mut self, _: &FactAssertion) -> Result<bool, Self::Error> {
            Ok(true)
        }

        fn record_accepted_fact(
            &mut self,
            accepted: &AcceptedFact,
            receipt: &VerificationReceipt,
        ) -> Result<bool, Self::Error> {
            self.accepted = Some((accepted.clone(), receipt.clone()));
            Ok(true)
        }

        fn fact_assertion(
            &self,
            _: &str,
            _: &ProjectRef,
            _: &str,
        ) -> Result<Option<FactAssertion>, Self::Error> {
            Ok(None)
        }

        fn accepted_facts(
            &self,
            _: &ProjectRef,
            _: &str,
        ) -> Result<Vec<FactAssertion>, Self::Error> {
            Ok(vec![])
        }
    }

    #[test]
    fn graph_writer_use_case_only_persists_domain_accepted_marker() {
        let fact = candidate();
        let receipt = receipt(&fact);
        let mut repository = Repository::default();
        let accepted = accept_fact(&mut repository, fact.clone(), receipt.clone()).unwrap();
        assert_eq!(
            accepted.assertion().claim_state(),
            graph_domain::ClaimState::Accepted
        );
        let (stored, stored_receipt) = repository.accepted.unwrap();
        assert_eq!(stored.assertion(), accepted.assertion());
        assert_eq!(stored_receipt, receipt);
        assert!(record_fact_candidate(&mut Repository::default(), fact).is_ok());
    }
}

use crate::{DomainError, ProjectRef, validate_text};

const MAX_FACT_TEXT_BYTES: usize = 16 * 1024;
const MAX_FACT_EVIDENCE: usize = 64;

impl std::str::FromStr for AssertionKind {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_str(value)
    }
}

impl std::str::FromStr for EvidenceKind {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_str(value)
    }
}

impl std::str::FromStr for ClaimState {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_str(value)
    }
}

/// The closed set of producers that can create a fact candidate. The kind is
/// provenance metadata; it is never an acceptance decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssertionKind {
    Parser,
    Resolver,
    Cpg,
    RuntimeTrace,
    Document,
    Human,
    SecurityFinding,
}

impl AssertionKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parser => "parser",
            Self::Resolver => "resolver",
            Self::Cpg => "cpg",
            Self::RuntimeTrace => "runtime_trace",
            Self::Document => "document",
            Self::Human => "human",
            Self::SecurityFinding => "security_finding",
        }
    }

    /// Parses the canonical assertion kind.
    ///
    /// # Errors
    /// Returns an error for an unknown wire value.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value {
            "parser" => Ok(Self::Parser),
            "resolver" => Ok(Self::Resolver),
            "cpg" => Ok(Self::Cpg),
            "runtime_trace" => Ok(Self::RuntimeTrace),
            "document" => Ok(Self::Document),
            "human" => Ok(Self::Human),
            "security_finding" => Ok(Self::SecurityFinding),
            _ => Err(DomainError::Invalid("unknown assertion kind")),
        }
    }
}

/// Evidence categories are deliberately typed so a future verifier cannot
/// accidentally treat a trace, document or finding as source proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceKind {
    Source,
    Artifact,
    AnalysisRun,
    RuntimeTrace,
    Document,
    Human,
}

impl EvidenceKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Artifact => "artifact",
            Self::AnalysisRun => "analysis_run",
            Self::RuntimeTrace => "runtime_trace",
            Self::Document => "document",
            Self::Human => "human",
        }
    }

    /// Parses the canonical evidence kind.
    ///
    /// # Errors
    /// Returns an error for an unknown wire value.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value {
            "source" => Ok(Self::Source),
            "artifact" => Ok(Self::Artifact),
            "analysis_run" => Ok(Self::AnalysisRun),
            "runtime_trace" => Ok(Self::RuntimeTrace),
            "document" => Ok(Self::Document),
            "human" => Ok(Self::Human),
            _ => Err(DomainError::Invalid("unknown evidence kind")),
        }
    }
}

/// A typed reference to an immutable evidence record. It is a claim until a
/// verifier resolves it in the exact project and graph snapshot.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceRef {
    id: String,
    kind: EvidenceKind,
}

impl EvidenceRef {
    pub fn new(id: String, kind: EvidenceKind) -> Result<Self, DomainError> {
        validate_fact_text("evidence id", &id)?;
        Ok(Self { id, kind })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> EvidenceKind {
        self.kind
    }
}

/// Immutable producer identity. A run ID is provenance, not execution proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Producer {
    analyzer: String,
    version: String,
    run_id: String,
}

impl Producer {
    pub fn new(analyzer: String, version: String, run_id: String) -> Result<Self, DomainError> {
        validate_fact_text("producer analyzer", &analyzer)?;
        validate_fact_text("producer version", &version)?;
        validate_fact_text("producer run id", &run_id)?;
        Ok(Self {
            analyzer,
            version,
            run_id,
        })
    }

    pub fn analyzer(&self) -> &str {
        &self.analyzer
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }
}

/// The terminal decision recorded alongside a fact. A decision is not
/// inferred from confidence; it is produced by the graph-writer boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl ClaimDecision {
    pub fn state(&self) -> ClaimState {
        match self {
            Self::Accepted { .. } => ClaimState::Accepted,
            Self::Rejected { .. } => ClaimState::Rejected,
            Self::Superseded { .. } => ClaimState::Superseded,
            Self::Expired { .. } => ClaimState::Expired,
        }
    }
}

/// Trust state of a graph assertion. Only GraphWriter can create Accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimState {
    Candidate,
    Accepted,
    Rejected,
    Superseded,
    Expired,
}

impl ClaimState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::Expired => "expired",
        }
    }

    /// Parses a canonical, case-sensitive state name without normalization.
    /// Parsing `accepted` does not authorize accepting an assertion.
    ///
    /// # Errors
    /// Returns an error for any spelling other than `candidate`, `accepted`,
    /// `rejected`, `superseded`, or `expired`, including surrounding whitespace.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value {
            "candidate" => Ok(Self::Candidate),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "superseded" => Ok(Self::Superseded),
            "expired" => Ok(Self::Expired),
            _ => Err(DomainError::Invalid("unknown claim state")),
        }
    }
}

/// A verifier-owned receipt binding all evidence to one immutable candidate.
/// Its constructor creates metadata only; the state transition still occurs
/// exclusively through `GraphWriter::accept`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReceipt {
    receipt_id: String,
    assertion_id: String,
    project: ProjectRef,
    graph_version: String,
    evidence: Vec<EvidenceRef>,
    verifier: String,
    verifier_version: String,
    verified_at_ms: i64,
}

impl VerificationReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        receipt_id: String,
        assertion_id: String,
        project: ProjectRef,
        graph_version: String,
        evidence: Vec<EvidenceRef>,
        verifier: String,
        verifier_version: String,
        verified_at_ms: i64,
    ) -> Result<Self, DomainError> {
        project.validate()?;
        validate_fact_text("verification receipt", &receipt_id)?;
        validate_fact_text("verification assertion", &assertion_id)?;
        validate_fact_text("verification graph version", &graph_version)?;
        validate_fact_text("verifier", &verifier)?;
        validate_fact_text("verifier version", &verifier_version)?;
        if verified_at_ms < 0 {
            return Err(DomainError::Invalid("verification timestamp"));
        }
        let evidence = canonical_evidence(evidence)?;
        Ok(Self {
            receipt_id,
            assertion_id,
            project,
            graph_version,
            evidence,
            verifier,
            verifier_version,
            verified_at_ms,
        })
    }

    pub fn receipt_id(&self) -> &str {
        &self.receipt_id
    }

    pub fn assertion_id(&self) -> &str {
        &self.assertion_id
    }

    pub fn project(&self) -> &ProjectRef {
        &self.project
    }

    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }

    pub fn evidence(&self) -> &[EvidenceRef] {
        &self.evidence
    }

    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    pub fn verifier_version(&self) -> &str {
        &self.verifier_version
    }

    pub fn verified_at_ms(&self) -> i64 {
        self.verified_at_ms
    }
}

/// A provenance-bearing graph fact. Domain fields are private so callers must
/// use validated construction and cannot mutate a stored assertion in place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactAssertion {
    assertion_id: String,
    project: ProjectRef,
    graph_version: String,
    subject: String,
    predicate: String,
    object: String,
    claim_state: ClaimState,
    assertion_kind: AssertionKind,
    evidence: Vec<EvidenceRef>,
    producer: Producer,
    decision: Option<ClaimDecision>,
}

impl FactAssertion {
    #[allow(clippy::too_many_arguments)]
    pub fn candidate(
        assertion_id: String,
        project: ProjectRef,
        graph_version: String,
        subject: String,
        predicate: String,
        object: String,
        assertion_kind: AssertionKind,
        evidence: Vec<EvidenceRef>,
        producer: Producer,
    ) -> Result<Self, DomainError> {
        let assertion = Self {
            assertion_id,
            project,
            graph_version,
            subject,
            predicate,
            object,
            claim_state: ClaimState::Candidate,
            assertion_kind,
            evidence: canonical_evidence(evidence)?,
            producer,
            decision: None,
        };
        assertion.validate()?;
        Ok(assertion)
    }

    /// Rehydrate an immutable ledger row. This is not a trust transition;
    /// accepted rows must carry a matching persisted decision.
    #[allow(clippy::too_many_arguments)]
    pub fn from_persistence(
        assertion_id: String,
        project: ProjectRef,
        graph_version: String,
        subject: String,
        predicate: String,
        object: String,
        claim_state: ClaimState,
        assertion_kind: AssertionKind,
        evidence: Vec<EvidenceRef>,
        producer: Producer,
        decision: Option<ClaimDecision>,
    ) -> Result<Self, DomainError> {
        let assertion = Self {
            assertion_id,
            project,
            graph_version,
            subject,
            predicate,
            object,
            claim_state,
            assertion_kind,
            evidence: canonical_evidence(evidence)?,
            producer,
            decision,
        };
        assertion.validate()?;
        Ok(assertion)
    }

    pub fn assertion_id(&self) -> &str {
        &self.assertion_id
    }

    pub fn project(&self) -> &ProjectRef {
        &self.project
    }

    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn predicate(&self) -> &str {
        &self.predicate
    }

    pub fn object(&self) -> &str {
        &self.object
    }

    pub fn claim_state(&self) -> ClaimState {
        self.claim_state
    }

    pub fn assertion_kind(&self) -> AssertionKind {
        self.assertion_kind
    }

    pub fn evidence(&self) -> &[EvidenceRef] {
        &self.evidence
    }

    pub fn producer(&self) -> &Producer {
        &self.producer
    }

    pub fn decision(&self) -> Option<&ClaimDecision> {
        self.decision.as_ref()
    }

    fn validate(&self) -> Result<(), DomainError> {
        self.project.validate()?;
        for (name, value) in [
            ("assertion id", &self.assertion_id),
            ("graph version", &self.graph_version),
            ("subject", &self.subject),
            ("predicate", &self.predicate),
            ("object", &self.object),
        ] {
            validate_fact_text(name, value)?;
        }
        if self.evidence.is_empty() {
            return Err(DomainError::Missing("fact evidence"));
        }
        if let Some(decision) = &self.decision {
            match decision {
                ClaimDecision::Accepted {
                    receipt_id,
                    verifier,
                    verifier_version,
                    decided_at_ms,
                } => {
                    validate_fact_text("decision receipt", receipt_id)?;
                    validate_fact_text("decision verifier", verifier)?;
                    validate_fact_text("decision verifier version", verifier_version)?;
                    if *decided_at_ms < 0 {
                        return Err(DomainError::Invalid("decision timestamp"));
                    }
                }
                ClaimDecision::Rejected { reason }
                | ClaimDecision::Expired { reason }
                | ClaimDecision::Superseded { reason, .. } => {
                    validate_fact_text("decision reason", reason)?;
                }
            }
            if let ClaimDecision::Superseded { replacement_id, .. } = decision {
                validate_fact_text("replacement assertion", replacement_id)?;
            }
        }
        if self.claim_state == ClaimState::Candidate && self.decision.is_some()
            || self.claim_state != ClaimState::Candidate
                && self.decision.as_ref().map(ClaimDecision::state) != Some(self.claim_state)
        {
            return Err(DomainError::Invalid("claim state and decision mismatch"));
        }
        Ok(())
    }
}

/// Accepted facts cannot be forged through a public constructor. They are
/// returned only by `GraphWriter::accept` after receipt binding succeeds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedFact(FactAssertion);

impl AcceptedFact {
    pub fn assertion(&self) -> &FactAssertion {
        &self.0
    }

    pub fn into_assertion(self) -> FactAssertion {
        self.0
    }
}

/// The sole domain boundary that creates an accepted fact or a terminal
/// decision. Persistence is intentionally outside this type.
#[derive(Clone, Copy, Debug, Default)]
pub struct GraphWriter;

impl GraphWriter {
    pub fn accept(
        &self,
        candidate: FactAssertion,
        receipt: VerificationReceipt,
    ) -> Result<AcceptedFact, DomainError> {
        if candidate.claim_state != ClaimState::Candidate {
            return Err(DomainError::Invalid("only a candidate can be accepted"));
        }
        if receipt.assertion_id != candidate.assertion_id
            || receipt.project != candidate.project
            || receipt.graph_version != candidate.graph_version
            || receipt.evidence != candidate.evidence
        {
            return Err(DomainError::Invalid(
                "verification receipt does not bind candidate",
            ));
        }
        let mut accepted = candidate;
        accepted.claim_state = ClaimState::Accepted;
        accepted.decision = Some(ClaimDecision::Accepted {
            receipt_id: receipt.receipt_id,
            verifier: receipt.verifier,
            verifier_version: receipt.verifier_version,
            decided_at_ms: receipt.verified_at_ms,
        });
        accepted.validate()?;
        Ok(AcceptedFact(accepted))
    }

    pub fn reject(
        &self,
        candidate: FactAssertion,
        reason: String,
    ) -> Result<FactAssertion, DomainError> {
        if candidate.claim_state != ClaimState::Candidate {
            return Err(DomainError::Invalid("only a candidate can be rejected"));
        }
        self.decide(candidate, ClaimDecision::Rejected { reason })
    }

    pub fn supersede(
        &self,
        fact: FactAssertion,
        replacement_id: String,
        reason: String,
    ) -> Result<FactAssertion, DomainError> {
        if fact.claim_state != ClaimState::Candidate && fact.claim_state != ClaimState::Accepted {
            return Err(DomainError::Invalid("only active facts can be superseded"));
        }
        validate_fact_text("replacement assertion", &replacement_id)?;
        self.decide(
            fact,
            ClaimDecision::Superseded {
                replacement_id,
                reason,
            },
        )
    }

    pub fn expire(
        &self,
        fact: FactAssertion,
        reason: String,
    ) -> Result<FactAssertion, DomainError> {
        if fact.claim_state != ClaimState::Candidate && fact.claim_state != ClaimState::Accepted {
            return Err(DomainError::Invalid("only active facts can expire"));
        }
        self.decide(fact, ClaimDecision::Expired { reason })
    }

    fn decide(
        &self,
        mut fact: FactAssertion,
        decision: ClaimDecision,
    ) -> Result<FactAssertion, DomainError> {
        validate_fact_text("decision reason", decision_reason(&decision))?;
        fact.claim_state = decision.state();
        fact.decision = Some(decision);
        fact.validate()?;
        Ok(fact)
    }
}

fn decision_reason(decision: &ClaimDecision) -> &str {
    match decision {
        ClaimDecision::Accepted { .. } => "accepted",
        ClaimDecision::Rejected { reason }
        | ClaimDecision::Superseded { reason, .. }
        | ClaimDecision::Expired { reason } => reason,
    }
}

fn canonical_evidence(mut evidence: Vec<EvidenceRef>) -> Result<Vec<EvidenceRef>, DomainError> {
    if evidence.is_empty() {
        return Err(DomainError::Missing("fact evidence"));
    }
    if evidence.len() > MAX_FACT_EVIDENCE {
        return Err(DomainError::Invalid("too many fact evidence references"));
    }
    evidence.sort();
    if evidence.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(DomainError::Invalid("duplicate fact evidence reference"));
    }
    Ok(evidence)
}

fn validate_fact_text(name: &'static str, value: &str) -> Result<(), DomainError> {
    validate_text(name, value)?;
    if value.len() > MAX_FACT_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(DomainError::Invalid(
            "fact text is oversized or contains control characters",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn evidence(id: &str, kind: EvidenceKind) -> EvidenceRef {
        EvidenceRef::new(id.into(), kind).unwrap()
    }

    fn producer() -> Producer {
        Producer::new("codegraph".into(), "v1".into(), "run-1".into()).unwrap()
    }

    fn candidate() -> FactAssertion {
        FactAssertion::candidate(
            "fact-1".into(),
            project(),
            "graph-1".into(),
            "symbol:a".into(),
            "calls".into(),
            "symbol:b".into(),
            AssertionKind::Resolver,
            vec![evidence("source-1", EvidenceKind::Source)],
            producer(),
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
            "graph-writer".into(),
            "v1".into(),
            10,
        )
        .unwrap()
    }

    #[test]
    fn candidate_is_validated_and_evidence_is_canonicalized() {
        let fact = FactAssertion::candidate(
            "fact".into(),
            project(),
            "g".into(),
            "a".into(),
            "calls".into(),
            "b".into(),
            AssertionKind::Parser,
            vec![
                evidence("z", EvidenceKind::Artifact),
                evidence("a", EvidenceKind::Source),
            ],
            producer(),
        )
        .unwrap();
        assert_eq!(fact.claim_state(), ClaimState::Candidate);
        assert_eq!(fact.evidence()[0].id(), "a");
        assert!(
            FactAssertion::candidate(
                "fact".into(),
                project(),
                "g".into(),
                "a".into(),
                "p".into(),
                "b".into(),
                AssertionKind::Parser,
                vec![],
                producer()
            )
            .is_err()
        );
    }

    #[test]
    fn graph_writer_alone_produces_accepted_fact() {
        let fact = candidate();
        let accepted = GraphWriter.accept(fact.clone(), receipt(&fact)).unwrap();
        assert_eq!(accepted.assertion().claim_state(), ClaimState::Accepted);
        assert!(matches!(
            accepted.assertion().decision(),
            Some(ClaimDecision::Accepted { .. })
        ));
        assert!(
            GraphWriter
                .accept(fact.clone().reject_for_test(), receipt(&fact))
                .is_err()
        );
    }

    #[test]
    fn acceptance_requires_exact_scope_and_evidence() {
        let fact = candidate();
        let mut invalid = receipt(&fact);
        invalid.evidence = vec![evidence("other", EvidenceKind::Source)];
        assert!(GraphWriter.accept(fact.clone(), invalid).is_err());
        let mut invalid = receipt(&fact);
        invalid.graph_version = "other".into();
        assert!(GraphWriter.accept(fact, invalid).is_err());
    }

    #[test]
    fn terminal_state_machine_is_explicit_and_one_way() {
        let rejected = GraphWriter
            .reject(candidate(), "not enough evidence".into())
            .unwrap();
        assert_eq!(rejected.claim_state(), ClaimState::Rejected);
        assert!(GraphWriter.expire(rejected, "late".into()).is_err());

        let fact = candidate();
        let accepted = GraphWriter
            .accept(fact.clone(), receipt(&fact))
            .unwrap()
            .into_assertion();
        assert!(
            GraphWriter
                .reject(accepted.clone(), "late rejection".into())
                .is_err()
        );
        let expired = GraphWriter
            .expire(accepted, "snapshot expired".into())
            .unwrap();
        assert_eq!(expired.claim_state(), ClaimState::Expired);
        assert!(GraphWriter.accept(fact, receipt(&candidate())).is_ok());
    }

    #[test]
    fn persistence_rehydration_requires_matching_decision() {
        let fact = candidate();
        assert!(
            FactAssertion::from_persistence(
                fact.assertion_id().into(),
                fact.project().clone(),
                fact.graph_version().into(),
                fact.subject().into(),
                fact.predicate().into(),
                fact.object().into(),
                ClaimState::Accepted,
                fact.assertion_kind(),
                fact.evidence().to_vec(),
                fact.producer().clone(),
                None
            )
            .is_err()
        );
        let accepted = GraphWriter
            .accept(fact.clone(), receipt(&fact))
            .unwrap()
            .into_assertion();
        assert!(
            FactAssertion::from_persistence(
                accepted.assertion_id().into(),
                accepted.project().clone(),
                accepted.graph_version().into(),
                accepted.subject().into(),
                accepted.predicate().into(),
                accepted.object().into(),
                accepted.claim_state(),
                accepted.assertion_kind(),
                accepted.evidence().to_vec(),
                accepted.producer().clone(),
                accepted.decision().cloned()
            )
            .is_ok()
        );
    }

    #[test]
    fn persisted_decisions_validate_their_payload() {
        let fact = candidate();
        for decision in [
            ClaimDecision::Accepted {
                receipt_id: String::new(),
                verifier: "v".into(),
                verifier_version: "1".into(),
                decided_at_ms: 1,
            },
            ClaimDecision::Rejected {
                reason: String::new(),
            },
            ClaimDecision::Superseded {
                replacement_id: String::new(),
                reason: "reason".into(),
            },
            ClaimDecision::Expired {
                reason: String::new(),
            },
        ] {
            assert!(
                FactAssertion::from_persistence(
                    fact.assertion_id().into(),
                    fact.project().clone(),
                    fact.graph_version().into(),
                    fact.subject().into(),
                    fact.predicate().into(),
                    fact.object().into(),
                    decision.state(),
                    fact.assertion_kind(),
                    fact.evidence().to_vec(),
                    fact.producer().clone(),
                    Some(decision),
                )
                .is_err()
            );
        }
    }

    trait TestReject {
        fn reject_for_test(self) -> Self;
    }

    impl TestReject for FactAssertion {
        fn reject_for_test(self) -> Self {
            GraphWriter.reject(self, "test".into()).unwrap()
        }
    }
}

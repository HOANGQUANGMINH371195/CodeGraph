//! Role-scoped context-pack contracts.
//!
//! A context pack is an input/output contract for a worker, not a graph write
//! or execution permission. Claims remain candidates and a capability grant
//! is only a host-issued description until a later admission layer verifies it.

use std::collections::HashSet;
use std::str::FromStr;

use crate::{DomainError, FactAssertion, ProjectRef, validate_text};

const MAX_ID_BYTES: usize = 256;
const MAX_ROLE_BYTES: usize = 128;
const MAX_GRAPH_VERSION_BYTES: usize = 4 * 1024;
const MAX_PATH_BYTES: usize = 4 * 1024;
const MAX_NODE_ID_BYTES: usize = 4 * 1024;
const MAX_CONTEXT_REF_BYTES: usize = 4 * 1024;
const MAX_UNKNOWN_TEXT_BYTES: usize = 4 * 1024;
const MAX_SELECTOR_ITEMS: usize = 1_024;
const MAX_CLAIMS: usize = 2_048;
const MAX_UNKNOWNS: usize = 2_048;
const MAX_DEPTH: u16 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextOperation {
    Orientation,
    Navigate,
    Impact,
    Test,
    Handoff,
    Trace,
    ChangeReview,
}

impl ContextOperation {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Orientation => "orientation",
            Self::Navigate => "navigate",
            Self::Impact => "impact",
            Self::Test => "test",
            Self::Handoff => "handoff",
            Self::Trace => "trace",
            Self::ChangeReview => "change_review",
        }
    }

    /// Parses the canonical wire spelling.
    ///
    /// # Errors
    /// Returns an error when `value` is not a supported operation name.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value {
            "orientation" => Ok(Self::Orientation),
            "navigate" => Ok(Self::Navigate),
            "impact" => Ok(Self::Impact),
            "test" => Ok(Self::Test),
            "handoff" => Ok(Self::Handoff),
            "trace" => Ok(Self::Trace),
            "change_review" => Ok(Self::ChangeReview),
            _ => Err(DomainError::Invalid("unknown context operation")),
        }
    }
}

impl FromStr for ContextOperation {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        ContextOperation::from_str(value)
    }
}

/// Repository-relative scope used for both retrieval and worker admission.
/// Empty selectors are rejected so a caller cannot accidentally mean "all
/// repositories". A whole-repository scope must be an explicit path prefix
/// supplied by the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeSelector {
    project: ProjectRef,
    graph_version: String,
    paths: Vec<String>,
    node_ids: Vec<String>,
    max_depth: u16,
}

impl ScopeSelector {
    /// Constructs a bounded repository-relative retrieval scope.
    ///
    /// # Errors
    /// Returns an error when the project or graph version is invalid, no
    /// selector is supplied, an item budget is exceeded, or depth is too large.
    /// Invalid paths, invalid or oversized node IDs, and duplicate selectors
    /// are also rejected.
    pub fn new(
        project: ProjectRef,
        graph_version: String,
        mut paths: Vec<String>,
        mut node_ids: Vec<String>,
        max_depth: u16,
    ) -> Result<Self, DomainError> {
        project.validate()?;
        validate_bounded_text(
            "scope graph version",
            &graph_version,
            MAX_GRAPH_VERSION_BYTES,
        )?;
        if paths.is_empty() && node_ids.is_empty() {
            return Err(DomainError::Missing("scope selector"));
        }
        if paths.len() > MAX_SELECTOR_ITEMS || node_ids.len() > MAX_SELECTOR_ITEMS {
            return Err(DomainError::Invalid("scope selector exceeds item budget"));
        }
        if max_depth > MAX_DEPTH {
            return Err(DomainError::Invalid("scope traversal depth exceeds limit"));
        }
        for path in &paths {
            validate_path(path)?;
        }
        for node_id in &node_ids {
            validate_bounded_text("scope node id", node_id, MAX_NODE_ID_BYTES)?;
        }
        canonical_unique(&mut paths, "duplicate scope path")?;
        canonical_unique(&mut node_ids, "duplicate scope node")?;
        Ok(Self {
            project,
            graph_version,
            paths,
            node_ids,
            max_depth,
        })
    }

    pub fn project(&self) -> &ProjectRef {
        &self.project
    }

    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }

    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    pub fn node_ids(&self) -> &[String] {
        &self.node_ids
    }

    pub fn max_depth(&self) -> u16 {
        self.max_depth
    }

    #[must_use]
    pub fn contains_path(&self, path: &str) -> bool {
        self.paths.iter().any(|prefix| {
            path == prefix
                || path
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
    }

    #[must_use]
    pub fn contains_node(&self, node_id: &str) -> bool {
        self.node_ids.iter().any(|candidate| candidate == node_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    ReadGraph,
    ReadSource,
    ReadEvidence,
    ReadDocuments,
    ReadRuntime,
    RunAnalysis,
    RunTests,
    RunTerminal,
    SubmitClaims,
    RequestSubagent,
    WriteWorktree,
}

impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadGraph => "read_graph",
            Self::ReadSource => "read_source",
            Self::ReadEvidence => "read_evidence",
            Self::ReadDocuments => "read_documents",
            Self::ReadRuntime => "read_runtime",
            Self::RunAnalysis => "run_analysis",
            Self::RunTests => "run_tests",
            Self::RunTerminal => "run_terminal",
            Self::SubmitClaims => "submit_claims",
            Self::RequestSubagent => "request_subagent",
            Self::WriteWorktree => "write_worktree",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value {
            "read_graph" => Ok(Self::ReadGraph),
            "read_source" => Ok(Self::ReadSource),
            "read_evidence" => Ok(Self::ReadEvidence),
            "read_documents" => Ok(Self::ReadDocuments),
            "read_runtime" => Ok(Self::ReadRuntime),
            "run_analysis" => Ok(Self::RunAnalysis),
            "run_tests" => Ok(Self::RunTests),
            "run_terminal" => Ok(Self::RunTerminal),
            "submit_claims" => Ok(Self::SubmitClaims),
            "request_subagent" => Ok(Self::RequestSubagent),
            "write_worktree" => Ok(Self::WriteWorktree),
            _ => Err(DomainError::Invalid("unknown context capability")),
        }
    }
}

impl FromStr for Capability {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Capability::from_str(value)
    }
}

/// A host-issued capability description. It is not authentication, a lease,
/// GraphWriter authority or permission to accept a claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityGrant {
    grant_id: String,
    issuer: String,
    subject: String,
    role: String,
    scope: ScopeSelector,
    capabilities: Vec<Capability>,
    issued_at_ms: u64,
    expires_at_ms: u64,
    parent_grant_id: Option<String>,
}

impl CapabilityGrant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        grant_id: String,
        issuer: String,
        subject: String,
        role: String,
        scope: ScopeSelector,
        mut capabilities: Vec<Capability>,
        issued_at_ms: u64,
        expires_at_ms: u64,
        parent_grant_id: Option<String>,
    ) -> Result<Self, DomainError> {
        for (name, value, max) in [
            ("grant id", &grant_id, MAX_ID_BYTES),
            ("grant issuer", &issuer, MAX_ID_BYTES),
            ("grant subject", &subject, MAX_ID_BYTES),
            ("grant role", &role, MAX_ROLE_BYTES),
        ] {
            validate_bounded_text(name, value, max)?;
        }
        if capabilities.is_empty() || capabilities.len() > 32 {
            return Err(DomainError::Invalid("invalid capability grant set"));
        }
        if expires_at_ms <= issued_at_ms {
            return Err(DomainError::Invalid(
                "capability grant expiry is not after issue",
            ));
        }
        if let Some(parent) = &parent_grant_id {
            validate_bounded_text("parent grant id", parent, MAX_ID_BYTES)?;
        }
        canonical_unique(&mut capabilities, "duplicate context capability")?;
        Ok(Self {
            grant_id,
            issuer,
            subject,
            role,
            scope,
            capabilities,
            issued_at_ms,
            expires_at_ms,
            parent_grant_id,
        })
    }

    pub fn grant_id(&self) -> &str {
        &self.grant_id
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn scope(&self) -> &ScopeSelector {
        &self.scope
    }

    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    pub fn issued_at_ms(&self) -> u64 {
        self.issued_at_ms
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }

    pub fn parent_grant_id(&self) -> Option<&str> {
        self.parent_grant_id.as_deref()
    }

    #[must_use]
    pub fn is_valid_at(&self, now_ms: u64) -> bool {
        now_ms >= self.issued_at_ms && now_ms < self.expires_at_ms
    }

    #[must_use]
    pub fn allows(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownClaim {
    code: String,
    reason: String,
    subject: Option<String>,
    path: Option<String>,
    line: Option<u32>,
}

impl UnknownClaim {
    pub fn new(
        code: String,
        reason: String,
        subject: Option<String>,
        path: Option<String>,
        line: Option<u32>,
    ) -> Result<Self, DomainError> {
        validate_bounded_text("unknown claim code", &code, 256)?;
        validate_bounded_text("unknown claim reason", &reason, MAX_UNKNOWN_TEXT_BYTES)?;
        if let Some(subject) = &subject {
            validate_bounded_text("unknown claim subject", subject, MAX_UNKNOWN_TEXT_BYTES)?;
        }
        if let Some(path) = &path {
            validate_path(path)?;
        }
        if line == Some(0) {
            return Err(DomainError::Invalid("unknown claim line must be one-based"));
        }
        Ok(Self {
            code,
            reason,
            subject,
            path,
            line,
        })
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn line(&self) -> Option<u32> {
        self.line
    }
}

/// Worker output is deliberately candidate-only. The `verified` label means
/// the worker supplied a better-supported candidate section, not GraphWriter
/// acceptance; both sections still require independent verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimBundle {
    bundle_id: String,
    project: ProjectRef,
    graph_version: String,
    scope: ScopeSelector,
    verified: Vec<FactAssertion>,
    heuristic: Vec<FactAssertion>,
    unknowns: Vec<UnknownClaim>,
}

impl ClaimBundle {
    pub fn new(
        bundle_id: String,
        project: ProjectRef,
        graph_version: String,
        scope: ScopeSelector,
        verified: Vec<FactAssertion>,
        heuristic: Vec<FactAssertion>,
        unknowns: Vec<UnknownClaim>,
    ) -> Result<Self, DomainError> {
        validate_bounded_text("claim bundle id", &bundle_id, MAX_ID_BYTES)?;
        project.validate()?;
        validate_bounded_text(
            "claim graph version",
            &graph_version,
            MAX_GRAPH_VERSION_BYTES,
        )?;
        if scope.project() != &project || scope.graph_version() != graph_version {
            return Err(DomainError::Invalid(
                "claim bundle scope does not bind snapshot",
            ));
        }
        if verified.len().saturating_add(heuristic.len()) > MAX_CLAIMS {
            return Err(DomainError::Invalid("claim bundle exceeds claim budget"));
        }
        if unknowns.len() > MAX_UNKNOWNS {
            return Err(DomainError::Invalid("claim bundle exceeds unknown budget"));
        }
        let mut ids = HashSet::new();
        for claim in verified.iter().chain(&heuristic) {
            if claim.project() != &project
                || claim.graph_version() != graph_version
                || claim.claim_state() != crate::ClaimState::Candidate
                || claim.decision().is_some()
                || !ids.insert(claim.assertion_id())
            {
                return Err(DomainError::Invalid(
                    "claim bundle contains non-candidate, foreign or duplicate claim",
                ));
            }
        }
        for unknown in &unknowns {
            // Re-run the invariant for values that may have come from a
            // persistence adapter in a future version.
            UnknownClaim::new(
                unknown.code.clone(),
                unknown.reason.clone(),
                unknown.subject.clone(),
                unknown.path.clone(),
                unknown.line,
            )?;
        }
        Ok(Self {
            bundle_id,
            project,
            graph_version,
            scope,
            verified,
            heuristic,
            unknowns,
        })
    }

    pub fn bundle_id(&self) -> &str {
        &self.bundle_id
    }

    pub fn project(&self) -> &ProjectRef {
        &self.project
    }

    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }

    pub fn scope(&self) -> &ScopeSelector {
        &self.scope
    }

    pub fn verified(&self) -> &[FactAssertion] {
        &self.verified
    }

    pub fn heuristic(&self) -> &[FactAssertion] {
        &self.heuristic
    }

    pub fn unknowns(&self) -> &[UnknownClaim] {
        &self.unknowns
    }
}

/// Structured continuation data. It contains no capability escalation and no
/// raw transcript; `context_ref` points to a separately scoped pack/store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handoff {
    handoff_id: String,
    sender: String,
    recipient: Option<String>,
    project: ProjectRef,
    graph_version: String,
    scope: ScopeSelector,
    context_ref: String,
    claims: ClaimBundle,
    expires_at_ms: u64,
}

impl Handoff {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        handoff_id: String,
        sender: String,
        recipient: Option<String>,
        project: ProjectRef,
        graph_version: String,
        scope: ScopeSelector,
        context_ref: String,
        claims: ClaimBundle,
        expires_at_ms: u64,
    ) -> Result<Self, DomainError> {
        validate_bounded_text("handoff id", &handoff_id, MAX_ID_BYTES)?;
        validate_bounded_text("handoff sender", &sender, MAX_ID_BYTES)?;
        if let Some(recipient) = &recipient {
            validate_bounded_text("handoff recipient", recipient, MAX_ID_BYTES)?;
        }
        project.validate()?;
        validate_bounded_text(
            "handoff graph version",
            &graph_version,
            MAX_GRAPH_VERSION_BYTES,
        )?;
        validate_bounded_text(
            "handoff context reference",
            &context_ref,
            MAX_CONTEXT_REF_BYTES,
        )?;
        if expires_at_ms == 0 {
            return Err(DomainError::Invalid("handoff expiry must be positive"));
        }
        if scope.project() != &project
            || scope.graph_version() != graph_version
            || claims.project() != &project
            || claims.graph_version() != graph_version
            || claims.scope() != &scope
        {
            return Err(DomainError::Invalid(
                "handoff does not bind snapshot and scope",
            ));
        }
        Ok(Self {
            handoff_id,
            sender,
            recipient,
            project,
            graph_version,
            scope,
            context_ref,
            claims,
            expires_at_ms,
        })
    }

    pub fn handoff_id(&self) -> &str {
        &self.handoff_id
    }

    pub fn sender(&self) -> &str {
        &self.sender
    }

    pub fn recipient(&self) -> Option<&str> {
        self.recipient.as_deref()
    }

    pub fn project(&self) -> &ProjectRef {
        &self.project
    }

    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }

    pub fn scope(&self) -> &ScopeSelector {
        &self.scope
    }

    pub fn context_ref(&self) -> &str {
        &self.context_ref
    }

    pub fn claims(&self) -> &ClaimBundle {
        &self.claims
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UntrustedContentOrigin {
    Source,
    Document,
    ToolOutput,
    RuntimeTrace,
    HumanInput,
    ModelOutput,
}

impl UntrustedContentOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Document => "document",
            Self::ToolOutput => "tool_output",
            Self::RuntimeTrace => "runtime_trace",
            Self::HumanInput => "human_input",
            Self::ModelOutput => "model_output",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, DomainError> {
        match value {
            "source" => Ok(Self::Source),
            "document" => Ok(Self::Document),
            "tool_output" => Ok(Self::ToolOutput),
            "runtime_trace" => Ok(Self::RuntimeTrace),
            "human_input" => Ok(Self::HumanInput),
            "model_output" => Ok(Self::ModelOutput),
            _ => Err(DomainError::Invalid("unknown untrusted content origin")),
        }
    }
}

impl FromStr for UntrustedContentOrigin {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        UntrustedContentOrigin::from_str(value)
    }
}

/// This type has no trusted variant by design. Any content carried by a pack
/// must be interpreted as data, never as a new instruction or authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UntrustedContentLabel {
    origins: Vec<UntrustedContentOrigin>,
}

impl UntrustedContentLabel {
    pub fn new(mut origins: Vec<UntrustedContentOrigin>) -> Result<Self, DomainError> {
        if origins.is_empty() || origins.len() > 16 {
            return Err(DomainError::Invalid("invalid untrusted content label"));
        }
        canonical_unique(&mut origins, "duplicate untrusted content origin")?;
        Ok(Self { origins })
    }

    pub fn origins(&self) -> &[UntrustedContentOrigin] {
        &self.origins
    }
}

/// Pack metadata that can be reasoned about without deserializing or trusting
/// the agent-facing `ContextEnvelope` wire payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPack {
    pack_id: String,
    operation: ContextOperation,
    scope: ScopeSelector,
    grant: CapabilityGrant,
    handoff: Option<Handoff>,
    untrusted_content: UntrustedContentLabel,
}

impl ContextPack {
    pub fn new(
        pack_id: String,
        operation: ContextOperation,
        scope: ScopeSelector,
        grant: CapabilityGrant,
        handoff: Option<Handoff>,
        untrusted_content: UntrustedContentLabel,
    ) -> Result<Self, DomainError> {
        validate_bounded_text("context pack id", &pack_id, MAX_ID_BYTES)?;
        if grant.scope() != &scope {
            return Err(DomainError::Invalid(
                "capability grant widens context scope",
            ));
        }
        if let Some(handoff) = &handoff {
            if handoff.project() != scope.project()
                || handoff.graph_version() != scope.graph_version()
                || handoff.scope() != &scope
            {
                return Err(DomainError::Invalid("handoff widens context scope"));
            }
        }
        Ok(Self {
            pack_id,
            operation,
            scope,
            grant,
            handoff,
            untrusted_content,
        })
    }

    pub fn pack_id(&self) -> &str {
        &self.pack_id
    }

    pub fn operation(&self) -> ContextOperation {
        self.operation
    }

    pub fn scope(&self) -> &ScopeSelector {
        &self.scope
    }

    pub fn grant(&self) -> &CapabilityGrant {
        &self.grant
    }

    pub fn handoff(&self) -> Option<&Handoff> {
        self.handoff.as_ref()
    }

    pub fn untrusted_content(&self) -> &UntrustedContentLabel {
        &self.untrusted_content
    }
}

fn validate_bounded_text(
    name: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), DomainError> {
    validate_text(name, value)?;
    if value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(DomainError::Invalid(
            "context pack text is oversized or unsafe",
        ));
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), DomainError> {
    validate_bounded_text("scope path", path, MAX_PATH_BYTES)?;
    if path.starts_with('/')
        || path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(DomainError::Invalid(
            "scope path must be repository relative",
        ));
    }
    Ok(())
}

fn canonical_unique<T: Ord>(values: &mut Vec<T>, message: &'static str) -> Result<(), DomainError> {
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(DomainError::Invalid(message));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssertionKind, EvidenceKind, EvidenceRef, GraphWriter, Producer, VerificationReceipt,
    };

    fn project() -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: "head".into(),
            working_tree_fingerprint: "tree".into(),
            config_hash: "config".into(),
            ignore_policy_version: "ignore-v1".into(),
        }
    }

    fn scope() -> ScopeSelector {
        ScopeSelector::new(
            project(),
            "graph-v1".into(),
            vec!["src/auth".into()],
            vec!["symbol:auth.login".into()],
            4,
        )
        .unwrap()
    }

    fn claim(id: &str) -> FactAssertion {
        FactAssertion::candidate(
            id.into(),
            project(),
            "graph-v1".into(),
            "symbol:auth.login".into(),
            "calls".into(),
            "symbol:auth.store".into(),
            AssertionKind::Resolver,
            vec![EvidenceRef::new("source-1".into(), EvidenceKind::Source).unwrap()],
            Producer::new("resolver".into(), "v1".into(), "run-1".into()).unwrap(),
        )
        .unwrap()
    }

    fn grant() -> CapabilityGrant {
        CapabilityGrant::new(
            "grant-1".into(),
            "host".into(),
            "worker-1".into(),
            "flow-tracer".into(),
            scope(),
            vec![
                Capability::ReadGraph,
                Capability::ReadSource,
                Capability::SubmitClaims,
            ],
            10,
            20,
            None,
        )
        .unwrap()
    }

    #[test]
    fn scope_is_canonical_and_path_matching_respects_boundaries() {
        let selected = ScopeSelector::new(
            project(),
            "g".into(),
            vec!["src/z".into(), "src/a".into()],
            vec!["node:z".into(), "node:a".into()],
            0,
        )
        .unwrap();
        assert_eq!(selected.paths(), &["src/a", "src/z"]);
        assert!(selected.contains_path("src/a/file.rs"));
        assert!(!selected.contains_path("src/ab/file.rs"));
        assert!(selected.contains_node("node:a"));
    }

    #[test]
    fn scope_rejects_empty_duplicate_and_unsafe_selectors() {
        assert!(ScopeSelector::new(project(), "g".into(), vec![], vec![], 1).is_err());
        assert!(
            ScopeSelector::new(
                project(),
                "g".into(),
                vec!["src".into(), "src".into()],
                vec![],
                1
            )
            .is_err()
        );
        assert!(
            ScopeSelector::new(project(), "g".into(), vec!["../secrets".into()], vec![], 1)
                .is_err()
        );
        assert!(ScopeSelector::new(project(), "g".into(), vec!["src".into()], vec![], 65).is_err());
    }

    #[test]
    fn capability_grant_is_time_bounded_and_deduplicated() {
        let grant = grant();
        assert!(grant.is_valid_at(10));
        assert!(!grant.is_valid_at(20));
        assert!(grant.allows(Capability::ReadSource));
        assert!(
            CapabilityGrant::new(
                "g".into(),
                "i".into(),
                "s".into(),
                "r".into(),
                scope(),
                vec![Capability::ReadGraph, Capability::ReadGraph],
                1,
                2,
                None
            )
            .is_err()
        );
        assert!(
            CapabilityGrant::new(
                "g".into(),
                "i".into(),
                "s".into(),
                "r".into(),
                scope(),
                vec![Capability::ReadGraph],
                2,
                2,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn claim_bundle_keeps_verified_and_heuristic_candidates_out_of_acceptance() {
        let bundle = ClaimBundle::new(
            "bundle-1".into(),
            project(),
            "graph-v1".into(),
            scope(),
            vec![claim("fact-1")],
            vec![claim("fact-2")],
            vec![],
        )
        .unwrap();
        assert_eq!(bundle.verified().len(), 1);
        assert_eq!(bundle.heuristic().len(), 1);
        assert!(bundle.verified()[0].decision().is_none());

        let accepted = GraphWriter
            .accept(
                claim("accepted"),
                VerificationReceipt::new(
                    "receipt".into(),
                    "accepted".into(),
                    project(),
                    "graph-v1".into(),
                    vec![EvidenceRef::new("source-1".into(), EvidenceKind::Source).unwrap()],
                    "verifier".into(),
                    "v1".into(),
                    1,
                )
                .unwrap(),
            )
            .unwrap()
            .into_assertion();
        assert!(
            ClaimBundle::new(
                "bundle-2".into(),
                project(),
                "graph-v1".into(),
                scope(),
                vec![accepted],
                vec![],
                vec![]
            )
            .is_err()
        );
    }

    #[test]
    fn handoff_and_pack_reject_scope_widening_and_require_data_label() {
        let claims = ClaimBundle::new(
            "bundle-1".into(),
            project(),
            "graph-v1".into(),
            scope(),
            vec![claim("fact")],
            vec![],
            vec![],
        )
        .unwrap();
        let handoff = Handoff::new(
            "handoff-1".into(),
            "worker-1".into(),
            Some("worker-2".into()),
            project(),
            "graph-v1".into(),
            scope(),
            "context-1".into(),
            claims,
            100,
        )
        .unwrap();
        let label = UntrustedContentLabel::new(vec![
            UntrustedContentOrigin::Source,
            UntrustedContentOrigin::ToolOutput,
        ])
        .unwrap();
        let pack = ContextPack::new(
            "pack-1".into(),
            ContextOperation::Trace,
            scope(),
            grant(),
            Some(handoff),
            label,
        )
        .unwrap();
        assert_eq!(pack.operation(), ContextOperation::Trace);
        assert!(
            ContextPack::new(
                "pack-2".into(),
                ContextOperation::Trace,
                ScopeSelector::new(project(), "graph-v1".into(), vec!["src".into()], vec![], 4)
                    .unwrap(),
                grant(),
                None,
                UntrustedContentLabel::new(vec![UntrustedContentOrigin::Source]).unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn unknown_claim_line_and_content_label_are_bounded() {
        assert!(UnknownClaim::new("missing".into(), "reason".into(), None, None, Some(0)).is_err());
        assert!(UntrustedContentLabel::new(vec![]).is_err());
        assert!(
            UntrustedContentLabel::new(vec![
                UntrustedContentOrigin::Source,
                UntrustedContentOrigin::Source,
            ])
            .is_err()
        );
    }
}

use std::{error::Error, path::Path};

use graph_domain::{AnalysisRun, DomainError, ProjectRef, SourceEvidence};
use sha2::{Digest, Sha256};

/// A source adapter must enforce its configured project/snapshot binding and
/// filesystem scope. The verifier independently checks the returned bytes.
pub trait SourceReader {
    type Error: Error + Send + Sync + 'static;
    fn read_source(&self, evidence: &SourceEvidence, limit: usize) -> Result<Vec<u8>, Self::Error>;
}

/// Host boundary for the complete source snapshot selected by a verifier.
///
/// The caller-provided root and `ProjectRef` are claims/access inputs, not
/// proof. A trusted host adapter must bind the exact root to the requested
/// project and graph version before returning a binding. The binding itself is
/// an observation for this verification call; constructing one does not
/// authenticate an adapter.
pub trait SourceSnapshotAuthority {
    type Error: Error + Send + Sync + 'static;

    fn bind_source_snapshot(
        &self,
        root: &Path,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<SourceSnapshotBinding, Self::Error>;
}

/// Typed host observation that a source root was bound to one exact snapshot.
/// It is deliberately not a durable graph fact or an analyzer-execution proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSnapshotBinding {
    binding_id: String,
    project: ProjectRef,
    graph_version: String,
}

impl SourceSnapshotBinding {
    /// Creates the value returned by a trusted authority adapter. The
    /// `binding_id` is opaque to the application and should identify the
    /// host-selected root/snapshot without exposing credentials or paths.
    pub fn new(
        binding_id: String,
        project: ProjectRef,
        graph_version: String,
    ) -> Result<Self, DomainError> {
        project.validate()?;
        if binding_id.trim().is_empty() || binding_id.chars().any(char::is_control) {
            return Err(DomainError::Invalid("source snapshot binding id"));
        }
        if graph_version.trim().is_empty() || graph_version.chars().any(char::is_control) {
            return Err(DomainError::Invalid("source snapshot graph version"));
        }
        Ok(Self {
            binding_id,
            project,
            graph_version,
        })
    }

    pub fn binding_id(&self) -> &str {
        &self.binding_id
    }

    pub fn project(&self) -> &ProjectRef {
        &self.project
    }

    pub fn graph_version(&self) -> &str {
        &self.graph_version
    }

    fn matches(&self, project: &ProjectRef, graph_version: &str) -> bool {
        self.project == *project && self.graph_version == graph_version
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SourceLimits {
    max_source_bytes: usize,
    max_slice_bytes: usize,
}

impl SourceLimits {
    pub fn new(max_source_bytes: usize, max_slice_bytes: usize) -> Result<Self, DomainError> {
        if max_source_bytes == 0
            || max_source_bytes > 64 * 1024 * 1024
            || max_slice_bytes == 0
            || max_slice_bytes > max_source_bytes
        {
            return Err(DomainError::Invalid(
                "source limits require 0 < slice <= source <= 64 MiB",
            ));
        }
        Ok(Self {
            max_source_bytes,
            max_slice_bytes,
        })
    }
}

/// Bytes matching a citation's full-file hash and inclusive line range.
/// This does not accept an assertion, validate the analysis run, or prove that
/// an entire worktree matches the revision/fingerprint supplied by the caller.
/// Source text remains untrusted agent input even after its hash is checked.
#[derive(Debug)]
pub struct SourceSlice {
    evidence: SourceEvidence,
    text: String,
}

impl SourceSlice {
    pub fn evidence(&self) -> &SourceEvidence {
        &self.evidence
    }
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceVerificationError<E: Error + Send + Sync + 'static> {
    #[error("invalid source capture metadata: {0}")]
    InvalidEvidence(#[source] DomainError),
    #[error("source reader failed: {0}")]
    Reader(#[source] E),
    #[error("source exceeds the configured byte budget")]
    SourceTooLarge,
    #[error("source SHA-256 differs from the citation; reindex this snapshot")]
    HashMismatch,
    #[error("source is not UTF-8 text")]
    InvalidText,
    #[error("citation line range extends beyond the source")]
    InvalidRange,
    #[error("cited source slice exceeds the output byte budget; request a narrower range")]
    SliceTooLarge,
}

#[derive(Debug, thiserror::Error)]
pub enum SourceIdentityVerificationError<AE, RE, SE>
where
    AE: Error + Send + Sync + 'static,
    RE: Error + Send + Sync + 'static,
    SE: Error + Send + Sync + 'static,
{
    #[error("source snapshot authority failed: {0}")]
    SnapshotAuthority(#[source] AE),
    #[error("source snapshot authority returned a different project or graph version")]
    SnapshotMismatch,
    #[error("analysis-run lookup failed: {0}")]
    AnalysisRepository(#[source] RE),
    #[error("source evidence analysis run is not registered for this snapshot")]
    AnalysisRunUnavailable,
    #[error("registered analysis run does not exactly match source evidence")]
    AnalysisRunMismatch,
    #[error("source bytes failed verification after identity checks: {0}")]
    Source(#[source] SourceVerificationError<SE>),
}

#[derive(Debug, thiserror::Error)]
pub enum BoundSourceIdentityVerificationError<RE, SE>
where
    RE: Error + Send + Sync + 'static,
    SE: Error + Send + Sync + 'static,
{
    #[error("source snapshot authority returned a different project or graph version")]
    SnapshotMismatch,
    #[error("analysis-run lookup failed: {0}")]
    AnalysisRepository(#[source] RE),
    #[error("source evidence analysis run is not registered for this snapshot")]
    AnalysisRunUnavailable,
    #[error("registered analysis run does not exactly match source evidence")]
    AnalysisRunMismatch,
    #[error("source bytes failed verification after identity checks: {0}")]
    Source(#[source] SourceVerificationError<SE>),
}

/// Source evidence after the host snapshot and analyzer provenance have both
/// been checked. This remains source data, not a semantic relationship or an
/// accepted graph fact.
#[derive(Debug)]
pub struct VerifiedSourceIdentity {
    source: SourceSlice,
    snapshot: SourceSnapshotBinding,
    analysis_run: AnalysisRun,
}

impl VerifiedSourceIdentity {
    pub fn source(&self) -> &SourceSlice {
        &self.source
    }

    pub fn snapshot(&self) -> &SourceSnapshotBinding {
        &self.snapshot
    }

    pub fn analysis_run(&self) -> &AnalysisRun {
        &self.analysis_run
    }

    pub fn into_source(self) -> SourceSlice {
        self.source
    }
}

/// Capture fresh full-file evidence from one bounded read. The locator supplies
/// project/version/path, not an expected content hash. Neither its snapshot
/// binding nor the caller's analysis-run label is independently authenticated.
///
/// # Errors
/// Rejects invalid run metadata, reader failures, oversized/empty/non-text
/// buffers and invalid evidence. No database state is changed by this function.
pub fn capture_source<R: SourceReader>(
    reader: &R,
    locator: &SourceEvidence,
    analysis_run: &str,
    limits: SourceLimits,
) -> Result<SourceSlice, SourceVerificationError<R::Error>> {
    if analysis_run.trim().is_empty() {
        return Err(SourceVerificationError::InvalidEvidence(
            DomainError::Missing("analysis run"),
        ));
    }
    let bytes = reader
        .read_source(locator, limits.max_source_bytes)
        .map_err(SourceVerificationError::Reader)?;
    if bytes.len() > limits.max_source_bytes {
        return Err(SourceVerificationError::SourceTooLarge);
    }
    if bytes.len() > limits.max_slice_bytes {
        return Err(SourceVerificationError::SliceTooLarge);
    }
    let content_hash = format!("{:x}", Sha256::digest(&bytes));
    let text = String::from_utf8(bytes).map_err(|_| SourceVerificationError::InvalidText)?;
    if text.is_empty() || text.contains('\0') {
        return Err(SourceVerificationError::InvalidText);
    }
    let end_line = u32::try_from(text.split_inclusive('\n').count())
        .map_err(|_| SourceVerificationError::InvalidRange)?;
    let project = locator.project();
    let mut identity = Sha256::new();
    identity.update(b"project-graph/source-capture/v1\0");
    for part in [
        &project.repository_id,
        &project.worktree_id,
        &project.git_head,
        &project.working_tree_fingerprint,
        &project.config_hash,
        &project.ignore_policy_version,
        locator.graph_version(),
        locator.path(),
        &content_hash,
        analysis_run,
    ] {
        identity.update((part.len() as u64).to_be_bytes());
        identity.update(part.as_bytes());
    }
    identity.update(end_line.to_be_bytes());
    let evidence = SourceEvidence::new(
        format!("capture:{:x}", identity.finalize()),
        project.clone(),
        locator.graph_version().into(),
        locator.path().into(),
        content_hash,
        1,
        end_line,
        analysis_run.into(),
    )
    .map_err(SourceVerificationError::InvalidEvidence)?;
    Ok(SourceSlice { evidence, text })
}

pub fn verify_source<R: SourceReader>(
    reader: &R,
    evidence: &SourceEvidence,
    limits: SourceLimits,
) -> Result<SourceSlice, SourceVerificationError<R::Error>> {
    let bytes = reader
        .read_source(evidence, limits.max_source_bytes)
        .map_err(SourceVerificationError::Reader)?;
    if bytes.len() > limits.max_source_bytes {
        return Err(SourceVerificationError::SourceTooLarge);
    }
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if hash != evidence.content_sha256() {
        return Err(SourceVerificationError::HashMismatch);
    }
    let source = std::str::from_utf8(&bytes).map_err(|_| SourceVerificationError::InvalidText)?;
    if source.contains('\0') {
        return Err(SourceVerificationError::InvalidText);
    }
    let mut offset = 0;
    let mut start = None;
    let mut end = None;
    // Preserve original newline bytes. A final newline terminates its line,
    // it does not create a phantom empty line after EOF.
    for (index, line) in source.split_inclusive('\n').enumerate() {
        let number = index + 1;
        if number == evidence.start_line() as usize {
            start = Some(offset);
        }
        offset += line.len();
        if number == evidence.end_line() as usize {
            end = Some(offset);
            break;
        }
    }
    let (Some(start), Some(end)) = (start, end) else {
        return Err(SourceVerificationError::InvalidRange);
    };
    if end - start > limits.max_slice_bytes {
        return Err(SourceVerificationError::SliceTooLarge);
    }
    Ok(SourceSlice {
        evidence: evidence.clone(),
        text: source[start..end].into(),
    })
}

/// Verify source identity before reading source bytes.
///
/// The authority owns the root-to-snapshot decision. The analysis repository
/// must contain an exact immutable registration for the evidence's run,
/// project and graph version. Only then is the bounded source reader called.
/// A successful hash/line check still does not verify a semantic relationship
/// or accept a graph fact.
pub fn verify_source_identity<A, R, S>(
    authority: &A,
    analysis: &R,
    reader: &S,
    root: &Path,
    evidence: &SourceEvidence,
    limits: SourceLimits,
) -> Result<VerifiedSourceIdentity, SourceIdentityVerificationError<A::Error, R::Error, S::Error>>
where
    A: SourceSnapshotAuthority,
    R: crate::AnalysisRepository,
    S: SourceReader,
{
    let snapshot = authority
        .bind_source_snapshot(root, evidence.project(), evidence.graph_version())
        .map_err(SourceIdentityVerificationError::SnapshotAuthority)?;
    match verify_bound_source_identity(&snapshot, analysis, reader, evidence, limits) {
        Ok(verified) => Ok(verified),
        Err(BoundSourceIdentityVerificationError::SnapshotMismatch) => {
            Err(SourceIdentityVerificationError::SnapshotMismatch)
        }
        Err(BoundSourceIdentityVerificationError::AnalysisRepository(error)) => {
            Err(SourceIdentityVerificationError::AnalysisRepository(error))
        }
        Err(BoundSourceIdentityVerificationError::AnalysisRunUnavailable) => {
            Err(SourceIdentityVerificationError::AnalysisRunUnavailable)
        }
        Err(BoundSourceIdentityVerificationError::AnalysisRunMismatch) => {
            Err(SourceIdentityVerificationError::AnalysisRunMismatch)
        }
        Err(BoundSourceIdentityVerificationError::Source(error)) => {
            Err(SourceIdentityVerificationError::Source(error))
        }
    }
}

/// Verify source identity after a host has already materialized and bound the
/// source tree.
///
/// This is the handoff used by an owned materialized snapshot. It avoids
/// re-observing the mutable worktree after the materializer has returned: the
/// reader is kept alive by the materialized owner and the exact binding is
/// still checked before the analysis-run lookup and source read.
pub fn verify_bound_source_identity<R, S>(
    snapshot: &SourceSnapshotBinding,
    analysis: &R,
    reader: &S,
    evidence: &SourceEvidence,
    limits: SourceLimits,
) -> Result<VerifiedSourceIdentity, BoundSourceIdentityVerificationError<R::Error, S::Error>>
where
    R: crate::AnalysisRepository,
    S: SourceReader,
{
    if !snapshot.matches(evidence.project(), evidence.graph_version()) {
        return Err(BoundSourceIdentityVerificationError::SnapshotMismatch);
    }

    let analysis_run = analysis
        .source_analysis_run(evidence)
        .map_err(BoundSourceIdentityVerificationError::AnalysisRepository)?
        .ok_or(BoundSourceIdentityVerificationError::AnalysisRunUnavailable)?;
    if analysis_run.id() != evidence.analysis_run()
        || analysis_run.project() != evidence.project()
        || analysis_run.graph_version() != evidence.graph_version()
    {
        return Err(BoundSourceIdentityVerificationError::AnalysisRunMismatch);
    }

    let source = verify_source(reader, evidence, limits)
        .map_err(BoundSourceIdentityVerificationError::Source)?;
    Ok(VerifiedSourceIdentity {
        source,
        snapshot: snapshot.clone(),
        analysis_run,
    })
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use std::{cell::Cell, fmt, path::Path};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestError;

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("test failure")
        }
    }

    impl Error for TestError {}

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

    fn run() -> AnalysisRun {
        AnalysisRun::new(
            "run1".into(),
            project(),
            "g1".into(),
            "codegraph".into(),
            "1".into(),
            "a".repeat(64),
            "b".repeat(64),
        )
        .expect("test analysis run is valid")
    }

    fn evidence() -> SourceEvidence {
        SourceEvidence::new(
            "e1".into(),
            project(),
            "g1".into(),
            "src/main.rs".into(),
            format!("{:x}", Sha256::digest(b"abc\n")),
            1,
            1,
            "run1".into(),
        )
        .expect("test source evidence is valid")
    }

    struct Authority {
        result: Result<SourceSnapshotBinding, TestError>,
        calls: Cell<usize>,
    }

    impl SourceSnapshotAuthority for Authority {
        type Error = TestError;

        fn bind_source_snapshot(
            &self,
            _root: &Path,
            _project: &ProjectRef,
            _graph_version: &str,
        ) -> Result<SourceSnapshotBinding, Self::Error> {
            self.calls.set(self.calls.get() + 1);
            self.result.clone()
        }
    }

    struct Repository {
        run: Option<AnalysisRun>,
        calls: Cell<usize>,
    }

    impl crate::AnalysisRepository for Repository {
        type Error = TestError;

        fn record_analysis_run(&mut self, _: &AnalysisRun) -> Result<bool, Self::Error> {
            Ok(false)
        }

        fn analysis_run(
            &self,
            _id: &str,
            _project: &ProjectRef,
            _graph_version: &str,
        ) -> Result<Option<AnalysisRun>, Self::Error> {
            self.calls.set(self.calls.get() + 1);
            Ok(self.run.clone())
        }
    }

    struct Reader {
        calls: Cell<usize>,
    }

    impl SourceReader for Reader {
        type Error = TestError;

        fn read_source(
            &self,
            _evidence: &SourceEvidence,
            _limit: usize,
        ) -> Result<Vec<u8>, Self::Error> {
            self.calls.set(self.calls.get() + 1);
            Ok(b"abc\n".to_vec())
        }
    }

    fn authority() -> Authority {
        Authority {
            result: Ok(SourceSnapshotBinding::new(
                "host-snapshot-1".into(),
                project(),
                "g1".into(),
            )
            .expect("test binding is valid")),
            calls: Cell::new(0),
        }
    }

    fn limits() -> SourceLimits {
        SourceLimits::new(1024, 1024).expect("test limits are valid")
    }

    #[test]
    fn valid_snapshot_and_registered_run_are_required_before_source_read() {
        let authority = authority();
        let repository = Repository {
            run: Some(run()),
            calls: Cell::new(0),
        };
        let reader = Reader {
            calls: Cell::new(0),
        };
        let verified = verify_source_identity(
            &authority,
            &repository,
            &reader,
            Path::new("/trusted/root"),
            &evidence(),
            limits(),
        )
        .expect("identity is valid");

        assert_eq!(verified.source().text(), "abc\n");
        assert_eq!(verified.snapshot().binding_id(), "host-snapshot-1");
        assert_eq!(verified.analysis_run().id(), "run1");
        assert_eq!(authority.calls.get(), 1);
        assert_eq!(repository.calls.get(), 1);
        assert_eq!(reader.calls.get(), 1);
    }

    #[test]
    fn authority_failure_is_fail_closed_and_does_not_read_source() {
        let authority = Authority {
            result: Err(TestError),
            calls: Cell::new(0),
        };
        let repository = Repository {
            run: Some(run()),
            calls: Cell::new(0),
        };
        let reader = Reader {
            calls: Cell::new(0),
        };

        assert!(matches!(
            verify_source_identity(
                &authority,
                &repository,
                &reader,
                Path::new("/untrusted/root"),
                &evidence(),
                limits(),
            ),
            Err(SourceIdentityVerificationError::SnapshotAuthority(
                TestError
            ))
        ));
        assert_eq!(repository.calls.get(), 0);
        assert_eq!(reader.calls.get(), 0);
    }

    #[test]
    fn authority_binding_mismatch_is_rejected_before_run_lookup_or_read() {
        let mut wrong_project = project();
        wrong_project.worktree_id = "other".into();
        let authority = Authority {
            result: Ok(SourceSnapshotBinding::new(
                "wrong-snapshot".into(),
                wrong_project,
                "g1".into(),
            )
            .expect("test binding is valid")),
            calls: Cell::new(0),
        };
        let repository = Repository {
            run: Some(run()),
            calls: Cell::new(0),
        };
        let reader = Reader {
            calls: Cell::new(0),
        };

        assert!(matches!(
            verify_source_identity(
                &authority,
                &repository,
                &reader,
                Path::new("/trusted/root"),
                &evidence(),
                limits(),
            ),
            Err(SourceIdentityVerificationError::SnapshotMismatch)
        ));
        assert_eq!(repository.calls.get(), 0);
        assert_eq!(reader.calls.get(), 0);
    }

    #[test]
    fn missing_analysis_run_is_rejected_before_source_read() {
        let authority = authority();
        let repository = Repository {
            run: None,
            calls: Cell::new(0),
        };
        let reader = Reader {
            calls: Cell::new(0),
        };

        assert!(matches!(
            verify_source_identity(
                &authority,
                &repository,
                &reader,
                Path::new("/trusted/root"),
                &evidence(),
                limits(),
            ),
            Err(SourceIdentityVerificationError::AnalysisRunUnavailable)
        ));
        assert_eq!(repository.calls.get(), 1);
        assert_eq!(reader.calls.get(), 0);
    }

    #[test]
    fn mismatched_analysis_run_is_rejected_before_source_read() {
        let authority = authority();
        let mut wrong = run();
        wrong = AnalysisRun::new(
            "different".into(),
            wrong.project().clone(),
            wrong.graph_version().into(),
            wrong.analyzer().into(),
            wrong.analyzer_version().into(),
            wrong.configuration_sha256().into(),
            wrong.input_manifest_sha256().into(),
        )
        .expect("test analysis run is valid");
        let repository = Repository {
            run: Some(wrong),
            calls: Cell::new(0),
        };
        let reader = Reader {
            calls: Cell::new(0),
        };

        assert!(matches!(
            verify_source_identity(
                &authority,
                &repository,
                &reader,
                Path::new("/trusted/root"),
                &evidence(),
                limits(),
            ),
            Err(SourceIdentityVerificationError::AnalysisRunMismatch)
        ));
        assert_eq!(reader.calls.get(), 0);
    }

    #[test]
    fn invalid_binding_metadata_is_rejected() {
        assert!(SourceSnapshotBinding::new(" ".into(), project(), "g1".into()).is_err());
        assert!(SourceSnapshotBinding::new("host".into(), project(), " ".into()).is_err());
    }
}

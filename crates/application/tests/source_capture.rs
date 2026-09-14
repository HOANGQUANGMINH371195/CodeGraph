use std::{cell::RefCell, error::Error, fmt};

use graph_application::{SourceLimits, SourceReader, SourceVerificationError, capture_source};
use graph_domain::{DomainError, ProjectRef, SourceEvidence};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReaderError;

impl fmt::Display for ReaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("instrumented reader failure")
    }
}

impl Error for ReaderError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ReadObservation {
    calls: usize,
    limits: Vec<usize>,
}

struct MemoryReader {
    bytes: Vec<u8>,
    error: Option<ReaderError>,
    observation: RefCell<ReadObservation>,
}

impl MemoryReader {
    fn bytes(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.into(),
            error: None,
            observation: RefCell::new(ReadObservation::default()),
        }
    }

    fn failing() -> Self {
        Self {
            bytes: Vec::new(),
            error: Some(ReaderError),
            observation: RefCell::new(ReadObservation::default()),
        }
    }

    fn observation(&self) -> ReadObservation {
        self.observation.borrow().clone()
    }
}

impl SourceReader for MemoryReader {
    type Error = ReaderError;

    fn read_source(
        &self,
        _evidence: &SourceEvidence,
        limit: usize,
    ) -> Result<Vec<u8>, Self::Error> {
        let mut observation = self.observation.borrow_mut();
        observation.calls += 1;
        observation.limits.push(limit);
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(self.bytes.clone())
    }
}

fn project() -> ProjectRef {
    ProjectRef {
        repository_id: "repo".into(),
        worktree_id: "worktree".into(),
        git_head: "head".into(),
        working_tree_fingerprint: "tree".into(),
        config_hash: "config".into(),
        ignore_policy_version: "ignore-v1".into(),
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn locator(
    project: ProjectRef,
    graph_version: &str,
    path: &str,
    id: &str,
    old_hash: &str,
    start_line: u32,
    end_line: u32,
    locator_run: &str,
) -> SourceEvidence {
    SourceEvidence::new(
        id.into(),
        project,
        graph_version.into(),
        path.into(),
        old_hash.into(),
        start_line,
        end_line,
        locator_run.into(),
    )
    .unwrap()
}

fn capture_id(project: ProjectRef, graph_version: &str, path: &str, analysis_run: &str) -> String {
    let reader = MemoryReader::bytes(b"one\n");
    let locator = locator(
        project,
        graph_version,
        path,
        "old-locator",
        &hash(b"old-content"),
        9,
        12,
        "old-run",
    );
    capture_source(
        &reader,
        &locator,
        analysis_run,
        SourceLimits::new(1024, 1024).unwrap(),
    )
    .unwrap()
    .evidence()
    .id()
    .into()
}

#[test]
fn captures_once_with_the_source_bound_and_fresh_full_file_evidence() {
    let bytes = b"alpha\r\n\xCE\xB2eta\r\n";
    let reader = MemoryReader::bytes(bytes);
    let locator = locator(
        project(),
        "graph-old",
        "src/lib.rs",
        "old-id",
        &hash(b"old-content"),
        9,
        12,
        "old-run",
    );

    let captured = capture_source(
        &reader,
        &locator,
        "analysis-1",
        SourceLimits::new(1024, bytes.len()).unwrap(),
    )
    .unwrap();
    let evidence = captured.evidence();

    assert_eq!(
        reader.observation(),
        ReadObservation {
            calls: 1,
            limits: vec![1024]
        }
    );
    assert_eq!(captured.text(), "alpha\r\nβeta\r\n");
    assert_eq!(evidence.project(), locator.project());
    assert_eq!(evidence.graph_version(), "graph-old");
    assert_eq!(evidence.path(), "src/lib.rs");
    assert_eq!(evidence.content_sha256(), hash(bytes));
    assert_eq!(evidence.start_line(), 1);
    assert_eq!(evidence.end_line(), 2);
    assert_eq!(evidence.analysis_run(), "analysis-1");
    assert_ne!(evidence.id(), locator.id());
    let id_hash = evidence.id().strip_prefix("capture:").unwrap();
    assert_eq!(id_hash.len(), 64);
    assert!(id_hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
}

#[test]
fn final_newline_does_not_create_a_phantom_line() {
    for bytes in [b"one\n".as_slice(), b"one\r\ntwo\r\n".as_slice()] {
        let reader = MemoryReader::bytes(bytes);
        let locator = locator(
            project(),
            "graph",
            "src/file.rs",
            "old",
            &hash(b"old"),
            4,
            5,
            "old-run",
        );
        let captured = capture_source(
            &reader,
            &locator,
            "run",
            SourceLimits::new(1024, bytes.len()).unwrap(),
        )
        .unwrap();

        assert_eq!(captured.text().as_bytes(), bytes);
        assert_eq!(
            captured.evidence().end_line(),
            bytes.iter().filter(|&&b| b == b'\n').count().max(1) as u32
        );
    }
}

#[test]
fn identical_content_ignores_old_locator_id_range_and_hash() {
    let bytes = b"first\nsecond";
    let first_locator = locator(
        project(),
        "graph",
        "src/file.rs",
        "old-a",
        &hash(b"old-a"),
        7,
        9,
        "old-locator-run-a",
    );
    let second_locator = locator(
        project(),
        "graph",
        "src/file.rs",
        "old-b",
        &hash(b"old-b"),
        1,
        1,
        "old-locator-run-b",
    );

    let first = capture_source(
        &MemoryReader::bytes(bytes),
        &first_locator,
        "analysis-run",
        SourceLimits::new(1024, 1024).unwrap(),
    )
    .unwrap();
    let second = capture_source(
        &MemoryReader::bytes(bytes),
        &second_locator,
        "analysis-run",
        SourceLimits::new(1024, 1024).unwrap(),
    )
    .unwrap();

    assert_eq!(first.evidence(), second.evidence());
    assert_eq!(first.text(), second.text());
    assert_eq!(first.evidence().start_line(), 1);
    assert_eq!(first.evidence().end_line(), 2);
}

#[test]
fn changing_each_scope_version_path_and_run_field_changes_the_capture_id() {
    let baseline = capture_id(project(), "graph", "src/file.rs", "analysis-run");
    assert!(baseline.starts_with("capture:"));

    let mut repository = project();
    repository.repository_id = "repo-2".into();
    let mut worktree = project();
    worktree.worktree_id = "worktree-2".into();
    let mut git_head = project();
    git_head.git_head = "head-2".into();
    let mut fingerprint = project();
    fingerprint.working_tree_fingerprint = "tree-2".into();
    let mut config = project();
    config.config_hash = "config-2".into();
    let mut ignore_policy = project();
    ignore_policy.ignore_policy_version = "ignore-v2".into();

    for (field, changed_project) in [
        ("repository_id", repository),
        ("worktree_id", worktree),
        ("git_head", git_head),
        ("working_tree_fingerprint", fingerprint),
        ("config_hash", config),
        ("ignore_policy_version", ignore_policy),
    ] {
        assert_ne!(
            baseline,
            capture_id(changed_project, "graph", "src/file.rs", "analysis-run"),
            "changing {field} must change the capture ID"
        );
    }
    assert_ne!(
        baseline,
        capture_id(project(), "graph-2", "src/file.rs", "analysis-run")
    );
    assert_ne!(
        baseline,
        capture_id(project(), "graph", "src/main.rs", "analysis-run")
    );
    assert_ne!(
        baseline,
        capture_id(project(), "graph", "src/file.rs", "analysis-run-2")
    );
}

#[test]
fn capture_identity_matches_independent_encoding_and_resists_field_boundary_collisions() {
    // Independently computed with Node crypto, UTF-8 byte lengths encoded as
    // uint64 big endian and the line count as uint32 big endian.
    assert_eq!(
        capture_id(project(), "graph", "src/file.rs", "run"),
        "capture:54bea3e94e1c046b9c2cbfd61a2d1bb780666f484721d5a6a06fac6f3b9a9948"
    );
    let mut left = project();
    left.repository_id = "ab".into();
    left.worktree_id = "c".into();
    let mut right = project();
    right.repository_id = "a".into();
    right.worktree_id = "bc".into();
    assert_ne!(
        capture_id(left, "graph", "src/file.rs", "run"),
        capture_id(right, "graph", "src/file.rs", "run")
    );
}

#[test]
fn empty_analysis_run_is_invalid_before_any_read() {
    let reader = MemoryReader::bytes(b"source");
    let locator = locator(
        project(),
        "graph",
        "src/file.rs",
        "old",
        &hash(b"old"),
        1,
        1,
        "old-run",
    );

    let error = capture_source(
        &reader,
        &locator,
        " \t\n",
        SourceLimits::new(1024, 1024).unwrap(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        SourceVerificationError::InvalidEvidence(DomainError::Missing("analysis run"))
    ));
    assert_eq!(reader.observation(), ReadObservation::default());
}

#[test]
fn propagates_reader_errors_and_rejects_invalid_text() {
    let failing = MemoryReader::failing();
    let locator = locator(
        project(),
        "graph",
        "src/file.rs",
        "old",
        &hash(b"old"),
        1,
        1,
        "old-run",
    );
    assert!(matches!(
        capture_source(
            &failing,
            &locator,
            "run",
            SourceLimits::new(1024, 1024).unwrap()
        ),
        Err(SourceVerificationError::Reader(ReaderError))
    ));
    assert_eq!(failing.observation().calls, 1);

    for bytes in [Vec::new(), b"nul\0byte".to_vec(), vec![0xff, 0xfe]] {
        let reader = MemoryReader::bytes(&bytes);
        let error = capture_source(
            &reader,
            &locator,
            "run",
            SourceLimits::new(1024, 1024).unwrap(),
        )
        .unwrap_err();
        assert!(matches!(error, SourceVerificationError::InvalidText));
        assert_eq!(reader.observation().calls, 1);
    }
}

#[test]
fn rejects_reader_output_over_source_budget_even_when_reader_lies() {
    let reader = MemoryReader::bytes(b"1234");
    let locator = locator(
        project(),
        "graph",
        "src/file.rs",
        "old",
        &hash(b"old"),
        1,
        1,
        "old-run",
    );

    let error =
        capture_source(&reader, &locator, "run", SourceLimits::new(3, 3).unwrap()).unwrap_err();

    assert!(matches!(error, SourceVerificationError::SourceTooLarge));
    assert_eq!(
        reader.observation(),
        ReadObservation {
            calls: 1,
            limits: vec![3]
        }
    );
}

#[test]
fn slice_limit_applies_to_the_entire_capture() {
    let bytes = b"1234";
    let reader = MemoryReader::bytes(bytes);
    let locator = locator(
        project(),
        "graph",
        "src/file.rs",
        "old",
        &hash(b"old"),
        1,
        1,
        "old-run",
    );

    let error = capture_source(
        &reader,
        &locator,
        "run",
        SourceLimits::new(1024, 3).unwrap(),
    )
    .unwrap_err();

    assert!(matches!(error, SourceVerificationError::SliceTooLarge));
    assert_eq!(reader.observation().calls, 1);
}

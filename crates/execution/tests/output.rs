#![allow(clippy::unwrap_used)]
use graph_application::{
    ArtifactReader, ArtifactRepository, ArtifactVerificationError, ArtifactWriter,
};
use graph_domain::{
    Artifact, ArtifactProtection, ArtifactRetention, ProjectRef,
    execution::{ChildCompletion, ExecutionCompletion, ScopeCleanup, StopReason, StreamCompletion},
};
use graph_execution::{FixtureRun, OutputPublicationError, publish_fixture_outputs};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    io::{self, Read},
};

#[derive(Default)]
struct Registry(BTreeMap<String, Artifact>);
impl ArtifactRepository for Registry {
    type Error = io::Error;
    fn record_artifact(&mut self, artifact: &Artifact) -> io::Result<bool> {
        if let Some(existing) = self.0.get(artifact.id()) {
            if existing != artifact {
                return Err(io::Error::other("conflict"));
            }
            return Ok(false);
        }
        self.0.insert(artifact.id().into(), artifact.clone());
        Ok(true)
    }
    fn artifact(
        &self,
        id: &str,
        project: &ProjectRef,
        graph: &str,
    ) -> io::Result<Option<Artifact>> {
        Ok(self
            .0
            .get(id)
            .filter(|a| a.project() == project && a.graph_version() == graph)
            .cloned())
    }
}
#[derive(Default)]
struct Blobs {
    bytes: RefCell<BTreeMap<String, Vec<u8>>>,
    fail_stderr: Cell<bool>,
}
impl ArtifactReader for Blobs {
    fn open_artifact(&self, artifact: &Artifact) -> io::Result<Box<dyn Read>> {
        Ok(Box::new(io::Cursor::new(
            self.bytes
                .borrow()
                .get(artifact.id())
                .cloned()
                .ok_or_else(|| io::Error::other("missing"))?,
        )))
    }
}
impl ArtifactWriter for Blobs {
    fn write_artifact(
        &self,
        artifact: &Artifact,
        input: &mut dyn Read,
        max: u64,
    ) -> Result<bool, ArtifactVerificationError> {
        if artifact.kind() == "stderr" && self.fail_stderr.get() {
            return Err(io::Error::other("injected write failure").into());
        }
        let mut bytes = Vec::new();
        input.take(max + 1).read_to_end(&mut bytes)?;
        let mut stored = self.bytes.borrow_mut();
        if let Some(existing) = stored.get(artifact.id()) {
            if existing != &bytes {
                return Err(ArtifactVerificationError::HashMismatch);
            }
            return Ok(false);
        }
        stored.insert(artifact.id().into(), bytes);
        Ok(true)
    }
}
fn descriptor(kind: &str, bytes: &[u8]) -> Artifact {
    Artifact::new(
        kind.into(),
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "w".into(),
            git_head: "h".into(),
            working_tree_fingerprint: "f".into(),
            config_hash: "c".into(),
            ignore_policy_version: "i".into(),
        },
        "g".into(),
        "run".into(),
        format!("{:x}", Sha256::digest(bytes)),
        bytes.len() as u64,
        kind.into(),
        ArtifactRetention::Evidence,
        ArtifactProtection::Unreviewed,
    )
    .unwrap()
}
fn run() -> FixtureRun {
    FixtureRun {
        stdout: vec![0, 255, 10],
        stderr: vec![],
        elapsed_ms: 1,
        unreaped_child: None,
        completion: ExecutionCompletion {
            reason: StopReason::Exited,
            child: ChildCompletion::Reaped { exit_code: Some(0) },
            stdout: StreamCompletion::Complete,
            stderr: StreamCompletion::Complete,
            cleanup: ScopeCleanup::Unverifiable,
        },
    }
}
#[test]
fn binary_and_empty_outputs_publish_replay_without_changing_completion() {
    let run = run();
    let before = run.completion;
    let out = descriptor("stdout", &run.stdout);
    let err = descriptor("stderr", &run.stderr);
    let mut registry = Registry::default();
    let blobs = Blobs::default();
    let first = publish_fixture_outputs(&run, &out, &err, &mut registry, &blobs, 3).unwrap();
    assert!(first.stdout.blob_inserted && first.stderr.blob_inserted);
    assert_eq!(first.stdout.content.artifact(), &out);
    assert_eq!(first.stderr.content.artifact(), &err);
    let replay = publish_fixture_outputs(&run, &out, &err, &mut registry, &blobs, 3).unwrap();
    assert!(!replay.stdout.metadata_inserted && !replay.stderr.metadata_inserted);
    assert!(!replay.stdout.blob_inserted && !replay.stderr.blob_inserted);
    assert_eq!(run.completion, before);
}
#[test]
fn stderr_mismatch_and_total_budget_fail_before_any_write() {
    let run = run();
    let out = descriptor("stdout", &run.stdout);
    for (err, budget) in [
        (descriptor("stderr", b"wrong"), 100),
        (descriptor("stderr", b""), 2),
    ] {
        let mut registry = Registry::default();
        let blobs = Blobs::default();
        assert!(matches!(
            publish_fixture_outputs(&run, &out, &err, &mut registry, &blobs, budget),
            Err(OutputPublicationError::Preflight)
        ));
        assert!(registry.0.is_empty());
        assert!(blobs.bytes.borrow().is_empty());
    }
}
#[test]
fn stderr_failure_preserves_stdout_and_can_retry_without_execution() {
    let run = run();
    let out = descriptor("stdout", &run.stdout);
    let err = descriptor("stderr", &run.stderr);
    let mut registry = Registry::default();
    let blobs = Blobs::default();
    blobs.fail_stderr.set(true);
    let Err(OutputPublicationError::Stderr { stdout, .. }) =
        publish_fixture_outputs(&run, &out, &err, &mut registry, &blobs, 3)
    else {
        panic!("expected partial failure")
    };
    assert_eq!(stdout.content.artifact(), &out);
    assert_eq!(registry.0.len(), 2);
    assert_eq!(blobs.bytes.borrow().len(), 1);
    blobs.fail_stderr.set(false);
    let retry = publish_fixture_outputs(&run, &out, &err, &mut registry, &blobs, 3).unwrap();
    assert!(!retry.stdout.blob_inserted);
    assert!(retry.stderr.blob_inserted);
}

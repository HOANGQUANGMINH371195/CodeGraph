use super::*;
use graph_application::{
    AnalysisRepository, ArtifactReader, ArtifactRepository, ArtifactVerificationError,
    ArtifactWriter,
};
use graph_domain::{AnalysisRun, Artifact, RpcTerminalReceipt};
use graph_execution::{BoundRpcTerminal, RpcPublicationError};
use graph_source::DirectoryArtifacts;
use graph_store::StoreError;
use std::io::{self, Read};

#[path = "journal_kill.rs"]
mod journal_kill;

#[test]
fn incomplete_registered_journal_refuses_replay_then_retries_exact_preparation() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("db");
    let mut store = Store::open(&db).unwrap();
    let guard = finish(root.path(), &mut store, "rpc-context");
    let terminal = guard.get();
    let blobs = blobs(root.path(), terminal);
    let prepared = terminal
        .prepare_receipt(run(terminal), "out".into(), "err".into(), 100, 16384)
        .unwrap();
    let before = store.events(0, 100).unwrap();
    assert!(
        prepared
            .stage_registered_journal(
                &mut store,
                &FailStderr(&blobs),
                "journal".into(),
                65536,
                16384
            )
            .is_err()
    );
    drop(store);
    let mut store = Store::open(&db).unwrap();
    let launch = terminal.spawn_observation().launch();
    let manifest = store
        .artifact(
            "journal",
            launch.execution_snapshot(),
            launch.task().graph_version(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        blobs.open_artifact(&manifest).err().unwrap().kind(),
        io::ErrorKind::NotFound
    );
    graph_application::verify_artifact(&blobs, prepared.receipt().stdout().unwrap(), 16384)
        .unwrap();
    assert!(
        graph_execution::replay_rpc_journal(&mut store, &blobs, &manifest, launch, 65536, 16384)
            .is_err()
    );
    assert!(store.rpc_terminal_receipt(launch).unwrap().is_none());
    assert_eq!(store.events(0, 100).unwrap(), before);
    let retried = prepared
        .stage_registered_journal(&mut store, &blobs, "journal".into(), 65536, 16384)
        .unwrap();
    assert_eq!(retried, manifest);
    assert!(
        graph_execution::replay_rpc_journal(&mut store, &blobs, &manifest, launch, 65536, 16384)
            .unwrap()
    );
    assert_eq!(
        store.rpc_terminal_receipt(launch).unwrap().as_ref(),
        Some(prepared.receipt())
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("cwd/rpc-starts"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn journal_stages_verified_streams_then_manifest_without_ledger_changes() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(root.path().join("db")).unwrap();
    let guard = finish(root.path(), &mut store, "rpc-context");
    let terminal = guard.get();
    let blobs = blobs(root.path(), terminal);
    let prepared = terminal
        .prepare_receipt(run(terminal), "out".into(), "err".into(), 100, 16384)
        .unwrap();
    let before = store.events(0, 100).unwrap();
    struct FailStream;
    impl ArtifactReader for FailStream {
        fn open_artifact(&self, _: &Artifact) -> io::Result<Box<dyn Read>> {
            panic!("failed write must stop before read or manifest publication")
        }
    }
    impl ArtifactWriter for FailStream {
        fn write_artifact(
            &self,
            artifact: &Artifact,
            _: &mut dyn Read,
            _: u64,
        ) -> Result<bool, ArtifactVerificationError> {
            assert_eq!(artifact.kind(), "stdout");
            Err(io::Error::other("injected stream write failure").into())
        }
    }
    assert!(
        prepared
            .stage_journal(&FailStream, "journal".into(), 65536, 16384)
            .is_err()
    );
    let manifest = prepared
        .stage_journal(&blobs, "journal".into(), 65536, 16384)
        .unwrap();
    let loaded = graph_protocol::rpc_journal::decode_reader(
        blobs.open_artifact(&manifest).unwrap(),
        terminal.spawn_observation().launch(),
        65536,
        16384,
    )
    .unwrap();
    assert_eq!(&loaded, prepared.receipt());
    for artifact in [
        loaded.stdout().unwrap(),
        loaded.stderr().unwrap(),
        &manifest,
    ] {
        graph_application::verify_artifact(&blobs, artifact, 65536).unwrap();
    }
    assert_eq!(
        prepared
            .stage_journal(&blobs, "journal".into(), 65536, 16384)
            .unwrap(),
        manifest
    );
    assert_eq!(store.events(0, 100).unwrap(), before);
    let launch = terminal.spawn_observation().launch();
    assert_eq!(
        graph_execution::reopen_rpc_journal(&blobs, &manifest, launch, 65536, 16384).unwrap(),
        loaded
    );
    struct Damaged<'a> {
        blobs: &'a DirectoryArtifacts,
        manifest: bool,
    }
    impl ArtifactReader for Damaged<'_> {
        fn open_artifact(&self, artifact: &Artifact) -> io::Result<Box<dyn Read>> {
            if self.manifest && artifact.kind() == "rpc_journal_manifest" {
                let mut bytes = Vec::new();
                self.blobs
                    .open_artifact(artifact)?
                    .read_to_end(&mut bytes)?;
                bytes[0] ^= 1;
                return Ok(Box::new(io::Cursor::new(bytes)));
            }
            if !self.manifest && artifact.kind() == "stderr" {
                return Err(io::ErrorKind::NotFound.into());
            }
            self.blobs.open_artifact(artifact)
        }
    }
    for corrupt_manifest in [true, false] {
        assert!(
            graph_execution::replay_rpc_journal(
                &mut store,
                &Damaged {
                    blobs: &blobs,
                    manifest: corrupt_manifest
                },
                &manifest,
                launch,
                65536,
                16384,
            )
            .is_err()
        );
        assert_eq!(store.events(0, 100).unwrap(), before);
        assert!(
            graph_execution::reopen_rpc_journal(
                &Damaged {
                    blobs: &blobs,
                    manifest: corrupt_manifest
                },
                &manifest,
                launch,
                65536,
                16384
            )
            .is_err()
        );
    }
    assert_eq!(store.events(0, 100).unwrap(), before);
    let registered = prepared
        .stage_registered_journal(&mut store, &blobs, "journal".into(), 65536, 16384)
        .unwrap();
    assert_eq!(registered, manifest);
    drop(registered);
    drop(manifest);
    drop(store);
    let mut store = Store::open(root.path().join("db")).unwrap();
    let manifest = store
        .artifact(
            "journal",
            launch.execution_snapshot(),
            launch.task().graph_version(),
        )
        .unwrap()
        .unwrap();
    assert!(
        graph_execution::replay_rpc_journal(&mut store, &blobs, &manifest, launch, 65536, 16384)
            .unwrap()
    );
    let committed = store.events(0, 100).unwrap();
    drop(store);
    let mut store = Store::open(root.path().join("db")).unwrap();
    assert!(
        !graph_execution::replay_rpc_journal(&mut store, &blobs, &manifest, launch, 65536, 16384)
            .unwrap()
    );
    assert_eq!(store.events(0, 100).unwrap(), committed);
    use graph_application::RpcTerminalReceiptRepository;
    assert_eq!(
        store.rpc_terminal_receipt(launch).unwrap().as_ref(),
        Some(&loaded)
    );
    let conflicting = terminal
        .prepare_receipt(run(terminal), "out".into(), "err".into(), 101, 16384)
        .unwrap();
    let conflicting_manifest = conflicting
        .stage_journal(&blobs, "journal-conflict".into(), 65536, 16384)
        .unwrap();
    assert_ne!(conflicting.receipt(), &loaded);
    assert!(
        graph_execution::replay_rpc_journal(
            &mut store,
            &blobs,
            &conflicting_manifest,
            launch,
            65536,
            16384,
        )
        .is_err()
    );
    assert_eq!(store.events(0, 100).unwrap(), committed);
    assert_eq!(
        store.rpc_terminal_receipt(launch).unwrap().as_ref(),
        Some(&loaded)
    );
}

#[test]
fn reopened_rpc_outputs_are_reverified_without_new_authority_or_writes() {
    use graph_application::{RpcLaunchQueryRepository, RpcOutputError, verify_rpc_outputs};
    struct NoRead;
    impl ArtifactReader for NoRead {
        fn open_artifact(&self, _: &Artifact) -> io::Result<Box<dyn Read>> {
            panic!("precondition must precede blob reads");
        }
    }
    struct BadBytes(bool);
    impl ArtifactReader for BadBytes {
        fn open_artifact(&self, a: &Artifact) -> io::Result<Box<dyn Read>> {
            if self.0 {
                Err(io::Error::other("secret-blob-path"))
            } else {
                Ok(Box::new(io::Cursor::new(vec![
                    0_u8;
                    a.byte_length() as usize
                ])))
            }
        }
    }
    struct Changing<'a>(&'a Store, std::cell::Cell<usize>);
    impl RpcLaunchQueryRepository for Changing<'_> {
        type Error = StoreError;
        fn rpc_launch_snapshot(
            &self,
            id: &str,
            task: &TaskSpec,
        ) -> Result<Option<graph_application::RpcLaunchLedgerSnapshot>, StoreError> {
            let count = self.1.get();
            self.1.set(count + 1);
            if count == 0 {
                self.0.rpc_launch_snapshot(id, task)
            } else {
                Ok(None)
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("db");
    let mut store = Store::open(&db).unwrap();
    let guard = finish(root.path(), &mut store, "rpc-context");
    let terminal = guard.get();
    let launch = terminal.spawn_observation().launch();
    let blobs = blobs(root.path(), terminal);
    assert!(matches!(
        verify_rpc_outputs(&store, &NoRead, launch, 16384),
        Err(RpcOutputError::Unavailable)
    ));
    let prepared = terminal
        .prepare_receipt(run(terminal), "out".into(), "err".into(), 100, 16384)
        .unwrap();
    prepared.publish(&mut store, &blobs).unwrap();
    let before = store.events(0, 100).unwrap();
    drop(store);
    let store = Store::open(&db).unwrap();
    let total = prepared.receipt().stdout().unwrap().byte_length()
        + prepared.receipt().stderr().unwrap().byte_length();
    assert!(total > 0);
    assert!(matches!(
        verify_rpc_outputs(&store, &NoRead, launch, total - 1),
        Err(RpcOutputError::TooLarge)
    ));
    let wrong = changed(launch, |w| w.host_id.push('x'));
    assert!(matches!(
        verify_rpc_outputs(&store, &NoRead, &wrong, total),
        Err(RpcOutputError::ExpectationMismatch)
    ));
    for missing in [false, true] {
        let error = verify_rpc_outputs(&store, &BadBytes(missing), launch, total).unwrap_err();
        assert!(matches!(error, RpcOutputError::Content(_)));
        assert!(!format!("{error:?} {error}").contains("secret"));
    }
    let changing = Changing(&store, std::cell::Cell::new(0));
    assert!(matches!(
        verify_rpc_outputs(&changing, &blobs, launch, total),
        Err(RpcOutputError::Changed)
    ));
    assert_eq!(changing.1.get(), 2);
    let result = verify_rpc_outputs(&store, &blobs, launch, total).unwrap();
    assert_eq!(result.receipt(), prepared.receipt());
    assert_eq!(
        result.stdout().unwrap().artifact(),
        prepared.receipt().stdout().unwrap()
    );
    assert_eq!(
        result.stderr().unwrap().artifact(),
        prepared.receipt().stderr().unwrap()
    );
    assert_eq!(format!("{result:?}"), "RpcOutputObservation { .. }");
    assert_eq!(store.events(0, 100).unwrap(), before);
    assert_eq!(
        std::fs::read_to_string(root.path().join("cwd/rpc-starts"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

struct TerminalGuard(Option<BoundRpcTerminal>);
impl TerminalGuard {
    fn get(&self) -> &BoundRpcTerminal {
        self.0.as_ref().unwrap()
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if let Some(bound) = self.0.take() {
            let (_, mut terminal) = bound.into_parts();
            if let Some(mut child) = terminal.unreaped_child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
fn finish(root: &Path, store: &mut Store, mode: &str) -> TerminalGuard {
    std::fs::create_dir(root.join("cwd")).unwrap();
    let original = spec(store, &executable());
    let launch = changed(&original, |wire| wire.process.args = vec![mode.into()]);
    let host = TrustedRpcFixtureHost::new(launch.clone(), executable(), root.into(), environment());
    let mut guard = BoundGuard(Some(
        host.launch_bound(&launch, store, &AtomicBool::new(false))
            .unwrap(),
    ));
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut sent = false;
    loop {
        assert!(Instant::now() < deadline);
        let launched = guard.0.as_mut().unwrap();
        if launched.core().is_ready() && !sent {
            launched.prepare_request("fixture/ping", None).unwrap();
            sent = true;
        }
        if launched.poll(false, true).finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let terminal = match guard.0.take().unwrap().finish() {
        Ok(terminal) => TerminalGuard(Some(terminal)),
        Err(unfinished) => {
            guard.0 = Some(unfinished);
            panic!("premature finish");
        }
    };
    assert!(terminal.get().terminal().unreaped_child.is_none());
    terminal
}
fn run(terminal: &BoundRpcTerminal) -> AnalysisRun {
    let launch = terminal.spawn_observation().launch();
    AnalysisRun::new(
        "run".into(),
        launch.execution_snapshot().clone(),
        launch.task().graph_version().into(),
        "fixture".into(),
        "1".into(),
        "a".repeat(64),
        "b".repeat(64),
    )
    .unwrap()
}
fn blobs(root: &Path, terminal: &BoundRpcTerminal) -> DirectoryArtifacts {
    let path = root.join("blobs");
    std::fs::create_dir(&path).unwrap();
    let launch = terminal.spawn_observation().launch();
    DirectoryArtifacts::open(
        path,
        launch.execution_snapshot().clone(),
        launch.task().graph_version().into(),
    )
    .unwrap()
}
struct FailStderr<'a>(&'a DirectoryArtifacts);
impl ArtifactReader for FailStderr<'_> {
    fn open_artifact(&self, a: &Artifact) -> io::Result<Box<dyn Read>> {
        self.0.open_artifact(a)
    }
}
impl ArtifactWriter for FailStderr<'_> {
    fn write_artifact(
        &self,
        a: &Artifact,
        input: &mut dyn Read,
        max: u64,
    ) -> Result<bool, ArtifactVerificationError> {
        if a.kind() == "stderr" {
            return Err(io::Error::other("injected-secret-stderr-error").into());
        }
        self.0.write_artifact(a, input, max)
    }
}

#[test]
fn failed_stderr_preserves_verified_stdout_and_retries_without_spawning_again() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("db");
    let mut store = Store::open(&db).unwrap();
    let guard = finish(root.path(), &mut store, "rpc-context");
    let terminal = guard.get();
    let blobs = blobs(root.path(), terminal);
    let prepared = terminal
        .prepare_receipt(run(terminal), "out".into(), "err".into(), 100, 16384)
        .unwrap();
    let before = store.events(0, 100).unwrap();
    let failure = prepared
        .publish(&mut store, &FailStderr(&blobs))
        .unwrap_err();
    assert!(!format!("{failure:?}").contains("secret"));
    let RpcPublicationError::Stderr { stdout, .. } = failure else {
        panic!("wrong stage")
    };
    assert_eq!(
        stdout.content.artifact(),
        prepared.receipt().stdout().unwrap()
    );
    assert!(
        store
            .rpc_terminal_receipt(terminal.spawn_observation().launch())
            .unwrap()
            .is_none()
    );
    assert_eq!(store.events(0, 100).unwrap(), before);
    drop(store);
    let mut store = Store::open(&db).unwrap();
    let result = prepared.publish(&mut store, &blobs).unwrap();
    assert!(result.receipt_inserted);
    assert!(!result.stdout.blob_inserted);
    assert!(!result.stderr.metadata_inserted);
    assert!(result.stderr.blob_inserted);
    assert_eq!(
        store
            .rpc_terminal_receipt(terminal.spawn_observation().launch())
            .unwrap()
            .as_ref(),
        Some(prepared.receipt())
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("cwd/rpc-starts"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

struct FailReceipt<'a> {
    store: &'a mut Store,
    commit_first: bool,
}
impl ArtifactRepository for FailReceipt<'_> {
    type Error = StoreError;
    fn record_artifact(&mut self, a: &Artifact) -> Result<bool, StoreError> {
        self.store.record_artifact(a)
    }
    fn artifact(&self, id: &str, p: &ProjectRef, g: &str) -> Result<Option<Artifact>, StoreError> {
        self.store.artifact(id, p, g)
    }
}
impl AnalysisRepository for FailReceipt<'_> {
    type Error = StoreError;
    fn record_analysis_run(&mut self, r: &AnalysisRun) -> Result<bool, StoreError> {
        self.store.record_analysis_run(r)
    }
    fn analysis_run(
        &self,
        id: &str,
        p: &ProjectRef,
        g: &str,
    ) -> Result<Option<AnalysisRun>, StoreError> {
        self.store.analysis_run(id, p, g)
    }
}
impl RpcSpawnObservationRepository for FailReceipt<'_> {
    type Error = StoreError;
    fn record_rpc_spawn_observation(
        &mut self,
        r: &RpcSpawnObservation,
    ) -> Result<bool, StoreError> {
        self.store.record_rpc_spawn_observation(r)
    }
    fn rpc_spawn_observation(
        &self,
        l: &RpcLaunchSpec,
    ) -> Result<Option<RpcSpawnObservation>, StoreError> {
        self.store.rpc_spawn_observation(l)
    }
}
impl RpcTerminalReceiptRepository for FailReceipt<'_> {
    type Error = StoreError;
    fn record_rpc_terminal_receipt(&mut self, r: &RpcTerminalReceipt) -> Result<bool, StoreError> {
        if self.commit_first {
            self.store.record_rpc_terminal_receipt(r)?;
        }
        Err(StoreError::Invalid("injected-secret-receipt-error"))
    }
    fn rpc_terminal_receipt(
        &self,
        l: &RpcLaunchSpec,
    ) -> Result<Option<RpcTerminalReceipt>, StoreError> {
        self.store.rpc_terminal_receipt(l)
    }
}

#[test]
fn ambiguous_final_commit_reconciles_original_receipt_after_reopen() {
    for commit_first in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("db");
        let mut store = Store::open(&db).unwrap();
        let guard = finish(root.path(), &mut store, "rpc-context");
        let terminal = guard.get();
        let blobs = blobs(root.path(), terminal);
        let prepared = terminal
            .prepare_receipt(run(terminal), "out".into(), "err".into(), 100, 16384)
            .unwrap();
        let before = store.events(0, 100).unwrap();
        let error = prepared
            .publish(
                &mut FailReceipt {
                    store: &mut store,
                    commit_first,
                },
                &blobs,
            )
            .unwrap_err();
        assert!(!format!("{error:?}").contains("secret"));
        let RpcPublicationError::Receipt { stdout, stderr, .. } = error else {
            panic!("wrong stage")
        };
        assert_eq!(
            stdout.content.artifact(),
            prepared.receipt().stdout().unwrap()
        );
        assert_eq!(
            stderr.content.artifact(),
            prepared.receipt().stderr().unwrap()
        );
        drop(store);
        let mut store = Store::open(&db).unwrap();
        assert_eq!(
            store
                .rpc_terminal_receipt(terminal.spawn_observation().launch())
                .unwrap(),
            commit_first.then(|| prepared.receipt().clone())
        );
        let result = prepared.publish(&mut store, &blobs).unwrap();
        assert_eq!(result.receipt_inserted, !commit_first);
        assert!(!result.stdout.blob_inserted && !result.stderr.blob_inserted);
        assert_eq!(store.events(0, 100).unwrap().len(), before.len() + 1);
        assert_eq!(
            std::fs::read_to_string(root.path().join("cwd/rpc-starts"))
                .unwrap()
                .lines()
                .count(),
            1
        );
    }
}

#[test]
fn malformed_and_truncated_output_publish_evidence_without_upgrading_completion() {
    for mode in ["rpc-malformed", "rpc-stderr"] {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("db");
        let mut store = Store::open(&db).unwrap();
        let guard = finish(root.path(), &mut store, mode);
        let terminal = guard.get();
        let blobs = blobs(root.path(), terminal);
        if mode == "rpc-malformed" {
            assert_eq!(terminal.terminal().completion.reason, StopReason::HostError);
            assert_eq!(terminal.terminal().output.stdout, b"{\n");
        } else {
            assert_eq!(
                terminal.terminal().completion.reason,
                StopReason::OutputLimit
            );
            assert_eq!(
                terminal.terminal().completion.stderr,
                StreamCompletion::Truncated
            );
        }
        let prepared = terminal
            .prepare_receipt(run(terminal), "out".into(), "err".into(), 100, 16384)
            .unwrap();
        let result = prepared.publish(&mut store, &blobs).unwrap();
        assert!(result.receipt_inserted);
        drop(store);
        let store = Store::open(&db).unwrap();
        let receipt = store
            .rpc_terminal_receipt(terminal.spawn_observation().launch())
            .unwrap()
            .unwrap();
        assert_eq!(&receipt, prepared.receipt());
        assert_eq!(receipt.completion(), &terminal.terminal().completion);
        let fresh = graph_application::verify_rpc_outputs(
            &store,
            &blobs,
            terminal.spawn_observation().launch(),
            16384,
        )
        .unwrap();
        assert_eq!(fresh.receipt(), &receipt);
        assert_eq!(
            fresh.receipt().completion(),
            &terminal.terminal().completion
        );
        assert_eq!(
            receipt.pending().len(),
            terminal.terminal().requests.unresolved().len()
        );
        assert_eq!(
            receipt.stderr().unwrap().byte_length(),
            terminal.terminal().output.stderr.len() as u64
        );
    }
}

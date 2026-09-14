use super::*;
use graph_application::ArtifactDiscoveryRepository;
use std::process::{Child, Command, Stdio};

struct OwnedPublisher(Child);
impl Drop for OwnedPublisher {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn park_publisher(root: &Path, stage: &str) -> ! {
    std::fs::write(root.join("ready"), stage.as_bytes()).unwrap();
    loop {
        std::thread::park_timeout(Duration::from_secs(1));
    }
}

fn incomplete_stage(stage: &str) -> bool {
    ["registered", "stdout-cas", "stderr-cas"].contains(&stage)
}

struct PauseCas<'a> {
    blobs: &'a DirectoryArtifacts,
    root: &'a Path,
    stage: &'a str,
}
impl ArtifactReader for PauseCas<'_> {
    fn open_artifact(&self, artifact: &Artifact) -> io::Result<Box<dyn Read>> {
        self.blobs.open_artifact(artifact)
    }
}
impl ArtifactWriter for PauseCas<'_> {
    fn write_artifact(
        &self,
        artifact: &Artifact,
        input: &mut dyn Read,
        max_bytes: u64,
    ) -> Result<bool, ArtifactVerificationError> {
        if self.stage == "registered" {
            assert_eq!(artifact.kind(), "stdout");
            park_publisher(self.root, self.stage);
        }
        let inserted = self.blobs.write_artifact(artifact, input, max_bytes)?;
        if matches!(
            (self.stage, artifact.kind()),
            ("stdout-cas", "stdout")
                | ("stderr-cas", "stderr")
                | ("manifest-cas", "rpc_journal_manifest")
        ) {
            park_publisher(self.root, self.stage);
        }
        Ok(inserted)
    }
}

// Pause only after the real store returns a successful durable metadata commit.
struct PauseAfterArtifact<'a> {
    store: &'a mut Store,
    root: &'a Path,
    stage: &'a str,
}
impl ArtifactRepository for PauseAfterArtifact<'_> {
    type Error = StoreError;
    fn record_artifact(&mut self, artifact: &Artifact) -> Result<bool, StoreError> {
        let inserted = self.store.record_artifact(artifact)?;
        if artifact.kind() == self.stage {
            assert!(inserted);
            park_publisher(self.root, self.stage);
        }
        Ok(inserted)
    }
    fn artifact(&self, id: &str, p: &ProjectRef, g: &str) -> Result<Option<Artifact>, StoreError> {
        self.store.artifact(id, p, g)
    }
}
impl AnalysisRepository for PauseAfterArtifact<'_> {
    type Error = StoreError;
    fn record_analysis_run(&mut self, run: &AnalysisRun) -> Result<bool, StoreError> {
        self.store.record_analysis_run(run)
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
impl RpcSpawnObservationRepository for PauseAfterArtifact<'_> {
    type Error = StoreError;
    fn record_rpc_spawn_observation(
        &mut self,
        observation: &RpcSpawnObservation,
    ) -> Result<bool, StoreError> {
        self.store.record_rpc_spawn_observation(observation)
    }
    fn rpc_spawn_observation(
        &self,
        launch: &RpcLaunchSpec,
    ) -> Result<Option<RpcSpawnObservation>, StoreError> {
        self.store.rpc_spawn_observation(launch)
    }
}
impl RpcTerminalReceiptRepository for PauseAfterArtifact<'_> {
    type Error = StoreError;
    fn record_rpc_terminal_receipt(
        &mut self,
        receipt: &RpcTerminalReceipt,
    ) -> Result<bool, StoreError> {
        self.store.record_rpc_terminal_receipt(receipt)
    }
    fn rpc_terminal_receipt(
        &self,
        launch: &RpcLaunchSpec,
    ) -> Result<Option<RpcTerminalReceipt>, StoreError> {
        self.store.rpc_terminal_receipt(launch)
    }
}

#[test]
#[ignore = "owned journal publisher helper, invoked by parent kill test"]
fn journal_publisher_helper() {
    let root = PathBuf::from(std::env::var_os("GRAPH_JOURNAL_KILL_ROOT").unwrap());
    let stage = std::env::var("GRAPH_JOURNAL_KILL_STAGE").unwrap();
    assert!(
        [
            "registered",
            "stdout-cas",
            "stderr-cas",
            "manifest-cas",
            "handle",
            "stdout",
            "stderr",
            "terminal"
        ]
        .contains(&stage.as_str())
    );
    let mut store = Store::open(root.join("db")).unwrap();
    let guard = finish(&root, &mut store, "rpc-context");
    let terminal = guard.get();
    let blobs = blobs(&root, terminal);
    let prepared = terminal
        .prepare_receipt(run(terminal), "out".into(), "err".into(), 100, 16384)
        .unwrap();
    std::fs::write(
        root.join("expected.json"),
        serde_json::to_vec(&graph_protocol::RpcTerminalReceipt::from(
            prepared.receipt(),
        ))
        .unwrap(),
    )
    .unwrap();
    if incomplete_stage(&stage) || stage == "manifest-cas" {
        prepared
            .stage_registered_journal(
                &mut store,
                &PauseCas {
                    blobs: &blobs,
                    root: &root,
                    stage: &stage,
                },
                "journal".into(),
                65536,
                16384,
            )
            .unwrap();
        panic!("publisher did not pause at CAS boundary {stage}");
    }
    let manifest = prepared
        .stage_registered_journal(&mut store, &blobs, "journal".into(), 65536, 16384)
        .unwrap();
    if ["stdout", "stderr"].contains(&stage.as_str()) {
        graph_execution::replay_rpc_journal(
            &mut PauseAfterArtifact {
                store: &mut store,
                root: &root,
                stage: &stage,
            },
            &blobs,
            &manifest,
            terminal.spawn_observation().launch(),
            65536,
            16384,
        )
        .unwrap();
        panic!("publisher did not pause at requested artifact commit");
    }
    if stage == "terminal" {
        assert!(
            graph_execution::replay_rpc_journal(
                &mut store,
                &blobs,
                &manifest,
                terminal.spawn_observation().launch(),
                65536,
                16384
            )
            .unwrap()
        );
    }
    park_publisher(&root, &stage);
}

#[test]
#[ignore = "fresh recovery helper, invoked after publisher SIGKILL"]
fn journal_recovery_helper() {
    let root = PathBuf::from(std::env::var_os("GRAPH_JOURNAL_KILL_ROOT").unwrap());
    let stage = std::env::var("GRAPH_JOURNAL_KILL_STAGE").unwrap();
    let expected: graph_protocol::RpcTerminalReceipt =
        serde_json::from_slice(&std::fs::read(root.join("expected.json")).unwrap()).unwrap();
    let expected = expected.try_into_domain().unwrap();
    let launch = expected.spawn().launch();
    let mut store = Store::open(root.join("db")).unwrap();
    let mut cursor = String::new();
    let mut candidates = Vec::new();
    let mut exhausted = false;
    // Fixture scan budget; production hosts must choose their own traversal budget.
    for _ in 0..10 {
        let page = store
            .artifacts_after(
                launch.execution_snapshot(),
                launch.task().graph_version(),
                &cursor,
                1,
            )
            .unwrap();
        if page.is_empty() {
            exhausted = true;
            break;
        }
        cursor = page.last().unwrap().id().into();
        candidates.extend(
            page.into_iter()
                .filter(|artifact| artifact.kind() == "rpc_journal_manifest"),
        );
    }
    assert!(exhausted, "fixture discovery budget exhausted");
    assert_eq!(candidates.len(), 1, "ambiguous or missing fixture journal");
    let manifest = candidates.pop().unwrap();
    let blobs = DirectoryArtifacts::open(
        root.join("blobs"),
        launch.execution_snapshot().clone(),
        launch.task().graph_version().into(),
    )
    .unwrap();
    if incomplete_stage(&stage) {
        assert_incomplete_journal(&mut store, &blobs, &manifest, &expected, &stage);
        return;
    }
    assert_eq!(
        graph_execution::replay_rpc_journal(&mut store, &blobs, &manifest, launch, 65536, 16384)
            .unwrap(),
        stage != "terminal",
    );
    assert_eq!(
        store.rpc_terminal_receipt(launch).unwrap().as_ref(),
        Some(&expected)
    );
}

fn assert_incomplete_journal(
    store: &mut Store,
    blobs: &DirectoryArtifacts,
    manifest: &Artifact,
    expected: &RpcTerminalReceipt,
    stage: &str,
) {
    let launch = expected.spawn().launch();
    assert_eq!(
        blobs.open_artifact(manifest).err().unwrap().kind(),
        io::ErrorKind::NotFound
    );
    for (artifact, present) in [
        (expected.stdout().unwrap(), stage != "registered"),
        (expected.stderr().unwrap(), stage == "stderr-cas"),
    ] {
        if present {
            graph_application::verify_artifact(blobs, artifact, 16384).unwrap();
        } else {
            assert_eq!(
                blobs.open_artifact(artifact).err().unwrap().kind(),
                io::ErrorKind::NotFound
            );
        }
    }
    let before = store.events(0, 100).unwrap();
    assert!(
        graph_execution::replay_rpc_journal(store, blobs, manifest, launch, 65536, 16384).is_err()
    );
    assert_eq!(store.events(0, 100).unwrap(), before);
    assert!(store.rpc_terminal_receipt(launch).unwrap().is_none());
    assert_eq!(
        store
            .artifact(
                "journal",
                launch.execution_snapshot(),
                launch.task().graph_version()
            )
            .unwrap()
            .as_ref(),
        Some(manifest)
    );
}

#[test]
fn killed_journal_publisher_replays_without_another_rpc_spawn() {
    use std::os::unix::process::ExitStatusExt;
    for stage in [
        "registered",
        "stdout-cas",
        "stderr-cas",
        "manifest-cas",
        "handle",
        "stdout",
        "stderr",
        "terminal",
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut child = OwnedPublisher(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "publication::journal_kill::journal_publisher_helper",
                ])
                .env_clear()
                .env("GRAPH_JOURNAL_KILL_ROOT", root.path())
                .env("GRAPH_JOURNAL_KILL_STAGE", stage)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "publisher exited early"
            );
            if std::fs::read(root.path().join("ready")).ok().as_deref() == Some(stage.as_bytes()) {
                break;
            }
            assert!(Instant::now() < deadline, "publisher readiness timeout");
            std::thread::sleep(Duration::from_millis(5));
        }
        child.0.kill().unwrap();
        assert_eq!(child.0.wait().unwrap().signal(), Some(9));
        let expected: graph_protocol::RpcTerminalReceipt =
            serde_json::from_slice(&std::fs::read(root.path().join("expected.json")).unwrap())
                .unwrap();
        let expected = expected.try_into_domain().unwrap();
        let launch = expected.spawn().launch();
        let store = Store::open(root.path().join("db")).unwrap();
        let manifest = store
            .artifact(
                "journal",
                launch.execution_snapshot(),
                launch.task().graph_version(),
            )
            .unwrap()
            .unwrap();
        let blobs = DirectoryArtifacts::open(
            root.path().join("blobs"),
            launch.execution_snapshot().clone(),
            launch.task().graph_version().into(),
        )
        .unwrap();
        assert_eq!(
            store.rpc_terminal_receipt(launch).unwrap().is_some(),
            stage == "terminal"
        );
        for (artifact, present) in [
            (
                expected.stdout().unwrap(),
                ["stdout", "stderr", "terminal"].contains(&stage),
            ),
            (
                expected.stderr().unwrap(),
                ["stderr", "terminal"].contains(&stage),
            ),
        ] {
            assert_eq!(
                store
                    .artifact(
                        artifact.id(),
                        launch.execution_snapshot(),
                        launch.task().graph_version()
                    )
                    .unwrap()
                    .as_ref(),
                present.then_some(artifact),
                "unexpected partial ledger at {stage} for {}",
                artifact.kind(),
            );
        }
        let before_recovery = store.events(0, 100).unwrap();
        drop(store);
        let mut recovery = OwnedPublisher(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "publication::journal_kill::journal_recovery_helper",
                ])
                .env_clear()
                .env("GRAPH_JOURNAL_KILL_ROOT", root.path())
                .env("GRAPH_JOURNAL_KILL_STAGE", stage)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = recovery.0.try_wait().unwrap() {
                assert!(status.success(), "recovery failed at {stage}: {status}");
                break;
            }
            assert!(Instant::now() < deadline, "recovery timeout at {stage}");
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut store = Store::open(root.path().join("db")).unwrap();
        let events = store.events(0, 100).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.path().join("cwd/rpc-starts"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        if incomplete_stage(stage) {
            assert_eq!(events, before_recovery);
            assert_incomplete_journal(&mut store, &blobs, &manifest, &expected, stage);
            continue;
        }
        assert_eq!(
            events.len(),
            before_recovery.len() + usize::from(stage != "terminal")
        );
        assert_eq!(&events[..before_recovery.len()], before_recovery.as_slice());
        assert_eq!(
            store.rpc_terminal_receipt(launch).unwrap().as_ref(),
            Some(&expected)
        );
        assert_eq!(
            store.pending_events("journal-recovery", 100).unwrap(),
            events
        );
        for event in &events {
            store
                .acknowledge_event("journal-recovery", event.sequence)
                .unwrap();
        }
        drop(store);
        let mut store = Store::open(root.path().join("db")).unwrap();
        assert!(
            !graph_execution::replay_rpc_journal(
                &mut store, &blobs, &manifest, launch, 65536, 16384
            )
            .unwrap()
        );
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert!(
            store
                .pending_events("journal-recovery", 100)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            store.pending_events("independent-recovery", 100).unwrap(),
            events
        );
        assert_eq!(
            store.rpc_terminal_receipt(launch).unwrap().as_ref(),
            Some(&expected)
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("cwd/rpc-starts"))
                .unwrap()
                .lines()
                .count(),
            1
        );
    }
}

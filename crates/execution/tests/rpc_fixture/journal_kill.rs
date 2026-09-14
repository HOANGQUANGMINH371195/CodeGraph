use super::*;
use std::process::{Child, Command, Stdio};

struct OwnedPublisher(Child);
impl Drop for OwnedPublisher {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "owned journal publisher helper, invoked by parent kill test"]
fn journal_publisher_helper() {
    let root = PathBuf::from(std::env::var_os("GRAPH_JOURNAL_KILL_ROOT").unwrap());
    let stage = std::env::var("GRAPH_JOURNAL_KILL_STAGE").unwrap();
    assert!(["handle", "terminal"].contains(&stage.as_str()));
    let mut store = Store::open(root.join("db")).unwrap();
    let guard = finish(&root, &mut store, "rpc-context");
    let terminal = guard.get();
    let blobs = blobs(&root, terminal);
    let prepared = terminal
        .prepare_receipt(run(terminal), "out".into(), "err".into(), 100, 16384)
        .unwrap();
    let manifest = prepared
        .stage_registered_journal(&mut store, &blobs, "journal".into(), 65536, 16384)
        .unwrap();
    std::fs::write(
        root.join("expected.json"),
        serde_json::to_vec(&graph_protocol::RpcTerminalReceipt::from(
            prepared.receipt(),
        ))
        .unwrap(),
    )
    .unwrap();
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
    std::fs::write(root.join("ready"), stage.as_bytes()).unwrap();
    loop {
        std::thread::park_timeout(Duration::from_secs(1));
    }
}

#[test]
fn killed_journal_publisher_replays_without_another_rpc_spawn() {
    use std::os::unix::process::ExitStatusExt;
    for stage in ["handle", "terminal"] {
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
        let mut store = Store::open(root.path().join("db")).unwrap();
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
        assert_eq!(
            graph_execution::replay_rpc_journal(
                &mut store, &blobs, &manifest, launch, 65536, 16384
            )
            .unwrap(),
            stage == "handle"
        );
        let events = store.events(0, 100).unwrap();
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

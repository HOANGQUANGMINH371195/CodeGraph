use super::*;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct OwnedHost(Child);
impl Drop for OwnedHost {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "owned subprocess helper, invoked by killed_rpc_ledger_host_preserves_boundaries"]
fn rpc_ledger_host_fixture() {
    let root = std::path::PathBuf::from(std::env::var_os("PGA_RPC_KILL_ROOT").unwrap());
    let stage = std::env::var("PGA_RPC_KILL_STAGE").unwrap();
    assert!(["registered", "claim", "spawn", "uncommitted", "terminal"].contains(&stage.as_str()));
    let mut store = Store::open(root.join("db")).unwrap();
    let r = fixture_with_claim(&mut store, stage != "registered");
    if stage != "registered" && stage != "claim" {
        prerequisites(&mut store, &r);
    }
    if stage == "terminal" {
        store.record_rpc_terminal_receipt(&r).unwrap();
    }
    let tx = if stage == "uncommitted" {
        let tx = store
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let raw = serde_json::to_string(&graph_protocol::RpcTerminalReceipt::from(&r)).unwrap();
        tx.execute(
            include_str!("sql/insert_rpc_terminal.sql"),
            params![
                r.spawn().launch().id(),
                r.output_run().id(),
                r.stdout().map(|a| a.id()),
                r.stderr().map(|a| a.id()),
                r.finished_at_ms(),
                raw,
            ],
        )
        .unwrap();
        tx.execute(
            include_str!("sql/insert_task_event.sql"),
            params![
                r.spawn().launch().task().id().as_str(),
                EventKind::RpcTerminalRecorded.as_str(),
                r.finished_at_ms(),
                r.spawn().launch().id(),
            ],
        )
        .unwrap();
        Some(tx)
    } else {
        None
    };
    std::fs::write(root.join("ready"), stage.as_bytes()).unwrap();
    // Parent sends SIGKILL: neither Store nor transaction destructors may run.
    loop {
        std::hint::black_box(&tx);
        std::thread::park_timeout(Duration::from_secs(1));
    }
}

#[test]
fn killed_rpc_ledger_host_preserves_boundaries() {
    use std::os::unix::process::ExitStatusExt;
    for stage in ["registered", "claim", "spawn", "uncommitted", "terminal"] {
        let root = tempfile::tempdir().unwrap();
        let mut child = OwnedHost(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "rpc_terminal::tests::kill_tests::rpc_ledger_host_fixture",
                ])
                .env_clear()
                .env("PGA_RPC_KILL_ROOT", root.path())
                .env("PGA_RPC_KILL_STAGE", stage)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "fixture exited before marker: {stage}"
            );
            if std::fs::read(root.path().join("ready")).ok().as_deref() == Some(stage.as_bytes()) {
                break;
            }
            assert!(Instant::now() < deadline, "fixture deadline: {stage}");
            std::thread::sleep(Duration::from_millis(5));
        }
        child.0.kill().unwrap();
        assert_eq!(child.0.wait().unwrap().signal(), Some(9));
        let mut expected = Store::open(":memory:").unwrap();
        let r = fixture_with_claim(&mut expected, stage != "registered");
        let spawned = stage != "registered" && stage != "claim";
        if spawned {
            prerequisites(&mut expected, &r);
        }
        if stage == "terminal" {
            expected.record_rpc_terminal_receipt(&r).unwrap();
        }
        for _ in 0..2 {
            let mut recovered = Store::open(root.path().join("db")).unwrap();
            let l = r.spawn().launch();
            let s = recovered
                .rpc_launch_snapshot(l.id(), l.task())
                .unwrap()
                .unwrap();
            assert_eq!(s.spec(), l);
            assert_eq!(s.claimed_at_ms(), (stage != "registered").then_some(3));
            assert_eq!(s.spawn_observation(), spawned.then_some(r.spawn()));
            assert_eq!(s.terminal_receipt(), (stage == "terminal").then_some(&r));
            assert_eq!(counts(&recovered), counts(&expected));
            assert_eq!(
                recovered.events(0, 100).unwrap(),
                expected.events(0, 100).unwrap()
            );
            if stage != "registered" {
                assert!(!recovered.claim_rpc_launch(l, 6).unwrap());
            }
            if stage == "terminal" {
                assert!(!recovered.record_rpc_terminal_receipt(&r).unwrap());
            }
            assert_eq!(counts(&recovered), counts(&expected));
        }
        if stage == "registered" {
            let l = r.spawn().launch();
            let mut recovered = Store::open(root.path().join("db")).unwrap();
            assert!(matches!(
                recovered.claim_rpc_launch(l, l.origin_lease().expires_at_ms()),
                Err(StoreError::Unavailable)
            ));
            assert_eq!(counts(&recovered), counts(&expected));
            assert_eq!(
                recovered.events(0, 100).unwrap(),
                expected.events(0, 100).unwrap()
            );
            assert!(recovered.claim_rpc_launch(l, 6).unwrap());
            assert!(!recovered.claim_rpc_launch(l, 6).unwrap());
            assert!(expected.claim_rpc_launch(l, 6).unwrap());
            drop(recovered);
            let mut recovered = Store::open(root.path().join("db")).unwrap();
            let snapshot = recovered
                .rpc_launch_snapshot(l.id(), l.task())
                .unwrap()
                .unwrap();
            assert_eq!(snapshot.claimed_at_ms(), Some(6));
            assert!(snapshot.spawn_observation().is_none());
            assert!(snapshot.terminal_receipt().is_none());
            assert!(!recovered.claim_rpc_launch(l, 7).unwrap());
            assert_eq!(counts(&recovered), counts(&expected));
            assert_eq!(
                recovered.events(0, 100).unwrap(),
                expected.events(0, 100).unwrap()
            );
        }
    }
}

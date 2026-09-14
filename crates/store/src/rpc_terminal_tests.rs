use super::*;
use graph_application::{
    AnalysisRepository, ArtifactRepository, RpcLaunchLedgerSnapshot, RpcLaunchQueryRepository,
    RpcLaunchRepository, RpcSpawnObservationRepository, TaskRepository,
};
use graph_domain::execution::{
    ChildCompletion, ExecutionCompletion, ScopeCleanup, StopReason, StreamCompletion,
};
use graph_domain::{
    AnalysisRun, Artifact, ArtifactProtection, ArtifactRetention, ProjectRef, RpcConnectionSpec,
    RpcProcessSpec, RpcSpawnDisposition, RpcSpawnObservation, TaskId, TaskSpec, WorkerId,
};

#[cfg(target_os = "linux")]
#[path = "rpc_terminal_kill_tests.rs"]
mod kill_tests;

fn fixture(store: &mut Store) -> RpcTerminalReceipt {
    fixture_with_claim(store, true)
}

fn fixture_with_claim(store: &mut Store, claim: bool) -> RpcTerminalReceipt {
    let project = ProjectRef {
        repository_id: "repo".into(),
        worktree_id: "main".into(),
        git_head: "head".into(),
        working_tree_fingerprint: "dirty".into(),
        config_hash: "cfg".into(),
        ignore_policy_version: "1".into(),
    };
    let task = TaskSpec::new(
        TaskId::new("task").unwrap(),
        project.clone(),
        "graph".into(),
        "reader".into(),
        "native".into(),
        vec!["src".into()],
        vec![],
        "context".into(),
        vec!["report".into()],
        100,
    )
    .unwrap();
    store.enqueue(&task, 0).unwrap();
    let lease = store
        .lease(task.id(), &WorkerId::new("worker").unwrap(), 1, 100)
        .unwrap();
    let launch = RpcLaunchSpec::new(
        "launch".into(),
        task,
        lease,
        "host".into(),
        "approval".into(),
        project.clone(),
        RpcProcessSpec::new(
            "fixture".into(),
            vec![],
            ".".into(),
            "a".repeat(64),
            "b".repeat(64),
            1000,
            100,
            4096,
            4096,
        )
        .unwrap(),
        RpcConnectionSpec::new("epoch".into(), "client".into(), "1".into(), false, 2, 4096)
            .unwrap(),
    )
    .unwrap();
    store.register_rpc_launch(&launch, 2).unwrap();
    if claim {
        assert!(store.claim_rpc_launch(&launch, 3).unwrap());
    }
    let spawn = RpcSpawnObservation::new(launch, 4, RpcSpawnDisposition::Spawned, Some(7)).unwrap();
    let run = AnalysisRun::new(
        "run".into(),
        project.clone(),
        "graph".into(),
        "host".into(),
        "1".into(),
        "c".repeat(64),
        "d".repeat(64),
    )
    .unwrap();
    let output = |kind: &str| {
        Artifact::new(
            kind.into(),
            project.clone(),
            "graph".into(),
            "run".into(),
            "e".repeat(64),
            0,
            kind.into(),
            ArtifactRetention::Evidence,
            ArtifactProtection::Unreviewed,
        )
        .unwrap()
    };
    RpcTerminalReceipt::new(
        spawn,
        run,
        5,
        1,
        Some(output("stdout")),
        Some(output("stderr")),
        ExecutionCompletion {
            reason: StopReason::Exited,
            child: ChildCompletion::Reaped { exit_code: Some(0) },
            stdout: StreamCompletion::Complete,
            stderr: StreamCompletion::Complete,
            cleanup: ScopeCleanup::Unverifiable,
        },
        vec![graph_domain::UncertainRpc::new(1, "method".into()).unwrap()],
        None,
    )
    .unwrap()
}
fn prerequisites(store: &mut Store, r: &RpcTerminalReceipt) {
    store.record_rpc_spawn_observation(r.spawn()).unwrap();
    store.record_analysis_run(r.output_run()).unwrap();
    for a in [r.stdout(), r.stderr()].into_iter().flatten() {
        store.record_artifact(a).unwrap();
    }
}
fn counts(store: &Store) -> (i64, i64, i64) {
    store
        .0
        .query_row(
            include_str!("sql/fixture_rpc_terminal_counts.sql"),
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
}

#[test]
fn terminal_snapshot_is_linked_read_only_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db");
    let mut store = Store::open(&db).unwrap();
    let r = fixture(&mut store);
    let l = r.spawn().launch();
    assert!(
        RpcLaunchLedgerSnapshot::new(l.clone(), Some(3), None)
            .unwrap()
            .with_terminal_receipt(Some(r.clone()))
            .is_err()
    );
    let mut wrong = graph_protocol::RpcSpawnObservation::from(r.spawn());
    wrong.process_id = Some(99);
    assert!(
        RpcLaunchLedgerSnapshot::new(l.clone(), Some(3), Some(wrong.try_into_domain().unwrap()))
            .unwrap()
            .with_terminal_receipt(Some(r.clone()))
            .is_err()
    );
    prerequisites(&mut store, &r);
    assert!(
        store
            .rpc_launch_snapshot(l.id(), l.task())
            .unwrap()
            .unwrap()
            .terminal_receipt()
            .is_none()
    );
    store.record_rpc_terminal_receipt(&r).unwrap();
    let before = counts(&store);
    drop(store);
    let store = Store::open(&db).unwrap();
    let snapshot = store
        .rpc_launch_snapshot(l.id(), l.task())
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.terminal_receipt(), Some(&r));
    assert_eq!(snapshot.spawn_observation(), Some(r.spawn()));
    assert_eq!(snapshot.claimed_at_ms(), Some(3));
    let mut wrong = graph_protocol::TaskSpec::from(l.task());
    wrong.graph_version.push('x');
    assert!(
        store
            .rpc_launch_snapshot(l.id(), &wrong.try_into_domain().unwrap())
            .unwrap()
            .is_none()
    );
    assert_eq!(counts(&store), before);
}

#[test]
fn terminal_replay_reopen_conflict_and_immutable_storage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let mut store = Store::open(&path).unwrap();
    let r = fixture(&mut store);
    prerequisites(&mut store, &r);
    let before = counts(&store);
    assert!(store.record_rpc_terminal_receipt(&r).unwrap());
    assert_eq!(counts(&store), (1, before.1 + 1, before.2 + 1));
    assert!(!store.record_rpc_terminal_receipt(&r).unwrap());
    let mut changed = graph_protocol::RpcTerminalReceipt::from(&r);
    changed.finished_at_ms += 1;
    assert!(matches!(
        store.record_rpc_terminal_receipt(&changed.try_into_domain().unwrap()),
        Err(StoreError::Conflict)
    ));
    assert!(
        store
            .0
            .execute_batch(include_str!("sql/fixture_rpc_terminal_update.sql"))
            .is_err()
    );
    assert!(
        store
            .0
            .execute_batch(include_str!("sql/fixture_rpc_terminal_delete.sql"))
            .is_err()
    );
    let events = store.events(0, 100).unwrap();
    assert_eq!(events.last().unwrap().kind, EventKind::RpcTerminalRecorded);
    assert_eq!(events.last().unwrap().payload, "launch");
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(
        store.rpc_terminal_receipt(r.spawn().launch()).unwrap(),
        Some(r.clone())
    );
    assert!(!store.record_rpc_terminal_receipt(&r).unwrap());
    assert_eq!(store.events(0, 100).unwrap(), events);
    assert!(!store.claim_rpc_launch(r.spawn().launch(), 10000).unwrap());
    let mut launch = graph_protocol::RpcLaunchSpec::from(r.spawn().launch());
    launch.host_id = "other".into();
    assert!(
        store
            .rpc_terminal_receipt(&launch.try_into_domain().unwrap())
            .unwrap()
            .is_none()
    );
}

#[test]
fn terminal_requires_all_prerequisites_and_exact_registered_descriptors() {
    let mut store = Store::open(":memory:").unwrap();
    let r = fixture(&mut store);
    for stage in 0..4 {
        assert!(matches!(
            store.record_rpc_terminal_receipt(&r),
            Err(StoreError::Unavailable)
        ));
        assert_eq!(counts(&store).0, 0);
        match stage {
            0 => {
                store.record_rpc_spawn_observation(r.spawn()).unwrap();
            }
            1 => {
                store.record_analysis_run(r.output_run()).unwrap();
            }
            2 => {
                store.record_artifact(r.stdout().unwrap()).unwrap();
            }
            _ => {
                store.record_artifact(r.stderr().unwrap()).unwrap();
            }
        }
    }
    for field in 0..3 {
        let mut changed = graph_protocol::RpcTerminalReceipt::from(&r);
        match field {
            0 => changed.spawn.process_id = Some(8),
            1 => changed.output_run.analyzer_version = "different".into(),
            _ => changed.stdout.as_mut().unwrap().byte_length = 1,
        }
        assert!(matches!(
            store.record_rpc_terminal_receipt(&changed.try_into_domain().unwrap()),
            Err(StoreError::Conflict)
        ));
    }
    assert!(store.record_rpc_terminal_receipt(&r).unwrap());
}

#[test]
fn terminal_outbox_failure_rolls_back_receipt_and_event_then_retries_same_report() {
    let mut store = Store::open(":memory:").unwrap();
    let r = fixture(&mut store);
    prerequisites(&mut store, &r);
    let before = counts(&store);
    store
        .0
        .execute_batch(include_str!("sql/fixture_rpc_terminal_abort.sql"))
        .unwrap();
    assert!(store.record_rpc_terminal_receipt(&r).is_err());
    assert_eq!(counts(&store), before);
    assert!(
        store
            .rpc_terminal_receipt(r.spawn().launch())
            .unwrap()
            .is_none()
    );
    store
        .0
        .execute_batch(include_str!("sql/fixture_rpc_terminal_abort_remove.sql"))
        .unwrap();
    assert!(store.record_rpc_terminal_receipt(&r).unwrap());
}

#[test]
fn terminal_corruption_is_not_absence_or_replay_and_errors_hide_payload() {
    let mut store = Store::open(":memory:").unwrap();
    let r = fixture(&mut store);
    prerequisites(&mut store, &r);
    store.record_rpc_terminal_receipt(&r).unwrap();
    store
        .0
        .execute_batch(include_str!("sql/fixture_rpc_terminal_corrupt.sql"))
        .unwrap();
    let before = counts(&store);
    for error in [
        store.rpc_terminal_receipt(r.spawn().launch()).unwrap_err(),
        store.record_rpc_terminal_receipt(&r).unwrap_err(),
    ] {
        assert!(matches!(error, StoreError::Corrupt(_)));
        assert!(!error.to_string().contains("secret"));
    }
    assert_eq!(counts(&store), before);
}

#[test]
fn v12_upgrade_preserves_registered_prerequisites_and_history_without_terminal_fabrication() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let mut c = Connection::open(&path).unwrap();
    crate::embedded::migrations::runner()
        .set_target(refinery::Target::Version(12))
        .run(&mut c)
        .unwrap();
    let before = crate::embedded::migrations::runner()
        .get_applied_migrations(&mut c)
        .unwrap();
    let mut old = Store(c);
    let r = fixture(&mut old);
    prerequisites(&mut old, &r);
    let events = old.events(0, 100).unwrap();
    let artifacts: String = old
        .0
        .query_row(
            include_str!("sql/fixture_rpc_terminal_artifact_snapshot.sql"),
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(old);
    for _ in 0..2 {
        let mut store = Store::open(&path).unwrap();
        let after = crate::embedded::migrations::runner()
            .get_applied_migrations(&mut store.0)
            .unwrap();
        assert_eq!(&after[..12], before.as_slice());
        assert_eq!(after.len(), 18);
        assert_eq!(counts(&store).0, 0);
        assert_eq!(store.events(0, 100).unwrap(), events);
        let actual: String = store
            .0
            .query_row(
                include_str!("sql/fixture_rpc_terminal_artifact_snapshot.sql"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(actual, artifacts);
        assert_eq!(
            store
                .rpc_spawn_observation(r.spawn().launch())
                .unwrap()
                .as_ref(),
            Some(r.spawn())
        );
        assert_eq!(
            store
                .analysis_run(
                    r.output_run().id(),
                    r.output_run().project(),
                    r.output_run().graph_version()
                )
                .unwrap()
                .as_ref(),
            Some(r.output_run())
        );
        assert!(
            store
                .rpc_terminal_receipt(r.spawn().launch())
                .unwrap()
                .is_none()
        );
    }
    assert!(
        Store::open(&path)
            .unwrap()
            .record_rpc_terminal_receipt(&r)
            .unwrap()
    );
}

#[test]
fn eight_connections_commit_one_terminal_report_even_when_reports_disagree() {
    for differing in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("race.db");
        let mut store = Store::open(&path).unwrap();
        let r = fixture(&mut store);
        prerequisites(&mut store, &r);
        let before = counts(&store);
        let connections: Vec<_> = (0..8).map(|_| Store::open(&path).unwrap()).collect();
        let barrier = std::sync::Barrier::new(8);
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = connections
                .into_iter()
                .enumerate()
                .map(|(i, mut store)| {
                    let mut wire = graph_protocol::RpcTerminalReceipt::from(&r);
                    if differing {
                        wire.finished_at_ms += i as i64;
                    }
                    let report = wire.try_into_domain().unwrap();
                    let barrier = &barrier;
                    scope.spawn(move || {
                        barrier.wait();
                        let result = store.record_rpc_terminal_receipt(&report);
                        (report, result)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(
            results
                .iter()
                .filter(|(_, result)| matches!(result, Ok(true)))
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|(_, result)| if differing {
                    matches!(result, Err(StoreError::Conflict))
                } else {
                    matches!(result, Ok(false))
                })
                .count(),
            7
        );
        let winner = &results
            .iter()
            .find(|(_, result)| matches!(result, Ok(true)))
            .unwrap()
            .0;
        assert_eq!(counts(&store), (1, before.1 + 1, before.2 + 1));
        drop(store);
        let mut reopened = Store::open(&path).unwrap();
        assert_eq!(
            reopened
                .rpc_terminal_receipt(r.spawn().launch())
                .unwrap()
                .as_ref(),
            Some(winner)
        );
        assert!(!reopened.record_rpc_terminal_receipt(winner).unwrap());
        assert_eq!(
            reopened.events(0, 100).unwrap().last().unwrap().at_ms,
            winner.finished_at_ms()
        );
    }
}

#[test]
fn pinned_launch_read_does_not_mix_in_later_terminal_commit() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("snapshot.db");
    let mut writer = Store::open(&db).unwrap();
    let r = fixture(&mut writer);
    let reader = Store::open(&db).unwrap();
    let launch = r.spawn().launch();
    let tx = reader.0.unchecked_transaction().unwrap();
    // Pin the WAL snapshot with the same launch join used by inspection.
    let old_spawn: Option<String> = tx
        .query_row(
            include_str!("sql/select_rpc_launch_snapshot.sql"),
            [launch.id()],
            |row| row.get(7),
        )
        .unwrap();
    assert!(old_spawn.is_none());
    prerequisites(&mut writer, &r);
    writer.record_rpc_terminal_receipt(&r).unwrap();
    assert!(crate::rpc_spawn::read(&tx, launch.id()).unwrap().is_none());
    assert!(super::read(&tx, launch.id()).unwrap().is_none());
    tx.commit().unwrap();
    let before = counts(&writer);
    let snapshot = reader
        .rpc_launch_snapshot(launch.id(), launch.task())
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.spawn_observation(), Some(r.spawn()));
    assert_eq!(snapshot.terminal_receipt(), Some(&r));
    assert_eq!(counts(&writer), before);
}

#[test]
fn eight_inspectors_observe_coherent_staged_terminal_publication() {
    use std::sync::mpsc;
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("inspectors.db");
    let mut writer = Store::open(&db).unwrap();
    let r = fixture(&mut writer);
    let before = counts(&writer);
    // Open/migrate before racing; the race is queries against publication.
    let readers: Vec<_> = (0..8).map(|_| Store::open(&db).unwrap()).collect();
    let (ready_tx, ready_rx) = mpsc::channel();
    std::thread::scope(|scope| {
        let mut starts = Vec::new();
        let mut finishes = Vec::new();
        let mut handles = Vec::new();
        for reader in readers {
            let (start_tx, start_rx) = mpsc::channel();
            let (finish_tx, finish_rx) = mpsc::channel();
            starts.push(start_tx);
            finishes.push(finish_tx);
            let ready = ready_tx.clone();
            let r = &r;
            handles.push(scope.spawn(move || {
                let l = r.spawn().launch();
                let initial = reader
                    .rpc_launch_snapshot(l.id(), l.task())
                    .unwrap()
                    .unwrap();
                assert!(initial.spawn_observation().is_none());
                assert!(initial.terminal_receipt().is_none());
                ready.send(()).unwrap();
                start_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                let mut seen_spawn = false;
                let mut seen_terminal = false;
                for _ in 0..100 {
                    let s = reader
                        .rpc_launch_snapshot(l.id(), l.task())
                        .unwrap()
                        .unwrap();
                    assert_eq!(s.spec(), l);
                    assert_eq!(s.claimed_at_ms(), Some(3));
                    if let Some(spawn) = s.spawn_observation() {
                        assert_eq!(spawn, r.spawn());
                        seen_spawn = true;
                    } else {
                        assert!(!seen_spawn);
                    }
                    if let Some(terminal) = s.terminal_receipt() {
                        assert_eq!(terminal, r);
                        assert_eq!(s.spawn_observation(), Some(terminal.spawn()));
                        seen_terminal = true;
                    } else {
                        assert!(!seen_terminal);
                    }
                }
                finish_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                let final_state = reader
                    .rpc_launch_snapshot(l.id(), l.task())
                    .unwrap()
                    .unwrap();
                assert_eq!(final_state.terminal_receipt(), Some(r));
            }));
        }
        for _ in 0..8 {
            ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        }
        for start in starts {
            start.send(()).unwrap();
        }
        prerequisites(&mut writer, &r);
        assert!(writer.record_rpc_terminal_receipt(&r).unwrap());
        for finish in finishes {
            finish.send(()).unwrap();
        }
        for handle in handles {
            handle.join().unwrap();
        }
    });
    // Spawn and terminal each add one event/outbox row; readers add none.
    assert_eq!(counts(&writer), (1, before.1 + 2, before.2 + 2));
    assert!(!writer.claim_rpc_launch(r.spawn().launch(), 6).unwrap());
    drop(writer);
    let reopened = Store::open(&db).unwrap();
    assert_eq!(
        reopened
            .rpc_launch_snapshot(r.spawn().launch().id(), r.spawn().launch().task())
            .unwrap()
            .unwrap()
            .terminal_receipt(),
        Some(&r)
    );
}

#[test]
fn late_terminal_report_does_not_undo_cancel_or_refresh_expired_claim() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("late.db");
    let mut store = Store::open(&path).unwrap();
    let r = fixture(&mut store);
    prerequisites(&mut store, &r);
    let actor = WorkerId::new("coordinator").unwrap();
    assert!(
        store
            .cancel(r.spawn().launch().task().id(), &actor, "stop", 200)
            .unwrap()
    );
    let before = store.events(0, 100).unwrap();
    let mut wire = graph_protocol::RpcTerminalReceipt::from(&r);
    wire.finished_at_ms = 300;
    let late = wire.try_into_domain().unwrap();
    assert!(store.record_rpc_terminal_receipt(&late).unwrap());
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(
        store.rpc_terminal_receipt(r.spawn().launch()).unwrap(),
        Some(late.clone())
    );
    assert!(!store.record_rpc_terminal_receipt(&late).unwrap());
    assert!(
        !store
            .cancel(r.spawn().launch().task().id(), &actor, "again", 400)
            .unwrap()
    );
    assert!(!store.claim_rpc_launch(r.spawn().launch(), 400).unwrap());
    assert!(
        store
            .lease(r.spawn().launch().task().id(), &actor, 400, 100)
            .is_err()
    );
    let after = store.events(0, 100).unwrap();
    assert_eq!(&after[..before.len()], before.as_slice());
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(after.last().unwrap().kind, EventKind::RpcTerminalRecorded);
}

#[test]
fn terminal_damaged_references_fail_before_scope_filter_without_writes() {
    for sql in [
        include_str!("sql/fixture_rpc_terminal_damage_time.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_run_index.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_stdout_index.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_stderr_index.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_spawn_missing.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_run_missing.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_artifact_missing.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_run_scope.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_artifact_scope.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_run_descriptor.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_artifact_descriptor.sql"),
        include_str!("sql/fixture_rpc_terminal_damage_spawn_descriptor.sql"),
    ] {
        let mut store = Store::open(":memory:").unwrap();
        let r = fixture(&mut store);
        prerequisites(&mut store, &r);
        store.record_rpc_terminal_receipt(&r).unwrap();
        store
            .0
            .execute_batch(include_str!("sql/fixture_rpc_terminal_damage_setup.sql"))
            .unwrap();
        store.0.execute_batch(sql).unwrap();
        let before = counts(&store);
        let mut other = graph_protocol::RpcLaunchSpec::from(r.spawn().launch());
        other.host_id = "other".into();
        let mut other_task = graph_protocol::TaskSpec::from(r.spawn().launch().task());
        other_task.graph_version.push('x');
        for task in [
            r.spawn().launch().task().clone(),
            other_task.try_into_domain().unwrap(),
        ] {
            assert!(
                matches!(
                    store.rpc_launch_snapshot(r.spawn().launch().id(), &task),
                    Err(StoreError::Corrupt(_))
                ),
                "{sql}"
            );
        }
        for result in [
            store.rpc_terminal_receipt(r.spawn().launch()),
            store.rpc_terminal_receipt(&other.try_into_domain().unwrap()),
        ] {
            assert!(
                matches!(result, Err(StoreError::Corrupt(_))),
                "{sql}: {result:?}"
            );
        }
        assert!(
            matches!(
                store.record_rpc_terminal_receipt(&r),
                Err(StoreError::Corrupt(_))
            ),
            "{sql}"
        );
        assert_eq!(counts(&store), before);
    }
}

//! Historical reports only: these tests never execute or authenticate a process.
use graph_application::{
    RpcLaunchQueryRepository, RpcLaunchRepository, RpcSpawnObservationRepository, TaskRepository,
};
use graph_domain::{
    EventKind, ProjectRef, RpcConnectionSpec, RpcLaunchSpec, RpcProcessSpec,
    RpcSpawnDisposition as Disposition, RpcSpawnObservation, TaskId, TaskSpec, WorkerId,
};
use graph_store::{Store, StoreError};
use rusqlite::{Connection, params};
use std::{path::Path, sync::Barrier, thread};

fn fixture(store: &mut Store) -> RpcLaunchSpec {
    let task = TaskSpec::new(
        TaskId::new("task").unwrap(),
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "source".into(),
            git_head: "head".into(),
            working_tree_fingerprint: "dirty-source".into(),
            config_hash: "config".into(),
            ignore_policy_version: "1".into(),
        },
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
        .lease(task.id(), &WorkerId::new("worker").unwrap(), 10, 100)
        .unwrap();
    let mut snapshot = task.project().clone();
    snapshot.worktree_id = "execution".into();
    RpcLaunchSpec::new(
        "launch".into(),
        task,
        lease,
        "host".into(),
        "unverified-approval".into(),
        snapshot,
        RpcProcessSpec::new(
            "/fixture/never-executed".into(),
            vec!["".into(), "secret-argument".into(), "tiếng Việt".into()],
            "src".into(),
            "a".repeat(64),
            "b".repeat(64),
            1000,
            200,
            4096,
            8192,
        )
        .unwrap(),
        RpcConnectionSpec::new(
            "epoch".into(),
            "fixture".into(),
            "1.2".into(),
            true,
            7,
            16384,
        )
        .unwrap(),
    )
    .unwrap()
}

fn claimed(store: &mut Store) -> RpcLaunchSpec {
    let spec = fixture(store);
    assert!(store.register_rpc_launch(&spec, 11).unwrap());
    assert!(store.claim_rpc_launch(&spec, 12).unwrap());
    spec
}

fn observation(spec: &RpcLaunchSpec, disposition: Disposition, at: i64) -> RpcSpawnObservation {
    RpcSpawnObservation::new(
        spec.clone(),
        at,
        disposition,
        (disposition == Disposition::Spawned).then_some(42),
    )
    .unwrap()
}

fn changed(
    spec: &RpcLaunchSpec,
    edit: impl FnOnce(&mut graph_protocol::RpcLaunchSpec),
) -> RpcLaunchSpec {
    let mut wire = graph_protocol::RpcLaunchSpec::from(spec);
    edit(&mut wire);
    wire.try_into_domain().unwrap()
}

fn counts(path: &Path) -> (i64, i64, i64, i64, i64) {
    Connection::open(path)
        .unwrap()
        .query_row(include_str!("sql/rpc_spawn_counts.sql"), [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
}

fn assert_observed_event(store: &Store, path: &Path, report: &RpcSpawnObservation) {
    let events = store.events(0, 100).unwrap();
    let reports: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EventKind::RpcSpawnObserved)
        .collect();
    assert_eq!(reports.len(), 1);
    let event = reports[0];
    assert_eq!(event.at_ms, report.observed_at_ms());
    assert_eq!(event.task_id, *report.launch().task().id());
    assert_eq!(event.payload, report.launch().id());
    let outbox: i64 = Connection::open(path)
        .unwrap()
        .query_row(
            include_str!("sql/rpc_spawn_event_outbox.sql"),
            [event.sequence],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(outbox, 1);
}

#[test]
fn all_dispositions_survive_reopen_and_exact_replay_without_reclaim() {
    for disposition in [
        Disposition::CancelledBeforeSpawn,
        Disposition::ExpiredBeforeSpawn,
        Disposition::SpawnFailed,
        Disposition::Spawned,
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("spawn.sqlite");
        let mut store = Store::open(&path).unwrap();
        let spec = claimed(&mut store);
        let report = observation(&spec, disposition, 13);
        assert!(store.rpc_spawn_observation(&spec).unwrap().is_none());
        let before = counts(&path);
        assert!(store.record_rpc_spawn_observation(&report).unwrap());
        assert_eq!(
            counts(&path),
            (before.0 + 1, before.1 + 1, before.2 + 1, before.3, before.4)
        );
        assert_observed_event(&store, &path, &report);
        let after = counts(&path);
        let events = store.events(0, 100).unwrap();
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(
            store.rpc_spawn_observation(&spec).unwrap(),
            Some(report.clone())
        );
        let snapshot = store
            .rpc_launch_snapshot(spec.id(), spec.task())
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.claimed_at_ms(), Some(12));
        assert_eq!(snapshot.spawn_observation(), Some(&report));
        assert!(!store.record_rpc_spawn_observation(&report).unwrap());
        assert!(!store.claim_rpc_launch(&spec, 200).unwrap());
        assert_eq!(
            store
                .rpc_launch_snapshot(spec.id(), spec.task())
                .unwrap()
                .unwrap()
                .claimed_at_ms(),
            Some(12)
        );
        assert_eq!(counts(&path), after);
        assert_eq!(store.events(0, 100).unwrap(), events);
    }
}

#[test]
fn historical_reports_can_be_written_after_cancel_or_expiry() {
    for cancel in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("late.sqlite");
        let mut store = Store::open(&path).unwrap();
        let spec = claimed(&mut store);
        if cancel {
            store
                .cancel(
                    spec.task().id(),
                    &WorkerId::new("coordinator").unwrap(),
                    "stop",
                    13,
                )
                .unwrap();
        }
        let disposition = if cancel {
            Disposition::CancelledBeforeSpawn
        } else {
            Disposition::ExpiredBeforeSpawn
        };
        let report = observation(&spec, disposition, spec.origin_lease().expires_at_ms() + 1);
        assert!(store.record_rpc_spawn_observation(&report).unwrap());
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(
            store.rpc_spawn_observation(&spec).unwrap(),
            Some(report.clone())
        );
        assert!(!store.record_rpc_spawn_observation(&report).unwrap());
        assert!(!store.claim_rpc_launch(&spec, 500).unwrap());
        assert_observed_event(&store, &path, &report);
    }
}

#[test]
fn absent_and_unclaimed_launches_reject_without_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = fixture(&mut store);
    for registered in [false, true] {
        if registered {
            store.register_rpc_launch(&spec, 11).unwrap();
        }
        let before = counts(&path);
        for disposition in [
            Disposition::CancelledBeforeSpawn,
            Disposition::ExpiredBeforeSpawn,
            Disposition::SpawnFailed,
            Disposition::Spawned,
        ] {
            assert!(matches!(
                store.record_rpc_spawn_observation(&observation(&spec, disposition, 13)),
                Err(StoreError::Unavailable)
            ));
            assert!(store.rpc_spawn_observation(&spec).unwrap().is_none());
        }
        assert_eq!(counts(&path), before);
    }
}

#[test]
fn every_full_spec_component_is_bound_for_writes_and_filtered_for_reads() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("scope.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = claimed(&mut store);
    for recorded in [false, true] {
        if recorded {
            store
                .record_rpc_spawn_observation(&observation(&spec, Disposition::Spawned, 13))
                .unwrap();
        }
        let before = counts(&path);
        for field in 0..13 {
            let wrong = changed(&spec, |w| match field {
                0 => w.host_id.push('x'),
                1 => w.approval_id.push('x'),
                2 => w.connection.epoch.push('x'),
                3 => w.connection.max_frame += 1,
                4 => w.process.args.push("extra".into()),
                5 => w.process.environment_sha256 = "c".repeat(64),
                6 => w.execution_snapshot.git_head.push('x'),
                7 => w.origin_lease.owner.push('x'),
                8 => w.origin_lease.expires_at_ms += 1,
                9 => w.origin_lease.fencing_token += 1,
                10 => w.task.context_ref.push('x'),
                11 => w.task.project.working_tree_fingerprint.push('x'),
                _ => w.process.executable_sha256 = "d".repeat(64),
            });
            assert!(
                store.rpc_spawn_observation(&wrong).unwrap().is_none(),
                "field {field}"
            );
            assert!(
                matches!(
                    store.record_rpc_spawn_observation(&observation(
                        &wrong,
                        Disposition::Spawned,
                        13
                    )),
                    Err(StoreError::Conflict)
                ),
                "field {field}"
            );
        }
        let missing = changed(&spec, |w| w.id = "missing".into());
        assert!(store.rpc_spawn_observation(&missing).unwrap().is_none());
        assert!(matches!(
            store.record_rpc_spawn_observation(&observation(&missing, Disposition::Spawned, 13)),
            Err(StoreError::Unavailable)
        ));
        assert_eq!(counts(&path), before);
    }
}

#[test]
fn changed_report_conflicts_and_preserves_original() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("immutable.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = claimed(&mut store);
    let original = observation(&spec, Disposition::Spawned, 13);
    store.record_rpc_spawn_observation(&original).unwrap();
    let before = counts(&path);
    for different in [
        observation(&spec, Disposition::Spawned, 14),
        observation(&spec, Disposition::SpawnFailed, 13),
        RpcSpawnObservation::new(spec.clone(), 13, Disposition::Spawned, Some(43)).unwrap(),
    ] {
        assert!(matches!(
            store.record_rpc_spawn_observation(&different),
            Err(StoreError::Conflict)
        ));
        assert_eq!(
            store.rpc_spawn_observation(&spec).unwrap(),
            Some(original.clone())
        );
        assert_eq!(counts(&path), before);
    }
}

#[test]
fn event_and_outbox_failures_roll_back_observation_and_allow_retry() {
    for (fail, restore) in [
        (
            include_str!("sql/rpc_spawn_fail_event.sql"),
            include_str!("sql/rpc_spawn_restore_event.sql"),
        ),
        (
            include_str!("sql/rpc_spawn_fail_outbox.sql"),
            include_str!("sql/rpc_spawn_restore_outbox.sql"),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("rollback.sqlite");
        let mut store = Store::open(&path).unwrap();
        let spec = claimed(&mut store);
        let report = observation(&spec, Disposition::Spawned, 13);
        let db = Connection::open(&path).unwrap();
        db.execute_batch(fail).unwrap();
        let before = counts(&path);
        let events = store.events(0, 100).unwrap();
        assert!(matches!(
            store.record_rpc_spawn_observation(&report),
            Err(StoreError::Sql(_))
        ));
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert!(store.rpc_spawn_observation(&spec).unwrap().is_none());
        assert_eq!(counts(&path), before);
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(
            store
                .rpc_launch_snapshot(spec.id(), spec.task())
                .unwrap()
                .unwrap()
                .claimed_at_ms(),
            Some(12)
        );
        db.execute_batch(restore).unwrap();
        assert!(store.record_rpc_spawn_observation(&report).unwrap());
        assert!(!store.record_rpc_spawn_observation(&report).unwrap());
        assert_observed_event(&store, &path, &report);
    }
}

#[test]
fn eight_independent_connections_record_one_observation_event_and_outbox() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("race.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = claimed(&mut store);
    let report = observation(&spec, Disposition::Spawned, 13);
    let before = counts(&path);
    let connections: Vec<_> = (0..8).map(|_| Store::open(&path).unwrap()).collect();
    let barrier = Barrier::new(8);
    let results = thread::scope(|scope| {
        let handles: Vec<_> = connections
            .into_iter()
            .map(|mut connection| {
                let barrier = &barrier;
                let report = &report;
                scope.spawn(move || {
                    barrier.wait();
                    connection.record_rpc_spawn_observation(report)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|&&inserted| inserted).count(), 1);
    assert_eq!(results.iter().filter(|&&inserted| !inserted).count(), 7);
    assert_eq!(
        counts(&path),
        (before.0 + 1, before.1 + 1, before.2 + 1, before.3, before.4)
    );
    assert_eq!(
        store.rpc_spawn_observation(&spec).unwrap(),
        Some(report.clone())
    );
    assert_observed_event(&store, &path, &report);
}

fn assert_sanitized(error: StoreError) {
    assert!(matches!(error, StoreError::Corrupt(_)), "{error:?}");
    assert!(!error.to_string().contains("secret"));
    assert!(!format!("{error:?}").contains("secret"));
}

#[test]
fn corrupt_descriptor_and_denormalized_time_fail_closed_before_scope_filtering() {
    for case in 0..7 {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("corrupt.sqlite");
        let mut store = Store::open(&path).unwrap();
        let spec = claimed(&mut store);
        let report = observation(&spec, Disposition::Spawned, 13);
        store.record_rpc_spawn_observation(&report).unwrap();
        let mut wire = graph_protocol::RpcSpawnObservation::from(&report);
        let mut at = 13;
        match case {
            1 => wire.launch.id = "secret-other-launch".into(),
            2 => wire
                .launch
                .process
                .args
                .push("secret-other-argument".into()),
            3 => wire.process_id = Some(0),
            4 => wire.disposition = "secret-invalid".into(),
            5 => at = 14,
            6 => wire.observed_at_ms = -1,
            _ => (),
        }
        let raw = if case == 0 {
            "{secret-invalid-json".into()
        } else {
            serde_json::to_string(&wire).unwrap()
        };
        Connection::open(&path)
            .unwrap()
            .execute(
                include_str!("sql/rpc_spawn_corrupt_observation.sql"),
                params![spec.id(), raw, at],
            )
            .unwrap();
        let before = counts(&path);
        let events = store.events(0, 100).unwrap();
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert_sanitized(store.rpc_spawn_observation(&spec).unwrap_err());
        let wrong = changed(&spec, |w| w.host_id.push('x'));
        assert_sanitized(store.rpc_spawn_observation(&wrong).unwrap_err());
        assert_sanitized(
            store
                .rpc_launch_snapshot(spec.id(), spec.task())
                .unwrap_err(),
        );
        let mut wrong_task = graph_protocol::TaskSpec::from(spec.task());
        wrong_task.context_ref.push('x');
        assert_sanitized(
            store
                .rpc_launch_snapshot(spec.id(), &wrong_task.try_into_domain().unwrap())
                .unwrap_err(),
        );
        assert_sanitized(store.record_rpc_spawn_observation(&report).unwrap_err());
        assert_eq!(counts(&path), before);
        assert_eq!(store.events(0, 100).unwrap(), events);
    }
}

#[test]
fn corrupt_registered_launch_or_claim_is_not_a_valid_observation() {
    for corruption in [
        include_str!("sql/rpc_spawn_corrupt_host.sql"),
        include_str!("sql/rpc_spawn_corrupt_launch.sql"),
        include_str!("sql/rpc_spawn_corrupt_task.sql"),
        include_str!("sql/rpc_spawn_corrupt_claim.sql"),
        include_str!("sql/rpc_spawn_remove_claim.sql"),
        include_str!("sql/rpc_spawn_remove_launch.sql"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("linkage.sqlite");
        let mut store = Store::open(&path).unwrap();
        let spec = claimed(&mut store);
        let report = observation(&spec, Disposition::Spawned, 13);
        store.record_rpc_spawn_observation(&report).unwrap();
        Connection::open(&path)
            .unwrap()
            .execute_batch(corruption)
            .unwrap();
        let before = counts(&path);
        let events = store.events(0, 100).unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_sanitized(store.rpc_spawn_observation(&spec).unwrap_err());
        let wrong = changed(&spec, |w| w.approval_id.push('x'));
        assert_sanitized(store.rpc_spawn_observation(&wrong).unwrap_err());
        assert_eq!(counts(&path), before);
        assert_eq!(store.events(0, 100).unwrap(), events);
    }
}

#[test]
fn combined_snapshot_rejects_observation_without_a_claim() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("orphan-claim.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = claimed(&mut store);
    let report = observation(&spec, Disposition::Spawned, 13);
    store.record_rpc_spawn_observation(&report).unwrap();
    Connection::open(&path)
        .unwrap()
        .execute_batch(include_str!("sql/rpc_spawn_remove_claim.sql"))
        .unwrap();
    let before = counts(&path);
    assert_sanitized(
        store
            .rpc_launch_snapshot(spec.id(), spec.task())
            .unwrap_err(),
    );
    assert_eq!(counts(&path), before);
}

#[test]
fn combined_read_model_requires_an_exact_claimed_launch_for_observation() {
    use graph_application::RpcLaunchLedgerSnapshot;
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(root.path().join("model.sqlite")).unwrap();
    let spec = fixture(&mut store);
    let report = observation(&spec, Disposition::Spawned, 13);
    assert!(RpcLaunchLedgerSnapshot::new(spec.clone(), None, Some(report.clone())).is_err());
    assert!(RpcLaunchLedgerSnapshot::new(spec.clone(), Some(-1), Some(report.clone())).is_err());
    let wrong = changed(&spec, |wire| wire.host_id.push('x'));
    assert!(RpcLaunchLedgerSnapshot::new(wrong, Some(12), Some(report.clone())).is_err());
    let snapshot =
        RpcLaunchLedgerSnapshot::new(spec.clone(), Some(12), Some(report.clone())).unwrap();
    assert_eq!(snapshot.spawn_observation(), Some(&report));
    assert!(
        store
            .rpc_launch_snapshot(spec.id(), spec.task())
            .unwrap()
            .is_none()
    );
}

#[test]
fn concurrent_combined_snapshots_never_pair_an_observation_with_a_missing_claim() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("snapshot-race.sqlite");
    let mut writer = Store::open(&path).unwrap();
    let spec = fixture(&mut writer);
    writer.register_rpc_launch(&spec, 11).unwrap();
    let report = observation(&spec, Disposition::Spawned, 13);
    let before = counts(&path);
    let readers = (0..8)
        .map(|_| Store::open(&path).unwrap())
        .collect::<Vec<_>>();
    let barrier = Barrier::new(9);
    thread::scope(|scope| {
        for reader in readers {
            let (barrier, spec, report) = (&barrier, &spec, &report);
            scope.spawn(move || {
                let initial = reader
                    .rpc_launch_snapshot(spec.id(), spec.task())
                    .unwrap()
                    .unwrap();
                assert!(initial.claimed_at_ms().is_none() && initial.spawn_observation().is_none());
                barrier.wait();
                let mut last = 0;
                for _ in 0..100 {
                    let snapshot = reader
                        .rpc_launch_snapshot(spec.id(), spec.task())
                        .unwrap()
                        .unwrap();
                    assert_eq!(snapshot.spec(), spec);
                    let stage = match (snapshot.claimed_at_ms(), snapshot.spawn_observation()) {
                        (None, None) => 0,
                        (Some(12), None) => 1,
                        (Some(12), Some(actual)) => {
                            assert_eq!(actual, report);
                            2
                        }
                        _ => panic!("inconsistent snapshot"),
                    };
                    assert!(stage >= last);
                    last = stage;
                }
            });
        }
        barrier.wait();
        writer.claim_rpc_launch(&spec, 12).unwrap();
        writer.record_rpc_spawn_observation(&report).unwrap();
    });
    assert_eq!(
        writer
            .rpc_launch_snapshot(spec.id(), spec.task())
            .unwrap()
            .unwrap()
            .spawn_observation(),
        Some(&report)
    );
    assert_eq!(
        counts(&path),
        (
            before.0 + 1,
            before.1 + 2,
            before.2 + 2,
            before.3 + 1,
            before.4
        )
    );
}

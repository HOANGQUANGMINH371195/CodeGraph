//! Durable ledger assertions only: fixture approval labels grant no host authority.
use graph_application::{
    RpcLaunchLedgerSnapshot, RpcLaunchQueryRepository, RpcLaunchRepository, TaskRepository,
};
use graph_domain::{
    EventKind, Lease, ProjectRef, RpcConnectionSpec, RpcLaunchSpec, RpcProcessSpec, TaskId,
    TaskSpec, WorkerId,
};
use graph_store::{Store, StoreError};
use rusqlite::Connection;
use std::{path::Path, sync::Barrier, thread};

fn task(id: &str) -> TaskSpec {
    TaskSpec::new(
        TaskId::new(id).unwrap(),
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
    .unwrap()
}

fn launch(task: TaskSpec, lease: Lease, id: &str, host: &str, epoch: &str) -> RpcLaunchSpec {
    let mut snapshot = task.project().clone();
    snapshot.worktree_id = "execution".into();
    snapshot.git_head = "execution-head".into();
    snapshot.working_tree_fingerprint = "execution-fingerprint".into();
    RpcLaunchSpec::new(
        id.into(),
        task,
        lease,
        host.into(),
        "unverified-approval-reference".into(),
        snapshot,
        RpcProcessSpec::new(
            "/fixture/never-executed".into(),
            vec![
                "".into(),
                "a b".into(),
                "$(literal);*".into(),
                "tiếng Việt".into(),
            ],
            "src".into(),
            "a".repeat(64),
            "b".repeat(64),
            1000,
            200,
            4096,
            8192,
        )
        .unwrap(),
        RpcConnectionSpec::new(epoch.into(), "fixture".into(), "1.2".into(), true, 7, 16384)
            .unwrap(),
    )
    .unwrap()
}

fn fixture(store: &mut Store) -> RpcLaunchSpec {
    let task = task("task");
    store.enqueue(&task, 0).unwrap();
    let lease = store
        .lease(task.id(), &WorkerId::new("worker").unwrap(), 10, 100)
        .unwrap();
    launch(task, lease, "launch", "host", "epoch")
}

fn changed(
    spec: &RpcLaunchSpec,
    change: impl FnOnce(&mut graph_protocol::RpcLaunchSpec),
) -> RpcLaunchSpec {
    let mut wire = graph_protocol::RpcLaunchSpec::from(spec);
    change(&mut wire);
    wire.try_into_domain().unwrap()
}

fn counts(path: &Path) -> (i64, i64, i64, i64) {
    Connection::open(path)
        .unwrap()
        .query_row(include_str!("sql/rpc_ledger_counts.sql"), [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .unwrap()
}

fn assert_event(store: &Store, path: &Path, kind: EventKind, at_ms: i64) {
    let events = store.events(0, 100).unwrap();
    let event = events.last().unwrap();
    assert_eq!(event.kind, kind);
    assert_eq!(event.at_ms, at_ms);
    let outbox: i64 = Connection::open(path)
        .unwrap()
        .query_row(
            include_str!("sql/rpc_event_outbox_count.sql"),
            [event.sequence],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(outbox, 1);
}

#[test]
fn inspection_is_historical_exact_scoped_and_does_not_claim_or_refresh() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("query.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = fixture(&mut store);
    assert!(
        store
            .rpc_launch_snapshot(spec.id(), spec.task())
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        store.rpc_launch_snapshot(" ", spec.task()),
        Err(StoreError::Invalid(_))
    ));
    for stage in 0..3 {
        if stage == 0 {
            store.register_rpc_launch(&spec, 11).unwrap();
        }
        if stage == 1 {
            store.claim_rpc_launch(&spec, 12).unwrap();
        }
        if stage == 2 {
            store
                .cancel(
                    spec.task().id(),
                    &WorkerId::new("coordinator").unwrap(),
                    "stop",
                    13,
                )
                .unwrap();
        }
        let before = counts(&path);
        let events = store.events(0, 100).unwrap();
        drop(store);
        store = Store::open(&path).unwrap();
        let snapshot = store
            .rpc_launch_snapshot(spec.id(), spec.task())
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.spec(), &spec);
        assert_eq!(
            snapshot.claimed_at_ms(),
            if stage == 0 { None } else { Some(12) }
        );
        assert!(
            store
                .rpc_launch_snapshot("missing", spec.task())
                .unwrap()
                .is_none()
        );
        for field in 0..8 {
            let mut wire = graph_protocol::TaskSpec::from(spec.task());
            match field {
                0 => wire.project.repository_id.push('x'),
                1 => wire.project.worktree_id.push('x'),
                2 => wire.project.git_head.push('x'),
                3 => wire.project.working_tree_fingerprint.push('x'),
                4 => wire.project.config_hash.push('x'),
                5 => wire.project.ignore_policy_version.push('x'),
                6 => wire.context_ref.push('x'),
                _ => wire.graph_version.push('x'),
            }
            let wrong = wire.try_into_domain().unwrap();
            assert!(
                store
                    .rpc_launch_snapshot(spec.id(), &wrong)
                    .unwrap()
                    .is_none()
            );
        }
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(counts(&path), before);
    }
    assert!(!store.claim_rpc_launch(&spec, i64::MAX).unwrap());
}

#[test]
fn inspection_rejects_corrupt_linkage_and_timestamp_without_repair() {
    for corrupt_time in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("corrupt-query.sqlite");
        let mut store = Store::open(&path).unwrap();
        let spec = fixture(&mut store);
        store.register_rpc_launch(&spec, 11).unwrap();
        store.claim_rpc_launch(&spec, 12).unwrap();
        let before = counts(&path);
        let events = store.events(0, 100).unwrap();
        let db = Connection::open(&path).unwrap();
        if corrupt_time {
            // Deliberately damaged owned fixture, never a production repair path.
            db.execute_batch(include_str!("sql/rpc_disable_fixture_checks.sql"))
                .unwrap();
            db.execute(include_str!("sql/rpc_corrupt_claim_time.sql"), [spec.id()])
                .unwrap();
        } else {
            db.execute(
                include_str!("sql/rpc_corrupt_host.sql"),
                rusqlite::params![spec.id(), "wrong-host"],
            )
            .unwrap();
        }
        drop(store);
        let store = Store::open(&path).unwrap();
        for _ in 0..2 {
            assert!(matches!(
                store.rpc_launch_snapshot(spec.id(), spec.task()),
                Err(StoreError::Corrupt(_))
            ));
        }
        assert_eq!(counts(&path), before);
        assert_eq!(store.events(0, 100).unwrap(), events);
        let at: i64 = db
            .query_row(include_str!("sql/rpc_claim_time.sql"), [spec.id()], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(at, if corrupt_time { -1 } else { 12 });
    }
}

#[test]
fn concurrent_snapshot_reads_never_consume_the_launch_claim() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("concurrent-query.sqlite");
    let mut writer = Store::open(&path).unwrap();
    let spec = fixture(&mut writer);
    writer.register_rpc_launch(&spec, 11).unwrap();
    let before = counts(&path);
    let readers = (0..8)
        .map(|_| Store::open(&path).unwrap())
        .collect::<Vec<_>>();
    let barrier = Barrier::new(9);
    thread::scope(|scope| {
        for reader in readers {
            let spec = &spec;
            let barrier = &barrier;
            scope.spawn(move || {
                assert_eq!(
                    reader
                        .rpc_launch_snapshot(spec.id(), spec.task())
                        .unwrap()
                        .unwrap()
                        .claimed_at_ms(),
                    None
                );
                barrier.wait();
                let mut saw_claim = false;
                for _ in 0..100 {
                    let snapshot = reader
                        .rpc_launch_snapshot(spec.id(), spec.task())
                        .unwrap()
                        .unwrap();
                    assert_eq!(snapshot.spec(), spec);
                    let at = snapshot.claimed_at_ms();
                    assert!(at.is_none() || at == Some(12));
                    if saw_claim {
                        assert_eq!(at, Some(12));
                    }
                    saw_claim |= at.is_some();
                }
            });
        }
        barrier.wait();
        assert!(writer.claim_rpc_launch(&spec, 12).unwrap());
    });
    assert_eq!(
        counts(&path),
        (before.0, before.1 + 1, before.2 + 1, before.3 + 1)
    );
    assert_eq!(
        writer
            .rpc_launch_snapshot(spec.id(), spec.task())
            .unwrap()
            .unwrap()
            .claimed_at_ms(),
        Some(12)
    );
}

#[test]
fn ledger_read_model_validates_timestamp_without_claiming_execution() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(root.path().join("snapshot-model.sqlite")).unwrap();
    let spec = fixture(&mut store);
    assert!(RpcLaunchLedgerSnapshot::new(spec.clone(), Some(-1), None).is_err());
    for at in [None, Some(0), Some(i64::MAX)] {
        let snapshot = RpcLaunchLedgerSnapshot::new(spec.clone(), at, None).unwrap();
        assert_eq!(snapshot.spec(), &spec);
        assert_eq!(snapshot.claimed_at_ms(), at);
    }
    assert!(
        store
            .rpc_launch_snapshot(spec.id(), spec.task())
            .unwrap()
            .is_none()
    );
}

#[test]
fn inspection_does_not_echo_corrupt_descriptor_payload() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("corrupt-descriptor.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = fixture(&mut store);
    store.register_rpc_launch(&spec, 11).unwrap();
    let before = counts(&path);
    let events = store.events(0, 100).unwrap();
    let db = Connection::open(&path).unwrap();
    for raw in [
        "{secret-invalid-json",
        r#"{"schema_version":"secret-invalid-type"}"#,
    ] {
        db.execute(
            include_str!("sql/rpc_corrupt_descriptor.sql"),
            rusqlite::params![spec.id(), raw],
        )
        .unwrap();
        let error = store
            .rpc_launch_snapshot(spec.id(), spec.task())
            .unwrap_err();
        assert!(matches!(error, StoreError::Corrupt(_)));
        assert!(!error.to_string().contains("secret"));
        assert!(!format!("{error:?}").contains("secret"));
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(counts(&path), before);
    }
}

#[test]
fn negative_time_rejects_without_changing_absent_registered_or_claimed_launch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("negative-time.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = fixture(&mut store);
    for stage in 0..3 {
        if stage == 1 {
            assert!(store.register_rpc_launch(&spec, 11).unwrap());
        } else if stage == 2 {
            assert!(store.claim_rpc_launch(&spec, 12).unwrap());
        }
        let before = counts(&path);
        let events = store.events(0, 100).unwrap();
        let readback = store.rpc_launch(spec.id(), spec.task()).unwrap();
        for now in [-1, i64::MIN] {
            assert!(matches!(
                store.register_rpc_launch(&spec, now),
                Err(StoreError::Invalid(_))
            ));
            assert!(matches!(
                store.claim_rpc_launch(&spec, now),
                Err(StoreError::Invalid(_))
            ));
        }
        assert_eq!(counts(&path), before);
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(store.rpc_launch(spec.id(), spec.task()).unwrap(), readback);
    }
    assert!(!store.claim_rpc_launch(&spec, 13).unwrap());
    let claimed_at: i64 = Connection::open(&path)
        .unwrap()
        .query_row(include_str!("sql/rpc_claim_time.sql"), [spec.id()], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(claimed_at, 12);
}

#[test]
fn mismatched_host_column_is_corrupt_without_events_or_claim_reset() {
    for claimed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corrupt-host.sqlite");
        let mut store = Store::open(&path).unwrap();
        let spec = fixture(&mut store);
        assert!(store.register_rpc_launch(&spec, 11).unwrap());
        if claimed {
            assert!(store.claim_rpc_launch(&spec, 12).unwrap());
        }
        let before = counts(&path);
        let events = store.events(0, 100).unwrap();
        let fixture = Connection::open(&path).unwrap();
        assert_eq!(
            fixture
                .execute(
                    include_str!("sql/rpc_corrupt_host.sql"),
                    rusqlite::params![spec.id(), "mismatched-host"],
                )
                .unwrap(),
            1
        );
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert!(matches!(
            store.rpc_launch(spec.id(), spec.task()),
            Err(StoreError::Corrupt(_))
        ));
        assert!(matches!(
            store.register_rpc_launch(&spec, 13),
            Err(StoreError::Corrupt(_))
        ));
        assert!(matches!(
            store.claim_rpc_launch(&spec, 13),
            Err(StoreError::Corrupt(_))
        ));
        // Rejection must neither repair the descriptor linkage nor reset a consumed claim.
        assert!(matches!(
            store.rpc_launch(spec.id(), spec.task()),
            Err(StoreError::Corrupt(_))
        ));
        assert_eq!(counts(&path), before);
        assert_eq!(store.events(0, 100).unwrap(), events);
        if claimed {
            let claimed_at: i64 = fixture
                .query_row(include_str!("sql/rpc_claim_time.sql"), [spec.id()], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(claimed_at, 12);
        }
    }
}

#[test]
fn full_readback_reopen_and_historical_replay_do_not_append_events() {
    assert_eq!(
        EventKind::RpcLaunchRegistered.as_str(),
        "rpc_launch_registered"
    );
    assert_eq!(EventKind::RpcLaunchClaimed.as_str(), "rpc_launch_claimed");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ledger.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = fixture(&mut store);
    assert_eq!(store.rpc_launch(spec.id(), spec.task()).unwrap(), None);
    let before = counts(&path);
    assert!(matches!(
        store.claim_rpc_launch(&spec, 11),
        Err(StoreError::Unavailable)
    ));
    assert_eq!(counts(&path), before);
    assert!(store.register_rpc_launch(&spec, 11).unwrap());
    assert_eq!(counts(&path), (1, 0, before.2 + 1, before.3 + 1));
    assert_event(&store, &path, EventKind::RpcLaunchRegistered, 11);
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(
        store.rpc_launch(spec.id(), spec.task()).unwrap(),
        Some(spec.clone())
    );
    assert_eq!(store.rpc_launch("missing", spec.task()).unwrap(), None);
    let before = counts(&path);
    let events = store.events(0, 100).unwrap();
    assert!(!store.register_rpc_launch(&spec, 12).unwrap());
    assert!(!store.register_rpc_launch(&spec, i64::MAX).unwrap());
    assert_eq!(store.events(0, 100).unwrap(), events);
    assert_eq!(counts(&path), before);
    assert!(store.claim_rpc_launch(&spec, 13).unwrap());
    assert_event(&store, &path, EventKind::RpcLaunchClaimed, 13);
    assert_eq!(counts(&path), (1, 1, before.2 + 1, before.3 + 1));
    drop(store);
    let mut store = Store::open(&path).unwrap();
    let events = store.events(0, 100).unwrap();
    let before = counts(&path);
    for now in [14, 110, i64::MAX] {
        assert!(!store.claim_rpc_launch(&spec, now).unwrap());
    }
    assert_eq!(store.events(0, 100).unwrap(), events);
    assert_eq!(counts(&path), before);
    store
        .cancel(
            spec.task().id(),
            &WorkerId::new("coordinator").unwrap(),
            "stop",
            15,
        )
        .unwrap();
    let events = store.events(0, 100).unwrap();
    let before = counts(&path);
    assert!(!store.register_rpc_launch(&spec, 16).unwrap());
    assert!(!store.claim_rpc_launch(&spec, i64::MAX).unwrap());
    assert_eq!(
        store.rpc_launch(spec.id(), spec.task()).unwrap(),
        Some(spec.clone())
    );
    assert_eq!(store.events(0, 100).unwrap(), events);
    assert_eq!(counts(&path), before);
    let claimed_at: i64 = Connection::open(&path)
        .unwrap()
        .query_row(include_str!("sql/rpc_claim_time.sql"), [spec.id()], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(claimed_at, 13);
}

#[test]
fn changed_id_content_conflicts_even_after_claim_and_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("conflict.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = fixture(&mut store);
    store.register_rpc_launch(&spec, 11).unwrap();
    for stage in 0..3 {
        if stage == 1 {
            assert!(store.claim_rpc_launch(&spec, 12).unwrap());
        }
        if stage == 2 {
            store
                .cancel(
                    spec.task().id(),
                    &WorkerId::new("coordinator").unwrap(),
                    "stop",
                    13,
                )
                .unwrap();
        }
        let before = counts(&path);
        let events = store.events(0, 100).unwrap();
        for field in 0..12 {
            let wrong = changed(&spec, |w| match field {
                0 => w.host_id.push('x'),
                1 => w.approval_id.push('x'),
                2 => w.connection.epoch.push('x'),
                3 => w.connection.max_pending += 1,
                4 => w.process.args.push("extra".into()),
                5 => w.process.environment_sha256 = "c".repeat(64),
                6 => w.execution_snapshot.git_head.push('x'),
                7 => w.execution_snapshot.working_tree_fingerprint.push('x'),
                8 => w.task.context_ref.push('x'),
                9 => w.origin_lease.owner.push('x'),
                10 => w.origin_lease.fencing_token += 1,
                _ => w.origin_lease.expires_at_ms += 1,
            });
            assert!(
                matches!(
                    store.register_rpc_launch(&wrong, 14),
                    Err(StoreError::Conflict)
                ),
                "stage {stage}, field {field}"
            );
            assert!(
                matches!(
                    store.claim_rpc_launch(&wrong, 14),
                    Err(StoreError::Conflict)
                ),
                "stage {stage}, field {field}"
            );
        }
        assert_eq!(counts(&path), before);
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(
            store.rpc_launch(spec.id(), spec.task()).unwrap(),
            Some(spec.clone())
        );
    }
}

#[test]
fn alias_ids_cannot_reuse_task_fence_or_host_epoch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("aliases.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = fixture(&mut store);
    store.register_rpc_launch(&spec, 11).unwrap();
    let other_task = task("other-task");
    store.enqueue(&other_task, 12).unwrap();
    let lease = store
        .lease(
            other_task.id(),
            &WorkerId::new("other-worker").unwrap(),
            12,
            100,
        )
        .unwrap();
    let same_host_epoch = launch(other_task, lease, "other-launch", "host", "epoch");
    let same_task_fence = changed(&spec, |w| {
        w.id = "alias".into();
        w.host_id = "other-host".into();
        w.connection.epoch = "other-epoch".into();
    });
    let before = counts(&path);
    for alias in [&same_task_fence, &same_host_epoch] {
        assert!(matches!(
            store.register_rpc_launch(alias, 12),
            Err(StoreError::Conflict)
        ));
        assert!(matches!(
            store.claim_rpc_launch(alias, 12),
            Err(StoreError::Unavailable)
        ));
        assert_eq!(store.rpc_launch(alias.id(), alias.task()).unwrap(), None);
    }
    assert_eq!(counts(&path), before);
    // Uniqueness is across registrations, including historical cancelled ones.
    store
        .cancel(
            spec.task().id(),
            &WorkerId::new("coordinator").unwrap(),
            "stop",
            13,
        )
        .unwrap();
    let before = counts(&path);
    for alias in [&same_task_fence, &same_host_epoch] {
        assert!(store.register_rpc_launch(alias, 14).is_err());
        assert!(store.claim_rpc_launch(alias, 14).is_err());
        assert_eq!(store.rpc_launch(alias.id(), alias.task()).unwrap(), None);
    }
    assert_eq!(counts(&path), before);
    // Epoch is scoped by host, not globally unique.
    assert!(matches!(
        store.register_rpc_launch(&same_host_epoch, 14),
        Err(StoreError::Conflict)
    ));
    let independent = changed(&same_host_epoch, |w| w.host_id = "other-host".into());
    assert!(store.register_rpc_launch(&independent, 14).unwrap());
}

#[test]
fn registration_requires_existing_leased_exact_task_owner_fence_and_expiry() {
    for case in 0..7 {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lease.sqlite");
        let mut store = Store::open(&path).unwrap();
        let task = task("task");
        let lease =
            Lease::issue(task.id().clone(), WorkerId::new("worker").unwrap(), 1, 110).unwrap();
        let mut spec = launch(task.clone(), lease, "launch", "host", "epoch");
        if case != 0 {
            store.enqueue(&task, 0).unwrap();
        }
        if case >= 2 {
            store
                .lease(task.id(), &WorkerId::new("worker").unwrap(), 10, 100)
                .unwrap();
        }
        spec = changed(&spec, |w| match case {
            2 => w.task.context_ref.push('x'),
            3 => w.origin_lease.owner.push('x'),
            4 => w.origin_lease.fencing_token += 1,
            5 => w.origin_lease.expires_at_ms += 1,
            _ => (),
        });
        if case == 6 {
            store
                .cancel(
                    task.id(),
                    &WorkerId::new("coordinator").unwrap(),
                    "stop",
                    11,
                )
                .unwrap();
        }
        let before = counts(&path);
        let result = store.register_rpc_launch(&spec, 12);
        if case == 2 {
            assert!(matches!(result, Err(StoreError::Conflict)));
        } else {
            assert!(
                matches!(result, Err(StoreError::Unavailable)),
                "case {case}"
            );
        }
        assert!(
            matches!(
                store.claim_rpc_launch(&spec, 12),
                Err(StoreError::Unavailable)
            ),
            "case {case}"
        );
        assert_eq!(store.rpc_launch(spec.id(), &task).unwrap(), None);
        assert_eq!(counts(&path), before);
    }
}

#[test]
fn readback_requires_complete_task_contract() {
    let mut store = Store::open(":memory:").unwrap();
    let spec = fixture(&mut store);
    store.register_rpc_launch(&spec, 11).unwrap();
    for field in 0..7 {
        let wrong = changed(&spec, |w| match field {
            0 => {
                w.task.project.repository_id.push('x');
                w.execution_snapshot.repository_id.push('x');
            }
            1 => w.task.project.worktree_id.push('x'),
            2 => w.task.project.git_head.push('x'),
            3 => w.task.project.working_tree_fingerprint.push('x'),
            4 => {
                w.task.project.config_hash.push('x');
                w.execution_snapshot.config_hash.push('x');
            }
            5 => {
                w.task.project.ignore_policy_version.push('x');
                w.execution_snapshot.ignore_policy_version.push('x');
            }
            _ => w.task.context_ref.push('x'),
        });
        assert_eq!(
            store.rpc_launch(spec.id(), wrong.task()).unwrap(),
            None,
            "field {field}"
        );
    }
}

#[test]
fn expiry_boundary_blocks_first_registration_and_first_claim() {
    for now in [109, 110, 111] {
        for register_first in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("boundary.sqlite");
            let mut store = Store::open(&path).unwrap();
            let spec = fixture(&mut store);
            if register_first {
                store.register_rpc_launch(&spec, 11).unwrap();
            }
            let before = counts(&path);
            let result = if register_first {
                store.claim_rpc_launch(&spec, now)
            } else {
                store.register_rpc_launch(&spec, now)
            };
            if now == 109 {
                assert!(result.unwrap());
            } else {
                assert!(matches!(result, Err(StoreError::Unavailable)));
                assert_eq!(counts(&path), before);
                if register_first {
                    assert!(!store.register_rpc_launch(&spec, now).unwrap());
                }
            }
        }
    }
}

#[test]
fn cancelled_registration_replays_but_unclaimed_launch_cannot_claim() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cancelled.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = fixture(&mut store);
    store.register_rpc_launch(&spec, 11).unwrap();
    store
        .cancel(
            spec.task().id(),
            &WorkerId::new("coordinator").unwrap(),
            "stop",
            12,
        )
        .unwrap();
    drop(store);
    let mut store = Store::open(&path).unwrap();
    let before = counts(&path);
    assert!(!store.register_rpc_launch(&spec, 13).unwrap());
    assert!(matches!(
        store.claim_rpc_launch(&spec, 13),
        Err(StoreError::Unavailable)
    ));
    assert_eq!(counts(&path), before);
    assert_eq!(
        store.rpc_launch(spec.id(), spec.task()).unwrap(),
        Some(spec)
    );
}

#[test]
fn expired_lease_reacquisition_fences_unclaimed_launch_and_preserves_claimed_replay() {
    for already_claimed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fence.sqlite");
        let mut store = Store::open(&path).unwrap();
        let spec = fixture(&mut store);
        store.register_rpc_launch(&spec, 11).unwrap();
        if already_claimed {
            assert!(store.claim_rpc_launch(&spec, 12).unwrap());
        }
        let lease = store
            .lease(spec.task().id(), spec.origin_lease().owner(), 110, 100)
            .unwrap();
        assert!(lease.fencing_token() > spec.origin_lease().fencing_token());
        let before = counts(&path);
        assert!(!store.register_rpc_launch(&spec, 111).unwrap());
        let result = store.claim_rpc_launch(&spec, 111);
        if already_claimed {
            assert!(!result.unwrap());
        } else {
            assert!(result.is_err());
        }
        let stale_alias = changed(&spec, |w| {
            w.id = "stale-alias".into();
            w.connection.epoch = "stale-epoch".into();
        });
        assert!(store.register_rpc_launch(&stale_alias, 111).is_err());
        assert_eq!(counts(&path), before);
        let next = launch(spec.task().clone(), lease, "next", "host", "next-epoch");
        assert!(store.register_rpc_launch(&next, 111).unwrap());
        assert!(store.claim_rpc_launch(&next, 112).unwrap());
    }
}

#[test]
fn eight_independent_claim_connections_have_exactly_one_durable_winner() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("race.sqlite");
    let mut store = Store::open(&path).unwrap();
    let spec = fixture(&mut store);
    store.register_rpc_launch(&spec, 11).unwrap();
    let before = counts(&path);
    // Open before the barrier so migration/open contention cannot impersonate a claim race.
    let connections: Vec<_> = (0..8).map(|_| Store::open(&path).unwrap()).collect();
    let barrier = Barrier::new(8);
    let results = thread::scope(|scope| {
        let workers: Vec<_> = connections
            .into_iter()
            .map(|mut connection| {
                let spec = &spec;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    connection.claim_rpc_launch(spec, 12)
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    let results: Vec<_> = results.into_iter().map(Result::unwrap).collect();
    assert_eq!(results.iter().filter(|won| **won).count(), 1);
    assert_eq!(results.iter().filter(|won| !**won).count(), 7);
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert!(!store.claim_rpc_launch(&spec, i64::MAX).unwrap());
    assert_eq!(counts(&path), (1, 1, before.2 + 1, before.3 + 1));
    assert_event(&store, &path, EventKind::RpcLaunchClaimed, 12);
}

#[test]
fn event_and_outbox_faults_roll_back_registration_and_claim_then_retry() {
    for claim in [false, true] {
        for outbox in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("rollback.sqlite");
            let mut store = Store::open(&path).unwrap();
            let spec = fixture(&mut store);
            if claim {
                store.register_rpc_launch(&spec, 11).unwrap();
            }
            let before = counts(&path);
            let events = store.events(0, 100).unwrap();
            let fixture = Connection::open(&path).unwrap();
            fixture
                .execute_batch(if outbox {
                    include_str!("sql/rpc_fail_outbox.sql")
                } else {
                    include_str!("sql/rpc_fail_event.sql")
                })
                .unwrap();
            let result = if claim {
                store.claim_rpc_launch(&spec, 12)
            } else {
                store.register_rpc_launch(&spec, 12)
            };
            let error = result.unwrap_err();
            assert!(matches!(&error, StoreError::Sql(_)));
            assert!(error.to_string().contains(if outbox {
                "rpc fixture outbox failure"
            } else {
                "rpc fixture event failure"
            }));
            assert_eq!(counts(&path), before);
            assert_eq!(store.events(0, 100).unwrap(), events);
            drop(store);
            let mut store = Store::open(&path).unwrap();
            assert_eq!(counts(&path), before);
            assert_eq!(
                store.rpc_launch(spec.id(), spec.task()).unwrap(),
                if claim { Some(spec.clone()) } else { None }
            );
            // Drop only the named test-owned trigger; retain the production outbox trigger.
            fixture
                .execute_batch(if outbox {
                    include_str!("sql/rpc_drop_fail_outbox.sql")
                } else {
                    include_str!("sql/rpc_drop_fail_event.sql")
                })
                .unwrap();
            if claim {
                assert!(store.claim_rpc_launch(&spec, 13).unwrap());
                assert!(!store.claim_rpc_launch(&spec, 14).unwrap());
                assert_eq!(counts(&path), (1, 1, before.2 + 1, before.3 + 1));
                assert_event(&store, &path, EventKind::RpcLaunchClaimed, 13);
            } else {
                assert!(store.register_rpc_launch(&spec, 13).unwrap());
                assert!(!store.register_rpc_launch(&spec, 14).unwrap());
                assert_eq!(counts(&path), (1, 0, before.2 + 1, before.3 + 1));
                assert_event(&store, &path, EventKind::RpcLaunchRegistered, 13);
            }
        }
    }
}

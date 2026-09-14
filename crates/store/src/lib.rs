//! SQLite implementation of the task repository port.

mod analysis;
mod artifact;
mod artifact_observation;
mod cancellation;
mod check_policy;
mod deployment;
mod evidence;
mod execution_launch;
mod execution_plan;
mod execution_receipt;
mod fact;
mod integration;
mod lease;
#[cfg(test)]
mod migration_tests;
mod outbox;
mod rpc_launch;
mod rpc_spawn;
mod rpc_terminal;
mod sql_link;
mod submission_query;
mod task_query;

use std::{
    path::Path,
    time::{Duration, Instant},
};

use graph_application::TaskRepository;
use graph_domain::{
    DomainError, EventKind, Lease, TaskEvent, TaskId, TaskSpec, TaskState, WorkerId,
};
use graph_protocol::TaskSpec as TaskSpecWire;
use refinery::embed_migrations;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use thiserror::Error;

mod embedded {
    use super::*;

    embed_migrations!("migrations");
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Migration(#[from] refinery::Error),
    #[error(
        "unsupported SQLite user_version {0}; explicit legacy schema inspection/import is required; no automatic conversion was attempted"
    )]
    UnsupportedUserVersion(i64),
    #[error("database contains invalid persisted data: {0}")]
    Corrupt(String),
    #[error("record id already exists with different immutable content")]
    Conflict,
    #[error("task is unavailable, dependencies are unfinished, or lease is stale")]
    Unavailable,
    #[error("invalid repository request: {0}")]
    Invalid(&'static str),
}

/// Owns one SQLite connection. SQLite transactions are intentionally scoped to
/// individual repository calls so no lock can cross a model or shell operation.
pub struct Store(Connection);

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        // This schema uses refinery history, never PRAGMA user_version. Reject
        // ambiguous legacy/foreign ownership before configuring journal mode
        // or attempting migrations; never fabricate history or clear the marker.
        let user_version: i64 =
            connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if user_version != 0 {
            return Err(StoreError::UnsupportedUserVersion(user_version));
        }
        // Changing journal mode may return BUSY immediately even with SQLite's
        // busy timeout when several processes create the database together.
        let started = Instant::now();
        loop {
            match connection.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA foreign_keys=ON;
                 PRAGMA synchronous=FULL;",
            ) {
                Ok(()) => break,
                Err(error)
                    if error.sqlite_error_code() == Some(rusqlite::ErrorCode::DatabaseBusy)
                        && started.elapsed() < Duration::from_secs(5) =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error.into()),
            }
        }

        // Group pending versions in one transaction so another opener cannot
        // observe partially advanced history and repeatedly race each version.
        // Another process may commit the selected migration between refinery's
        // history read and its write transaction. A fresh runner revalidates
        // the committed checksums; it never ignores migration errors or marks
        // an unapplied migration as applied. A persistent error is returned.
        if embedded::migrations::runner()
            .set_grouped(true)
            .set_abort_missing(true)
            .set_abort_divergent(true)
            .run(&mut connection)
            .is_err()
        {
            embedded::migrations::runner()
                .set_grouped(true)
                .set_abort_missing(true)
                .set_abort_divergent(true)
                .run(&mut connection)?;
        }
        Ok(Self(connection))
    }

    fn enqueue_task(&mut self, task: &TaskSpec, now_ms: i64) -> Result<bool, StoreError> {
        let spec = serde_json::to_string(&TaskSpecWire::from(task))?;
        let transaction = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT spec FROM tasks WHERE id=?1",
                [task.id().as_str()],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            return if existing == spec {
                Ok(false)
            } else {
                Err(StoreError::Conflict)
            };
        }

        transaction.execute(
            "INSERT INTO tasks(id,spec,state) VALUES(?1,?2,?3)",
            params![task.id().as_str(), spec, TaskState::Queued.as_db()],
        )?;
        for dependency in task.dependencies() {
            transaction.execute(
                "INSERT INTO dependencies(task_id,dependency_id) VALUES(?1,?2)",
                params![task.id().as_str(), dependency.as_str()],
            )?;
        }
        transaction.execute(
            "INSERT INTO events(task_id,kind,at_ms,payload) VALUES(?1,?2,?3,?4)",
            params![task.id().as_str(), EventKind::Queued.as_str(), now_ms, spec],
        )?;
        transaction.commit()?;
        Ok(true)
    }

    fn lease_task(
        &mut self,
        task_id: &TaskId,
        owner: &WorkerId,
        now_ms: i64,
        duration_ms: i64,
    ) -> Result<Lease, StoreError> {
        let transaction = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let lease = lease::issue(&transaction, task_id, owner, now_ms, duration_ms)?;
        transaction.commit()?;
        Ok(lease)
    }

    fn submit_candidate(
        &mut self,
        lease: &Lease,
        artifact: &str,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let transaction = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE tasks SET state='submitted'
             WHERE id=?1 AND state='leased' AND owner=?2 AND fence=?3 AND expires>?4",
            params![
                lease.task_id().as_str(),
                lease.owner().as_str(),
                lease.fencing_token(),
                now_ms,
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::Unavailable);
        }
        transaction.execute(
            "INSERT INTO events(task_id,kind,at_ms,payload) VALUES(?1,?2,?3,?4)",
            params![
                lease.task_id().as_str(),
                EventKind::Submitted.as_str(),
                now_ms,
                artifact
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    // Historical transition fixture only. Production integration must be rebuilt
    // around verified candidate/snapshot/check receipts, not caller text.
    #[cfg(test)]
    fn integrate_candidate_fixture(
        &mut self,
        task_id: &TaskId,
        verification: &str,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let transaction = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE tasks SET state='integrated' WHERE id=?1 AND state='submitted'",
            [task_id.as_str()],
        )?;
        if changed != 1 {
            return Err(StoreError::Unavailable);
        }
        transaction.execute(
            "INSERT INTO events(task_id,kind,at_ms,payload) VALUES(?1,?2,?3,?4)",
            params![
                task_id.as_str(),
                EventKind::Integrated.as_str(),
                now_ms,
                verification
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn task_events(&self, after: i64, limit: u32) -> Result<Vec<TaskEvent>, StoreError> {
        let mut statement = self.0.prepare(
            "SELECT seq,task_id,kind,at_ms,payload FROM events WHERE seq>?1 ORDER BY seq LIMIT ?2",
        )?;
        let rows = statement.query_map(params![after, limit.min(1000)], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let raw = rows.collect::<Result<Vec<_>, _>>()?;
        raw.into_iter()
            .map(|(sequence, task_id, kind, at_ms, payload)| {
                Ok(TaskEvent {
                    sequence,
                    task_id: TaskId::new(task_id).map_err(domain_corruption)?,
                    kind: EventKind::from_str(&kind).map_err(domain_corruption)?,
                    at_ms,
                    payload,
                })
            })
            .collect()
    }
}

impl TaskRepository for Store {
    type Error = StoreError;

    fn enqueue(&mut self, task: &TaskSpec, now_ms: i64) -> Result<bool, Self::Error> {
        self.enqueue_task(task, now_ms)
    }

    fn lease(
        &mut self,
        task_id: &TaskId,
        owner: &WorkerId,
        now_ms: i64,
        duration_ms: i64,
    ) -> Result<Lease, Self::Error> {
        self.lease_task(task_id, owner, now_ms, duration_ms)
    }

    fn submit(&mut self, lease: &Lease, artifact: &str, now_ms: i64) -> Result<(), Self::Error> {
        self.submit_candidate(lease, artifact, now_ms)
    }

    fn cancel(
        &mut self,
        task_id: &TaskId,
        requested_by: &WorkerId,
        reason: &str,
        now_ms: i64,
    ) -> Result<bool, Self::Error> {
        self.cancel_task(task_id, requested_by, reason, now_ms)
    }

    fn integrate(
        &mut self,
        _task_id: &TaskId,
        _verification: &str,
        _now_ms: i64,
    ) -> Result<(), Self::Error> {
        Err(StoreError::Invalid(
            "string-based integration is disabled; verified integration authority is not implemented",
        ))
    }

    fn events(&self, after: i64, limit: u32) -> Result<Vec<TaskEvent>, Self::Error> {
        self.task_events(after, limit)
    }
}

fn domain_corruption(error: DomainError) -> StoreError {
    StoreError::Corrupt(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_application::TaskService;
    use graph_domain::ProjectRef;

    #[test]
    fn submitted_candidate_survives_restart_without_events_and_hides_after_cancel() {
        use graph_application::SubmissionQueryRepository;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("candidate.db");
        let mut service = service(&path);
        service.enqueue(task("a"), 0).unwrap();
        let id = TaskId::new("a").unwrap();
        let store = service.into_inner();
        assert!(store.submitted_candidate(&id).unwrap().is_none());
        let mut service = TaskService::new(store);
        let lease = service.lease("a", "worker", 1, 100).unwrap();
        service.submit(lease, "unverified-artifact", 2).unwrap();
        let store = service.into_inner();
        let before = store.events(0, 100).unwrap();
        let candidate = store.submitted_candidate(&id).unwrap().unwrap();
        assert_eq!(candidate.spec, task("a"));
        assert_eq!(candidate.artifact, "unverified-artifact");
        assert_eq!(candidate.owner.as_str(), "worker");
        assert_eq!(candidate.fencing_token, 1);
        assert_eq!(candidate.submitted_at_ms, 2);
        assert_eq!(
            candidate.submission_sequence,
            before.last().unwrap().sequence
        );
        assert_eq!(store.events(0, 100).unwrap(), before);
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(store.submitted_candidate(&id).unwrap(), Some(candidate));
        store
            .cancel(&id, &WorkerId::new("coordinator").unwrap(), "stop", 3)
            .unwrap();
        assert!(store.submitted_candidate(&id).unwrap().is_none());
    }

    #[test]
    fn submitted_candidate_rejects_missing_duplicate_and_invalid_ledger_data() {
        use graph_application::SubmissionQueryRepository;
        for mutation in [
            "UPDATE events SET kind='leased' WHERE kind='submitted'",
            "INSERT INTO events(task_id,kind,at_ms,payload) VALUES('a','submitted',3,'duplicate')",
            "UPDATE tasks SET owner=NULL WHERE id='a'",
            "UPDATE tasks SET fence=0 WHERE id='a'",
            "UPDATE tasks SET spec='{}' WHERE id='a'",
            "UPDATE events SET payload='' WHERE kind='submitted'",
            "UPDATE events SET at_ms=-1 WHERE kind='submitted'",
        ] {
            let mut service = service(":memory:");
            service.enqueue(task("a"), 0).unwrap();
            let lease = service.lease("a", "worker", 1, 100).unwrap();
            service.submit(lease, "candidate", 2).unwrap();
            let store = service.into_inner();
            store.0.execute_batch(mutation).unwrap();
            assert!(
                matches!(
                    store.submitted_candidate(&TaskId::new("a").unwrap()),
                    Err(StoreError::Corrupt(_))
                ),
                "{mutation}"
            );
        }
    }

    #[test]
    fn task_snapshot_validates_persisted_identity_and_does_not_emit_events() {
        use graph_application::TaskQueryRepository;
        let mut service = service(":memory:");
        service.enqueue(task("a"), 0).unwrap();
        let store = service.into_inner();
        let id = TaskId::new("a").unwrap();
        let before = store.events(0, 100).unwrap();
        let snapshot = store.task_snapshot(&id).unwrap().unwrap();
        assert_eq!(snapshot.spec, task("a"));
        assert_eq!(snapshot.state, TaskState::Queued);
        assert!(
            store
                .task_snapshot(&TaskId::new("missing").unwrap())
                .unwrap()
                .is_none()
        );
        assert_eq!(store.events(0, 100).unwrap(), before);
        let wrong = serde_json::to_string(&TaskSpecWire::from(&task("b"))).unwrap();
        for invalid in [wrong.as_str(), "not-json", "{}"] {
            store
                .0
                .execute("UPDATE tasks SET spec=?1 WHERE id='a'", [invalid])
                .unwrap();
            assert!(matches!(
                store.task_snapshot(&id),
                Err(StoreError::Corrupt(_))
            ));
        }
    }

    #[test]
    fn caller_text_cannot_integrate_submitted_task_or_emit_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.db");
        let mut tasks = service(&path);
        tasks.enqueue(task("a"), 0).unwrap();
        let lease = tasks.lease("a", "worker", 1, 100).unwrap();
        tasks.submit(lease, "candidate", 2).unwrap();
        let before = tasks.events(0, 100).unwrap();
        assert!(tasks.integrate("a", "all tests pass", 3).is_err());
        let mut store = tasks.into_inner();
        assert!(
            TaskRepository::integrate(&mut store, &TaskId::new("a").unwrap(), "receipt-id", 4)
                .is_err()
        );
        assert_eq!(store.events(0, 100).unwrap(), before);
        let state: String = store
            .0
            .query_row("SELECT state FROM tasks WHERE id='a'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(state, "submitted");
        let integrated_outbox: i64 = store
            .0
            .query_row(
                "SELECT count(*) FROM events WHERE kind='integrated'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(integrated_outbox, 0);
        drop(store);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.events(0, 100).unwrap(), before);
    }

    fn task(id: &str) -> TaskSpec {
        TaskSpec::new(
            TaskId::new(id).unwrap(),
            ProjectRef {
                repository_id: "fixture".into(),
                worktree_id: "main".into(),
                git_head: "abc".into(),
                working_tree_fingerprint: "clean".into(),
                config_hash: "cfg".into(),
                ignore_policy_version: "1".into(),
            },
            "gv1".into(),
            "implementer".into(),
            "plus-a".into(),
            vec!["src".into()],
            vec![],
            "ctx1".into(),
            vec!["patch".into()],
            6000,
        )
        .unwrap()
    }

    fn service(path: impl AsRef<Path>) -> TaskService<Store> {
        TaskService::new(Store::open(path).unwrap())
    }

    #[test]
    fn restart_preserves_events_and_expired_owner_cannot_submit() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let mut first = service(&path);
        assert!(first.enqueue(task("a"), 0).unwrap());
        assert!(!first.enqueue(task("a"), 1).unwrap());
        let old = first.lease("a", "old", 10, 10).unwrap();
        drop(first);
        let mut restored = service(&path);
        let new = restored.lease("a", "new", 20, 10).unwrap();
        assert!(new.fencing_token() > old.fencing_token());
        assert!(restored.submit(old, "old.patch", 21).is_err());
        restored.submit(new, "new.patch", 21).unwrap();
        assert_eq!(restored.events(0, 100).unwrap().len(), 4);
    }

    #[test]
    fn cancellation_revokes_authority_and_replays_once_after_reopen() {
        for initial in [TaskState::Queued, TaskState::Leased, TaskState::Submitted] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("cancel.db");
            let mut coordinator = service(&path);
            coordinator.enqueue(task("a"), 0).unwrap();
            let lease = if initial == TaskState::Queued {
                None
            } else {
                Some(coordinator.lease("a", "worker", 1, 100).unwrap())
            };
            if initial == TaskState::Submitted {
                coordinator
                    .submit(lease.clone().unwrap(), "patch", 2)
                    .unwrap();
            }
            assert!(
                coordinator
                    .cancel("a", "operator", "user changed direction", 3)
                    .unwrap()
            );
            drop(coordinator);
            let mut reopened = service(&path);
            assert!(
                !reopened
                    .cancel("a", "another", "must not replace reason", 4)
                    .unwrap()
            );
            assert!(reopened.lease("a", "replacement", 200, 100).is_err());
            assert!(
                reopened
                    .integrate("a", "untrusted verification", 5)
                    .is_err()
            );
            if let Some(lease) = lease {
                assert!(reopened.submit(lease, "late patch", 5).is_err());
            }
            let events = reopened.events(0, 100).unwrap();
            let cancelled: Vec<_> = events
                .iter()
                .filter(|e| e.kind == EventKind::Cancelled)
                .collect();
            assert_eq!(cancelled.len(), 1);
            let payload: serde_json::Value = serde_json::from_str(&cancelled[0].payload).unwrap();
            assert_eq!(payload["reason"], "user changed direction");
            assert_eq!(payload["requested_by"], "operator");
            assert_eq!(payload["previous_state"], initial.as_db());
            let store = reopened.into_inner();
            assert_eq!(store.pending_events("executor", 100).unwrap(), events);
            let owner: Option<String> = store
                .0
                .query_row("SELECT owner FROM tasks WHERE id='a'", [], |r| r.get(0))
                .unwrap();
            assert_eq!(owner, None);
        }
    }

    #[test]
    fn cancellation_failure_rolls_back_state_event_and_outbox() {
        let mut coordinator = service(":memory:");
        coordinator.enqueue(task("a"), 0).unwrap();
        let lease = coordinator.lease("a", "worker", 1, 100).unwrap();
        let mut store = coordinator.into_inner();
        store.0.execute_batch("CREATE TRIGGER reject_cancel BEFORE INSERT ON events WHEN NEW.kind='cancelled' BEGIN SELECT RAISE(ABORT, 'injected event failure'); END;").unwrap();
        assert!(
            store
                .cancel(
                    &TaskId::new("a").unwrap(),
                    &WorkerId::new("operator").unwrap(),
                    "stop",
                    2
                )
                .is_err()
        );
        assert_eq!(store.pending_events("executor", 100).unwrap().len(), 2);
        store.submit(&lease, "still authorized", 3).unwrap();
        assert_eq!(
            store.events(0, 100).unwrap().last().unwrap().kind,
            EventKind::Submitted
        );
    }

    #[test]
    fn policy_event_and_outbox_failures_roll_back_registration() {
        use graph_application::CheckPolicyRepository;
        for table in ["events", "event_outbox"] {
            let mut store = Store::open(":memory:").unwrap();
            let expected = task("policy");
            store.enqueue(&expected, 0).unwrap();
            let before = store.events(0, 100).unwrap();
            let policy = graph_domain::RequiredChecks::new(vec!["test".into()]).unwrap();
            assert!(store.register_check_policy(&expected, &policy, -1).is_err());
            assert_eq!(store.check_policy(&expected).unwrap(), None);
            // Fixed test-only table names; no user input in injected DDL.
            store.0.execute_batch(&format!("CREATE TRIGGER reject_policy_insert BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'injected failure'); END;")).unwrap();
            assert!(store.register_check_policy(&expected, &policy, 1).is_err());
            assert_eq!(store.check_policy(&expected).unwrap(), None);
            assert_eq!(store.events(0, 100).unwrap(), before);
            assert_eq!(store.pending_events("policy", 100).unwrap(), before);
            store
                .0
                .execute_batch("DROP TRIGGER reject_policy_insert;")
                .unwrap();
            assert!(store.register_check_policy(&expected, &policy, 2).unwrap());
            assert_eq!(store.events(0, 100).unwrap().len(), before.len() + 1);
        }
    }

    #[test]
    fn replay_does_not_invent_registration_time_for_legacy_policy() {
        use graph_application::CheckPolicyRepository;
        let mut store = Store::open(":memory:").unwrap();
        let expected = task("legacy-policy");
        store.enqueue(&expected, 0).unwrap();
        let policy = graph_domain::RequiredChecks::new(vec!["test".into()]).unwrap();
        let descriptor =
            serde_json::to_string(&graph_protocol::RequiredChecks::from(&policy)).unwrap();
        // Prior V7 implementation persisted this row without an event.
        store
            .0
            .execute(
                include_str!("sql/insert_task_check_policy.sql"),
                params![expected.id().as_str(), descriptor],
            )
            .unwrap();
        let before = store.events(0, 100).unwrap();
        assert!(
            !store
                .register_check_policy(&expected, &policy, 123)
                .unwrap()
        );
        assert_eq!(store.events(0, 100).unwrap(), before);
        assert_eq!(
            store.pending_events("policy-projector", 100).unwrap(),
            before
        );
    }

    #[test]
    fn guarded_lease_event_failure_preserves_fence_and_policy() {
        use graph_application::{CheckPolicyRepository, PolicyLeaseRepository};
        for table in ["events", "event_outbox"] {
            let mut store = Store::open(":memory:").unwrap();
            let expected = task("guarded");
            let owner = WorkerId::new("worker").unwrap();
            let policy = graph_domain::RequiredChecks::new(vec!["test".into()]).unwrap();
            store.enqueue(&expected, 0).unwrap();
            store.register_check_policy(&expected, &policy, 1).unwrap();
            let before = store.events(0, 100).unwrap();
            store.0.execute_batch(&format!("CREATE TRIGGER reject_lease_event BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'injected failure'); END;")).unwrap();
            assert!(
                store
                    .lease_with_policy(&expected, &policy, &owner, 2, 100)
                    .is_err()
            );
            assert_eq!(store.events(0, 100).unwrap(), before);
            assert_eq!(store.pending_events("dispatcher", 100).unwrap(), before);
            assert_eq!(store.check_policy(&expected).unwrap(), Some(policy.clone()));
            store
                .0
                .execute_batch("DROP TRIGGER reject_lease_event;")
                .unwrap();
            let lease = store
                .lease_with_policy(&expected, &policy, &owner, 3, 100)
                .unwrap();
            assert_eq!(lease.fencing_token(), 1);
        }
    }

    #[test]
    fn guarded_lease_contenders_have_one_durable_winner() {
        use graph_application::{CheckPolicyRepository, PolicyLeaseRepository};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("guarded-race.db");
        let mut store = Store::open(&path).unwrap();
        let expected = task("race");
        let policy = graph_domain::RequiredChecks::new(vec!["test".into()]).unwrap();
        store.enqueue(&expected, 0).unwrap();
        store.register_check_policy(&expected, &policy, 1).unwrap();
        let connections: Vec<_> = (0..8).map(|_| Store::open(&path).unwrap()).collect();
        let barrier = std::sync::Barrier::new(8);
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = connections
                .into_iter()
                .enumerate()
                .map(|(index, mut connection)| {
                    let expected = &expected;
                    let policy = &policy;
                    let barrier = &barrier;
                    scope.spawn(move || {
                        barrier.wait();
                        connection.lease_with_policy(
                            expected,
                            policy,
                            &WorkerId::new(format!("worker-{index}")).unwrap(),
                            2,
                            100,
                        )
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        let mut winner = None;
        for result in results {
            match result {
                Ok(lease) => assert!(winner.replace(lease).is_none(), "two leases granted"),
                Err(StoreError::Unavailable) => {}
                Err(error) => panic!("unexpected contention failure: {error}"),
            }
        }
        let winner = winner.expect("one worker must receive a lease");
        assert_eq!(winner.fencing_token(), 1);
        drop(store);
        let store = Store::open(&path).unwrap();
        let events = store.events(0, 100).unwrap();
        assert_eq!(
            events.iter().map(|event| event.kind).collect::<Vec<_>>(),
            vec![
                EventKind::Queued,
                EventKind::CheckPolicyRegistered,
                EventKind::Leased
            ]
        );
        let lease: graph_protocol::Lease = serde_json::from_str(&events[2].payload).unwrap();
        assert_eq!(lease, graph_protocol::Lease::from(&winner));
        assert_eq!(store.pending_events("dispatcher", 100).unwrap(), events);
    }

    #[test]
    fn registration_races_with_guarded_and_legacy_lease_serialize() {
        use graph_application::{CheckPolicyRepository, PolicyLeaseRepository};
        for guarded in [false, true] {
            for _ in 0..8 {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("registration-race.db");
                let mut store = Store::open(&path).unwrap();
                let expected = task("race");
                let policy = graph_domain::RequiredChecks::new(vec!["test".into()]).unwrap();
                let owner = WorkerId::new("worker").unwrap();
                store.enqueue(&expected, 0).unwrap();
                let mut registration = Store::open(&path).unwrap();
                let mut leasing = Store::open(&path).unwrap();
                let barrier = std::sync::Barrier::new(2);
                let (registered, leased) = std::thread::scope(|scope| {
                    let left = scope.spawn(|| {
                        barrier.wait();
                        registration.register_check_policy(&expected, &policy, 1)
                    });
                    let right = scope.spawn(|| {
                        barrier.wait();
                        if guarded {
                            leasing.lease_with_policy(&expected, &policy, &owner, 1, 100)
                        } else {
                            leasing.lease(expected.id(), &owner, 1, 100)
                        }
                    });
                    (left.join().unwrap(), right.join().unwrap())
                });
                match (registered, leased) {
                    (Ok(true), Ok(lease)) => assert_eq!(lease.fencing_token(), 1),
                    (Ok(true), Err(StoreError::Unavailable)) if guarded => {
                        // Leasing may have observed no policy. It must not have mutated state.
                        let lease = store
                            .lease_with_policy(&expected, &policy, &owner, 2, 100)
                            .unwrap();
                        assert_eq!(lease.fencing_token(), 1);
                    }
                    (Err(StoreError::Unavailable), Ok(lease)) if !guarded => {
                        assert_eq!(lease.fencing_token(), 1);
                        assert_eq!(store.check_policy(&expected).unwrap(), None);
                    }
                    other => panic!("invalid serialized outcome: {other:?}"),
                }
                drop(store);
                let store = Store::open(&path).unwrap();
                let events = store.events(0, 100).unwrap();
                let kinds: Vec<_> = events.iter().map(|event| event.kind).collect();
                if store.check_policy(&expected).unwrap().is_some() {
                    assert_eq!(
                        kinds,
                        vec![
                            EventKind::Queued,
                            EventKind::CheckPolicyRegistered,
                            EventKind::Leased
                        ]
                    );
                } else {
                    assert!(!guarded);
                    assert_eq!(kinds, vec![EventKind::Queued, EventKind::Leased]);
                }
                assert_eq!(store.pending_events("dispatcher", 100).unwrap(), events);
            }
        }
    }

    #[test]
    fn cancellation_rejects_terminal_missing_and_invalid_requests() {
        let mut coordinator = service(":memory:");
        coordinator.enqueue(task("a"), 0).unwrap();
        for (actor, reason) in [("", "stop"), ("operator", "  ")] {
            assert!(coordinator.cancel("a", actor, reason, 1).is_err());
        }
        assert!(
            coordinator
                .cancel("missing", "operator", "stop", 1)
                .is_err()
        );
        let lease = coordinator.lease("a", "worker", 1, 100).unwrap();
        coordinator.submit(lease, "patch", 2).unwrap();
        let mut fixture_store = coordinator.into_inner();
        fixture_store
            .integrate_candidate_fixture(&TaskId::new("a").unwrap(), "test fixture", 3)
            .unwrap();
        let mut coordinator = TaskService::new(fixture_store);
        assert!(coordinator.cancel("a", "operator", "stop", 4).is_err());
        let mut store = coordinator.into_inner();
        store
            .0
            .execute("UPDATE tasks SET state='rejected' WHERE id='a'", [])
            .unwrap();
        assert!(
            store
                .cancel(
                    &TaskId::new("a").unwrap(),
                    &WorkerId::new("operator").unwrap(),
                    "stop",
                    5
                )
                .is_err()
        );
        assert_eq!(store.events(0, 100).unwrap().len(), 4);
    }

    #[test]
    fn cancellation_does_not_unlock_dependents() {
        let mut coordinator = service(":memory:");
        coordinator.enqueue(task("a"), 0).unwrap();
        let base = task("b");
        let child = TaskSpec::new(
            TaskId::new("b").unwrap(),
            base.project().clone(),
            "gv1".into(),
            "implementer".into(),
            "plus-a".into(),
            vec!["src".into()],
            vec![TaskId::new("a").unwrap()],
            "ctx1".into(),
            vec!["patch".into()],
            6000,
        )
        .unwrap();
        coordinator.enqueue(child, 0).unwrap();
        coordinator.cancel("a", "operator", "stop", 1).unwrap();
        assert!(coordinator.lease("b", "worker", 2, 100).is_err());
        // Cancellation is scoped: descendants remain queued, not silently
        // cancelled or promoted. The scheduler must resolve their disposition.
        let store = coordinator.into_inner();
        let state: String = store
            .0
            .query_row("SELECT state FROM tasks WHERE id='b'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(state, "queued");
    }

    #[test]
    fn cancellation_races_serialize_with_cancel_submit_and_test_only_integration_fixture() {
        for operation in ["cancel", "submit", "integrate"] {
            for _ in 0..8 {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("race.db");
                let mut setup = service(&path);
                setup.enqueue(task("a"), 0).unwrap();
                let lease = setup.lease("a", "worker", 1, 100).unwrap();
                if operation == "integrate" {
                    setup.submit(lease.clone(), "candidate", 2).unwrap();
                }
                drop(setup);
                let mut left = service(&path);
                let mut right = service(&path);
                let barrier = std::sync::Barrier::new(2);
                let (cancelled, competing) = std::thread::scope(|scope| {
                    let ready = &barrier;
                    let a = scope.spawn(move || {
                        ready.wait();
                        left.cancel("a", "coordinator", "stop", 3)
                    });
                    let b = scope.spawn(move || {
                        ready.wait();
                        match operation {
                            "cancel" => right.cancel("a", "other", "stop too", 3),
                            "submit" => right.submit(lease, "candidate", 3).map(|()| true),
                            _ => right
                                .into_inner()
                                .integrate_candidate_fixture(
                                    &TaskId::new("a").unwrap(),
                                    "test fixture",
                                    3,
                                )
                                .map(|()| true)
                                .map_err(graph_application::ServiceError::Repository),
                        }
                    });
                    (a.join().unwrap(), b.join().unwrap())
                });
                let store = Store::open(&path).unwrap();
                let events = store.events(0, 100).unwrap();
                let state: String = store
                    .0
                    .query_row("SELECT state FROM tasks WHERE id='a'", [], |r| r.get(0))
                    .unwrap();
                if operation == "cancel" {
                    assert_ne!(cancelled.unwrap(), competing.unwrap());
                    assert_eq!(state, "cancelled");
                } else if operation == "integrate" {
                    assert_ne!(cancelled.is_ok(), competing.is_ok());
                    assert_eq!(
                        state,
                        if cancelled.is_ok() {
                            "cancelled"
                        } else {
                            "integrated"
                        }
                    );
                } else {
                    assert!(cancelled.unwrap());
                    assert_eq!(state, "cancelled");
                    if competing.is_ok() {
                        let submitted = events
                            .iter()
                            .position(|e| e.kind == EventKind::Submitted)
                            .unwrap();
                        let cancelled = events
                            .iter()
                            .position(|e| e.kind == EventKind::Cancelled)
                            .unwrap();
                        assert!(submitted < cancelled);
                    } else {
                        assert!(!events.iter().any(|e| e.kind == EventKind::Submitted));
                    }
                }
                let terminal: Vec<_> = events
                    .iter()
                    .filter(|e| matches!(e.kind, EventKind::Cancelled | EventKind::Integrated))
                    .collect();
                assert_eq!(terminal.len(), 1);
                assert_eq!(store.pending_events("executor", 100).unwrap(), events);
            }
        }
    }

    #[test]
    fn failed_dependency_insert_rolls_back_task_and_event() {
        let mut store = service(":memory:");
        let invalid = TaskSpec::new(
            TaskId::new("a").unwrap(),
            task("a").project().clone(),
            "gv1".into(),
            "implementer".into(),
            "plus-a".into(),
            vec!["src".into()],
            vec![TaskId::new("absent").unwrap()],
            "ctx1".into(),
            vec!["patch".into()],
            6000,
        )
        .unwrap();
        assert!(store.enqueue(invalid, 0).is_err());
        assert!(store.events(0, 100).unwrap().is_empty());
        assert!(store.enqueue(task("a"), 1).unwrap());
    }

    #[test]
    fn dependency_readiness_with_test_only_integration_fixture() {
        let mut store = service(":memory:");
        store.enqueue(task("a"), 0).unwrap();
        let child = TaskSpec::new(
            TaskId::new("b").unwrap(),
            task("a").project().clone(),
            "gv1".into(),
            "implementer".into(),
            "plus-a".into(),
            vec!["src".into()],
            vec![TaskId::new("a").unwrap()],
            "ctx1".into(),
            vec!["patch".into()],
            6000,
        )
        .unwrap();
        store.enqueue(child, 0).unwrap();
        let lease = store.lease("a", "worker", 0, 100).unwrap();
        store.submit(lease, "candidate.patch", 1).unwrap();
        assert!(store.lease("b", "worker", 2, 100).is_err());
        let mut fixture_store = store.into_inner();
        fixture_store
            .integrate_candidate_fixture(&TaskId::new("a").unwrap(), "test fixture", 3)
            .unwrap();
        let mut store = TaskService::new(fixture_store);
        assert!(store.lease("b", "worker", 4, 100).is_ok());
    }

    #[test]
    fn concurrent_connections_grant_only_one_lease() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        service(&path).enqueue(task("a"), 0).unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers: Vec<_> = (0..2)
            .map(|number| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let mut worker = service(path);
                    barrier.wait();
                    worker.lease("a", &number.to_string(), 1, 100).is_ok()
                })
            })
            .collect();
        let granted = workers
            .into_iter()
            .map(|worker| usize::from(worker.join().unwrap()))
            .sum::<usize>();
        assert_eq!(granted, 1);
    }

    #[test]
    fn fresh_database_has_versioned_migration_history() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let _store = service(&path);
        let connection = Connection::open(path).unwrap();
        let migrations: i64 = connection
            .query_row("SELECT COUNT(*) FROM refinery_schema_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(migrations, 17);
        let tasks_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tasks_table, "tasks");
    }

    #[test]
    fn v6_upgrade_preserves_task_and_does_not_fabricate_policy() {
        use graph_application::CheckPolicyRepository;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v6.db");
        let mut connection = Connection::open(&path).unwrap();
        let migrations: Vec<_> = embedded::migrations::runner()
            .get_migrations()
            .iter()
            .filter(|migration| migration.version() <= 6)
            .cloned()
            .collect();
        refinery::Runner::new(&migrations)
            .set_grouped(true)
            .run(&mut connection)
            .unwrap();
        let expected = task("policy-upgrade");
        let mut legacy = Store(connection);
        legacy.enqueue(&expected, 0).unwrap();
        let events = legacy.events(0, 100).unwrap();
        drop(legacy);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(store.check_policy(&expected).unwrap(), None);
        let policy = graph_domain::RequiredChecks::new(vec!["test".into()]).unwrap();
        assert!(store.register_check_policy(&expected, &policy, 0).unwrap());
        // Simulate malformed persisted data, not a supported mutation endpoint.
        store
            .0
            .execute(
                "UPDATE task_check_policies SET descriptor = ?1",
                [r#"{"schema_version":2,"names":["test"]}"#],
            )
            .unwrap();
        assert!(matches!(
            store.check_policy(&expected),
            Err(StoreError::Corrupt(_))
        ));
        assert!(matches!(
            store.register_check_policy(&expected, &policy, 0),
            Err(StoreError::Corrupt(_))
        ));
    }

    #[test]
    fn migration_checksum_drift_refuses_open_without_losing_tasks() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        service(&path).enqueue(task("preserved"), 0).unwrap();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute("UPDATE refinery_schema_history SET checksum='0'", [])
            .unwrap();
        drop(connection);

        assert!(matches!(Store::open(&path), Err(StoreError::Migration(_))));
        let connection = Connection::open(&path).unwrap();
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE id='preserved'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn concurrent_first_open_initializes_schema_once() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    Store::open(path).map(|_| ())
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
        let connection = Connection::open(&path).unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM refinery_schema_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 17);
    }

    #[test]
    fn outbox_replays_after_restart_with_independent_ordered_consumers() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let mut tasks = service(&path);
        tasks.enqueue(task("a"), 0).unwrap();
        tasks.lease("a", "worker", 1, 100).unwrap();
        let mut store = tasks.into_inner();
        let pending = store.pending_events("graph", 100).unwrap();
        assert_eq!(pending.len(), 2);
        assert!(
            store
                .acknowledge_event("graph", pending[1].sequence)
                .is_err()
        );
        store
            .acknowledge_event("graph", pending[0].sequence)
            .unwrap();
        drop(store);

        let mut restored = Store::open(&path).unwrap();
        let remaining = restored.pending_events("graph", 100).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].sequence, pending[1].sequence);
        assert_eq!(restored.pending_events("vector", 100).unwrap().len(), 2);
        restored
            .acknowledge_event("graph", remaining[0].sequence)
            .unwrap();
        restored
            .acknowledge_event("graph", remaining[0].sequence)
            .unwrap();
        assert!(restored.pending_events("graph", 100).unwrap().is_empty());
    }

    #[test]
    fn rolled_back_event_does_not_escape_into_outbox() {
        let mut store = Store::open(":memory:").unwrap();
        store.enqueue(&task("a"), 0).unwrap();
        let first = store.pending_events("graph", 100).unwrap();
        {
            let tx = store.0.transaction().unwrap();
            tx.execute("INSERT INTO events(task_id,kind,at_ms,payload) VALUES('a','submitted',1,'candidate')", []).unwrap();
            // Drop without commit simulates a failed application operation.
        }
        assert_eq!(store.pending_events("graph", 100).unwrap(), first);
        let count: i64 = store
            .0
            .query_row("SELECT COUNT(*) FROM event_outbox", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn upgrade_backfills_outbox_and_preserves_active_lease() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let mut connection = Connection::open(&path).unwrap();
        embedded::migrations::runner()
            .set_target(refinery::Target::Version(1))
            .run(&mut connection)
            .unwrap();
        let mut legacy = Store(connection);
        legacy.enqueue(&task("legacy"), 0).unwrap();
        let lease = legacy
            .lease(
                &TaskId::new("legacy").unwrap(),
                &WorkerId::new("worker").unwrap(),
                1,
                100,
            )
            .unwrap();
        let before = legacy.events(0, 100).unwrap();
        drop(legacy);

        let mut upgraded = Store::open(&path).unwrap();
        assert_eq!(upgraded.pending_events("graph", 100).unwrap(), before);
        upgraded.submit(&lease, "candidate.patch", 2).unwrap();
        let after = upgraded.pending_events("graph", 100).unwrap();
        assert_eq!(after.len(), 3);
        assert_eq!(after[2].kind, EventKind::Submitted);
        for event in after {
            upgraded.acknowledge_event("graph", event.sequence).unwrap();
        }
        drop(upgraded);
        assert!(
            Store::open(&path)
                .unwrap()
                .pending_events("graph", 100)
                .unwrap()
                .is_empty()
        );
    }
}

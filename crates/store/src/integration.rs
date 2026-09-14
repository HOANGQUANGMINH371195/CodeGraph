use crate::{Store, StoreError};
use graph_application::VerifiedIntegrationRepository;
use graph_domain::{EventKind, IntegrationDecision, TargetHeadVerification};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

impl VerifiedIntegrationRepository for Store {
    type Error = StoreError;

    fn integrate_verified(
        &mut self,
        decision: &IntegrationDecision,
        target: &TargetHeadVerification,
    ) -> Result<bool, StoreError> {
        if target.task() != decision.task() || target.observed_target() != decision.task().project()
        {
            return Err(StoreError::Conflict);
        }
        let decision_wire =
            serde_json::to_string(&graph_protocol::IntegrationDecision::from(decision))?;
        let target_wire =
            serde_json::to_string(&graph_protocol::TargetHeadVerification::from(target))?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(String, String)> = tx
            .query_row(
                include_str!("sql/select_integration_decision.sql"),
                [decision.task().id().as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((stored_decision, stored_target)) = existing {
            if stored_decision == decision_wire && stored_target == target_wire {
                tx.commit()?;
                return Ok(false);
            }
            return Err(StoreError::Conflict);
        }
        let (stored_task, state, _, policy) =
            crate::check_policy::read(&tx, decision.task())?.ok_or(StoreError::Unavailable)?;
        if stored_task != *decision.task()
            || state != "submitted"
            || policy.as_ref() != Some(decision.policy())
        {
            return Err(StoreError::Conflict);
        }
        let submitted = crate::submission_query::read(&tx, decision.task().id())?
            .ok_or(StoreError::Unavailable)?;
        if submitted.artifact != decision.candidate().id() || submitted.spec != *decision.task() {
            return Err(StoreError::Conflict);
        }
        for receipt in decision.receipts() {
            let run = receipt.binding().run_id();
            let plan = crate::execution_plan::read(&tx, run)?.ok_or(StoreError::Unavailable)?;
            let stored_receipt = crate::execution_receipt::read(&tx, run, decision.task())?
                .ok_or(StoreError::Unavailable)?;
            if stored_receipt != *receipt || !plan.matches_receipt(receipt) {
                return Err(StoreError::Conflict);
            }
            crate::execution_plan::validate_candidate(&tx, receipt.binding())?;
        }
        let changed = tx.execute(
            include_str!("sql/integrate_submitted_task.sql"),
            [decision.task().id().as_str()],
        )?;
        if changed != 1 {
            return Err(StoreError::Unavailable);
        }
        tx.execute(
            include_str!("sql/insert_integration_decision.sql"),
            params![decision.task().id().as_str(), decision_wire, target_wire],
        )?;
        tx.execute(
            include_str!("sql/insert_task_event.sql"),
            params![
                decision.task().id().as_str(),
                EventKind::Integrated.as_str(),
                decision.accepted_at_ms(),
                decision.verifier_version()
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_application::{
        AnalysisRepository, ArtifactReader, ArtifactRepository, CheckPolicyRepository,
        ExecutionPlanRepository, ExecutionReceiptRepository, PolicyLeaseRepository,
        SubmissionQueryRepository, TargetHeadVerifier, TaskQueryRepository, TaskRepository,
        verify_integration,
    };
    use graph_domain::execution::{
        ChildCompletion, ExecutionCompletion, ScopeCleanup, StopReason, StreamCompletion,
    };
    use graph_domain::{
        AnalysisRun, Artifact, ArtifactProtection, ArtifactRetention, CheckCommand,
        CheckRunBinding, ExecutionPlan, ExecutionReceipt, Lease, ProjectRef, RequiredChecks,
        TaskId, TaskSpec, TaskState, WorkerId,
    };
    use std::io::{self, Cursor, Read};

    const X_SHA256: &str = "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881";

    fn project() -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: "head".into(),
            working_tree_fingerprint: "clean".into(),
            config_hash: "config".into(),
            ignore_policy_version: "v1".into(),
        }
    }
    fn task() -> TaskSpec {
        TaskSpec::new(
            TaskId::new("task").unwrap(),
            project(),
            "graph".into(),
            "worker".into(),
            "lane".into(),
            vec!["src".into()],
            vec![],
            "ctx".into(),
            vec!["patch".into()],
            100,
        )
        .unwrap()
    }
    fn artifact(id: &str, project: ProjectRef, graph: &str, run: &str, kind: &str) -> Artifact {
        Artifact::new(
            id.into(),
            project,
            graph.into(),
            run.into(),
            X_SHA256.into(),
            1,
            kind.into(),
            ArtifactRetention::Evidence,
            ArtifactProtection::Unreviewed,
        )
        .unwrap()
    }
    fn setup_at(path: &str) -> (Store, TaskSpec, Artifact, RequiredChecks) {
        let mut store = Store::open(path).unwrap();
        let task = task();
        let policy = RequiredChecks::new(vec!["lint".into(), "test".into()]).unwrap();
        store.enqueue(&task, 0).unwrap();
        store.register_check_policy(&task, &policy, 1).unwrap();
        let candidate = artifact(
            "candidate",
            task.project().clone(),
            task.graph_version(),
            "candidate-run",
            "patch",
        );
        let run = AnalysisRun::new(
            "candidate-run".into(),
            task.project().clone(),
            task.graph_version().into(),
            "fixture".into(),
            "v1".into(),
            "b".repeat(64),
            "c".repeat(64),
        )
        .unwrap();
        store.record_analysis_run(&run).unwrap();
        store.record_artifact(&candidate).unwrap();
        let lease = store
            .lease_with_policy(&task, &policy, &WorkerId::new("worker").unwrap(), 2, 100)
            .unwrap();
        store.submit(&lease, candidate.id(), 3).unwrap();
        (store, task, candidate, policy)
    }
    fn setup() -> (Store, TaskSpec, Artifact, RequiredChecks) {
        setup_at(":memory:")
    }
    fn receipt(
        store: &mut Store,
        task: &TaskSpec,
        candidate: &Artifact,
        policy: &RequiredChecks,
        name: &str,
        run: &str,
    ) -> ExecutionReceipt {
        receipt_with_completion(
            store,
            task,
            candidate,
            policy,
            name,
            run,
            ExecutionCompletion {
                reason: StopReason::Exited,
                child: ChildCompletion::Reaped { exit_code: Some(0) },
                stdout: StreamCompletion::Complete,
                stderr: StreamCompletion::Complete,
                cleanup: ScopeCleanup::Complete,
            },
        )
    }

    fn receipt_with_completion(
        store: &mut Store,
        task: &TaskSpec,
        candidate: &Artifact,
        policy: &RequiredChecks,
        name: &str,
        run: &str,
        completion: ExecutionCompletion,
    ) -> ExecutionReceipt {
        let submitted = store.submitted_candidate(task.id()).unwrap().unwrap();
        let binding = CheckRunBinding::new(
            run.into(),
            task.clone(),
            Lease::issue(
                task.id().clone(),
                submitted.owner,
                submitted.fencing_token,
                102,
            )
            .unwrap(),
            submitted.submission_sequence,
            candidate.clone(),
            policy.clone(),
            name.into(),
            CheckCommand::new(
                "fixture".into(),
                vec![],
                ".".into(),
                "b".repeat(64),
                "c".repeat(64),
                10,
                10,
                10,
                10,
            )
            .unwrap(),
        )
        .unwrap();
        let mut snapshot = task.project().clone();
        snapshot.worktree_id = format!("exec-{run}");
        snapshot.working_tree_fingerprint = format!("applied-{run}");
        let receipt = ExecutionReceipt::new(
            binding.clone(),
            "host".into(),
            snapshot.clone(),
            4,
            5,
            1,
            Some(artifact(
                &format!("{run}-out"),
                snapshot.clone(),
                task.graph_version(),
                run,
                "stdout",
            )),
            Some(artifact(
                &format!("{run}-err"),
                snapshot,
                task.graph_version(),
                run,
                "stderr",
            )),
            completion,
        )
        .unwrap();
        let output_run = AnalysisRun::new(
            run.into(),
            receipt.execution_snapshot().clone(),
            task.graph_version().into(),
            "fixture".into(),
            "v1".into(),
            "b".repeat(64),
            "c".repeat(64),
        )
        .unwrap();
        store.record_analysis_run(&output_run).unwrap();
        store
            .record_artifact(receipt.stdout().as_ref().unwrap())
            .unwrap();
        store
            .record_artifact(receipt.stderr().as_ref().unwrap())
            .unwrap();
        store
            .register_execution_plan(
                &ExecutionPlan::new(binding, "host".into(), receipt.execution_snapshot().clone())
                    .unwrap(),
            )
            .unwrap();
        store.record_execution_receipt(&receipt).unwrap();
        receipt
    }
    #[test]
    fn verified_integration_is_atomic_idempotent_and_outboxed() {
        let (mut store, task, candidate, policy) = setup();
        let test = receipt(&mut store, &task, &candidate, &policy, "test", "run-test");
        let lint = receipt(&mut store, &task, &candidate, &policy, "lint", "run-lint");
        let decision = IntegrationDecision::new(
            task.clone(),
            candidate,
            policy,
            vec![test, lint],
            6,
            "w1-i/v1".into(),
        )
        .unwrap();
        let target = TargetHeadVerification::new(
            task.clone(),
            task.project().clone(),
            6,
            "target/v1".into(),
        )
        .unwrap();
        assert!(store.integrate_verified(&decision, &target).unwrap());
        assert!(!store.integrate_verified(&decision, &target).unwrap());
        assert_eq!(
            store.task_snapshot(task.id()).unwrap().unwrap().state,
            TaskState::Integrated
        );
        let event = store
            .events(0, 100)
            .unwrap()
            .into_iter()
            .find(|event| event.kind == EventKind::Integrated)
            .unwrap();
        let count: i64 = store
            .0
            .query_row(
                "SELECT count(*) FROM event_outbox WHERE event_sequence=?1",
                [event.sequence],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn outbox_failure_rolls_back_state_and_decision() {
        let (mut store, task, candidate, policy) = setup();
        let test = receipt(&mut store, &task, &candidate, &policy, "test", "run-test");
        let lint = receipt(&mut store, &task, &candidate, &policy, "lint", "run-lint");
        let decision = IntegrationDecision::new(
            task.clone(),
            candidate,
            policy,
            vec![test, lint],
            6,
            "w1-i/v1".into(),
        )
        .unwrap();
        let target = TargetHeadVerification::new(
            task.clone(),
            task.project().clone(),
            6,
            "target/v1".into(),
        )
        .unwrap();
        store.0.execute_batch("CREATE TRIGGER fail_integration_outbox BEFORE INSERT ON event_outbox WHEN EXISTS (SELECT 1 FROM events WHERE seq=NEW.event_sequence AND kind='integrated') BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
        assert!(store.integrate_verified(&decision, &target).is_err());
        assert_eq!(
            store.task_snapshot(task.id()).unwrap().unwrap().state,
            TaskState::Submitted
        );
        let decisions: i64 = store
            .0
            .query_row(
                "SELECT count(*) FROM verified_integration_decisions",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(decisions, 0);
    }

    #[test]
    fn cancellation_winner_prevents_verified_integration() {
        let (mut store, task, candidate, policy) = setup();
        let test = receipt(&mut store, &task, &candidate, &policy, "test", "run-test");
        let lint = receipt(&mut store, &task, &candidate, &policy, "lint", "run-lint");
        let decision = IntegrationDecision::new(
            task.clone(),
            candidate,
            policy,
            vec![test, lint],
            6,
            "w1-i/v1".into(),
        )
        .unwrap();
        let target = TargetHeadVerification::new(
            task.clone(),
            task.project().clone(),
            6,
            "target/v1".into(),
        )
        .unwrap();
        assert!(
            store
                .cancel(task.id(), &WorkerId::new("coordinator").unwrap(), "stop", 6)
                .unwrap()
        );
        assert!(matches!(
            store.integrate_verified(&decision, &target),
            Err(StoreError::Conflict)
        ));
        assert_eq!(
            store.task_snapshot(task.id()).unwrap().unwrap().state,
            TaskState::Cancelled
        );
        let decisions: i64 = store
            .0
            .query_row(
                "SELECT count(*) FROM verified_integration_decisions",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(decisions, 0);
    }

    struct FixtureReader;
    impl ArtifactReader for FixtureReader {
        fn open_artifact(&self, _artifact: &Artifact) -> io::Result<Box<dyn Read>> {
            Ok(Box::new(Cursor::new(b"x")))
        }
    }
    struct FixtureTarget;
    impl TargetHeadVerifier for FixtureTarget {
        type Error = io::Error;
        fn verify_target_head(
            &self,
            task: &TaskSpec,
        ) -> Result<TargetHeadVerification, Self::Error> {
            TargetHeadVerification::new(
                task.clone(),
                task.project().clone(),
                7,
                "fixture-target/v1".into(),
            )
            .map_err(|error| io::Error::other(error.to_string()))
        }
    }

    #[test]
    fn application_verifier_rechecks_bytes_before_atomic_integration() {
        let (mut store, task, candidate, policy) = setup();
        receipt(&mut store, &task, &candidate, &policy, "test", "run-test");
        receipt(&mut store, &task, &candidate, &policy, "lint", "run-lint");
        let too_small = verify_integration(
            &store,
            &FixtureReader,
            &FixtureTarget,
            &task,
            &["run-test".into(), "run-lint".into()],
            1,
            3,
            "fixture-verifier/v1".into(),
        );
        assert!(matches!(
            too_small,
            Err(graph_application::IntegrationVerificationError::OutputTooLarge)
        ));
        assert_eq!(
            store.task_snapshot(task.id()).unwrap().unwrap().state,
            TaskState::Submitted
        );
        let verified = verify_integration(
            &store,
            &FixtureReader,
            &FixtureTarget,
            &task,
            &["run-test".into(), "run-lint".into()],
            1,
            4,
            "fixture-verifier/v1".into(),
        )
        .unwrap();
        let mut service = graph_application::TaskService::new(store);
        assert!(service.integrate_verified(verified).unwrap());
        let store = service.into_inner();
        assert_eq!(
            store.task_snapshot(task.id()).unwrap().unwrap().state,
            TaskState::Integrated
        );
    }

    #[test]
    fn invalid_check_sets_are_rejected_before_target_access() {
        struct UnavailableTarget;
        impl TargetHeadVerifier for UnavailableTarget {
            type Error = io::Error;
            fn verify_target_head(
                &self,
                _: &TaskSpec,
            ) -> Result<TargetHeadVerification, Self::Error> {
                panic!("invalid receipt set must not access the target")
            }
        }
        let (mut store, task, candidate, policy) = setup();
        receipt(&mut store, &task, &candidate, &policy, "test", "run-test");
        receipt(
            &mut store,
            &task,
            &candidate,
            &policy,
            "test",
            "run-test-again",
        );
        for ids in [
            vec!["run-test".into()],
            vec!["run-test".into(), "run-test-again".into()],
        ] {
            let result = verify_integration(
                &store,
                &FixtureReader,
                &UnavailableTarget,
                &task,
                &ids,
                1,
                4,
                "fixture/v1".into(),
            );
            assert!(matches!(
                result,
                Err(graph_application::IntegrationVerificationError::Decision(_))
            ));
            assert_eq!(
                store.task_snapshot(task.id()).unwrap().unwrap().state,
                TaskState::Submitted
            );
        }
        let passed = ExecutionCompletion {
            reason: StopReason::Exited,
            child: ChildCompletion::Reaped { exit_code: Some(0) },
            stdout: StreamCompletion::Complete,
            stderr: StreamCompletion::Complete,
            cleanup: ScopeCleanup::Complete,
        };
        for (index, completion) in [
            ExecutionCompletion {
                child: ChildCompletion::Reaped { exit_code: Some(1) },
                ..passed
            },
            ExecutionCompletion {
                reason: StopReason::TimedOut,
                ..passed
            },
            ExecutionCompletion {
                reason: StopReason::Cancelled,
                ..passed
            },
            ExecutionCompletion {
                cleanup: ScopeCleanup::Unverifiable,
                ..passed
            },
        ]
        .into_iter()
        .enumerate()
        {
            let run = format!("failed-lint-{index}");
            receipt_with_completion(
                &mut store, &task, &candidate, &policy, "lint", &run, completion,
            );
            let result = verify_integration(
                &store,
                &FixtureReader,
                &UnavailableTarget,
                &task,
                &["run-test".into(), run],
                1,
                4,
                "fixture/v1".into(),
            );
            assert!(
                matches!(
                    result,
                    Err(graph_application::IntegrationVerificationError::Decision(_))
                ),
                "completion {completion:?}: {result:?}"
            );
            assert_eq!(
                store.task_snapshot(task.id()).unwrap().unwrap().state,
                TaskState::Submitted
            );
        }
    }

    #[test]
    fn verified_decision_replays_after_reopen() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state.sqlite");
        let (mut store, task, candidate, policy) = setup_at(path.to_str().unwrap());
        let test = receipt(&mut store, &task, &candidate, &policy, "test", "run-test");
        let lint = receipt(&mut store, &task, &candidate, &policy, "lint", "run-lint");
        let decision = IntegrationDecision::new(
            task.clone(),
            candidate,
            policy,
            vec![test, lint],
            6,
            "w1-i/v1".into(),
        )
        .unwrap();
        let target = TargetHeadVerification::new(
            task.clone(),
            task.project().clone(),
            6,
            "target/v1".into(),
        )
        .unwrap();
        assert!(store.integrate_verified(&decision, &target).unwrap());
        drop(store);
        let mut reopened = Store::open(&path).unwrap();
        assert!(!reopened.integrate_verified(&decision, &target).unwrap());
        assert_eq!(
            reopened.task_snapshot(task.id()).unwrap().unwrap().state,
            TaskState::Integrated
        );
    }

    #[test]
    fn two_connections_commit_one_decision_and_one_replay() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state.sqlite");
        let (mut store, task, candidate, policy) = setup_at(path.to_str().unwrap());
        let test = receipt(&mut store, &task, &candidate, &policy, "test", "run-test");
        let lint = receipt(&mut store, &task, &candidate, &policy, "lint", "run-lint");
        let decision = IntegrationDecision::new(
            task.clone(),
            candidate,
            policy,
            vec![test, lint],
            6,
            "w1-i/v1".into(),
        )
        .unwrap();
        let target = TargetHeadVerification::new(
            task.clone(),
            task.project().clone(),
            6,
            "target/v1".into(),
        )
        .unwrap();
        drop(store);
        let first = Store::open(&path).unwrap();
        let second = Store::open(&path).unwrap();
        let first_decision = decision.clone();
        let first_target = target.clone();
        let first = std::thread::spawn(move || {
            let mut store = first;
            store.integrate_verified(&first_decision, &first_target)
        });
        let second_decision = decision;
        let second_target = target;
        let second = std::thread::spawn(move || {
            let mut store = second;
            store.integrate_verified(&second_decision, &second_target)
        });
        let outcomes = [
            first.join().unwrap().unwrap(),
            second.join().unwrap().unwrap(),
        ];
        assert_eq!(outcomes.iter().filter(|inserted| **inserted).count(), 1);
        assert_eq!(outcomes.iter().filter(|inserted| !**inserted).count(), 1);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(
            reopened.task_snapshot(task.id()).unwrap().unwrap().state,
            TaskState::Integrated
        );
    }
}

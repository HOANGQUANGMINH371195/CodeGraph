use graph_application::CheckPolicyRepository;
use graph_application::PolicyLeaseRepository;
use graph_application::{
    AnalysisRepository, ArtifactReader, ArtifactRepository, ArtifactVerificationError,
    SubmissionContentError, SubmissionQueryRepository, SubmittedCandidate, TaskRepository,
    verify_submitted_content,
};
use graph_domain::RequiredChecks;
use graph_domain::{Artifact, TaskId, TaskSpec, WorkerId};
use graph_store::Store;
use serde_json::json;
use std::io::{self, Cursor, Read};
use std::{cell::RefCell, collections::VecDeque};

// Deliberately violates repository filtering to test the application boundary.
struct ScriptedRepository {
    observations: RefCell<VecDeque<io::Result<Option<SubmittedCandidate>>>>,
    artifact: Option<Artifact>,
}

impl SubmissionQueryRepository for ScriptedRepository {
    type Error = io::Error;
    fn submitted_candidate(&self, _: &TaskId) -> Result<Option<SubmittedCandidate>, Self::Error> {
        self.observations
            .borrow_mut()
            .pop_front()
            .expect("unexpected query")
    }
}

impl ArtifactRepository for ScriptedRepository {
    type Error = io::Error;
    fn record_artifact(&mut self, _: &Artifact) -> Result<bool, Self::Error> {
        panic!("verifier must not write")
    }
    fn artifact(
        &self,
        _: &str,
        _: &graph_domain::ProjectRef,
        _: &str,
    ) -> Result<Option<Artifact>, Self::Error> {
        Ok(Some(
            self.artifact.clone().expect("unexpected artifact lookup"),
        ))
    }
}

fn scripted(candidate: &SubmittedCandidate, artifact: Option<Artifact>) -> ScriptedRepository {
    ScriptedRepository {
        observations: RefCell::new(VecDeque::from([Ok(Some(candidate.clone()))])),
        artifact,
    }
}

impl CheckPolicyRepository for ScriptedRepository {
    type Error = io::Error;

    fn register_check_policy(
        &mut self,
        _: &TaskSpec,
        _: &RequiredChecks,
        _: i64,
    ) -> Result<bool, Self::Error> {
        panic!("reconciliation must not write")
    }

    fn check_policy(&self, _: &TaskSpec) -> Result<Option<RequiredChecks>, Self::Error> {
        Ok(Some(RequiredChecks::new(vec!["test".into()]).unwrap()))
    }
}

#[test]
fn check_binding_recheck_rejects_changed_missing_and_failed_observations() {
    use graph_application::{CheckBindingError, reconcile_check_binding};

    let mut store = Store::open(":memory:").unwrap();
    let (expected, artifact) = fixture_with_policy(&mut store, true, true);
    let request = binding(&store, &expected, &artifact);
    let candidate = store.submitted_candidate(expected.id()).unwrap().unwrap();
    for field in 0..8 {
        let mut changed = candidate.clone();
        match field {
            0 => changed.spec = task(&artifact, "changed"),
            1 => changed.owner = WorkerId::new("other").unwrap(),
            2 => changed.fencing_token += 1,
            3 => changed.submission_sequence += 1,
            4 => changed.submitted_at_ms += 1,
            5 => changed.artifact = "other".into(),
            _ => {}
        }
        let next = match field {
            6 => Ok(None),
            7 => Err(io::Error::other("recheck failed")),
            _ => Ok(Some(changed)),
        };
        let repository = scripted(&candidate, Some(artifact.clone()));
        repository.observations.borrow_mut().push_back(next);
        let result = reconcile_check_binding(&repository, &request);
        if field == 7 {
            assert!(matches!(result, Err(CheckBindingError::Repository(_))));
        } else {
            assert!(
                matches!(result, Err(CheckBindingError::Changed)),
                "field {field}"
            );
        }
        assert!(repository.observations.borrow().is_empty());
    }
}

#[test]
fn substituted_artifact_identity_and_every_snapshot_field_fail_before_io() {
    let mut store = Store::open(":memory:").unwrap();
    let (task, artifact) = fixture(&mut store, true);
    let candidate = store.submitted_candidate(task.id()).unwrap().unwrap();
    let unopened = Reader {
        on_open: || panic!("substituted descriptor"),
        bytes: b"",
    };
    for field in 0..8 {
        let mut wire = graph_protocol::Artifact::from(&artifact);
        let value = match field {
            0 => &mut wire.id,
            1 => &mut wire.graph_version,
            2 => &mut wire.project.repository_id,
            3 => &mut wire.project.worktree_id,
            4 => &mut wire.project.git_head,
            5 => &mut wire.project.working_tree_fingerprint,
            6 => &mut wire.project.config_hash,
            _ => &mut wire.project.ignore_policy_version,
        };
        value.push_str("-other");
        let repository = scripted(&candidate, Some(wire.try_into_domain().unwrap()));
        assert!(
            matches!(
                verify_submitted_content(&repository, &unopened, &task, 3),
                Err(SubmissionContentError::ScopeMismatch)
            ),
            "field {field}"
        );
    }
}

#[test]
fn malformed_submission_binding_fails_before_artifact_lookup() {
    let mut store = Store::open(":memory:").unwrap();
    let (task, _) = fixture(&mut store, true);
    let candidate = store.submitted_candidate(task.id()).unwrap().unwrap();
    let unopened = Reader {
        on_open: || panic!("invalid binding"),
        bytes: b"",
    };
    for field in 0..4 {
        let mut invalid = candidate.clone();
        match field {
            0 => invalid.fencing_token = 0,
            1 => invalid.submission_sequence = 0,
            2 => invalid.submitted_at_ms = -1,
            _ => invalid.artifact = " \t".into(),
        }
        assert!(
            matches!(
                verify_submitted_content(&scripted(&invalid, None), &unopened, &task, 3),
                Err(SubmissionContentError::ScopeMismatch)
            ),
            "field {field}"
        );
    }
}

#[test]
fn recheck_compares_every_candidate_binding_not_just_artifact() {
    let mut store = Store::open(":memory:").unwrap();
    let (expected, artifact) = fixture(&mut store, true);
    let candidate = store.submitted_candidate(expected.id()).unwrap().unwrap();
    let reader = Reader {
        on_open: || {},
        bytes: b"abc",
    };
    for field in 0..6 {
        let mut changed = candidate.clone();
        match field {
            0 => changed.spec = task(&artifact, "changed"),
            1 => changed.owner = WorkerId::new("other").unwrap(),
            2 => changed.fencing_token += 1,
            3 => changed.submission_sequence += 1,
            4 => changed.submitted_at_ms += 1,
            _ => changed.artifact = "other".into(),
        }
        let repository = scripted(&candidate, Some(artifact.clone()));
        repository
            .observations
            .borrow_mut()
            .push_back(Ok(Some(changed)));
        assert!(
            matches!(
                verify_submitted_content(&repository, &reader, &expected, 3),
                Err(SubmissionContentError::Changed)
            ),
            "field {field}"
        );
    }
}

#[test]
fn failed_repository_recheck_never_returns_verified_content() {
    let mut store = Store::open(":memory:").unwrap();
    let (task, artifact) = fixture(&mut store, true);
    let candidate = store.submitted_candidate(task.id()).unwrap().unwrap();
    let repository = scripted(&candidate, Some(artifact));
    repository
        .observations
        .borrow_mut()
        .push_back(Err(io::Error::other("read failed")));
    let reader = Reader {
        on_open: || {},
        bytes: b"abc",
    };
    assert!(matches!(
        verify_submitted_content(&repository, &reader, &task, 3),
        Err(SubmissionContentError::Repository(_))
    ));
}

struct Reader<F: Fn()> {
    on_open: F,
    bytes: &'static [u8],
}
impl<F: Fn()> ArtifactReader for Reader<F> {
    fn open_artifact(&self, _: &Artifact) -> io::Result<Box<dyn Read>> {
        (self.on_open)();
        Ok(Box::new(Cursor::new(self.bytes)))
    }
}

fn fixture(store: &mut Store, registered: bool) -> (TaskSpec, Artifact) {
    fixture_with_policy(store, registered, false)
}

fn fixture_with_policy(
    store: &mut Store,
    registered: bool,
    with_policy: bool,
) -> (TaskSpec, Artifact) {
    let project = json!({"repository_id":"repo","worktree_id":"w","git_head":"h",
        "working_tree_fingerprint":"s","config_hash":"c","ignore_policy_version":"1"});
    let artifact: graph_protocol::Artifact = serde_json::from_value(json!({
        "schema_version":1,"id":"a1","project":project,"graph_version":"g1",
        "analysis_run":"r1","byte_length":3,"kind":"stdout","retention":"evidence",
        "declared_protection":"unreviewed",
        "content_sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    }))
    .unwrap();
    let artifact = artifact.try_into_domain().unwrap();
    if registered {
        let run: graph_protocol::AnalysisRun = serde_json::from_value(json!({
            "schema_version":1,"id":"r1","project":project,"graph_version":"g1",
            "analyzer":"fixture","analyzer_version":"1",
            "configuration_sha256":"a".repeat(64),"input_manifest_sha256":"b".repeat(64)
        }))
        .unwrap();
        store
            .record_analysis_run(&run.try_into_domain().unwrap())
            .unwrap();
        store.record_artifact(&artifact).unwrap();
    }
    let task = task(&artifact, "context");
    store.enqueue(&task, 0).unwrap();
    if with_policy {
        store
            .register_check_policy(&task, &RequiredChecks::new(vec!["test".into()]).unwrap(), 0)
            .unwrap();
    }
    let lease = store
        .lease(task.id(), &WorkerId::new("worker").unwrap(), 1, 100)
        .unwrap();
    store.submit(&lease, artifact.id(), 2).unwrap();
    (task, artifact)
}

fn task(artifact: &Artifact, context: &str) -> TaskSpec {
    TaskSpec::new(
        TaskId::new("task").unwrap(),
        artifact.project().clone(),
        "g1".into(),
        "worker".into(),
        "native".into(),
        vec!["src".into()],
        vec![],
        context.into(),
        vec!["patch".into()],
        100,
    )
    .unwrap()
}

fn binding(
    store: &Store,
    expected: &TaskSpec,
    artifact: &Artifact,
) -> graph_domain::CheckRunBinding {
    let candidate = store.submitted_candidate(expected.id()).unwrap().unwrap();
    graph_domain::CheckRunBinding::new(
        "check-1".into(),
        expected.clone(),
        graph_domain::Lease::issue(
            expected.id().clone(),
            candidate.owner,
            candidate.fencing_token,
            101,
        )
        .unwrap(),
        candidate.submission_sequence,
        artifact.clone(),
        RequiredChecks::new(vec!["test".into()]).unwrap(),
        "test".into(),
        graph_domain::CheckCommand::new(
            "fixture".into(),
            vec![],
            ".".into(),
            "a".repeat(64),
            "b".repeat(64),
            1000,
            500,
            1024,
            1024,
        )
        .unwrap(),
    )
    .unwrap()
}

fn output_receipt(store: &mut Store, register_outputs: bool) -> graph_domain::ExecutionReceipt {
    use graph_domain::execution::{
        ChildCompletion, ExecutionCompletion, ScopeCleanup, StopReason, StreamCompletion,
    };
    let (task, candidate) = fixture_with_policy(store, true, true);
    let request = binding(store, &task, &candidate);
    let mut snapshot = task.project().clone();
    snapshot.worktree_id = "execution".into();
    snapshot.working_tree_fingerprint = "applied".into();
    let mut outputs = Vec::new();
    for kind in ["stdout", "stderr"] {
        let mut wire = graph_protocol::Artifact::from(&candidate);
        wire.id = kind.into();
        wire.kind = kind.into();
        wire.project = (&snapshot).into();
        wire.analysis_run = request.run_id().into();
        outputs.push(wire.try_into_domain().unwrap());
    }
    if register_outputs {
        let run: graph_protocol::AnalysisRun = serde_json::from_value(json!({
            "schema_version":1,"id":request.run_id(),"project":graph_protocol::ProjectRef::from(&snapshot),
            "graph_version":"g1","analyzer":"fixture","analyzer_version":"1",
            "configuration_sha256":"a".repeat(64),"input_manifest_sha256":"b".repeat(64)
        })).unwrap();
        store
            .record_analysis_run(&run.try_into_domain().unwrap())
            .unwrap();
        for output in &outputs {
            store.record_artifact(output).unwrap();
        }
    }
    graph_domain::ExecutionReceipt::new(
        request,
        "host".into(),
        snapshot,
        3,
        4,
        1,
        Some(outputs[0].clone()),
        Some(outputs[1].clone()),
        ExecutionCompletion {
            reason: StopReason::Exited,
            child: ChildCompletion::Reaped { exit_code: Some(0) },
            stdout: StreamCompletion::Complete,
            stderr: StreamCompletion::Complete,
            cleanup: ScopeCleanup::Unverifiable,
        },
    )
    .unwrap()
}

#[test]
fn receipt_outputs_verify_registered_bytes_without_promoting_execution_claims() {
    use graph_application::{ReceiptOutputError, verify_receipt_outputs};
    let mut store = Store::open(":memory:").unwrap();
    let report = output_receipt(&mut store, true);
    let events = store.events(0, 100).unwrap();
    let opened = RefCell::new(Vec::new());
    struct Outputs<'a>(&'a RefCell<Vec<String>>);
    impl ArtifactReader for Outputs<'_> {
        fn open_artifact(&self, artifact: &Artifact) -> io::Result<Box<dyn Read>> {
            self.0.borrow_mut().push(artifact.id().into());
            Ok(Box::new(Cursor::new(b"abc")))
        }
    }
    let observed = verify_receipt_outputs(
        &store,
        &Outputs(&opened),
        &report,
        report.binding(),
        "host",
        report.execution_snapshot(),
        6,
    )
    .unwrap();
    assert_eq!(*opened.borrow(), ["stdout", "stderr"]);
    assert_eq!(observed.receipt(), &report);
    assert_eq!(
        observed.stdout().unwrap().artifact(),
        report.stdout().as_ref().unwrap()
    );
    assert_eq!(
        observed.stderr().unwrap().artifact(),
        report.stderr().as_ref().unwrap()
    );
    assert_eq!(
        observed.receipt().completion().reported_check_outcome(),
        graph_domain::CheckOutcome::Unknown
    );
    assert_eq!(store.events(0, 100).unwrap(), events);
    let corrupt = Reader {
        on_open: || {},
        bytes: b"bad",
    };
    assert!(matches!(
        verify_receipt_outputs(
            &store,
            &corrupt,
            &report,
            report.binding(),
            "host",
            report.execution_snapshot(),
            6
        ),
        Err(ReceiptOutputError::Content(
            ArtifactVerificationError::HashMismatch
        ))
    ));
}

fn execution_plan(report: &graph_domain::ExecutionReceipt) -> graph_domain::ExecutionPlan {
    graph_domain::ExecutionPlan::new(
        report.binding().clone(),
        report.host_id().clone(),
        report.execution_snapshot().clone(),
    )
    .unwrap()
}

#[test]
fn launch_claim_is_one_shot_across_restart_and_cancellation() {
    use graph_application::{ExecutionLaunchRepository, ExecutionPlanRepository};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("launch.sqlite");
    let mut store = Store::open(&path).unwrap();
    let report = output_receipt(&mut store, true);
    let plan = execution_plan(&report);
    store.register_execution_plan(&plan).unwrap();
    let before = store.events(0, 100).unwrap();
    assert!(store.claim_execution_launch(&plan, 10).unwrap());
    assert!(!store.claim_execution_launch(&plan, 11).unwrap());
    let events = store.events(0, 100).unwrap();
    assert_eq!(events.len(), before.len() + 1);
    let event = events.last().unwrap();
    assert_eq!(event.kind, graph_domain::EventKind::ExecutionLaunchClaimed);
    assert_eq!(event.payload, plan.binding().run_id());
    assert_eq!(event.at_ms, 10);
    let count: i64 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            include_str!("sql/count_launch_outbox_fixture.sql"),
            [event.sequence],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        store.pending_events("launch-observer", 100).unwrap(),
        events
    );
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert!(!store.claim_execution_launch(&plan, i64::MAX).unwrap());
    assert_eq!(store.events(0, 100).unwrap(), events);
    store
        .cancel(
            plan.binding().task().id(),
            &WorkerId::new("host").unwrap(),
            "stop",
            12,
        )
        .unwrap();
    assert!(!store.claim_execution_launch(&plan, 13).unwrap());
}

#[test]
fn launch_claim_rejects_missing_changed_cancelled_or_receipted_plan() {
    use graph_application::{
        ExecutionLaunchRepository, ExecutionPlanRepository, ExecutionReceiptRepository,
    };
    for case in 0..5 {
        let mut store = Store::open(":memory:").unwrap();
        let report = output_receipt(&mut store, true);
        let plan = execution_plan(&report);
        if case != 0 {
            store.register_execution_plan(&plan).unwrap();
        }
        if case == 1 {
            store
                .cancel(
                    plan.binding().task().id(),
                    &WorkerId::new("host").unwrap(),
                    "stop",
                    10,
                )
                .unwrap();
        }
        if case == 2 {
            store.record_execution_receipt(&report).unwrap();
        }
        let mut wire = graph_protocol::ExecutionPlan::from(&plan);
        if case == 3 {
            wire.host_id.push('x');
        }
        let expected = wire.try_into_domain().unwrap();
        let events = store.events(0, 100).unwrap();
        assert!(
            store
                .claim_execution_launch(&expected, if case == 4 { -1 } else { 11 })
                .is_err()
        );
        assert_eq!(store.events(0, 100).unwrap(), events);
    }
}

#[test]
fn launch_claim_contenders_have_one_durable_winner() {
    use graph_application::{ExecutionLaunchRepository, ExecutionPlanRepository};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("launch-race.sqlite");
    let mut store = Store::open(&path).unwrap();
    let report = output_receipt(&mut store, true);
    let plan = execution_plan(&report);
    store.register_execution_plan(&plan).unwrap();
    let before = store.events(0, 100).unwrap();
    let connections: Vec<_> = (0..8).map(|_| Store::open(&path).unwrap()).collect();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let workers: Vec<_> = connections
        .into_iter()
        .map(|mut store| {
            let barrier = barrier.clone();
            let plan = plan.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.claim_execution_launch(&plan, 10).unwrap()
            })
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|claimed| **claimed).count(), 1);
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert!(!store.claim_execution_launch(&plan, 20).unwrap());
    assert_eq!(store.events(0, 100).unwrap().len(), before.len() + 1);
}

#[test]
fn launch_event_failure_rolls_back_claim_and_allows_first_retry() {
    use graph_application::{ExecutionLaunchRepository, ExecutionPlanRepository};
    for fail_outbox in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("launch-rollback.sqlite");
        let mut store = Store::open(&path).unwrap();
        let report = output_receipt(&mut store, true);
        let plan = execution_plan(&report);
        store.register_execution_plan(&plan).unwrap();
        let before = store.events(0, 100).unwrap();
        let fixture = rusqlite::Connection::open(&path).unwrap();
        fixture
            .execute_batch(if fail_outbox {
                include_str!("sql/fail_launch_outbox_fixture.sql")
            } else {
                include_str!("sql/fail_launch_event_fixture.sql")
            })
            .unwrap();
        assert!(store.claim_execution_launch(&plan, 10).is_err());
        assert_eq!(store.events(0, 100).unwrap(), before);
        fixture
            .execute_batch(include_str!("sql/restore_launch_event_fixture.sql"))
            .unwrap();
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert!(store.claim_execution_launch(&plan, 11).unwrap());
        assert!(!store.claim_execution_launch(&plan, 12).unwrap());
    }
}

#[test]
fn execution_plan_registration_replays_after_restart_without_relaunch_authority() {
    use graph_application::{ExecutionPlanRepository, ExecutionReceiptRepository};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plans.sqlite");
    let mut store = Store::open(&path).unwrap();
    let report = output_receipt(&mut store, true);
    let plan = execution_plan(&report);
    let events = store.events(0, 100).unwrap();
    assert!(store.register_execution_plan(&plan).unwrap());
    assert!(!store.register_execution_plan(&plan).unwrap());
    assert_eq!(store.events(0, 100).unwrap(), events);
    for field in 0..3 {
        let mut wrong = graph_protocol::ExecutionReceipt::from(&report);
        match field {
            0 => wrong.host_id = "other".into(),
            1 => wrong.binding.command.program = "other".into(),
            _ => wrong.binding.command.args.push("another".into()),
        }
        assert!(matches!(
            store.record_execution_receipt(&wrong.try_into_domain().unwrap()),
            Err(graph_store::StoreError::Conflict)
        ));
    }
    assert!(store.record_execution_receipt(&report).unwrap());
    store
        .cancel(
            plan.binding().task().id(),
            &WorkerId::new("host").unwrap(),
            "stop",
            5,
        )
        .unwrap();
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .execution_plan(plan.binding().run_id(), plan.binding().task())
            .unwrap(),
        Some(plan.clone())
    );
    assert!(!store.register_execution_plan(&plan).unwrap());
    assert!(
        plan.matches_receipt(
            &store
                .execution_receipt(plan.binding().run_id(), plan.binding().task())
                .unwrap()
                .unwrap()
        )
    );
    let mut wrong = graph_protocol::ExecutionPlan::from(&plan);
    wrong.host_id = "other".into();
    assert!(matches!(
        store.register_execution_plan(&wrong.try_into_domain().unwrap()),
        Err(graph_store::StoreError::Conflict)
    ));
    let mut task = graph_protocol::TaskSpec::from(plan.binding().task());
    task.context_ref.push('x');
    assert!(
        store
            .execution_plan(plan.binding().run_id(), &task.try_into_domain().unwrap())
            .unwrap()
            .is_none()
    );
}

#[test]
fn execution_plan_cannot_be_registered_after_receipt_or_cancellation() {
    use graph_application::{ExecutionPlanRepository, ExecutionReceiptRepository};
    for has_receipt in [false, true] {
        let mut store = Store::open(":memory:").unwrap();
        let report = output_receipt(&mut store, true);
        let plan = execution_plan(&report);
        if has_receipt {
            store.record_execution_receipt(&report).unwrap();
        } else {
            store
                .cancel(
                    plan.binding().task().id(),
                    &WorkerId::new("host").unwrap(),
                    "stop",
                    5,
                )
                .unwrap();
        }
        assert!(store.register_execution_plan(&plan).is_err());
        assert!(
            store
                .execution_plan(plan.binding().run_id(), plan.binding().task())
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn execution_plan_requires_registered_policy_candidate_and_exact_submission() {
    use graph_application::ExecutionPlanRepository;
    for case in 0..5 {
        let mut store = Store::open(":memory:").unwrap();
        let (task, candidate) = fixture_with_policy(&mut store, case != 0, case != 1);
        let request = binding(&store, &task, &candidate);
        let mut wire = graph_protocol::CheckRunBinding::from(&request);
        match case {
            2 => wire.submission_sequence += 1,
            3 => wire.origin_lease.owner = "other".into(),
            4 => wire.candidate.content_sha256 = "d".repeat(64),
            _ => {}
        }
        let plan = graph_domain::ExecutionPlan::new(
            wire.try_into_domain().unwrap(),
            "host".into(),
            task.project().clone(),
        )
        .unwrap();
        assert!(store.register_execution_plan(&plan).is_err(), "case {case}");
        assert!(
            store
                .execution_plan(plan.binding().run_id(), plan.binding().task())
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn execution_plan_wire_is_strict_and_checks_target_policy() {
    let mut store = Store::open(":memory:").unwrap();
    let report = output_receipt(&mut store, true);
    let plan = execution_plan(&report);
    let original = serde_json::to_value(graph_protocol::ExecutionPlan::from(&plan)).unwrap();
    let decoded: graph_protocol::ExecutionPlan = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(decoded.try_into_domain().unwrap(), plan);
    for field in original.as_object().unwrap().keys() {
        let mut wrong = original.clone();
        wrong.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<graph_protocol::ExecutionPlan>(wrong).is_err());
    }
    let mut wrong = original.clone();
    wrong["authorized"] = json!(true);
    assert!(serde_json::from_value::<graph_protocol::ExecutionPlan>(wrong).is_err());
    for field in ["repository_id", "config_hash", "ignore_policy_version"] {
        let mut wrong = original.clone();
        wrong["execution_snapshot"][field] = json!("other");
        assert!(
            serde_json::from_value::<graph_protocol::ExecutionPlan>(wrong)
                .unwrap()
                .try_into_domain()
                .is_err()
        );
    }
    let mut wrong = original;
    wrong["schema_version"] = json!(2);
    assert!(
        serde_json::from_value::<graph_protocol::ExecutionPlan>(wrong)
            .unwrap()
            .try_into_domain()
            .is_err()
    );
}

#[test]
fn execution_plan_rejects_orphan_candidate_analysis_registration() {
    use graph_application::ExecutionPlanRepository;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orphan.sqlite");
    let mut store = Store::open(&path).unwrap();
    let report = output_receipt(&mut store, true);
    let plan = execution_plan(&report);
    let corrupt = rusqlite::Connection::open(&path).unwrap();
    corrupt.pragma_update(None, "foreign_keys", false).unwrap();
    corrupt
        .execute_batch(include_str!(
            "sql/disable_analysis_delete_guard_fixture.sql"
        ))
        .unwrap();
    corrupt
        .execute(
            include_str!("sql/remove_candidate_analysis_fixture.sql"),
            [plan.binding().candidate().analysis_run()],
        )
        .unwrap();
    assert!(matches!(
        store.register_execution_plan(&plan),
        Err(graph_store::StoreError::Corrupt(_))
    ));
    assert!(
        store
            .execution_plan(plan.binding().run_id(), plan.binding().task())
            .unwrap()
            .is_none()
    );
}

#[test]
fn corrupt_execution_plan_blocks_read_replay_and_receipt_without_repair() {
    use graph_application::{ExecutionPlanRepository, ExecutionReceiptRepository};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt-plan.sqlite");
    let mut store = Store::open(&path).unwrap();
    let report = output_receipt(&mut store, true);
    let plan = execution_plan(&report);
    store.register_execution_plan(&plan).unwrap();
    let events = store.events(0, 100).unwrap();
    for field in 0..3 {
        let mut wire = graph_protocol::ExecutionPlan::from(&plan);
        match field {
            0 => wire.schema_version = 2,
            1 => wire.binding.run_id = "other".into(),
            _ => wire.binding.task.context_ref = "other".into(),
        }
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                include_str!("sql/corrupt_execution_plan.sql"),
                rusqlite::params![
                    plan.binding().run_id(),
                    serde_json::to_string(&wire).unwrap()
                ],
            )
            .unwrap();
        assert!(matches!(
            store.execution_plan(plan.binding().run_id(), plan.binding().task()),
            Err(graph_store::StoreError::Corrupt(_))
        ));
        assert!(matches!(
            store.register_execution_plan(&plan),
            Err(graph_store::StoreError::Corrupt(_))
        ));
        assert!(matches!(
            store.record_execution_receipt(&report),
            Err(graph_store::StoreError::Corrupt(_))
        ));
        assert!(
            store
                .execution_receipt(plan.binding().run_id(), plan.binding().task())
                .unwrap()
                .is_none()
        );
        assert_eq!(store.events(0, 100).unwrap(), events);
    }
}

#[test]
fn execution_plan_races_serialize_with_conflicting_receipt_and_cancellation() {
    use graph_application::{ExecutionPlanRepository, ExecutionReceiptRepository};
    for cancel in [false, true] {
        for _ in 0..8 {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("race-plan.sqlite");
            let mut store = Store::open(&path).unwrap();
            let report = output_receipt(&mut store, true);
            let plan = execution_plan(&report);
            let mut wire = graph_protocol::ExecutionReceipt::from(&report);
            wire.host_id = "other-host".into();
            let competing = wire.try_into_domain().unwrap();
            let mut contender = Store::open(&path).unwrap();
            let barrier = std::sync::Barrier::new(2);
            let (registered, other) = std::thread::scope(|scope| {
                let worker = scope.spawn(|| {
                    barrier.wait();
                    contender.register_execution_plan(&plan)
                });
                barrier.wait();
                let other = if cancel {
                    store.cancel(
                        plan.binding().task().id(),
                        &WorkerId::new("host").unwrap(),
                        "stop",
                        5,
                    )
                } else {
                    store.record_execution_receipt(&competing)
                };
                (worker.join().unwrap(), other)
            });
            drop(store);
            let store = Store::open(&path).unwrap();
            let persisted = store
                .execution_plan(plan.binding().run_id(), plan.binding().task())
                .unwrap();
            if cancel {
                assert!(matches!(other, Ok(true)));
                match registered {
                    Ok(true) => assert_eq!(persisted, Some(plan.clone())),
                    Err(graph_store::StoreError::Unavailable) => assert!(persisted.is_none()),
                    other => panic!("unexpected registration: {other:?}"),
                }
                assert!(
                    store
                        .submitted_candidate(plan.binding().task().id())
                        .unwrap()
                        .is_none()
                );
            } else {
                match (registered, other) {
                    (Ok(true), Err(graph_store::StoreError::Conflict)) => {
                        assert_eq!(persisted, Some(plan.clone()));
                        assert!(
                            store
                                .execution_receipt(plan.binding().run_id(), plan.binding().task())
                                .unwrap()
                                .is_none()
                        );
                    }
                    (Err(graph_store::StoreError::Conflict), Ok(true)) => {
                        assert!(persisted.is_none());
                        assert_eq!(
                            store
                                .execution_receipt(plan.binding().run_id(), plan.binding().task())
                                .unwrap(),
                            Some(competing)
                        );
                    }
                    other => panic!("non-serial outcome: {other:?}"),
                }
            }
        }
    }
}

#[test]
fn stored_execution_receipt_replays_after_restart_and_cancellation_without_authority() {
    use graph_application::ExecutionReceiptRepository;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("receipts.sqlite");
    let mut store = Store::open(&path).unwrap();
    let report = output_receipt(&mut store, true);
    store
        .cancel(
            report.binding().task().id(),
            &WorkerId::new("host").unwrap(),
            "stop",
            5,
        )
        .unwrap();
    let events = store.events(0, 100).unwrap();
    assert!(store.record_execution_receipt(&report).unwrap());
    assert!(!store.record_execution_receipt(&report).unwrap());
    assert_eq!(store.events(0, 100).unwrap(), events);
    drop(store);
    let mut store = Store::open(&path).unwrap();
    let actual = store
        .execution_receipt(report.binding().run_id(), report.binding().task())
        .unwrap()
        .unwrap();
    assert_eq!(actual, report);
    assert_eq!(
        actual.completion().reported_check_outcome(),
        graph_domain::CheckOutcome::Unknown
    );
    assert!(!store.record_execution_receipt(&report).unwrap());
    assert_eq!(store.events(0, 100).unwrap(), events);
    assert!(
        store
            .submitted_candidate(report.binding().task().id())
            .unwrap()
            .is_none()
    );
    for field in 0..4 {
        let mut changed = graph_protocol::ExecutionReceipt::from(&report);
        match field {
            0 => changed.host_id = "other-host".into(),
            1 => changed.elapsed_ms += 1,
            2 => changed.binding.command.program = "other-command".into(),
            _ => changed.completion.cleanup = graph_protocol::execution::ScopeCleanup::Complete,
        }
        assert!(matches!(
            store.record_execution_receipt(&changed.try_into_domain().unwrap()),
            Err(graph_store::StoreError::Conflict)
        ));
        assert_eq!(
            store
                .execution_receipt(report.binding().run_id(), report.binding().task())
                .unwrap(),
            Some(report.clone())
        );
    }
}

#[test]
fn receipt_contenders_have_one_durable_insertion_for_replay_and_conflict() {
    use graph_application::ExecutionReceiptRepository;
    use std::sync::{Arc, Barrier};
    for conflicting in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("race.sqlite");
        let mut store = Store::open(&path).unwrap();
        let report = output_receipt(&mut store, true);
        let events = store.events(0, 100).unwrap();
        drop(store);
        let barrier = Arc::new(Barrier::new(8));
        // Open before spawning, so a connection error cannot strand the barrier.
        let stores: Vec<_> = (0..8).map(|_| Store::open(&path).unwrap()).collect();
        let workers: Vec<_> = stores
            .into_iter()
            .enumerate()
            .map(|(index, mut store)| {
                let barrier = Arc::clone(&barrier);
                let mut wire = graph_protocol::ExecutionReceipt::from(&report);
                if conflicting {
                    wire.host_id = format!("host-{index}");
                }
                let claim = wire.try_into_domain().unwrap();
                std::thread::spawn(move || {
                    barrier.wait();
                    let result = store.record_execution_receipt(&claim);
                    (claim, result)
                })
            })
            .collect();
        let results: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        let winners: Vec<_> = results
            .iter()
            .filter(|(_, result)| matches!(result, Ok(true)))
            .collect();
        assert_eq!(winners.len(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|(_, result)| if conflicting {
                    matches!(result, Err(graph_store::StoreError::Conflict))
                } else {
                    matches!(result, Ok(false))
                })
                .count(),
            7
        );
        let mut store = Store::open(&path).unwrap();
        assert_eq!(
            store
                .execution_receipt(report.binding().run_id(), report.binding().task())
                .unwrap()
                .as_ref(),
            Some(&winners[0].0)
        );
        assert!(!store.record_execution_receipt(&winners[0].0).unwrap());
        assert_eq!(store.events(0, 100).unwrap(), events);
    }
}

#[test]
#[ignore = "subprocess-only fixture; exercised by receipt_process_exit_preserves_commit_boundary"]
fn receipt_transaction_crash_fixture() {
    let path = std::env::var_os("PGA_RECEIPT_CRASH_FIXTURE_DB").expect("fixture DB required");
    let raw = std::env::var("PGA_RECEIPT_CRASH_FIXTURE_JSON").expect("fixture receipt required");
    let wire: graph_protocol::ExecutionReceipt = serde_json::from_str(&raw).unwrap();
    let receipt = wire.try_into_domain().unwrap();
    let mut connection = rusqlite::Connection::open(path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection
        .pragma_update(None, "synchronous", "FULL")
        .unwrap();
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    tx.execute(
        include_str!("../src/sql/insert_execution_receipt.sql"),
        rusqlite::params![
            receipt.binding().run_id(),
            receipt.binding().task().id().as_str(),
            raw
        ],
    )
    .unwrap();
    if std::env::var("PGA_RECEIPT_CRASH_FIXTURE_COMMIT").unwrap() == "yes" {
        tx.commit().unwrap();
    }
    // Deliberately skip destructors: this is not rollback-on-Drop evidence.
    std::process::exit(23);
}

#[test]
fn receipt_process_exit_preserves_commit_boundary() {
    use graph_application::ExecutionReceiptRepository;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    for committed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crash.sqlite");
        let mut store = Store::open(&path).unwrap();
        let report = output_receipt(&mut store, true);
        let events = store.events(0, 100).unwrap();
        drop(store);
        let raw = serde_json::to_string(&graph_protocol::ExecutionReceipt::from(&report)).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "receipt_transaction_crash_fixture"])
            .env("PGA_RECEIPT_CRASH_FIXTURE_DB", &path)
            .env("PGA_RECEIPT_CRASH_FIXTURE_JSON", raw)
            .env(
                "PGA_RECEIPT_CRASH_FIXTURE_COMMIT",
                if committed { "yes" } else { "no" },
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > Duration::from_secs(10) {
                let _ = child.kill();
                child.wait().unwrap();
                panic!("receipt crash fixture exceeded deadline");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(
            status.code(),
            Some(23),
            "fixture must reach intended exit boundary"
        );
        let mut store = Store::open(&path).unwrap();
        assert_eq!(
            store
                .execution_receipt(report.binding().run_id(), report.binding().task())
                .unwrap(),
            if committed {
                Some(report.clone())
            } else {
                None
            }
        );
        assert_eq!(store.record_execution_receipt(&report).unwrap(), !committed);
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(
            store
                .execution_receipt(report.binding().run_id(), report.binding().task())
                .unwrap(),
            Some(report)
        );
    }
}

#[test]
fn stored_execution_receipt_requires_exact_task_and_filters_complete_scope() {
    use graph_application::ExecutionReceiptRepository;
    let mut store = Store::open(":memory:").unwrap();
    let report = output_receipt(&mut store, false);
    // Diagnostic storage intentionally does not claim output registration or byte verification.
    assert!(store.record_execution_receipt(&report).unwrap());
    for field in 0..7 {
        let mut wire = graph_protocol::TaskSpec::from(report.binding().task());
        match field {
            0 => wire.project.repository_id.push('x'),
            1 => wire.project.worktree_id.push('x'),
            2 => wire.project.git_head.push('x'),
            3 => wire.project.working_tree_fingerprint.push('x'),
            4 => wire.project.config_hash.push('x'),
            5 => wire.project.ignore_policy_version.push('x'),
            _ => wire.context_ref.push('x'),
        }
        let task = wire.try_into_domain().unwrap();
        assert!(
            store
                .execution_receipt(report.binding().run_id(), &task)
                .unwrap()
                .is_none()
        );
    }
    let mut altered = graph_protocol::ExecutionReceipt::from(&report);
    altered.binding.task.context_ref.push('x');
    assert!(matches!(
        store.record_execution_receipt(&altered.try_into_domain().unwrap()),
        Err(graph_store::StoreError::Conflict)
    ));
    let mut empty = Store::open(":memory:").unwrap();
    assert!(matches!(
        empty.record_execution_receipt(&report),
        Err(graph_store::StoreError::Unavailable)
    ));
    assert!(
        empty
            .execution_receipt(report.binding().run_id(), report.binding().task())
            .unwrap()
            .is_none()
    );
}

#[test]
fn stored_execution_receipt_corruption_is_not_overwritten_by_replay() {
    use graph_application::ExecutionReceiptRepository;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt.sqlite");
    let mut store = Store::open(&path).unwrap();
    let report = output_receipt(&mut store, true);
    store.record_execution_receipt(&report).unwrap();
    for field in 0..3 {
        let mut wire = graph_protocol::ExecutionReceipt::from(&report);
        match field {
            0 => wire.schema_version = 999,
            1 => wire.binding.run_id = "another-id".into(),
            _ => wire.binding.task.context_ref.push('x'),
        }
        let raw = serde_json::to_string(&wire).unwrap();
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                include_str!("sql/corrupt_execution_receipt.sql"),
                rusqlite::params![report.binding().run_id(), raw],
            )
            .unwrap();
        assert!(matches!(
            store.execution_receipt(report.binding().run_id(), report.binding().task()),
            Err(graph_store::StoreError::Corrupt(_))
        ));
        assert!(store.record_execution_receipt(&report).is_err());
        assert!(
            store
                .execution_receipt(report.binding().run_id(), report.binding().task())
                .is_err()
        );
    }
}

#[test]
fn receipt_output_expectations_budget_and_registration_fail_before_blob_open() {
    use graph_application::{ReceiptOutputError, verify_receipt_outputs};
    let mut store = Store::open(":memory:").unwrap();
    let report = output_receipt(&mut store, true);
    let unopened = Reader {
        on_open: || panic!("must not open"),
        bytes: b"",
    };
    for field in 0..3 {
        let mut expected = graph_protocol::CheckRunBinding::from(report.binding());
        let mut snapshot = report.execution_snapshot().clone();
        let host = if field == 0 { "other" } else { "host" };
        if field == 1 {
            expected.command.program = "other".into();
        }
        if field == 2 {
            snapshot.worktree_id = "other".into();
        }
        assert!(matches!(
            verify_receipt_outputs(
                &store,
                &unopened,
                &report,
                &expected.try_into_domain().unwrap(),
                host,
                &snapshot,
                6
            ),
            Err(ReceiptOutputError::ExpectationMismatch)
        ));
    }
    assert!(matches!(
        verify_receipt_outputs(
            &store,
            &unopened,
            &report,
            report.binding(),
            "host",
            report.execution_snapshot(),
            5
        ),
        Err(ReceiptOutputError::TooLarge)
    ));
    let mut missing = Store::open(":memory:").unwrap();
    let report = output_receipt(&mut missing, false);
    assert!(matches!(
        verify_receipt_outputs(
            &missing,
            &unopened,
            &report,
            report.binding(),
            "host",
            report.execution_snapshot(),
            6
        ),
        Err(ReceiptOutputError::OutputMismatch)
    ));
}

#[test]
fn receipt_output_read_cancellation_invalidates_observation() {
    use graph_application::{ReceiptOutputError, verify_receipt_outputs};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ledger.sqlite");
    let mut store = Store::open(&path).unwrap();
    let report = output_receipt(&mut store, true);
    let reader = Reader {
        on_open: || {
            Store::open(&path)
                .unwrap()
                .cancel(
                    report.binding().task().id(),
                    &WorkerId::new("host").unwrap(),
                    "stop",
                    5,
                )
                .unwrap();
        },
        bytes: b"abc",
    };
    assert!(matches!(
        verify_receipt_outputs(
            &store,
            &reader,
            &report,
            report.binding(),
            "host",
            report.execution_snapshot(),
            6
        ),
        Err(ReceiptOutputError::Changed)
    ));
}

#[test]
fn check_binding_reconciles_actual_ledger_without_mutation() {
    use graph_application::{CheckBindingError, reconcile_check_binding};
    let mut store = Store::open(":memory:").unwrap();
    let (expected, artifact) = fixture_with_policy(&mut store, true, true);
    let request = binding(&store, &expected, &artifact);
    let events = store.events(0, 100).unwrap();
    let observed = reconcile_check_binding(&store, &request).unwrap();
    assert_eq!(observed.submission_sequence, request.submission_sequence());
    assert_eq!(store.events(0, 100).unwrap(), events);
    for field in 0..5 {
        let mut wrong = graph_protocol::CheckRunBinding::from(&request);
        match field {
            0 => wrong.origin_lease.owner = "other".into(),
            1 => wrong.origin_lease.fencing_token += 1,
            2 => wrong.submission_sequence += 1,
            3 => wrong.candidate.id = "other".into(),
            _ => wrong.task.context_ref = "other".into(),
        }
        assert!(matches!(
            reconcile_check_binding(&store, &wrong.try_into_domain().unwrap()),
            Err(CheckBindingError::SubmissionMismatch)
        ));
    }
    let mut wrong = graph_protocol::CheckRunBinding::from(&request);
    wrong.policy.names.push("extra".into());
    assert!(matches!(
        reconcile_check_binding(&store, &wrong.try_into_domain().unwrap()),
        Err(CheckBindingError::PolicyMismatch)
    ));
    let mut wrong = graph_protocol::CheckRunBinding::from(&request);
    wrong.candidate.content_sha256 = "d".repeat(64);
    assert!(matches!(
        reconcile_check_binding(&store, &wrong.try_into_domain().unwrap()),
        Err(CheckBindingError::CandidateMismatch)
    ));
    assert_eq!(store.events(0, 100).unwrap(), events);
    store
        .cancel(expected.id(), &WorkerId::new("host").unwrap(), "stop", 3)
        .unwrap();
    assert!(matches!(
        reconcile_check_binding(&store, &request),
        Err(CheckBindingError::Unavailable)
    ));
}

#[test]
fn check_binding_reconciliation_rejects_missing_registrations() {
    use graph_application::{CheckBindingError, reconcile_check_binding};
    let mut no_policy = Store::open(":memory:").unwrap();
    let (task, artifact) = fixture(&mut no_policy, true);
    let request = binding(&no_policy, &task, &artifact);
    assert!(matches!(
        reconcile_check_binding(&no_policy, &request),
        Err(CheckBindingError::PolicyMismatch)
    ));
    let mut no_artifact = Store::open(":memory:").unwrap();
    let (task, artifact) = fixture_with_policy(&mut no_artifact, false, true);
    let request = binding(&no_artifact, &task, &artifact);
    assert!(matches!(
        reconcile_check_binding(&no_artifact, &request),
        Err(CheckBindingError::CandidateMismatch)
    ));
}

#[test]
fn check_policy_is_immutable_scoped_and_survives_restart() {
    let mut source = Store::open(":memory:").unwrap();
    let (expected, artifact) = fixture(&mut source, false);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("policies.db");
    let mut store = Store::open(&path).unwrap();
    let policy = RequiredChecks::new(vec!["test".into(), "lint".into()]).unwrap();
    assert!(store.register_check_policy(&expected, &policy, 0).is_err());
    store.enqueue(&expected, 0).unwrap();
    assert_eq!(store.check_policy(&expected).unwrap(), None);
    let before = store.events(0, 100).unwrap();
    assert!(store.register_check_policy(&expected, &policy, 0).unwrap());
    let registered = store.events(0, 100).unwrap();
    assert_eq!(&registered[..before.len()], before.as_slice());
    assert_eq!(registered.len(), before.len() + 1);
    let event = registered.last().unwrap();
    assert_eq!(event.kind, graph_domain::EventKind::CheckPolicyRegistered);
    assert_eq!(event.at_ms, 0);
    let payload: graph_protocol::RequiredChecks = serde_json::from_str(&event.payload).unwrap();
    assert_eq!(payload.try_into_domain().unwrap(), policy);
    assert!(
        store
            .register_check_policy(&task(&artifact, "wrong"), &policy, 0)
            .is_err()
    );
    assert_eq!(store.check_policy(&task(&artifact, "wrong")).unwrap(), None);
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.check_policy(&expected).unwrap(), Some(policy.clone()));
    assert_eq!(
        store.pending_events("policy-projector", 100).unwrap(),
        registered
    );
    store
        .acknowledge_event("policy-projector", registered[0].sequence)
        .unwrap();
    store
        .acknowledge_event("policy-projector", event.sequence)
        .unwrap();
    assert!(
        store
            .pending_events("policy-projector", 100)
            .unwrap()
            .is_empty()
    );
    let reordered = RequiredChecks::new(vec!["lint".into(), "test".into()]).unwrap();
    assert!(
        !store
            .register_check_policy(&expected, &reordered, 0)
            .unwrap()
    );
    assert!(!store.register_check_policy(&expected, &policy, 99).unwrap());
    assert_eq!(store.events(0, 100).unwrap(), registered);
    store
        .lease(expected.id(), &WorkerId::new("worker").unwrap(), 1, 100)
        .unwrap();
    assert!(!store.register_check_policy(&expected, &policy, 0).unwrap());
    let weaker = RequiredChecks::new(vec!["lint".into()]).unwrap();
    assert!(store.register_check_policy(&expected, &weaker, 0).is_err());
    assert_eq!(store.check_policy(&expected).unwrap(), Some(policy));
}

#[test]
fn no_policy_is_fabricated_or_first_registered_after_lease() {
    let mut source = Store::open(":memory:").unwrap();
    let (expected, _) = fixture(&mut source, false);
    let policy = RequiredChecks::new(vec!["test".into()]).unwrap();
    assert!(source.register_check_policy(&expected, &policy, 0).is_err());
    let mut store = Store::open(":memory:").unwrap();
    store.enqueue(&expected, 0).unwrap();
    store
        .lease(expected.id(), &WorkerId::new("worker").unwrap(), 1, 100)
        .unwrap();
    assert!(store.register_check_policy(&expected, &policy, 0).is_err());
    assert_eq!(store.check_policy(&expected).unwrap(), None);
}

#[test]
fn guarded_lease_requires_exact_policy_and_contract_before_fencing() {
    let mut source = Store::open(":memory:").unwrap();
    let (expected, artifact) = fixture(&mut source, false);
    let mut store = Store::open(":memory:").unwrap();
    let owner = WorkerId::new("worker").unwrap();
    let policy = RequiredChecks::new(vec!["test".into()]).unwrap();
    store.enqueue(&expected, 0).unwrap();
    let before = store.events(0, 100).unwrap();
    assert!(
        store
            .lease_with_policy(&expected, &policy, &owner, 1, 100)
            .is_err()
    );
    assert_eq!(store.events(0, 100).unwrap(), before);
    store.register_check_policy(&expected, &policy, 1).unwrap();
    let before = store.events(0, 100).unwrap();
    let different = RequiredChecks::new(vec!["lint".into()]).unwrap();
    assert!(
        store
            .lease_with_policy(&expected, &different, &owner, 2, 100)
            .is_err()
    );
    assert!(
        store
            .lease_with_policy(&task(&artifact, "wrong"), &policy, &owner, 2, 100)
            .is_err()
    );
    for (now, duration) in [(-1, 100), (2, 0), (2, -1), (i64::MAX, 1)] {
        assert!(
            store
                .lease_with_policy(&expected, &policy, &owner, now, duration)
                .is_err()
        );
    }
    assert_eq!(store.events(0, 100).unwrap(), before);
    let first = store
        .lease_with_policy(&expected, &policy, &owner, 2, 100)
        .unwrap();
    assert_eq!(first.fencing_token(), 1);
    assert!(
        store
            .lease_with_policy(&expected, &policy, &owner, 101, 100)
            .is_err()
    );
    let second = store
        .lease_with_policy(&expected, &policy, &owner, 102, 100)
        .unwrap();
    assert_eq!(second.fencing_token(), 2);
    assert!(store.submit(&first, artifact.id(), 103).is_err());
    store.cancel(expected.id(), &owner, "stop", 104).unwrap();
    assert!(
        store
            .lease_with_policy(&expected, &policy, &owner, 105, 100)
            .is_err()
    );
    assert!(store.submit(&second, artifact.id(), 106).is_err());
}

#[test]
fn verifies_content_without_mutating_submission_and_rejects_bad_bytes() {
    let mut store = Store::open(":memory:").unwrap();
    let (task, artifact) = fixture(&mut store, true);
    let before = store.events(0, 100).unwrap();
    let good = Reader {
        on_open: || {},
        bytes: b"abc",
    };
    let verified = verify_submitted_content(&store, &good, &task, 3).unwrap();
    assert_eq!(verified.content().artifact(), &artifact);
    assert_eq!(verified.candidate().spec, task);
    assert_eq!(store.events(0, 100).unwrap(), before);
    let bad = Reader {
        on_open: || {},
        bytes: b"abd",
    };
    assert!(matches!(
        verify_submitted_content(&store, &bad, &task, 3),
        Err(SubmissionContentError::Content(
            ArtifactVerificationError::HashMismatch
        ))
    ));
    let unopened = Reader {
        on_open: || panic!("must reject before open"),
        bytes: b"",
    };
    assert!(matches!(
        verify_submitted_content(&store, &unopened, &task, 2),
        Err(SubmissionContentError::Content(
            ArtifactVerificationError::TooLarge
        ))
    ));
}

#[test]
fn rejects_missing_artifact_and_different_task_contract_before_io() {
    let mut store = Store::open(":memory:").unwrap();
    let (expected, artifact) = fixture(&mut store, false);
    let unopened = Reader {
        on_open: || panic!("must reject before open"),
        bytes: b"",
    };
    assert!(matches!(
        verify_submitted_content(&store, &unopened, &expected, 3),
        Err(SubmissionContentError::ArtifactMissing)
    ));
    assert!(matches!(
        verify_submitted_content(&store, &unopened, &task(&artifact, "changed"), 3),
        Err(SubmissionContentError::ScopeMismatch)
    ));
}

#[test]
fn cancellation_during_read_invalidates_content_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut store = Store::open(&path).unwrap();
    let (task, _) = fixture(&mut store, true);
    let reader = Reader {
        on_open: || {
            let mut other = Store::open(&path).unwrap();
            other
                .cancel(task.id(), &WorkerId::new("coordinator").unwrap(), "stop", 3)
                .unwrap();
        },
        bytes: b"abc",
    };
    assert!(matches!(
        verify_submitted_content(&store, &reader, &task, 3),
        Err(SubmissionContentError::Changed)
    ));
    let unopened = Reader {
        on_open: || panic!("cancelled"),
        bytes: b"",
    };
    assert!(matches!(
        verify_submitted_content(&store, &unopened, &task, 3),
        Err(SubmissionContentError::Unavailable)
    ));
}

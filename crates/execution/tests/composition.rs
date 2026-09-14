#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use graph_application::{
    AnalysisRepository, CheckPolicyRepository, ExecutionLaunchRepository, ExecutionPlanRepository,
    ExecutionReceiptRepository, PolicyLeaseRepository, SubmissionQueryRepository, TaskRepository,
    environment_fingerprint, ingest_artifact, verify_receipt_outputs,
};
use graph_domain::{
    AnalysisRun, Artifact, ArtifactProtection, ArtifactRetention, CheckCommand, CheckOutcome,
    CheckRunBinding, ExecutionPlan, ExecutionReceipt, ProjectRef, RequiredChecks, TaskId, TaskSpec,
    WorkerId, execution::ScopeCleanup,
};
use graph_execution::{publish_fixture_outputs, run_trusted_fixture};
use graph_source::DirectoryArtifacts;
use graph_store::Store;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}
fn artifact(project: &ProjectRef, id: &str, run: &str, kind: &str, bytes: &[u8]) -> Artifact {
    Artifact::new(
        id.into(),
        project.clone(),
        "g".into(),
        run.into(),
        digest(bytes),
        bytes.len() as u64,
        kind.into(),
        ArtifactRetention::Evidence,
        ArtifactProtection::Unreviewed,
    )
    .unwrap()
}
fn analysis(project: &ProjectRef, id: &str) -> AnalysisRun {
    // These manifest/config hashes identify test constants, not a real repo scan.
    AnalysisRun::new(
        id.into(),
        project.clone(),
        "g".into(),
        "owned-fixture".into(),
        "1".into(),
        digest(b"fixture-config"),
        digest(b"fixture-manifest"),
    )
    .unwrap()
}

#[test]
fn owned_execution_publishes_durable_receipt_without_cleanup_authority() {
    exercise_receipt("binary");
}

#[test]
fn zero_exit_with_matching_git_target_cannot_grant_integration() {
    exercise_receipt("visibility");
}

#[test]
fn timed_out_receipt_preserves_stop_and_empty_blob_dedup_after_reopen() {
    exercise_receipt("sleep");
}

#[test]
fn truncated_receipt_preserves_retained_prefix_without_becoming_complete() {
    exercise_receipt("flood-stdout");
}

#[test]
fn spawn_failure_receipt_survives_reopen_without_regranting_claim() {
    exercise_receipt("spawn-failed");
}

fn exercise_receipt(mode: &str) {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("state.sqlite");
    let blobs_path = root.path().join("blobs");
    let work = root.path().join("work");
    std::fs::create_dir(&blobs_path).unwrap();
    std::fs::create_dir(&work).unwrap();
    let mut project = ProjectRef {
        repository_id: "owned-fixture".into(),
        worktree_id: "w".into(),
        git_head: "fixture".into(),
        working_tree_fingerprint: "fixture-v1".into(),
        config_hash: "c".into(),
        ignore_policy_version: "1".into(),
    };
    let target = if mode == "visibility" {
        for args in [
            vec!["init", "--quiet"],
            vec!["config", "user.name", "fixture"],
            vec!["config", "user.email", "fixture@example.invalid"],
            vec![
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "--quiet",
                "-m",
                "fixture",
            ],
        ] {
            assert!(
                std::process::Command::new("git")
                    .arg("-C")
                    .arg(&work)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        let authority = graph_source::GitSnapshotAuthority::new(
            &work,
            "owned-fixture".into(),
            "w".into(),
            "c".into(),
            "1".into(),
            4096,
        )
        .unwrap();
        project = authority.current_project().unwrap();
        Some(graph_source::GitTargetHeadVerifier::new(authority, "git-target/v1".into()).unwrap())
    } else {
        None
    };
    let mut store = Store::open(&db).unwrap();
    let blobs = DirectoryArtifacts::open(&blobs_path, project.clone(), "g".into()).unwrap();
    let candidate = artifact(
        &project,
        "candidate",
        "candidate-run",
        "patch",
        b"fixture-only",
    );
    store
        .record_analysis_run(&analysis(&project, "candidate-run"))
        .unwrap();
    ingest_artifact(
        &mut store,
        &blobs,
        &candidate,
        &mut b"fixture-only".as_slice(),
        64,
    )
    .unwrap();
    let task = TaskSpec::new(
        TaskId::new("task").unwrap(),
        project.clone(),
        "g".into(),
        "worker".into(),
        "native".into(),
        vec!["src".into()],
        vec![],
        "fixture-context".into(),
        vec!["patch".into()],
        100,
    )
    .unwrap();
    let policy = RequiredChecks::new(vec!["test".into()]).unwrap();
    store.enqueue(&task, 0).unwrap();
    store.register_check_policy(&task, &policy, 0).unwrap();
    let lease = store
        .lease_with_policy(&task, &policy, &WorkerId::new("worker").unwrap(), 1, 100)
        .unwrap();
    store.submit(&lease, candidate.id(), 2).unwrap();
    let submitted = store.submitted_candidate(task.id()).unwrap().unwrap();
    let mut exe = Path::new(env!("CARGO_BIN_EXE_graph-execution-fixture"))
        .canonicalize()
        .unwrap();
    if mode == "spawn-failed" {
        use std::os::unix::fs::PermissionsExt;
        let denied = root.path().join("non-executable-fixture");
        std::fs::copy(&exe, &denied).unwrap();
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o600)).unwrap();
        exe = denied;
    }
    let env = BTreeMap::new();
    let command = CheckCommand::new(
        exe.to_str().unwrap().into(),
        vec![mode.into()],
        ".".into(),
        digest(&std::fs::read(&exe).unwrap()),
        environment_fingerprint(&env).unwrap(),
        if mode == "sleep" { 100 } else { 5000 },
        1000,
        4096,
        4096,
    )
    .unwrap();
    let binding = CheckRunBinding::new(
        "check-run".into(),
        task.clone(),
        lease,
        submitted.submission_sequence,
        candidate,
        policy,
        "test".into(),
        command.clone(),
    )
    .unwrap();
    let plan = ExecutionPlan::new(binding.clone(), "fixture-host".into(), project.clone()).unwrap();
    assert!(store.register_execution_plan(&plan).unwrap());
    assert!(store.claim_execution_launch(&plan, 3).unwrap());
    let started = wall_ms();
    let run = run_trusted_fixture(&command, &exe, &work, &env, &AtomicBool::new(false)).unwrap();
    let finished = wall_ms();
    assert!(run.unreaped_child.is_none());
    use graph_domain::execution::{ChildCompletion, StopReason, StreamCompletion};
    if mode != "spawn-failed" {
        assert!(matches!(
            run.completion.child,
            ChildCompletion::Reaped { .. }
        ));
    }
    let expected_outcome = match mode {
        "visibility" => {
            assert_eq!(run.completion.reason, StopReason::Exited);
            assert_eq!(
                run.completion.child,
                ChildCompletion::Reaped { exit_code: Some(0) }
            );
            assert_eq!(run.completion.stdout, StreamCompletion::Complete);
            assert_eq!(run.completion.stderr, StreamCompletion::Complete);
            assert_eq!(run.completion.cleanup, ScopeCleanup::Unverifiable);
            CheckOutcome::Unknown
        }
        "binary" => {
            assert_eq!(run.completion.reason, StopReason::Exited);
            assert_eq!(
                run.completion.child,
                ChildCompletion::Reaped {
                    exit_code: Some(23)
                }
            );
            assert_eq!(run.completion.stdout, StreamCompletion::Complete);
            assert_eq!(run.completion.stderr, StreamCompletion::Complete);
            assert_eq!(run.stdout, b"\0\xffout\r\nno-final-newline");
            assert_eq!(run.stderr, b"\xfe\0err\n\r\x80");
            CheckOutcome::Unknown
        }
        "sleep" => {
            assert_eq!(run.completion.reason, StopReason::TimedOut);
            assert!(run.stdout.is_empty() && run.stderr.is_empty());
            CheckOutcome::TimedOut
        }
        "flood-stdout" => {
            assert_eq!(run.completion.reason, StopReason::OutputLimit);
            assert_eq!(run.completion.stdout, StreamCompletion::Truncated);
            assert_eq!(run.stdout, vec![0xa5; 4096]);
            assert!(run.stderr.is_empty());
            CheckOutcome::Unknown
        }
        "spawn-failed" => {
            assert_eq!(run.completion.reason, StopReason::SpawnFailed);
            assert_eq!(run.completion.child, ChildCompletion::NotSpawned);
            assert_eq!(run.completion.stdout, StreamCompletion::Incomplete);
            assert_eq!(run.completion.stderr, StreamCompletion::Incomplete);
            assert!(run.stdout.is_empty() && run.stderr.is_empty());
            CheckOutcome::Failed
        }
        _ => unreachable!(),
    };
    store
        .record_analysis_run(&analysis(&project, binding.run_id()))
        .unwrap();
    let out = artifact(&project, "out", binding.run_id(), "stdout", &run.stdout);
    let err = artifact(&project, "err", binding.run_id(), "stderr", &run.stderr);
    let publication = publish_fixture_outputs(&run, &out, &err, &mut store, &blobs, 8192).unwrap();
    assert!(publication.stdout.blob_inserted);
    assert_eq!(publication.stderr.blob_inserted, run.stdout != run.stderr);
    assert_eq!(publication.stdout.content.artifact(), &out);
    assert_eq!(publication.stderr.content.artifact(), &err);
    let receipt = ExecutionReceipt::new(
        binding.clone(),
        "fixture-host".into(),
        project.clone(),
        started,
        finished,
        run.elapsed_ms,
        Some(out.clone()),
        Some(err),
        run.completion,
    )
    .unwrap();
    assert!(store.record_execution_receipt(&receipt).unwrap());
    let before = store.events(0, 100).unwrap();
    if let Some(target) = &target {
        use graph_application::TargetHeadVerifier;
        assert_eq!(
            target.verify_target_head(&task).unwrap().observed_target(),
            &project
        );
        assert!(matches!(
            graph_application::verify_integration(
                &store,
                &blobs,
                target,
                &task,
                &[binding.run_id().into()],
                64,
                8192,
                "fixture/v1".into()
            ),
            Err(graph_application::IntegrationVerificationError::Decision(_))
        ));
        assert_eq!(store.events(0, 100).unwrap(), before);
    }
    let observed = verify_receipt_outputs(
        &store,
        &blobs,
        &receipt,
        &binding,
        "fixture-host",
        &project,
        8192,
    )
    .unwrap();
    assert_eq!(
        observed.receipt().completion().cleanup,
        if mode == "spawn-failed" {
            ScopeCleanup::Complete
        } else {
            ScopeCleanup::Unverifiable
        }
    );
    assert_eq!(
        observed.receipt().completion().reported_check_outcome(),
        expected_outcome
    );
    drop(blobs);
    drop(store);

    let mut store = Store::open(&db).unwrap();
    let blobs = DirectoryArtifacts::open(&blobs_path, project.clone(), "g".into()).unwrap();
    assert_eq!(
        store.execution_receipt(binding.run_id(), &task).unwrap(),
        Some(receipt.clone())
    );
    assert!(!store.claim_execution_launch(&plan, 4).unwrap());
    assert!(!store.record_execution_receipt(&receipt).unwrap());
    verify_receipt_outputs(
        &store,
        &blobs,
        &receipt,
        &binding,
        "fixture-host",
        &project,
        8192,
    )
    .unwrap();
    assert_eq!(store.events(0, 100).unwrap(), before);

    if let Some(target) = &target {
        assert!(matches!(
            graph_application::verify_integration(
                &store,
                &blobs,
                target,
                &task,
                &[binding.run_id().into()],
                64,
                8192,
                "fixture/v1".into()
            ),
            Err(graph_application::IntegrationVerificationError::Decision(_))
        ));
        assert_eq!(store.events(0, 100).unwrap(), before);
    }

    // Empty streams share a CAS blob; there is no same-length corruption of zero bytes.
    if out.byte_length() == 0 {
        assert_eq!(
            std::fs::read(blobs_path.join(out.content_sha256())).unwrap(),
            b""
        );
        return;
    }

    // Corrupt only this test's owned blob; a historical receipt cannot hide it.
    std::fs::write(
        blobs_path.join(out.content_sha256()),
        vec![0; out.byte_length() as usize],
    )
    .unwrap();
    assert!(matches!(
        verify_receipt_outputs(
            &store,
            &blobs,
            &receipt,
            &binding,
            "fixture-host",
            &project,
            8192
        ),
        Err(graph_application::ReceiptOutputError::Content(
            graph_application::ArtifactVerificationError::HashMismatch
        ))
    ));
    assert_eq!(
        store.execution_receipt(binding.run_id(), &task).unwrap(),
        Some(receipt)
    );
}

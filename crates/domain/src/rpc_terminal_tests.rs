use super::*;
use crate::{
    ArtifactProtection, ArtifactRetention, Lease, ProjectRef, RpcConnectionSpec, RpcLaunchSpec,
    RpcProcessSpec, TaskId, TaskSpec, WorkerId,
};

fn fixture() -> RpcTerminalReceipt {
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
    let lease = Lease::issue(task.id().clone(), WorkerId::new("worker").unwrap(), 1, 1000).unwrap();
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
            20,
            30,
        )
        .unwrap(),
        RpcConnectionSpec::new("epoch".into(), "client".into(), "1".into(), false, 2, 4096)
            .unwrap(),
    )
    .unwrap();
    let run = AnalysisRun::new(
        "output-run".into(),
        project,
        "graph".into(),
        "rpc-host".into(),
        "1".into(),
        "c".repeat(64),
        "d".repeat(64),
    )
    .unwrap();
    let stdout = artifact(&run, "out", "stdout", 20);
    let stderr = artifact(&run, "err", "stderr", 30);
    RpcTerminalReceipt::new(
        RpcSpawnObservation::new(launch, 100, RpcSpawnDisposition::Spawned, Some(7)).unwrap(),
        run,
        90,
        10,
        Some(stdout),
        Some(stderr),
        ExecutionCompletion {
            reason: StopReason::Exited,
            child: ChildCompletion::Reaped { exit_code: Some(0) },
            stdout: StreamCompletion::Complete,
            stderr: StreamCompletion::Complete,
            cleanup: ScopeCleanup::Unverifiable,
        },
        vec![UncertainRpc::new(2, "secret-method".into()).unwrap()],
        Some(RpcInputProgress::new(4096, 512, false).unwrap()),
    )
    .unwrap()
}
fn artifact(run: &AnalysisRun, id: &str, kind: &str, length: u64) -> Artifact {
    Artifact::new(
        id.into(),
        run.project().clone(),
        run.graph_version().into(),
        run.id().into(),
        "e".repeat(64),
        length,
        kind.into(),
        ArtifactRetention::Evidence,
        ArtifactProtection::Unreviewed,
    )
    .unwrap()
}
fn rebuild(r: RpcTerminalReceipt) -> Result<RpcTerminalReceipt, DomainError> {
    RpcTerminalReceipt::new(
        r.spawn,
        r.output_run,
        r.finished_at_ms,
        r.supervised_elapsed_ms,
        r.stdout,
        r.stderr,
        r.completion,
        r.pending,
        r.input_progress,
    )
}

#[test]
fn valid_receipt_keeps_uncertainty_even_after_zero_exit_and_redacts_debug() {
    let r = fixture();
    assert_eq!(rebuild(r.clone()).unwrap(), r);
    assert_eq!(r.spawn().process_id(), Some(7));
    assert_eq!(r.output_run().id(), "output-run");
    assert_eq!(r.finished_at_ms(), 90); // wall clock moved backwards
    assert_eq!(r.supervised_elapsed_ms(), 10);
    assert_eq!(r.stdout().unwrap().byte_length(), 20);
    assert_eq!(r.stderr().unwrap().byte_length(), 30);
    assert_eq!(r.pending()[0].id(), 2);
    assert_eq!(r.pending()[0].method(), "secret-method");
    assert_eq!(r.completion().reason, StopReason::Exited);
    assert_eq!(r.input_progress().unwrap().written(), 512);
    assert!(!r.input_progress().unwrap().failed());
    assert_eq!(format!("{r:?}"), "RpcTerminalReceipt { .. }");
    assert!(!format!("{:?}", r.pending()).contains("secret-method"));
}

#[test]
fn rejects_nonspawn_and_inconsistent_terminal_process_claims() {
    for disposition in [
        RpcSpawnDisposition::CancelledBeforeSpawn,
        RpcSpawnDisposition::ExpiredBeforeSpawn,
        RpcSpawnDisposition::SpawnFailed,
    ] {
        let mut r = fixture();
        r.spawn = RpcSpawnObservation::new(r.spawn.launch().clone(), 0, disposition, None).unwrap();
        assert!(rebuild(r).is_err());
    }
    for child in [ChildCompletion::NotSpawned, ChildCompletion::Unreaped] {
        let mut r = fixture();
        r.completion.child = child;
        assert!(rebuild(r).is_err());
    }
    let mut r = fixture();
    r.completion.reason = StopReason::SpawnFailed;
    assert!(rebuild(r).is_err());
    for reason in [
        StopReason::Cancelled,
        StopReason::TimedOut,
        StopReason::OutputLimit,
        StopReason::HostError,
    ] {
        let mut r = fixture();
        r.completion.reason = reason;
        r.completion.child = ChildCompletion::Unreaped;
        assert!(rebuild(r.clone()).is_ok());
        r.completion.cleanup = ScopeCleanup::Complete;
        assert!(rebuild(r).is_err());
    }
}

#[test]
fn timing_pending_and_input_bounds_do_not_silently_normalize() {
    for (time, elapsed, valid) in [
        (0, 0, true),
        (i64::MAX, i64::MAX as u64, true),
        (-1, 0, false),
        (0, u64::MAX, false),
    ] {
        let mut r = fixture();
        r.finished_at_ms = time;
        r.supervised_elapsed_ms = elapsed;
        assert_eq!(rebuild(r).is_ok(), valid);
    }
    for ids in [
        vec![],
        vec![1, i64::MAX],
        vec![2, 1],
        vec![1, 1],
        vec![1, 2, 3],
    ] {
        let valid = ids.is_empty() || ids == [1, i64::MAX];
        let mut r = fixture();
        r.pending = ids
            .into_iter()
            .map(|id| UncertainRpc::new(id, "method".into()).unwrap())
            .collect();
        assert_eq!(rebuild(r).is_ok(), valid);
    }
    for id in [0, -1, i64::MIN] {
        assert!(UncertainRpc::new(id, "m".into()).is_err());
    }
    for method in ["".into(), " \n".into(), "a".repeat(257), "é".repeat(129)] {
        assert!(UncertainRpc::new(1, method).is_err());
    }
    for method in ["m\n".into(), "a".repeat(256), "é".repeat(128)] {
        assert!(UncertainRpc::new(1, method).is_ok());
    }
    assert!(RpcInputProgress::new(1, 2, false).is_err());
    assert!(RpcInputProgress::new(u64::MAX, 0, false).is_err());
    for total in [0, 4096, 4097] {
        let mut r = fixture();
        r.input_progress = Some(RpcInputProgress::new(total, total, true).unwrap());
        assert_eq!(rebuild(r).is_ok(), total <= 4096);
    }
}

#[test]
fn output_run_and_artifact_linkage_cannot_cross_snapshots_or_streams() {
    for change in 0..7 {
        let mut r = fixture();
        let mut project = r.output_run.project().clone();
        match change {
            0 => project.repository_id = "other".into(),
            1 => project.worktree_id = "other".into(),
            2 => project.git_head = "other".into(),
            3 => project.working_tree_fingerprint = "other".into(),
            4 => project.config_hash = "other".into(),
            5 => project.ignore_policy_version = "other".into(),
            _ => (),
        }
        r.output_run = AnalysisRun::new(
            "output-run".into(),
            project,
            if change == 6 { "other" } else { "graph" }.into(),
            "host".into(),
            "1".into(),
            "a".repeat(64),
            "b".repeat(64),
        )
        .unwrap();
        assert!(rebuild(r).is_err());
    }
    for change in 0..6 {
        let mut r = fixture();
        let mut project = r.output_run.project().clone();
        if change == 0 {
            project.git_head = "other".into();
        }
        r.stdout = Some(
            Artifact::new(
                if change == 5 { "err" } else { "out" }.into(),
                project,
                if change == 1 { "other" } else { "graph" }.into(),
                if change == 2 { "other" } else { "output-run" }.into(),
                "e".repeat(64),
                if change == 4 { 21 } else { 20 },
                if change == 3 { "stderr" } else { "stdout" }.into(),
                ArtifactRetention::Evidence,
                ArtifactProtection::Unreviewed,
            )
            .unwrap(),
        );
        assert!(rebuild(r).is_err());
    }
    let mut r = fixture();
    r.stderr = Some(artifact(&r.output_run, "err", "stderr", 31));
    assert!(rebuild(r).is_err());
}

#[test]
fn missing_artifacts_are_allowed_only_for_noncomplete_streams() {
    for stdout in [true, false] {
        for state in [
            StreamCompletion::Complete,
            StreamCompletion::Truncated,
            StreamCompletion::ReadFailed,
            StreamCompletion::Incomplete,
        ] {
            let mut r = fixture();
            if stdout {
                r.stdout = None;
                r.completion.stdout = state;
            } else {
                r.stderr = None;
                r.completion.stderr = state;
            }
            assert_eq!(rebuild(r).is_ok(), state != StreamCompletion::Complete);
        }
    }
}

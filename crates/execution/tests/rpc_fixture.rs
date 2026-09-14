#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
#[path = "rpc_fixture/publication.rs"]
mod publication;
#[path = "support/sandbox_requirement.rs"]
mod sandbox_requirement;
use graph_application::{
    RpcLaunchRepository, RpcSpawnObservationRepository, RpcTerminalReceiptRepository,
    TaskRepository, environment_fingerprint,
};
use graph_domain::execution::{ChildCompletion, ScopeCleanup, StopReason, StreamCompletion};
use graph_domain::{
    ProjectRef, RpcConnectionSpec, RpcLaunchSpec, RpcProcessSpec, RpcSpawnDisposition,
    RpcSpawnObservation, TaskId, TaskSpec, WorkerId,
};
use graph_execution::{
    ConnectionSupervisor, InputEvent, IoEvent, RpcFixtureLaunchError, SandboxEgressPolicy,
    SandboxError, TrustedRpcFixtureHost,
};
use graph_protocol::correlation::RoutedMessage;
use graph_store::Store;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Guard(Option<ConnectionSupervisor>);
impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(mut supervisor) = self.0.take() {
            while !supervisor.poll(true, false).finished {
                std::thread::sleep(Duration::from_millis(1));
            }
            if let Ok(mut result) = supervisor.finish() {
                if let Some(mut child) = result.unreaped_child.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
    }
}
fn environment() -> BTreeMap<String, String> {
    BTreeMap::from([("FIXTURE_KEY".into(), "literal value".into())])
}
fn executable() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_graph-execution-fixture"))
        .canonicalize()
        .unwrap()
}
fn spec(store: &mut Store, executable: &Path) -> RpcLaunchSpec {
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let task = TaskSpec::new(
        TaskId::new("task").unwrap(),
        ProjectRef {
            repository_id: "fixture".into(),
            worktree_id: "owned".into(),
            git_head: "unverified".into(),
            working_tree_fingerprint: "fixture".into(),
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
    store.enqueue(&task, now).unwrap();
    let lease = store
        .lease(task.id(), &WorkerId::new("worker").unwrap(), now, 60_000)
        .unwrap();
    RpcLaunchSpec::new(
        "launch".into(),
        task.clone(),
        lease,
        "host".into(),
        "fixture-only".into(),
        task.project().clone(),
        RpcProcessSpec::new(
            executable.to_str().unwrap().into(),
            vec![
                "rpc-context".into(),
                "".into(),
                "a b".into(),
                "$(literal);*".into(),
                "tiếng Việt".into(),
            ],
            "cwd".into(),
            format!("{:x}", Sha256::digest(std::fs::read(executable).unwrap())),
            environment_fingerprint(&environment()).unwrap(),
            3000,
            1000,
            8192,
            8192,
        )
        .unwrap(),
        RpcConnectionSpec::new(
            "epoch".into(),
            "fixture-client".into(),
            "1.2".into(),
            true,
            2,
            4096,
        )
        .unwrap(),
    )
    .unwrap()
}
fn changed(
    spec: &RpcLaunchSpec,
    change: impl FnOnce(&mut graph_protocol::RpcLaunchSpec),
) -> RpcLaunchSpec {
    let mut wire = graph_protocol::RpcLaunchSpec::from(spec);
    change(&mut wire);
    wire.try_into_domain().unwrap()
}

struct BoundGuard(Option<graph_execution::LaunchedRpc>);
impl Drop for BoundGuard {
    fn drop(&mut self) {
        if let Some(launched) = self.0.take() {
            drop(Guard(Some(launched.into_supervisor())));
        }
    }
}

// Deliberately dishonest test adapter: write reports success, readback differs.
struct LyingRpcOutput;
impl graph_application::ArtifactReader for LyingRpcOutput {
    fn open_artifact(&self, _: &graph_domain::Artifact) -> std::io::Result<Box<dyn std::io::Read>> {
        Ok(Box::new(std::io::Cursor::new(vec![0xff])))
    }
}
impl graph_application::ArtifactWriter for LyingRpcOutput {
    fn write_artifact(
        &self,
        _: &graph_domain::Artifact,
        _: &mut dyn std::io::Read,
        _: u64,
    ) -> Result<bool, graph_application::ArtifactVerificationError> {
        Ok(true)
    }
}

#[test]
fn bound_launch_preserves_exact_spawn_through_premature_finish_exit_and_cancel() {
    for cancel in [false, true] {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("cwd")).unwrap();
        let mut store = Store::open(root.path().join("bound.db")).unwrap();
        let spec = spec(&mut store, &executable());
        let host = TrustedRpcFixtureHost::new(
            spec.clone(),
            executable(),
            root.path().into(),
            environment(),
        );
        let mut guard = BoundGuard(Some(
            host.launch_bound(&spec, &mut store, &AtomicBool::new(false))
                .unwrap(),
        ));
        let observation = guard.0.as_ref().unwrap().spawn_observation().clone();
        assert_eq!(observation.launch(), &spec);
        assert_eq!(
            store.rpc_spawn_observation(&spec).unwrap(),
            Some(observation.clone())
        );
        assert_eq!(
            guard.0.as_ref().unwrap().input_progress().unwrap().written,
            0
        );
        let launched = guard.0.take().unwrap();
        guard.0 = Some(match launched.finish() {
            Err(unfinished) => unfinished,
            Ok(finished) => {
                let (_, mut terminal) = finished.into_parts();
                if let Some(mut child) = terminal.unreaped_child.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                panic!("unpolled supervisor cannot finish");
            }
        });
        assert_eq!(guard.0.as_ref().unwrap().spawn_observation(), &observation);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut sent = false;
        loop {
            assert!(Instant::now() < deadline);
            let launched = guard.0.as_mut().unwrap();
            if !cancel && launched.core().is_ready() && !sent {
                launched.prepare_request("fixture/ping", None).unwrap();
                sent = true;
            }
            if launched.poll(cancel, !cancel).finished {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let finished = match guard.0.take().unwrap().finish() {
            Ok(finished) => finished,
            Err(unfinished) => {
                guard.0 = Some(unfinished);
                panic!("completed poll must finish");
            }
        };
        assert_eq!(finished.spawn_observation(), &observation);
        assert_eq!(
            finished.terminal().requests.epoch(),
            spec.connection().epoch()
        );
        let run = graph_domain::AnalysisRun::new(
            "rpc-output-run".into(),
            spec.execution_snapshot().clone(),
            spec.task().graph_version().into(),
            "fixture-host".into(),
            "1".into(),
            "a".repeat(64),
            "b".repeat(64),
        )
        .unwrap();
        let events_before = store.events(0, 100).unwrap();
        assert!(
            finished
                .prepare_receipt(run.clone(), "same".into(), "same".into(), 0, 16384)
                .is_err()
        );
        assert!(
            finished
                .prepare_receipt(run.clone(), "out".into(), "err".into(), -1, 16384)
                .is_err()
        );
        let mut wrong_run = graph_protocol::AnalysisRun::from(&run);
        wrong_run.project.git_head = "wrong".into();
        assert!(
            finished
                .prepare_receipt(
                    wrong_run.try_into_domain().unwrap(),
                    "out".into(),
                    "err".into(),
                    0,
                    16384
                )
                .is_err()
        );
        assert_eq!(store.events(0, 100).unwrap(), events_before);
        let prepared = finished
            .prepare_receipt(
                run,
                "out".into(),
                "err".into(),
                observation.observed_at_ms(),
                16384,
            )
            .unwrap();
        let receipt = prepared.receipt().clone();
        assert_eq!(receipt.spawn(), &observation);
        assert_eq!(receipt.completion(), &finished.terminal().completion);
        assert_eq!(
            receipt.stdout().unwrap().content_sha256(),
            format!("{:x}", Sha256::digest(&finished.terminal().output.stdout))
        );
        assert_eq!(
            receipt.stderr().unwrap().byte_length(),
            finished.terminal().output.stderr.len() as u64
        );
        assert_eq!(
            receipt.stdout().unwrap().declared_protection(),
            graph_domain::ArtifactProtection::Unreviewed
        );
        assert!(matches!(
            prepared.publish(&mut store, &LyingRpcOutput),
            Err(graph_execution::RpcPublicationError::Stdout(_))
        ));
        assert!(store.rpc_terminal_receipt(&spec).unwrap().is_none());
        assert_eq!(finished.spawn_observation(), &observation);
        let blobs_path = root.path().join("blobs");
        std::fs::create_dir(&blobs_path).unwrap();
        let blobs = graph_source::DirectoryArtifacts::open(
            &blobs_path,
            spec.execution_snapshot().clone(),
            spec.task().graph_version().into(),
        )
        .unwrap();
        let published = prepared.publish(&mut store, &blobs).unwrap();
        assert!(published.receipt_inserted);
        assert_eq!(
            published.stdout.content.artifact(),
            receipt.stdout().unwrap()
        );
        assert_eq!(
            published.stderr.content.artifact(),
            receipt.stderr().unwrap()
        );
        assert!(!published.stdout.metadata_inserted); // retained after failed readback
        let events = store.events(0, 100).unwrap();
        assert_eq!(events.len(), events_before.len() + 1);
        drop(store);
        store = Store::open(root.path().join("bound.db")).unwrap();
        assert_eq!(
            store.rpc_terminal_receipt(&spec).unwrap(),
            Some(receipt.clone())
        );
        let replay = prepared.publish(&mut store, &blobs).unwrap();
        assert!(
            !replay.receipt_inserted
                && !replay.stdout.blob_inserted
                && !replay.stderr.blob_inserted
        );
        assert_eq!(store.events(0, 100).unwrap(), events);
        let (_, mut terminal) = finished.into_parts();
        let unexpected_unreaped = terminal.unreaped_child.is_some();
        if let Some(mut child) = terminal.unreaped_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        assert!(!unexpected_unreaped);
        assert_eq!(
            terminal.completion.reason,
            if cancel {
                StopReason::Cancelled
            } else {
                StopReason::Exited
            }
        );
        assert_eq!(
            terminal.requests.unresolved().len(),
            if cancel { 1 } else { 0 }
        );
        assert!(
            terminal
                .requests
                .unresolved()
                .all(|(_, pending)| pending.uncertain())
        );
        assert_eq!(
            store.rpc_spawn_observation(&spec).unwrap(),
            Some(observation)
        );
        assert!(matches!(
            host.launch_bound(&spec, &mut store, &AtomicBool::new(false)),
            Err(RpcFixtureLaunchError::AlreadyClaimed)
        ));
    }
}

#[test]
fn host_launches_exact_context_and_reopen_does_not_spawn_again() {
    let root = tempfile::tempdir().unwrap();
    let cwd = root.path().join("cwd");
    std::fs::create_dir(&cwd).unwrap();
    let db = root.path().join("ledger.sqlite");
    let mut store = Store::open(&db).unwrap();
    let spec = spec(&mut store, &executable());
    let host = TrustedRpcFixtureHost::new(
        spec.clone(),
        executable(),
        root.path().into(),
        environment(),
    );
    let cancelled = AtomicBool::new(false);
    let mut guard = Guard(Some(host.launch(&spec, &mut store, &cancelled).unwrap()));
    assert_eq!(
        guard.0.as_ref().unwrap().input_progress().unwrap().written,
        0
    );
    let mut sent = false;
    let mut matched = 0;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline);
        let supervisor = guard.0.as_mut().unwrap();
        if supervisor.core().is_ready() && !sent {
            supervisor.prepare_request("fixture/ping", None).unwrap();
            sent = true;
        }
        let tick = supervisor.poll(false, true);
        if let Some(IoEvent::Message(InputEvent::Routed(RoutedMessage::Matched { .. }))) =
            tick.event
        {
            matched += 1;
        }
        if tick.finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let result = match guard.0.take().unwrap().finish() {
        Ok(result) => result,
        Err(supervisor) => {
            guard.0 = Some(supervisor);
            panic!("premature finish");
        }
    };
    assert_eq!(matched, 1);
    assert!(result.unreaped_child.is_none());
    assert_eq!(
        result.completion.child,
        ChildCompletion::Reaped { exit_code: Some(0) }
    );
    assert_eq!(result.completion.reason, StopReason::Exited);
    assert_eq!(result.completion.cleanup, ScopeCleanup::Unverifiable);
    assert_eq!(result.completion.stdout, StreamCompletion::Complete);
    assert_eq!(result.completion.stderr, StreamCompletion::Complete);
    assert!(result.io_error.is_none() && result.process_error.is_none());
    let context: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cwd.join("rpc-context.json")).unwrap()).unwrap();
    assert_eq!(context["argv"], serde_json::json!(spec.process().args()));
    assert_eq!(
        context["cwd"],
        cwd.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(context["environment"], serde_json::json!(environment()));
    assert_eq!(
        context["initialize"]["params"]["clientInfo"]["name"],
        "fixture-client"
    );
    assert_eq!(
        context["initialize"]["params"]["clientInfo"]["version"],
        "1.2"
    );
    assert_eq!(
        context["initialize"]["params"]["capabilities"]["experimentalApi"],
        true
    );
    let observation = store.rpc_spawn_observation(&spec).unwrap().unwrap();
    assert_eq!(observation.disposition(), RpcSpawnDisposition::Spawned);
    let pid: u32 = std::fs::read_to_string(cwd.join("rpc-starts"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(observation.process_id(), Some(pid));
    let events = store.events(0, 100).unwrap();
    drop(store);
    let mut store = Store::open(&db).unwrap();
    assert!(matches!(
        host.launch(&spec, &mut store, &cancelled),
        Err(RpcFixtureLaunchError::AlreadyClaimed)
    ));
    assert_eq!(
        store.rpc_spawn_observation(&spec).unwrap(),
        Some(observation)
    );
    assert_eq!(store.events(0, 100).unwrap(), events);
    assert_eq!(
        std::fs::read_to_string(cwd.join("rpc-starts"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn rejected_request_environment_setup_and_precancel_leave_ledger_untouched() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("cwd")).unwrap();
    let mut store = Store::open(root.path().join("ledger.sqlite")).unwrap();
    let spec = spec(&mut store, &executable());
    let before = store.events(0, 100).unwrap();
    let host = TrustedRpcFixtureHost::new(
        spec.clone(),
        executable(),
        root.path().into(),
        environment(),
    );
    let no = AtomicBool::new(false);
    let altered = changed(&spec, |wire| wire.process.args.push("extra".into()));
    assert!(matches!(
        host.launch(&altered, &mut store, &no),
        Err(RpcFixtureLaunchError::Rejected)
    ));
    assert!(matches!(
        host.launch(&spec, &mut store, &AtomicBool::new(true)),
        Err(RpcFixtureLaunchError::Cancelled { claimed: false })
    ));
    let wrong_env = TrustedRpcFixtureHost::new(
        spec.clone(),
        executable(),
        root.path().into(),
        BTreeMap::new(),
    );
    assert!(matches!(
        wrong_env.launch(&spec, &mut store, &no),
        Err(RpcFixtureLaunchError::Preflight(_))
    ));
    let invalid = changed(&spec, |wire| wire.connection.max_frame = 1);
    let host = TrustedRpcFixtureHost::new(
        invalid.clone(),
        executable(),
        root.path().into(),
        environment(),
    );
    assert!(matches!(
        host.launch(&invalid, &mut store, &no),
        Err(RpcFixtureLaunchError::Setup(_))
    ));
    assert_eq!(store.events(0, 100).unwrap(), before);
    assert!(store.rpc_launch(spec.id(), spec.task()).unwrap().is_none());
    assert!(!root.path().join("cwd/rpc-starts").exists());
}

#[test]
fn spawn_failure_consumes_claim_across_reopen() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("cwd")).unwrap();
    let owned = root.path().join("not-executable");
    std::fs::copy(executable(), &owned).unwrap();
    std::fs::set_permissions(&owned, std::fs::Permissions::from_mode(0o600)).unwrap();
    let db = root.path().join("ledger.sqlite");
    let mut store = Store::open(&db).unwrap();
    let spec = spec(&mut store, &owned);
    let host = TrustedRpcFixtureHost::new(spec.clone(), owned, root.path().into(), environment());
    let no = AtomicBool::new(false);
    match host.launch(&spec, &mut store, &no) {
        Err(RpcFixtureLaunchError::Spawn(error)) => {
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied)
        }
        _ => panic!("expected spawn permission failure"),
    }
    let observation = store.rpc_spawn_observation(&spec).unwrap().unwrap();
    assert_eq!(observation.disposition(), RpcSpawnDisposition::SpawnFailed);
    assert_eq!(observation.process_id(), None);
    let events = store.events(0, 100).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == graph_domain::EventKind::RpcLaunchClaimed)
            .count(),
        1
    );
    drop(store);
    let mut store = Store::open(&db).unwrap();
    assert!(matches!(
        host.launch(&spec, &mut store, &no),
        Err(RpcFixtureLaunchError::AlreadyClaimed)
    ));
    assert_eq!(store.events(0, 100).unwrap(), events);
    assert!(!root.path().join("cwd/rpc-starts").exists());
}

#[test]
fn wrong_hash_and_symlink_escape_fail_before_registration() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("cwd")).unwrap();
    let mut store = Store::open(root.path().join("ledger.sqlite")).unwrap();
    let spec = spec(&mut store, &executable());
    let before = store.events(0, 100).unwrap();
    let wrong_hash = changed(&spec, |wire| {
        wire.process.executable_sha256 = "0".repeat(64)
    });
    for requested in [&wrong_hash, &spec] {
        let host = TrustedRpcFixtureHost::new(
            requested.clone(),
            executable(),
            root.path().into(),
            environment(),
        );
        assert!(matches!(
            host.launch(requested, &mut store, &AtomicBool::new(false)),
            Err(RpcFixtureLaunchError::Preflight(_))
        ));
        assert!(store.rpc_launch(spec.id(), spec.task()).unwrap().is_none());
        assert_eq!(store.events(0, 100).unwrap(), before);
    }
    assert!(!outside.path().join("rpc-starts").exists());
}

// Deterministic cancellation at the durable-claim boundary, using the actual
// SQLite repository rather than timing a thread against OS spawn.
struct CancelAfterClaim<'a> {
    store: &'a mut Store,
    cancelled: &'a AtomicBool,
}
impl RpcLaunchRepository for CancelAfterClaim<'_> {
    type Error = graph_store::StoreError;
    fn register_rpc_launch(&mut self, spec: &RpcLaunchSpec, now: i64) -> Result<bool, Self::Error> {
        self.store.register_rpc_launch(spec, now)
    }
    fn rpc_launch(&self, id: &str, task: &TaskSpec) -> Result<Option<RpcLaunchSpec>, Self::Error> {
        self.store.rpc_launch(id, task)
    }
    fn claim_rpc_launch(&mut self, spec: &RpcLaunchSpec, now: i64) -> Result<bool, Self::Error> {
        let claimed = self.store.claim_rpc_launch(spec, now)?;
        if claimed {
            self.cancelled
                .store(true, std::sync::atomic::Ordering::Release);
        }
        Ok(claimed)
    }
}

impl RpcSpawnObservationRepository for CancelAfterClaim<'_> {
    type Error = graph_store::StoreError;
    fn record_rpc_spawn_observation(
        &mut self,
        observation: &RpcSpawnObservation,
    ) -> Result<bool, Self::Error> {
        self.store.record_rpc_spawn_observation(observation)
    }
    fn rpc_spawn_observation(
        &self,
        launch: &RpcLaunchSpec,
    ) -> Result<Option<RpcSpawnObservation>, Self::Error> {
        self.store.rpc_spawn_observation(launch)
    }
}

#[test]
fn cancellation_after_durable_claim_does_not_spawn_or_reset_claim() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("cwd")).unwrap();
    let db = root.path().join("ledger.sqlite");
    let mut store = Store::open(&db).unwrap();
    let spec = spec(&mut store, &executable());
    let host = TrustedRpcFixtureHost::new(
        spec.clone(),
        executable(),
        root.path().into(),
        environment(),
    );
    let cancelled = AtomicBool::new(false);
    let mut repository = CancelAfterClaim {
        store: &mut store,
        cancelled: &cancelled,
    };
    assert!(matches!(
        host.launch(&spec, &mut repository, &cancelled),
        Err(RpcFixtureLaunchError::Cancelled { claimed: true })
    ));
    let observation = store.rpc_spawn_observation(&spec).unwrap().unwrap();
    assert_eq!(
        observation.disposition(),
        RpcSpawnDisposition::CancelledBeforeSpawn
    );
    assert_eq!(observation.process_id(), None);
    let events = store.events(0, 100).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind == graph_domain::EventKind::RpcLaunchClaimed)
            .count(),
        1
    );
    drop(store);
    let mut store = Store::open(&db).unwrap();
    assert!(matches!(
        host.launch(&spec, &mut store, &AtomicBool::new(false)),
        Err(RpcFixtureLaunchError::AlreadyClaimed)
    ));
    assert_eq!(store.events(0, 100).unwrap(), events);
    assert!(!root.path().join("cwd/rpc-starts").exists());
}

struct FailingRecorder<'a> {
    store: &'a mut Store,
    commit_first: bool,
}
impl RpcLaunchRepository for FailingRecorder<'_> {
    type Error = graph_store::StoreError;
    fn register_rpc_launch(
        &mut self,
        launch: &RpcLaunchSpec,
        now: i64,
    ) -> Result<bool, Self::Error> {
        self.store.register_rpc_launch(launch, now)
    }
    fn rpc_launch(&self, id: &str, task: &TaskSpec) -> Result<Option<RpcLaunchSpec>, Self::Error> {
        self.store.rpc_launch(id, task)
    }
    fn claim_rpc_launch(&mut self, launch: &RpcLaunchSpec, now: i64) -> Result<bool, Self::Error> {
        self.store.claim_rpc_launch(launch, now)
    }
}
impl RpcSpawnObservationRepository for FailingRecorder<'_> {
    type Error = graph_store::StoreError;
    fn record_rpc_spawn_observation(
        &mut self,
        observation: &RpcSpawnObservation,
    ) -> Result<bool, Self::Error> {
        if self.commit_first {
            self.store.record_rpc_spawn_observation(observation)?;
        }
        Err(graph_store::StoreError::Invalid(
            "injected recording failure",
        ))
    }
    fn rpc_spawn_observation(
        &self,
        launch: &RpcLaunchSpec,
    ) -> Result<Option<RpcSpawnObservation>, Self::Error> {
        self.store.rpc_spawn_observation(launch)
    }
}
struct ReturnedChild(std::process::Child);
impl Drop for ReturnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn recording_error_returns_owned_child_and_never_retries_spawn() {
    for commit_first in [false, true] {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("cwd")).unwrap();
        let db = root.path().join("ledger.sqlite");
        let mut store = Store::open(&db).unwrap();
        let spec = spec(&mut store, &executable());
        let host = TrustedRpcFixtureHost::new(
            spec.clone(),
            executable(),
            root.path().into(),
            environment(),
        );
        let result = host.launch(
            &spec,
            &mut FailingRecorder {
                store: &mut store,
                commit_first,
            },
            &AtomicBool::new(false),
        );
        let (failure, child, spawn_error) = match result {
            Err(RpcFixtureLaunchError::Observation {
                failure,
                child: Some(child),
                spawn_error,
            }) => (failure, ReturnedChild(child), spawn_error),
            Ok(supervisor) => {
                let _guard = Guard(Some(supervisor));
                panic!("recording must fail");
            }
            Err(error) => panic!("wrong failure {error:?}"),
        };
        let mut child = child;
        assert!(spawn_error.is_none());
        assert_eq!(failure.disposition, RpcSpawnDisposition::Spawned);
        assert!(matches!(
            failure.error,
            graph_execution::RpcSpawnRecordError::Ledger(_)
        ));
        let observation = failure.observation.unwrap();
        assert_eq!(observation.process_id(), Some(child.0.id()));
        assert!(child.0.stdin.is_some() && child.0.stdout.is_some() && child.0.stderr.is_some());
        assert!(child.0.try_wait().unwrap().is_none());
        assert!(!root.path().join("cwd/rpc-context.json").exists()); // no initialize sent
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        let events = store.events(0, 100).unwrap();
        drop(store);
        let mut store = Store::open(&db).unwrap();
        assert_eq!(
            store.rpc_spawn_observation(&spec).unwrap(),
            commit_first.then_some(observation)
        );
        assert!(matches!(
            host.launch(&spec, &mut store, &AtomicBool::new(false)),
            Err(RpcFixtureLaunchError::AlreadyClaimed)
        ));
        assert_eq!(store.events(0, 100).unwrap(), events);
    }
}

#[test]
fn recording_error_after_spawn_failure_keeps_original_os_error() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("cwd")).unwrap();
    let owned = root.path().join("not-executable");
    std::fs::copy(executable(), &owned).unwrap();
    std::fs::set_permissions(&owned, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut store = Store::open(root.path().join("ledger.sqlite")).unwrap();
    let spec = spec(&mut store, &owned);
    let host = TrustedRpcFixtureHost::new(spec.clone(), owned, root.path().into(), environment());
    match host.launch(
        &spec,
        &mut FailingRecorder {
            store: &mut store,
            commit_first: false,
        },
        &AtomicBool::new(false),
    ) {
        Err(RpcFixtureLaunchError::Observation {
            failure,
            child: None,
            spawn_error: Some(error),
        }) => {
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
            assert_eq!(failure.disposition, RpcSpawnDisposition::SpawnFailed);
            assert_eq!(failure.observation.unwrap().process_id(), None);
        }
        Ok(supervisor) => {
            let _guard = Guard(Some(supervisor));
            panic!("unexpected spawn");
        }
        Err(error) => panic!("wrong failure {error:?}"),
    }
    assert!(store.rpc_spawn_observation(&spec).unwrap().is_none());
    assert!(matches!(
        host.launch(&spec, &mut store, &AtomicBool::new(false)),
        Err(RpcFixtureLaunchError::AlreadyClaimed)
    ));
    assert!(!root.path().join("cwd/rpc-starts").exists());
}

#[test]
fn sandboxed_rpc_launch_uses_same_supervisor_and_declared_mount() {
    let Some(runtime) = sandbox_requirement::runtime_or_skip() else {
        return;
    };
    let root = tempfile::tempdir().unwrap();
    let cwd = root.path().join("cwd");
    std::fs::create_dir(&cwd).unwrap();
    let copied = root.path().join("fixture-bin");
    std::fs::copy(executable(), &copied).unwrap();
    std::fs::set_permissions(&copied, std::fs::Permissions::from_mode(0o755)).unwrap();
    let copied = copied.canonicalize().unwrap();
    let mut store = Store::open(root.path().join("sandbox.sqlite")).unwrap();
    let spec = spec(&mut store, &copied);
    let host = TrustedRpcFixtureHost::new_sandboxed(
        spec.clone(),
        copied,
        root.path().into(),
        environment(),
        runtime,
        SandboxEgressPolicy::DenyAll,
    );
    let mut launched = match host.launch_bound(&spec, &mut store, &AtomicBool::new(false)) {
        Ok(launched) => launched,
        Err(RpcFixtureLaunchError::Preflight(graph_execution::FixtureError::Sandbox(
            SandboxError::Unsupported,
        ))) => {
            eprintln!("sandbox RPC integration unavailable after probe");
            return;
        }
        Err(error) => panic!("sandbox RPC launch failed: {error:?}"),
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut sent = false;
    let mut matched = 0;
    loop {
        assert!(Instant::now() < deadline);
        if launched.core().is_ready() && !sent {
            launched.prepare_request("fixture/ping", None).unwrap();
            sent = true;
        }
        let tick = launched.poll(false, true);
        if let Some(IoEvent::Message(InputEvent::Routed(RoutedMessage::Matched { .. }))) =
            tick.event
        {
            matched += 1;
        }
        if tick.finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let result = match launched.finish() {
        Ok(result) => result,
        Err(unfinished) => {
            let _guard = BoundGuard(Some(unfinished));
            panic!("completed sandbox poll must finish");
        }
    };
    assert_eq!(matched, 1);
    assert_eq!(
        result.terminal().completion.child,
        ChildCompletion::Reaped { exit_code: Some(0) }
    );
    assert_eq!(result.terminal().completion.reason, StopReason::Exited);
    assert_eq!(
        result.terminal().completion.stdout,
        StreamCompletion::Complete
    );
    assert_eq!(
        result.terminal().completion.stderr,
        StreamCompletion::Complete
    );
    assert!(result.terminal().unreaped_child.is_none());
    assert!(result.terminal().io_error.is_none() && result.terminal().process_error.is_none());
    let context: serde_json::Value =
        serde_json::from_slice(&std::fs::read(cwd.join("rpc-context.json")).unwrap()).unwrap();
    assert_eq!(context["argv"], serde_json::json!(spec.process().args()));
    assert_eq!(context["cwd"], "/mnt/cwd");
    let child_environment = context["environment"].as_object().unwrap();
    assert_eq!(child_environment["FIXTURE_KEY"], "literal value");
    assert_eq!(child_environment["PWD"], "/mnt/cwd");
    assert_eq!(child_environment.len(), 2);
    assert_eq!(
        store
            .rpc_spawn_observation(&spec)
            .unwrap()
            .unwrap()
            .process_id(),
        result.spawn_observation().process_id()
    );
    assert!(result.spawn_observation().process_id().is_some());
}

#[test]
fn sandboxed_rpc_allow_list_fails_before_register_or_spawn() {
    let Some(runtime) = sandbox_requirement::runtime_or_skip() else {
        return;
    };
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("cwd")).unwrap();
    let copied = root.path().join("fixture-bin");
    std::fs::copy(executable(), &copied).unwrap();
    std::fs::set_permissions(&copied, std::fs::Permissions::from_mode(0o755)).unwrap();
    let copied = copied.canonicalize().unwrap();
    let mut store = Store::open(root.path().join("sandbox.sqlite")).unwrap();
    let spec = spec(&mut store, &copied);
    let before = store.events(0, 100).unwrap();
    let host = TrustedRpcFixtureHost::new_sandboxed(
        spec.clone(),
        copied,
        root.path().into(),
        environment(),
        runtime,
        SandboxEgressPolicy::AllowList(vec!["example.com".into()]),
    );
    assert!(matches!(
        host.launch(&spec, &mut store, &AtomicBool::new(false)),
        Err(RpcFixtureLaunchError::Preflight(
            graph_execution::FixtureError::Sandbox(SandboxError::UnverifiableEgress)
        ))
    ));
    assert_eq!(store.events(0, 100).unwrap(), before);
    assert!(store.rpc_launch(spec.id(), spec.task()).unwrap().is_none());
    assert!(!root.path().join("cwd/rpc-starts").exists());
}

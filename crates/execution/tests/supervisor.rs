#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use graph_domain::execution::{ChildCompletion, ScopeCleanup, StopReason, StreamCompletion};
use graph_execution::{
    ConnectionSetup, ConnectionSupervisor, InputEvent, IoEvent, OutputLimits, SupervisedConnection,
    SupervisionLimits,
};
use graph_protocol::correlation::RoutedMessage;
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
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
fn spawn(mode: &str, root: &std::path::Path, timeout: u64) -> Guard {
    spawn_with_frame(mode, root, timeout, 4096)
}
fn spawn_with_frame(mode: &str, root: &std::path::Path, timeout: u64, frame: usize) -> Guard {
    let setup = ConnectionSetup::new(
        "epoch",
        2,
        frame,
        "client",
        "1",
        false,
        OutputLimits::new(8192, 512 * 1024).unwrap(),
        SupervisionLimits::new(timeout, 1000).unwrap(),
    )
    .unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_graph-execution-fixture"))
        .arg(mode)
        .env_clear()
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    match setup.attach(child) {
        Ok(supervisor) => {
            let guard = Guard(Some(supervisor));
            let supervisor = guard.0.as_ref().unwrap();
            let progress = supervisor.input_progress().unwrap();
            assert_eq!(progress.written, 0);
            assert!(progress.total > 0 && !progress.failed);
            assert!(!supervisor.core().is_ready());
            assert_eq!(supervisor.core().unresolved().count(), 1);
            guard
        }
        Err(mut failure) => {
            let _ = failure.child.kill();
            let _ = failure.child.wait();
            panic!("fixture attachment failed: {}", failure.error);
        }
    }
}
fn finish(guard: &mut Guard) -> SupervisedConnection {
    let supervisor = guard.0.take().unwrap();
    match supervisor.finish() {
        Ok(result) => result,
        Err(supervisor) => {
            guard.0 = Some(supervisor);
            panic!("premature finish")
        }
    }
}
fn assert_reaped(result: &SupervisedConnection) {
    assert!(result.unreaped_child.is_none());
    assert!(matches!(
        result.completion.child,
        ChildCompletion::Reaped { .. }
    ));
    assert_eq!(result.completion.cleanup, ScopeCleanup::Unverifiable);
}

#[test]
fn supervisor_runs_rpc_drains_both_streams_and_reaps() {
    let root = tempfile::tempdir().unwrap();
    let mut guard = spawn("rpc-stderr", root.path(), 3000);
    let mut sent = false;
    let mut matches = 0;
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
            matches += 1;
        }
        if tick.finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let result = finish(&mut guard);
    assert_reaped(&result);
    assert_eq!(matches, 1);
    assert_eq!(result.completion.reason, StopReason::Exited);
    assert_eq!(
        result.completion.child,
        ChildCompletion::Reaped { exit_code: Some(0) }
    );
    assert_eq!(result.completion.stdout, StreamCompletion::Complete);
    assert_eq!(result.completion.stderr, StreamCompletion::Complete);
    assert_eq!(result.output.stderr, vec![0xa5; 512 * 1024]);
    assert!(result.io_error.is_none() && result.process_error.is_none());
    let completion = result.completion;
    let elapsed = result.elapsed_ms;
    let progress = result.connection.progress();
    let stdout = result.output.stdout.clone();
    let terminal = result.into_terminal();
    assert_eq!(terminal.requests.epoch(), "epoch");
    assert_eq!(terminal.requests.unresolved().len(), 0);
    assert_eq!(terminal.input_progress, progress);
    assert_eq!(terminal.completion, completion);
    assert_eq!(terminal.elapsed_ms, elapsed);
    assert_eq!(terminal.output.stdout, stdout);
    assert_eq!(terminal.output.stderr, vec![0xa5; 512 * 1024]);
    assert!(terminal.unreaped_child.is_none());
    assert!(terminal.io_error.is_none() && terminal.process_error.is_none());
}

#[test]
fn timeout_and_cancel_progress_without_an_event_consumer() {
    for cancelled in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut guard = spawn(
            if cancelled { "rpc-stderr" } else { "sleep" },
            root.path(),
            100,
        );
        let start = Instant::now();
        loop {
            assert!(start.elapsed() < Duration::from_secs(3));
            let tick = guard.0.as_mut().unwrap().poll(
                cancelled && start.elapsed() >= Duration::from_millis(30),
                false,
            );
            assert!(tick.event.is_none());
            if tick.finished {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let result = finish(&mut guard);
        assert_reaped(&result);
        assert_eq!(
            result.completion.reason,
            if cancelled {
                StopReason::Cancelled
            } else {
                StopReason::TimedOut
            }
        );
        assert!(
            result
                .connection
                .core()
                .unresolved()
                .all(|(_, request)| request.uncertain())
        );
        assert_eq!(result.connection.core().unresolved().count(), 1);
        assert!(result.output.stdout.len() <= 8192 && result.output.stderr.len() <= 512 * 1024);
        let completion = result.completion;
        let progress = result.connection.progress();
        let terminal = result.into_terminal();
        assert_eq!(terminal.completion, completion);
        assert_eq!(terminal.input_progress, progress);
        assert_eq!(terminal.requests.epoch(), "epoch");
        let pending: Vec<_> = terminal.requests.unresolved().collect();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, 1);
        assert_eq!(pending[0].1.method(), "initialize");
        assert!(pending[0].1.uncertain());
    }
}

#[test]
fn protocol_error_stops_admission_and_reaps_without_losing_pending() {
    let root = tempfile::tempdir().unwrap();
    let mut guard = spawn("rpc-malformed", root.path(), 1000);
    let start = Instant::now();
    loop {
        assert!(start.elapsed() < Duration::from_secs(3));
        if guard.0.as_mut().unwrap().poll(false, true).finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        guard
            .0
            .as_mut()
            .unwrap()
            .prepare_request("fixture/ping", None)
            .is_err()
    );
    let result = finish(&mut guard);
    assert_reaped(&result);
    assert_eq!(result.completion.reason, StopReason::HostError);
    assert!(result.io_error.is_some());
    assert_eq!(result.output.stdout, b"{\n");
    assert_eq!(result.connection.core().unresolved().count(), 1);
    let terminal = result.into_terminal();
    assert!(terminal.io_error.is_some());
    assert_eq!(terminal.completion.reason, StopReason::HostError);
    assert_eq!(terminal.output.stdout, b"{\n");
    assert_eq!(terminal.requests.unresolved().len(), 1);
    assert!(terminal.requests.unresolved().all(|(_, p)| p.uncertain()));
}

#[test]
fn cancel_blocked_rpc_preserves_partial_delivery_and_pending_identity() {
    let root = tempfile::tempdir().unwrap();
    let mut guard = spawn_with_frame("rpc-no-read", root.path(), 4000, 1024 * 1024);
    let start = Instant::now();
    loop {
        assert!(start.elapsed() < Duration::from_secs(3));
        let supervisor = guard.0.as_mut().unwrap();
        assert!(!supervisor.poll(false, true).finished);
        if supervisor.core().is_ready() && root.path().join("input-paused").exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let supervisor = guard.0.as_mut().unwrap();
    let id = supervisor
        .prepare_request(
            "fixture/ping",
            Some(serde_json::json!({"payload": "x".repeat(512 * 1024)})),
        )
        .unwrap();
    assert_eq!(id, graph_protocol::rpc::RequestId::Integer(2));
    for _ in 0..256 {
        let tick = supervisor.poll(false, false);
        assert!(!tick.finished && tick.event.is_none());
    }
    let partial = supervisor.input_progress().unwrap();
    assert!(partial.written > 0 && partial.written < partial.total);
    assert!(!partial.failed);
    for _ in 0..32 {
        assert!(!supervisor.poll(false, false).finished);
        assert_eq!(supervisor.input_progress(), Some(partial));
    }
    let cancelled_at = Instant::now();
    loop {
        assert!(cancelled_at.elapsed() < Duration::from_millis(1500));
        let tick = supervisor.poll(true, false);
        assert!(tick.event.is_none());
        assert_eq!(supervisor.input_progress(), Some(partial));
        if tick.finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(supervisor.prepare_request("fixture/ping", None).is_err());
    let result = finish(&mut guard);
    assert_reaped(&result);
    assert_eq!(result.completion.reason, StopReason::Cancelled);
    assert_eq!(result.completion.stdout, StreamCompletion::Complete);
    assert_eq!(result.completion.stderr, StreamCompletion::Complete);
    assert!(result.io_error.is_none() && result.process_error.is_none());
    assert_eq!(result.connection.progress(), Some(partial));
    let unresolved: Vec<_> = result.connection.core().unresolved().collect();
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].0, 2);
    assert_eq!(unresolved[0].1.method(), "fixture/ping");
    assert!(unresolved[0].1.uncertain());
    let terminal = result.into_terminal();
    assert_eq!(terminal.input_progress, Some(partial));
    assert_eq!(terminal.completion.reason, StopReason::Cancelled);
    assert!(terminal.unreaped_child.is_none());
    let unresolved: Vec<_> = terminal.requests.unresolved().collect();
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].0, 2);
    assert_eq!(unresolved[0].1.method(), "fixture/ping");
    assert!(unresolved[0].1.uncertain());
}

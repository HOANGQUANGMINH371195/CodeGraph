#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use graph_execution::{ConnectionInput, ConnectionInputError, InputEvent};
use graph_protocol::{correlation::RoutedMessage, rpc::RequestId};
use std::{
    io::{self, Read},
    process::{Child, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};
struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn spawn(mode: &str) -> (Owned, ConnectionInput, ChildStdout) {
    let mut child = Owned(
        Command::new(env!("CARGO_BIN_EXE_graph-execution-fixture"))
            .arg(mode)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let connection = ConnectionInput::begin(
        child.0.stdin.take().unwrap(),
        "epoch",
        2,
        4096,
        "fixture-client",
        "1",
        false,
    )
    .unwrap();
    let stdout = child.0.stdout.take().unwrap();
    let flags = rustix::fs::fcntl_getfl(&stdout).unwrap();
    rustix::fs::fcntl_setfl(&stdout, flags | rustix::fs::OFlags::NONBLOCK).unwrap();
    (child, connection, stdout)
}
fn event(connection: &mut ConnectionInput, stdout: &mut ChildStdout) -> InputEvent {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        assert!(Instant::now() < deadline, "peer event timed out");
        let mut byte = [0];
        match stdout.read(&mut byte) {
            Ok(1) => {
                let (used, event) = connection.receive("epoch", &byte).unwrap();
                assert_eq!(used, 1);
                if let Some(event) = event {
                    return event;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1))
            }
            other => panic!("unexpected peer read: {other:?}"),
        }
    }
}
fn ready(connection: &mut ConnectionInput, stdout: &mut ChildStdout) {
    assert!(matches!(
        connection.prepare_request("fixture/ping", None),
        Err(ConnectionInputError::Busy)
    ));
    assert_eq!(connection.core().unresolved().count(), 1);
    let _ = connection.pump().unwrap();
    assert!(matches!(
        event(connection, stdout),
        InputEvent::AcknowledgementQueued
    ));
    assert!(!connection.core().is_ready());
    let _ = connection.pump().unwrap();
    assert!(connection.core().is_ready());
}

#[test]
fn pipe_handshake_confirms_ack_only_after_write_then_correlates_response() {
    let (_child, mut connection, mut stdout) = spawn("rpc-peer");
    ready(&mut connection, &mut stdout);
    let id = connection
        .prepare_request("fixture/ping", Some(serde_json::json!({"value":1})))
        .unwrap();
    assert_eq!(id, RequestId::Integer(2));
    assert!(matches!(
        connection.prepare_request("fixture/ping", None),
        Err(ConnectionInputError::Busy)
    ));
    assert_eq!(connection.core().unresolved().count(), 1);
    let _ = connection.pump().unwrap();
    let InputEvent::Routed(RoutedMessage::Matched { pending, .. }) =
        event(&mut connection, &mut stdout)
    else {
        panic!("expected correlated reply")
    };
    assert_eq!(pending.method(), "fixture/ping");
    assert_eq!(connection.core().unresolved().count(), 0);
    let _ = connection.close();
}

#[test]
fn close_retains_unsent_request_as_uncertain_and_is_idempotent() {
    let (_child, mut connection, mut stdout) = spawn("rpc-peer");
    ready(&mut connection, &mut stdout);
    connection.prepare_request("fixture/ping", None).unwrap();
    let progress = connection.close().unwrap();
    assert_eq!(progress.written, 0);
    assert!(progress.total > 0);
    assert_eq!(connection.close(), Some(progress));
    let pending: Vec<_> = connection.core().unresolved().collect();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].1.uncertain());
    assert_eq!(pending[0].1.method(), "fixture/ping");
    assert!(connection.prepare_request("fixture/ping", None).is_err());
}

#[test]
fn failed_initialize_write_preserves_pending_identity() {
    let (mut child, mut connection, _stdout) = spawn("unknown-mode");
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(matches!(
        connection.pump(),
        Err(ConnectionInputError::Io(_))
    ));
    assert!(connection.progress().unwrap().failed);
    let pending: Vec<_> = connection.core().unresolved().collect();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].0, 1);
    assert!(pending[0].1.uncertain());
    assert_eq!(pending[0].1.method(), "initialize");
    assert!(!connection.core().is_ready());
}

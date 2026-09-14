#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use graph_domain::execution::StreamCompletion;
use graph_execution::{
    ConnectionInput, ConnectionIo, ConnectionIoError, InputEvent, IoEvent, OutputLimits,
};
use graph_protocol::correlation::RoutedMessage;
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn spawn(mode: &str, stderr_cap: u64) -> (Owned, ConnectionIo) {
    spawn_with_limits(mode, 4096, stderr_cap)
}
fn spawn_with_limits(mode: &str, stdout_cap: u64, stderr_cap: u64) -> (Owned, ConnectionIo) {
    let mut child = Owned(
        Command::new(env!("CARGO_BIN_EXE_graph-execution-fixture"))
            .arg(mode)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let input = ConnectionInput::begin(
        child.0.stdin.take().unwrap(),
        "epoch",
        2,
        4096,
        "client",
        "1",
        false,
    )
    .unwrap();
    let io = ConnectionIo::new(
        input,
        "epoch".into(),
        child.0.stdout.take(),
        child.0.stderr.take(),
        OutputLimits::new(stdout_cap, stderr_cap).unwrap(),
    );
    (child, io)
}

#[test]
fn actual_pipe_frame_failures_preserve_raw_bytes_and_pending_identity() {
    use graph_execution::ConnectionInputError;
    use graph_protocol::{connection::ConnectionError, framing::FrameError};
    for mode in ["rpc-oversized", "rpc-malformed", "rpc-truncated"] {
        let (_child, mut io) = spawn_with_limits(mode, 8192, 64);
        let deadline = Instant::now() + Duration::from_secs(3);
        let error = loop {
            assert!(Instant::now() < deadline, "{mode} stalled");
            match io.poll() {
                Err(error) => break error,
                Ok(None) => std::thread::sleep(Duration::from_millis(1)),
                Ok(Some(_)) => panic!("bad frame produced event"),
            }
        };
        match (mode, error) {
            (
                "rpc-oversized",
                ConnectionIoError::Input(ConnectionInputError::Protocol(ConnectionError::Frame(
                    FrameError::TooLarge,
                ))),
            ) => {}
            (
                "rpc-truncated",
                ConnectionIoError::Input(ConnectionInputError::Protocol(ConnectionError::Frame(
                    FrameError::Truncated,
                ))),
            ) => {}
            (
                "rpc-malformed",
                ConnectionIoError::Input(ConnectionInputError::Protocol(ConnectionError::Decode(
                    _,
                ))),
            ) => {}
            (_, error) => panic!("wrong error for {mode}: {error}"),
        }
        assert!(matches!(io.poll(), Err(ConnectionIoError::Stopped)));
        let (connection, snapshot) = io.finish();
        let pending: Vec<_> = connection.core().unresolved().collect();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, 1);
        assert_eq!(pending[0].1.method(), "initialize");
        assert!(pending[0].1.uncertain());
        match mode {
            "rpc-oversized" => {
                assert!(snapshot.stdout.len() >= 4096 && snapshot.stdout.len() <= 4097);
                assert_eq!(&snapshot.stdout[..4096], vec![b'x'; 4096]);
            }
            "rpc-malformed" => assert_eq!(snapshot.stdout, b"{\n"),
            _ => {
                assert_eq!(snapshot.stdout, b"{\"id\":1");
                // Complete raw stream is distinct from a truncated protocol frame.
                assert_eq!(snapshot.stdout_state, StreamCompletion::Complete);
            }
        }
    }
}

#[test]
fn exact_limit_reply_queues_ack_without_fabricating_write_completion() {
    let (_child, mut io) = spawn_with_limits("rpc-exact", 8192, 64);
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        assert!(Instant::now() < deadline);
        match io.poll().unwrap() {
            Some(IoEvent::Message(InputEvent::AcknowledgementQueued)) => break,
            None => std::thread::sleep(Duration::from_millis(1)),
            _ => panic!("unexpected event"),
        }
    }
    assert!(!io.core().is_ready());
    let (connection, snapshot) = io.finish();
    assert_eq!(snapshot.stdout.len(), 4096);
    assert_eq!(snapshot.stdout.last(), Some(&b'\n'));
    assert_eq!(connection.progress().unwrap().written, 0);
    assert_eq!(connection.core().unresolved().count(), 0);
}
#[test]
fn stderr_without_newlines_and_jsonl_both_progress_under_caps() {
    let (mut child, mut io) = spawn("rpc-stderr", 512 * 1024);
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut sent = false;
    let mut matched = 0;
    let mut eof = false;
    loop {
        assert!(Instant::now() < deadline, "combined I/O stalled");
        if io.core().is_ready() && !sent {
            io.prepare_request("fixture/ping", None).unwrap();
            sent = true;
        }
        match io.poll().unwrap() {
            Some(IoEvent::Message(InputEvent::Routed(RoutedMessage::Matched {
                pending, ..
            }))) => {
                assert_eq!(pending.method(), "fixture/ping");
                matched += 1;
            }
            Some(IoEvent::StdoutEof) => eof = true,
            _ => {}
        }
        if eof && io.is_drained() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(matched, 1);
    let (connection, snapshot) = io.finish();
    assert_eq!(connection.core().unresolved().count(), 0);
    assert_eq!(snapshot.stderr, vec![0xa5; 512 * 1024]);
    assert_eq!(snapshot.stdout_state, StreamCompletion::Complete);
    assert_eq!(snapshot.stderr_state, StreamCompletion::Complete);
    assert!(snapshot.stdout.len() <= 4096);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn stderr_overflow_stops_io_and_retains_exact_prefix() {
    let (_child, mut io) = spawn("rpc-stderr", 64);
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        assert!(Instant::now() < deadline);
        match io.poll() {
            Err(ConnectionIoError::OutputLimit) => break,
            Err(error) => panic!("unexpected failure: {error}"),
            Ok(_) => std::thread::sleep(Duration::from_millis(1)),
        }
    }
    assert!(matches!(io.poll(), Err(ConnectionIoError::Stopped)));
    let (connection, snapshot) = io.finish();
    assert!(!connection.core().is_ready());
    assert_eq!(snapshot.stderr, vec![0xa5; 64]);
    assert_eq!(snapshot.stderr_state, StreamCompletion::Truncated);
}
#[test]
fn stop_does_not_require_consumer_or_invent_eof() {
    let (_child, mut io) = spawn("rpc-peer", 4096);
    let start = Instant::now();
    io.stop();
    let (connection, snapshot) = io.finish();
    assert!(start.elapsed() < Duration::from_millis(100));
    assert_eq!(snapshot.stdout_state, StreamCompletion::Incomplete);
    assert_eq!(snapshot.stderr_state, StreamCompletion::Incomplete);
    assert!(snapshot.stdout.is_empty() && snapshot.stderr.is_empty());
    assert_eq!(connection.core().unresolved().count(), 1);
    assert!(connection.core().unresolved().next().unwrap().1.uncertain());
}

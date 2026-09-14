#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use graph_execution::NonblockingInput;
use std::{
    io,
    path::Path,
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
fn child(mode: &str, root: &Path) -> Owned {
    Owned(
        Command::new(env!("CARGO_BIN_EXE_graph-execution-fixture"))
            .arg(mode)
            .env_clear()
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}
fn finish(child: &mut Owned) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            return status;
        }
        assert!(Instant::now() < deadline, "owned child failed to terminate");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn blocked_child_input_closes_without_waiting_for_pending_bytes() {
    let root = tempfile::tempdir().unwrap();
    let mut child = child("sleep", root.path());
    let mut writer = NonblockingInput::new(child.0.stdin.take().unwrap(), 1024 * 1024).unwrap();
    writer.begin(&vec![0x5a; 1024 * 1024]).unwrap();
    let start = Instant::now();
    for _ in 0..256 {
        let _ = writer.pump().unwrap();
    }
    let partial = writer.progress();
    assert!(partial.written > 0 && partial.written < partial.total);
    assert!(!partial.failed);
    assert_eq!(
        writer.begin(b"replace").unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(writer.close(), partial);
    assert!(start.elapsed() < Duration::from_millis(500));
    child.0.kill().unwrap();
    let _ = finish(&mut child);
}

#[test]
fn completed_pipe_write_preserves_exact_bytes_and_close_delivers_eof() {
    let root = tempfile::tempdir().unwrap();
    let mut child = child("read-input", root.path());
    let mut writer = NonblockingInput::new(child.0.stdin.take().unwrap(), 32).unwrap();
    for bad in [&b""[..], &[1; 33][..]] {
        assert_eq!(
            writer.begin(bad).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    for frame in [&b"\0\xff\n"[..], &b"next"[..]] {
        writer.begin(frame).unwrap();
        let progress = writer.pump().unwrap();
        assert_eq!(progress.total, frame.len());
        assert_eq!(progress.written, frame.len());
        assert!(!progress.failed);
    }
    let final_progress = writer.close();
    assert_eq!(final_progress.written, 4);
    assert!(finish(&mut child).success());
    assert_eq!(
        std::fs::read(root.path().join("received-input")).unwrap(),
        b"\0\xff\nnext"
    );
}

#[test]
fn broken_pipe_retains_failed_frame_and_rejects_automatic_retry() {
    let root = tempfile::tempdir().unwrap();
    let mut child = child("unknown-mode", root.path());
    let mut writer = NonblockingInput::new(child.0.stdin.take().unwrap(), 32).unwrap();
    let _ = finish(&mut child);
    writer.begin(b"request").unwrap();
    assert_eq!(writer.pump().unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    let failed = writer.progress();
    assert!(failed.failed);
    assert_eq!((failed.written, failed.total), (0, 7));
    assert!(writer.begin(b"retry").is_err());
    assert!(writer.pump().is_err());
    assert_eq!(writer.close(), failed);
}

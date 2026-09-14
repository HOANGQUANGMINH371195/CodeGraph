#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use graph_execution::{AttachmentError, ConnectionSetup, OutputLimits, SupervisionLimits};
use std::process::{Child, Command, Stdio};

struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn setup(
    epoch: &str,
    pending: usize,
    frame: usize,
    name: &str,
    version: &str,
) -> Result<ConnectionSetup, graph_execution::ConnectionInputError> {
    ConnectionSetup::new(
        epoch,
        pending,
        frame,
        name,
        version,
        false,
        OutputLimits::new(4096, 4096).unwrap(),
        SupervisionLimits::new(3000, 1000).unwrap(),
    )
}

#[test]
fn invalid_configuration_is_rejected_without_acquiring_a_child() {
    for (epoch, pending, frame, name, version) in [
        ("", 2, 4096, "client", "1"),
        ("epoch", 0, 4096, "client", "1"),
        ("epoch", 4097, 4096, "client", "1"),
        ("epoch", 2, 0, "client", "1"),
        ("epoch", 2, 1, "client", "1"),
        ("epoch", 2, 1024 * 1024 + 1, "client", "1"),
        ("epoch", 2, 4096, "", "1"),
        ("epoch", 2, 4096, "client", ""),
    ] {
        assert!(setup(epoch, pending, frame, name, version).is_err());
    }
    assert!(setup("epoch", 2, 4096, "client", "1").is_ok());
}

#[test]
fn missing_pipe_returns_same_owned_child_without_extracting_other_handles() {
    for missing in ["stdin", "stdout", "stderr"] {
        let root = tempfile::tempdir().unwrap();
        let setup = setup("epoch", 2, 4096, "client", "1").unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_graph-execution-fixture"));
        command.arg("sleep").env_clear().current_dir(root.path());
        command.stdin(if missing == "stdin" {
            Stdio::null()
        } else {
            Stdio::piped()
        });
        command.stdout(if missing == "stdout" {
            Stdio::null()
        } else {
            Stdio::piped()
        });
        command.stderr(if missing == "stderr" {
            Stdio::null()
        } else {
            Stdio::piped()
        });
        let child = command.spawn().unwrap();
        let id = child.id();
        let failure = match setup.attach(child) {
            Err(failure) => failure,
            Ok(mut supervisor) => {
                while !supervisor.poll(true, false).finished {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                if let Ok(result) = supervisor.finish() {
                    if let Some(child) = result.unreaped_child {
                        drop(Owned(child));
                    }
                }
                panic!("missing pipe was accepted");
            }
        };
        // Establish cleanup guard before any assertions about the failure.
        let mut child = Owned(failure.child);
        assert!(matches!(failure.error, AttachmentError::MissingPipe(pipe) if pipe == missing));
        assert_eq!(child.0.id(), id);
        assert_eq!(child.0.stdin.is_some(), missing != "stdin");
        assert_eq!(child.0.stdout.is_some(), missing != "stdout");
        assert_eq!(child.0.stderr.is_some(), missing != "stderr");
        assert!(child.0.try_wait().unwrap().is_none());
        child.0.kill().unwrap();
        let status = child.0.wait().unwrap();
        assert!(!status.success());
        assert_eq!(child.0.try_wait().unwrap(), Some(status));
    }
}

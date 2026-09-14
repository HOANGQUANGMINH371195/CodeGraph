use crate::process_scope::OwnedProcessGroup;
use crate::{FixtureError, FixtureRun, SandboxEgressPolicy, SandboxRuntime};
use graph_domain::{
    CheckCommand,
    execution::{ChildCompletion, ExecutionCompletion, ScopeCleanup, StopReason, StreamCompletion},
};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Read},
    os::fd::AsFd,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub(crate) struct Pipe<R> {
    pub(crate) reader: Option<R>,
    pub(crate) bytes: Vec<u8>,
    pub(crate) state: StreamCompletion,
    cap: usize,
}
impl<R: Read + AsFd> Pipe<R> {
    pub(crate) fn new(reader: Option<R>, cap: u64) -> Self {
        let ready = reader.as_ref().is_some_and(|r| {
            rustix::fs::fcntl_getfl(r)
                .and_then(|flags| rustix::fs::fcntl_setfl(r, flags | rustix::fs::OFlags::NONBLOCK))
                .is_ok()
        });
        Self {
            reader: if ready { reader } else { None },
            bytes: Vec::new(),
            state: if ready {
                StreamCompletion::Incomplete
            } else {
                StreamCompletion::ReadFailed
            },
            cap: cap as usize,
        }
    }
    pub(crate) fn pump(&mut self) {
        if self.state != StreamCompletion::Incomplete {
            return;
        }
        let Some(reader) = &mut self.reader else {
            return;
        };
        let mut buffer = [0_u8; 8192];
        let remaining = self.cap - self.bytes.len();
        let requested = buffer.len().min(remaining + 1);
        match reader.read(&mut buffer[..requested]) {
            Ok(0) => {
                self.state = StreamCompletion::Complete;
                self.reader = None;
            }
            Ok(count) => {
                self.bytes
                    .extend_from_slice(&buffer[..count.min(remaining)]);
                if count > remaining {
                    self.state = StreamCompletion::Truncated;
                    self.reader = None;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => {
                self.state = StreamCompletion::ReadFailed;
                self.reader = None;
            }
        }
    }
}

pub(crate) fn digest_executable(path: &Path) -> Result<String, FixtureError> {
    const MAX_EXECUTABLE: u64 = 64 * 1024 * 1024;
    let metadata = path.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_EXECUTABLE {
        return Err(FixtureError::Preflight);
    }
    let mut file = File::open(path)?.take(MAX_EXECUTABLE + 1);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 16384];
    let mut length = 0_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        length += count as u64;
        if length > MAX_EXECUTABLE {
            return Err(FixtureError::Preflight);
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn not_spawned(reason: StopReason) -> FixtureRun {
    FixtureRun {
        completion: ExecutionCompletion {
            reason,
            child: ChildCompletion::NotSpawned,
            stdout: StreamCompletion::Incomplete,
            stderr: StreamCompletion::Incomplete,
            cleanup: ScopeCleanup::Complete,
        },
        stdout: Vec::new(),
        stderr: Vec::new(),
        elapsed_ms: 0,
        unreaped_child: None,
    }
}

fn observe_stop(
    previous: Option<StopReason>,
    child: ChildCompletion,
    cancelled: bool,
    runtime_expired: bool,
) -> Option<StopReason> {
    previous.or_else(|| {
        if cancelled {
            Some(StopReason::Cancelled)
        } else if child == ChildCompletion::Unreaped && runtime_expired {
            Some(StopReason::TimedOut)
        } else {
            None
        }
    })
}

pub(super) fn run(
    command: &CheckCommand,
    executable: &Path,
    root: &Path,
    environment: &BTreeMap<String, String>,
    cancelled: &AtomicBool,
) -> Result<FixtureRun, FixtureError> {
    let args = command
        .args()
        .iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
    run_with_launch(
        command,
        executable,
        executable,
        &args,
        root,
        environment,
        cancelled,
    )
}

pub(super) fn run_sandboxed(
    command: &CheckCommand,
    executable: &Path,
    root: &Path,
    environment: &BTreeMap<String, String>,
    egress: &SandboxEgressPolicy,
    runtime: &SandboxRuntime,
    cancelled: &AtomicBool,
) -> Result<FixtureRun, FixtureError> {
    let plan = runtime.plan(command, executable, root, environment, egress)?;
    run_with_launch(
        command,
        plan.backend(),
        executable,
        plan.args(),
        root,
        environment,
        cancelled,
    )
}

fn run_with_launch(
    command: &CheckCommand,
    launch_program: &Path,
    executable: &Path,
    launch_args: &[OsString],
    root: &Path,
    environment: &BTreeMap<String, String>,
    cancelled: &AtomicBool,
) -> Result<FixtureRun, FixtureError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(not_spawned(StopReason::Cancelled));
    }
    if !launch_program.is_absolute()
        || !launch_program.is_file()
        || !executable.is_absolute()
        || executable.to_str() != Some(command.program())
        || command.timeout_ms() > 5000
        || command.cleanup_timeout_ms() > 1000
        || command.stdout_max_bytes() > 1024 * 1024
        || command.stderr_max_bytes() > 1024 * 1024
        || graph_application::environment_fingerprint(environment)
            .map_err(|_| FixtureError::Preflight)?
            != command.environment_sha256()
        || digest_executable(executable)? != command.executable_sha256()
    {
        return Err(FixtureError::Preflight);
    }
    let root = root.canonicalize()?;
    let cwd = root.join(command.cwd()).canonicalize()?;
    if !root.is_dir() || !cwd.is_dir() || !cwd.starts_with(&root) {
        return Err(FixtureError::Preflight);
    }
    if cancelled.load(Ordering::Acquire) {
        return Ok(not_spawned(StopReason::Cancelled));
    }
    let start = Instant::now();
    let mut child = match Command::new(launch_program)
        .args(launch_args)
        .current_dir(cwd)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // The group is created by the host for this exact child. A group
        // signal is a teardown mechanism, not proof of full tree containment.
        .process_group(0)
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return Ok(not_spawned(StopReason::SpawnFailed)),
    };
    let mut process_group = OwnedProcessGroup::from_child(&child);
    let mut stdout = Pipe::new(child.stdout.take(), command.stdout_max_bytes());
    let mut stderr = Pipe::new(child.stderr.take(), command.stderr_max_bytes());
    let mut stop = None;
    let mut cleanup_start = None;
    let mut child_state = ChildCompletion::Unreaped;
    let mut kill_requested = false;
    loop {
        // After reaping, only the independent cleanup/drain deadline applies.
        stop = observe_stop(
            stop,
            child_state,
            cancelled.load(Ordering::Acquire),
            start.elapsed() >= Duration::from_millis(command.timeout_ms()),
        );
        if stop.is_some() && !kill_requested && child_state == ChildCompletion::Unreaped {
            // Direct owned child only. kill success is not reap/descendant cleanup proof.
            let _ = child.kill();
            kill_requested = true;
        }
        stdout.pump();
        stderr.pump();
        if stop.is_none() {
            if [stdout.state, stderr.state].contains(&StreamCompletion::Truncated) {
                stop = Some(StopReason::OutputLimit);
            } else if [stdout.state, stderr.state].contains(&StreamCompletion::ReadFailed) {
                stop = Some(StopReason::HostError);
            }
        }
        if child_state == ChildCompletion::Unreaped {
            match child.try_wait() {
                Ok(Some(status)) => {
                    child_state = ChildCompletion::Reaped {
                        exit_code: status.code(),
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    if stop.is_none() {
                        stop = Some(StopReason::HostError);
                    }
                }
            }
        }
        // Once the direct child has stopped, close the owned group as well so
        // descendants holding inherited stdout/stderr cannot outlive the
        // bounded drain window. This remains separate from reaping authority.
        if child_state != ChildCompletion::Unreaped || stop.is_some() {
            let _ = process_group.terminate();
        }
        if stop.is_some() || child_state != ChildCompletion::Unreaped {
            cleanup_start.get_or_insert_with(Instant::now);
        }
        if child_state != ChildCompletion::Unreaped
            && stdout.state != StreamCompletion::Incomplete
            && stderr.state != StreamCompletion::Incomplete
        {
            break;
        }
        if cleanup_start
            .is_some_and(|at| at.elapsed() >= Duration::from_millis(command.cleanup_timeout_ms()))
        {
            let _ = process_group.force_terminate();
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(FixtureRun {
        completion: ExecutionCompletion {
            reason: stop.unwrap_or(StopReason::Exited),
            child: child_state,
            stdout: stdout.state,
            stderr: stderr.state,
            cleanup: ScopeCleanup::Unverifiable,
        },
        stdout: stdout.bytes,
        stderr: stderr.bytes,
        elapsed_ms: start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        unreaped_child: if child_state == ChildCompletion::Unreaped {
            Some(child)
        } else {
            None
        },
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::{io::Write, os::unix::net::UnixStream};

    #[test]
    fn process_timeout_does_not_reclassify_post_reap_drain() {
        let reaped = ChildCompletion::Reaped { exit_code: Some(0) };
        assert_eq!(observe_stop(None, reaped, false, true), None);
        assert_eq!(
            observe_stop(None, ChildCompletion::Unreaped, false, true),
            Some(StopReason::TimedOut)
        );
        assert_eq!(
            observe_stop(None, reaped, true, true),
            Some(StopReason::Cancelled)
        );
        for previous in [
            StopReason::TimedOut,
            StopReason::Cancelled,
            StopReason::OutputLimit,
            StopReason::HostError,
        ] {
            assert_eq!(
                observe_stop(Some(previous), reaped, true, true),
                Some(previous)
            );
        }
    }

    #[test]
    fn retained_writer_prevents_eof_even_after_original_writer_closes() {
        let (reader, mut writer) = UnixStream::pair().unwrap();
        let retained = writer.try_clone().unwrap();
        let mut pipe = Pipe::new(Some(reader), 3);
        writer.write_all(b"abc").unwrap();
        drop(writer);
        pipe.pump();
        assert_eq!(pipe.bytes, b"abc");
        pipe.pump();
        assert_eq!(pipe.state, StreamCompletion::Incomplete);
        assert!(pipe.reader.is_some());
        drop(retained);
        pipe.pump();
        assert_eq!(pipe.state, StreamCompletion::Complete);
        assert!(pipe.reader.is_none());
    }

    #[test]
    fn missing_reader_is_failure_not_empty_complete_output() {
        let mut pipe = Pipe::<UnixStream>::new(None, 0);
        pipe.pump();
        assert_eq!(pipe.state, StreamCompletion::ReadFailed);
        assert!(pipe.bytes.is_empty());
    }
}

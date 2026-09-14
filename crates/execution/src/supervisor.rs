use crate::process_scope::OwnedProcessGroup;
use crate::{ConnectionInput, ConnectionIo, ConnectionIoError, IoEvent, OutputSnapshot};
use graph_domain::execution::{ChildCompletion, ExecutionCompletion, ScopeCleanup, StopReason};
use graph_protocol::{connection::Connection, rpc::RequestId};
use std::{
    io,
    process::Child,
    time::{Duration, Instant},
};

pub struct SupervisionLimits {
    runtime: Duration,
    cleanup: Duration,
}
impl SupervisionLimits {
    pub fn new(runtime_ms: u64, cleanup_ms: u64) -> io::Result<Self> {
        if !(1..=5000).contains(&runtime_ms) || !(1..=1000).contains(&cleanup_ms) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid supervision limits",
            ));
        }
        Ok(Self {
            runtime: Duration::from_millis(runtime_ms),
            cleanup: Duration::from_millis(cleanup_ms),
        })
    }
}
pub struct SupervisorTick {
    pub event: Option<IoEvent>,
    pub finished: bool,
}
#[must_use = "retain and reconcile any unreaped child and unresolved requests"]
pub struct SupervisedConnection {
    pub connection: ConnectionInput,
    pub output: OutputSnapshot,
    pub completion: ExecutionCompletion,
    pub elapsed_ms: u64,
    pub unreaped_child: Option<Child>,
    pub io_error: Option<ConnectionIoError>,
    pub process_error: Option<io::Error>,
}

/// Sealed protocol accounting alongside the original supervision result.
/// Not a durable/validated receipt or proof that an unresolved RPC did not run.
#[must_use = "retain and reconcile any unreaped child and unresolved requests"]
pub struct TerminalConnection {
    pub requests: graph_protocol::correlation::TerminalRequests,
    /// Last frame only; never infer a pending request association from this.
    pub input_progress: Option<crate::InputProgress>,
    pub output: OutputSnapshot,
    pub completion: ExecutionCompletion,
    /// Monotonic duration since supervisor attachment, not process spawn.
    pub elapsed_ms: u64,
    pub unreaped_child: Option<Child>,
    pub io_error: Option<ConnectionIoError>,
    pub process_error: Option<io::Error>,
}

impl SupervisedConnection {
    /// Transfers all evidence and owned process handles without retry or reap.
    pub fn into_terminal(self) -> TerminalConnection {
        let (requests, input_progress) = self.connection.into_terminal();
        TerminalConnection {
            requests,
            input_progress,
            output: self.output,
            completion: self.completion,
            elapsed_ms: self.elapsed_ms,
            unreaped_child: self.unreaped_child,
            io_error: self.io_error,
            process_error: self.process_error,
        }
    }
}
/// Host must attach the matching pipes/child and retain ownership until finish.
/// No spawn approval, sandbox containment, background task or implicit relaunch.
#[must_use = "poll and finish the supervisor; dropping Child does not reap it"]
pub struct ConnectionSupervisor {
    child: Child,
    process_group: OwnedProcessGroup,
    io: ConnectionIo,
    limits: SupervisionLimits,
    start: Instant,
    cleanup_start: Option<Instant>,
    state: ChildCompletion,
    stop: Option<StopReason>,
    killed: bool,
    done: bool,
    io_error: Option<ConnectionIoError>,
    process_error: Option<io::Error>,
}
impl ConnectionSupervisor {
    pub fn attach(child: Child, io: ConnectionIo, limits: SupervisionLimits) -> Self {
        let process_group = OwnedProcessGroup::from_child(&child);
        Self {
            child,
            process_group,
            io,
            limits,
            start: Instant::now(),
            cleanup_start: None,
            state: ChildCompletion::Unreaped,
            stop: None,
            killed: false,
            done: false,
            io_error: None,
            process_error: None,
        }
    }
    pub fn core(&self) -> &Connection {
        self.io.core()
    }
    /// Read-only delivery counters, retained through shutdown for reconciliation.
    pub fn input_progress(&self) -> Option<crate::InputProgress> {
        self.io.input_progress()
    }
    pub fn prepare_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<RequestId, ConnectionIoError> {
        if self.done || self.stop.is_some() || self.state != ChildCompletion::Unreaped {
            return Err(ConnectionIoError::Stopped);
        }
        self.io.prepare_request(method, params)
    }
    fn stop_for(&mut self, reason: StopReason) {
        if self.stop.is_none() {
            self.stop = Some(reason);
        }
        self.cleanup_start.get_or_insert_with(Instant::now);
        self.io.begin_shutdown();
    }
    /// Caller polls regularly. Control progresses even when events are declined;
    /// bytes remain bounded in raw capture, not an unbounded consumer queue.
    pub fn poll(&mut self, cancelled: bool, accept_events: bool) -> SupervisorTick {
        if self.done {
            return SupervisorTick {
                event: None,
                finished: true,
            };
        }
        if cancelled {
            self.stop_for(StopReason::Cancelled);
        } else if self.stop.is_none()
            && self.state == ChildCompletion::Unreaped
            && self.start.elapsed() >= self.limits.runtime
        {
            self.stop_for(StopReason::TimedOut);
        }
        if self.stop.is_some() && !self.killed && self.state == ChildCompletion::Unreaped {
            if let Err(error) = self.child.kill() {
                self.process_error = Some(error);
            }
            self.killed = true;
        }
        if self.state == ChildCompletion::Unreaped {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.state = ChildCompletion::Reaped {
                        exit_code: status.code(),
                    };
                    self.cleanup_start.get_or_insert_with(Instant::now);
                }
                Ok(None) => {}
                Err(error) => {
                    self.process_error.get_or_insert(error);
                    self.stop_for(StopReason::HostError);
                }
            }
        }
        // The direct child can exit while a descendant still owns a pipe.
        // Close the exact owned process group before the bounded drain deadline
        // expires; this is still not proof that a descendant escaped the group.
        if self.state != ChildCompletion::Unreaped || self.stop.is_some() {
            let _ = self.process_group.terminate();
        }
        let mut event = None;
        if self.io_error.is_none() {
            match self.io.poll_with_events(accept_events) {
                Ok(value) => event = value,
                Err(error) => {
                    let reason = if matches!(error, ConnectionIoError::OutputLimit) {
                        StopReason::OutputLimit
                    } else {
                        StopReason::HostError
                    };
                    self.io_error = Some(error);
                    self.stop_for(reason);
                }
            }
        }
        self.done = (self.state != ChildCompletion::Unreaped
            && (self.io_error.is_some() || (self.io.is_drained() && self.io.protocol_ended())))
            || self
                .cleanup_start
                .is_some_and(|start| start.elapsed() >= self.limits.cleanup);
        if self.done {
            let _ = self.process_group.force_terminate();
        }
        SupervisorTick {
            event,
            finished: self.done,
        }
    }
    /// A deadline is not proof of death: return unresolved Child to caller.
    pub fn finish(self) -> Result<SupervisedConnection, Self> {
        if !self.done {
            return Err(self);
        }
        let (connection, output) = self.io.finish();
        let completion = ExecutionCompletion {
            reason: self.stop.unwrap_or(StopReason::Exited),
            child: self.state,
            stdout: output.stdout_state,
            stderr: output.stderr_state,
            cleanup: ScopeCleanup::Unverifiable,
        };
        Ok(SupervisedConnection {
            connection,
            output,
            completion,
            elapsed_ms: self.start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            unreaped_child: if self.state == ChildCompletion::Unreaped {
                Some(self.child)
            } else {
                None
            },
            io_error: self.io_error,
            process_error: self.process_error,
        })
    }
}

use crate::{ConnectionInput, ConnectionInputError, InputEvent, linux::Pipe};
use graph_domain::execution::StreamCompletion;
use graph_protocol::{connection::Connection, rpc::RequestId};
use std::{
    io,
    process::{ChildStderr, ChildStdout},
};

pub struct OutputLimits {
    stdout: u64,
    stderr: u64,
}
impl OutputLimits {
    pub fn new(stdout: u64, stderr: u64) -> io::Result<Self> {
        if stdout > 1024 * 1024 || stderr > 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output cap exceeds adapter limit",
            ));
        }
        Ok(Self { stdout, stderr })
    }
}
pub struct OutputSnapshot {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_state: StreamCompletion,
    pub stderr_state: StreamCompletion,
}
pub enum IoEvent {
    Message(InputEvent),
    StdoutEof,
}
#[derive(Debug, thiserror::Error)]
pub enum ConnectionIoError {
    #[error("connection I/O stopped")]
    Stopped,
    #[error("retained output budget exhausted")]
    OutputLimit,
    #[error("output reader failed")]
    ReadFailed,
    #[error(transparent)]
    Input(#[from] ConnectionInputError),
}
/// Poll-driven pipe owner. Does not own/reap the child or establish sandbox scope.
pub struct ConnectionIo {
    input: ConnectionInput,
    epoch: String,
    stdout: Pipe<ChildStdout>,
    stderr: Pipe<ChildStderr>,
    parsed: usize,
    stdout_eof: bool,
    stopped: bool,
    shutting_down: bool,
}
impl ConnectionIo {
    pub fn new(
        input: ConnectionInput,
        epoch: String,
        stdout: Option<ChildStdout>,
        stderr: Option<ChildStderr>,
        limits: OutputLimits,
    ) -> Self {
        Self {
            input,
            epoch,
            stdout: Pipe::new(stdout, limits.stdout),
            stderr: Pipe::new(stderr, limits.stderr),
            parsed: 0,
            stdout_eof: false,
            stopped: false,
            shutting_down: false,
        }
    }
    pub fn core(&self) -> &Connection {
        self.input.core()
    }
    /// Local pipe delivery only, never peer acknowledgement.
    pub fn input_progress(&self) -> Option<crate::InputProgress> {
        self.input.progress()
    }
    pub fn is_drained(&self) -> bool {
        self.stdout.state == StreamCompletion::Complete
            && self.stderr.state == StreamCompletion::Complete
    }
    pub fn prepare_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<RequestId, ConnectionIoError> {
        if self.stopped || self.stdout_eof || self.shutting_down {
            return Err(ConnectionIoError::Stopped);
        }
        Ok(self.input.prepare_request(method, params)?)
    }
    pub fn poll(&mut self) -> Result<Option<IoEvent>, ConnectionIoError> {
        self.poll_with_events(true)
    }
    pub(crate) fn poll_with_events(
        &mut self,
        accept_events: bool,
    ) -> Result<Option<IoEvent>, ConnectionIoError> {
        if self.stopped {
            return Err(ConnectionIoError::Stopped);
        }
        let result = self.poll_inner(accept_events);
        if result.is_err() {
            self.stop();
        }
        result
    }
    fn poll_inner(&mut self, accept_events: bool) -> Result<Option<IoEvent>, ConnectionIoError> {
        if !self.stdout_eof && !self.shutting_down {
            let _ = self.input.pump()?;
        }
        self.stderr.pump();
        self.stdout.pump();
        let states = [self.stdout.state, self.stderr.state];
        if states.contains(&StreamCompletion::Truncated) {
            return Err(ConnectionIoError::OutputLimit);
        }
        if states.contains(&StreamCompletion::ReadFailed) {
            return Err(ConnectionIoError::ReadFailed);
        }
        if self.shutting_down || !accept_events {
            return Ok(None);
        }
        if self.parsed < self.stdout.bytes.len() {
            let (used, event) = self
                .input
                .receive(&self.epoch, &self.stdout.bytes[self.parsed..])?;
            self.parsed += used;
            if let Some(event) = event {
                return Ok(Some(IoEvent::Message(event)));
            }
        }
        if !self.stdout_eof
            && self.stdout.state == StreamCompletion::Complete
            && self.parsed == self.stdout.bytes.len()
        {
            self.input.eof()?;
            self.stdout_eof = true;
            return Ok(Some(IoEvent::StdoutEof));
        }
        Ok(None)
    }
    /// Control never waits for a data consumer. Incomplete reads stay incomplete.
    pub(crate) fn begin_shutdown(&mut self) {
        self.shutting_down = true;
        let _ = self.input.close();
    }
    pub(crate) fn protocol_ended(&self) -> bool {
        self.stdout_eof || self.shutting_down || self.stopped
    }
    pub fn stop(&mut self) {
        self.stopped = true;
        let _ = self.input.close();
        self.stdout.reader = None;
        self.stderr.reader = None;
    }
    /// Retain connection metadata for reconciliation and bytes for artifact storage.
    pub fn finish(mut self) -> (ConnectionInput, OutputSnapshot) {
        self.stop();
        (
            self.input,
            OutputSnapshot {
                stdout: self.stdout.bytes,
                stderr: self.stderr.bytes,
                stdout_state: self.stdout.state,
                stderr_state: self.stderr.state,
            },
        )
    }
}

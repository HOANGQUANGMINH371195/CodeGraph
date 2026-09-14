use crate::{InputProgress, NonblockingInput};
use graph_protocol::{
    connection::{Connection, ConnectionError, Incoming},
    correlation::RoutedMessage,
    rpc::RequestId,
};
use std::{io, process::ChildStdin};

#[derive(Debug, thiserror::Error)]
pub enum ConnectionInputError {
    #[error("outbound frame is still pending")]
    Busy,
    #[error(transparent)]
    Protocol(#[from] ConnectionError),
    #[error("connection input failed: {0}")]
    Io(#[from] io::Error),
}
pub enum InputEvent {
    AcknowledgementQueued,
    Routed(RoutedMessage),
}

/// Validated unsent initialization. Prepare before acquiring a child process.
pub(crate) struct PreparedConnectionInput {
    core: Connection,
    initial: Vec<u8>,
    max_frame: usize,
}
impl PreparedConnectionInput {
    pub fn new(
        epoch: &str,
        max_pending: usize,
        max_frame: usize,
        name: &str,
        version: &str,
        experimental: bool,
    ) -> Result<Self, ConnectionInputError> {
        if !(1..=1024 * 1024).contains(&max_frame) {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "invalid input frame cap").into(),
            );
        }
        let (core, initial) =
            Connection::begin(epoch, max_pending, max_frame, name, version, experimental)?;
        Ok(Self {
            core,
            initial,
            max_frame,
        })
    }
    /// Configures and queues bytes only; never writes to the peer.
    pub(crate) fn bind(self, pipe: ChildStdin) -> Result<ConnectionInput, ConnectionInputError> {
        let mut writer = NonblockingInput::new(pipe, self.max_frame)?;
        writer.begin(&self.initial)?;
        Ok(ConnectionInput {
            core: self.core,
            writer: Some(writer),
            acknowledging: false,
            closed_progress: None,
        })
    }
}

/// Bounded connection writer, not a complete transport or execution host.
/// Caller owns child/stdout/stderr, deadlines, permissions and epoch identity.
pub struct ConnectionInput {
    core: Connection,
    writer: Option<NonblockingInput>,
    acknowledging: bool,
    closed_progress: Option<InputProgress>,
}
impl ConnectionInput {
    /// Close stdin and permanently seal correlation. Last-frame counters are
    /// not peer acknowledgement and may describe a notification, not a request.
    pub fn into_terminal(
        mut self,
    ) -> (
        graph_protocol::correlation::TerminalRequests,
        Option<InputProgress>,
    ) {
        let progress = self.close();
        (self.core.into_terminal(), progress)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn begin(
        pipe: ChildStdin,
        epoch: &str,
        max_pending: usize,
        max_frame: usize,
        name: &str,
        version: &str,
        experimental: bool,
    ) -> Result<Self, ConnectionInputError> {
        PreparedConnectionInput::new(epoch, max_pending, max_frame, name, version, experimental)?
            .bind(pipe)
    }
    pub fn core(&self) -> &Connection {
        &self.core
    }
    pub fn progress(&self) -> Option<InputProgress> {
        self.writer
            .as_ref()
            .map(NonblockingInput::progress)
            .or(self.closed_progress)
    }
    fn idle(&self) -> Result<(), ConnectionInputError> {
        let writer = self.writer.as_ref().ok_or(ConnectionError::NotReady)?;
        let progress = writer.progress();
        if progress.failed || progress.written != progress.total {
            return Err(ConnectionInputError::Busy);
        }
        Ok(())
    }
    pub fn prepare_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<RequestId, ConnectionInputError> {
        self.idle()?;
        let request = match self.core.prepare_request(method, params) {
            Ok(request) => request,
            Err(error) => {
                if matches!(error, ConnectionError::Encode) {
                    let _ = self.close();
                }
                return Err(error.into());
            }
        };
        let result = self
            .writer
            .as_mut()
            .ok_or(ConnectionError::NotReady)?
            .begin(&request.bytes);
        if let Err(error) = result {
            let _ = self.close();
            return Err(error.into());
        }
        Ok(request.id)
    }
    pub fn pump(&mut self) -> Result<InputProgress, ConnectionInputError> {
        let result = self
            .writer
            .as_mut()
            .ok_or(ConnectionError::NotReady)?
            .pump();
        let progress = match result {
            Ok(progress) => progress,
            Err(error) => {
                let _ = self.close();
                return Err(error.into());
            }
        };
        if self.acknowledging && progress.written == progress.total {
            if let Err(error) = self.core.confirm_ack_written() {
                let _ = self.close();
                return Err(error.into());
            }
            self.acknowledging = false;
        }
        Ok(progress)
    }
    /// One frame/event; caller must retain and resubmit the unconsumed suffix.
    pub fn receive(
        &mut self,
        epoch: &str,
        bytes: &[u8],
    ) -> Result<(usize, Option<InputEvent>), ConnectionInputError> {
        let (used, incoming) = match self.core.receive(epoch, bytes) {
            Ok(value) => value,
            Err(error) => {
                let _ = self.close();
                return Err(error.into());
            }
        };
        let event = match incoming {
            Some(Incoming::Acknowledgement(bytes)) => {
                if let Err(error) = self.idle() {
                    let _ = self.close();
                    return Err(error);
                }
                if let Err(error) = self
                    .writer
                    .as_mut()
                    .ok_or(ConnectionError::NotReady)?
                    .begin(&bytes)
                {
                    let _ = self.close();
                    return Err(error.into());
                }
                self.acknowledging = true;
                Some(InputEvent::AcknowledgementQueued)
            }
            Some(Incoming::Routed(message)) => Some(InputEvent::Routed(message)),
            None => None,
        };
        Ok((used, event))
    }
    /// Actual stdout EOF; timeout is not EOF. Retain pending requests on failure.
    pub fn eof(&mut self) -> Result<(), ConnectionInputError> {
        let result = self.core.eof();
        let _ = self.close();
        Ok(result?)
    }
    pub fn close(&mut self) -> Option<InputProgress> {
        self.core.fail();
        if let Some(writer) = self.writer.take() {
            self.closed_progress = Some(writer.close());
        }
        self.closed_progress
    }
}

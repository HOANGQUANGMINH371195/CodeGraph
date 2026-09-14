use crate::connection_input::PreparedConnectionInput;
use crate::{
    ConnectionInputError, ConnectionIo, ConnectionSupervisor, OutputLimits, SupervisionLimits,
};
use std::process::Child;

/// No process, approval or account capability is acquired by preparing a setup.
pub struct ConnectionSetup {
    input: PreparedConnectionInput,
    epoch: String,
    output: OutputLimits,
    supervision: SupervisionLimits,
}

#[derive(Debug, thiserror::Error)]
pub enum AttachmentError {
    #[error("connection requires piped {0}")]
    MissingPipe(&'static str),
    #[error(transparent)]
    Input(#[from] ConnectionInputError),
}

/// No implicit kill/wait. The caller must retain, stop and reap this exact child.
/// No request bytes were written. On input setup error, stdin may be closed.
#[must_use = "attachment failure still owns the child; explicitly supervise and reap it"]
pub struct AttachmentFailure {
    pub child: Child,
    pub error: AttachmentError,
}

impl ConnectionSetup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        epoch: &str,
        max_pending: usize,
        max_frame: usize,
        name: &str,
        version: &str,
        experimental: bool,
        output: OutputLimits,
        supervision: SupervisionLimits,
    ) -> Result<Self, ConnectionInputError> {
        let input = PreparedConnectionInput::new(
            epoch,
            max_pending,
            max_frame,
            name,
            version,
            experimental,
        )?;
        Ok(Self {
            input,
            epoch: epoch.into(),
            output,
            supervision,
        })
    }

    /// Consumes one child and extracts its matching pipes. This is not spawn
    /// approval or executable identity verification. Poll the returned supervisor.
    pub fn attach(self, mut child: Child) -> Result<ConnectionSupervisor, AttachmentFailure> {
        let missing = if child.stdin.is_none() {
            Some("stdin")
        } else if child.stdout.is_none() {
            Some("stdout")
        } else if child.stderr.is_none() {
            Some("stderr")
        } else {
            None
        };
        if let Some(pipe) = missing {
            return Err(AttachmentFailure {
                child,
                error: AttachmentError::MissingPipe(pipe),
            });
        }
        let Some(stdin) = child.stdin.take() else {
            return Err(AttachmentFailure {
                child,
                error: AttachmentError::MissingPipe("stdin"),
            });
        };
        let input = match self.input.bind(stdin) {
            Ok(input) => input,
            Err(error) => {
                return Err(AttachmentFailure {
                    child,
                    error: error.into(),
                });
            }
        };
        let io = ConnectionIo::new(
            input,
            self.epoch,
            child.stdout.take(),
            child.stderr.take(),
            self.output,
        );
        Ok(ConnectionSupervisor::attach(child, io, self.supervision))
    }
}

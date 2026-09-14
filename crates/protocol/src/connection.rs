//! Sans-I/O native connection sequencing. The host owns/authenticates transport,
//! enforces permissions and queue budgets, and confirms actual writes.
use crate::{
    correlation::{CorrelationError, PendingRequest, PendingRequests, RoutedMessage},
    framing::{FrameError, LineDecoder},
    handshake::{Handshake, HandshakeError},
    rpc::{DecodeError, Message, Request, RequestId},
};

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("connection is closed or handshake is not ready")]
    NotReady,
    #[error("initialization is owned by the connection handshake")]
    ReservedMethod,
    #[error(transparent)]
    Correlation(#[from] CorrelationError),
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    Handshake(#[from] HandshakeError),
    #[error("outbound RPC encoding failed")]
    Encode,
}

pub enum Incoming {
    /// Unsent acknowledgement. Ready only after confirm_ack_written succeeds.
    Acknowledgement(Vec<u8>),
    Routed(RoutedMessage),
}

pub struct OutboundRequest {
    pub id: RequestId,
    pub bytes: Vec<u8>,
}

pub struct Connection {
    epoch: String,
    max_frame: usize,
    decoder: LineDecoder,
    pending: PendingRequests,
    handshake: Handshake,
    closed: bool,
}

impl Connection {
    /// Consume the protocol core after the host finishes processing input.
    /// This seals accounting only; it does not close/reap an OS process.
    pub fn into_terminal(self) -> crate::correlation::TerminalRequests {
        self.pending.into_terminal()
    }

    /// Creates unsent initialize frame. Host must use a fresh epoch per transport
    /// and fail this core after ambiguous/failed initialize write.
    pub fn begin(
        epoch: &str,
        max_pending: usize,
        max_frame: usize,
        name: &str,
        version: &str,
        experimental: bool,
    ) -> Result<(Self, Vec<u8>), ConnectionError> {
        let decoder = LineDecoder::new(max_frame)?;
        let mut pending = PendingRequests::new(epoch, max_pending)?;
        let id = pending.reserve("initialize")?;
        let (handshake, request) = Handshake::begin(id, name, version, experimental)?;
        let bytes = request
            .encode_line(max_frame)
            .map_err(|_| ConnectionError::Encode)?;
        Ok((
            Self {
                epoch: epoch.into(),
                max_frame,
                decoder,
                pending,
                handshake,
                closed: false,
            },
            bytes,
        ))
    }

    pub fn is_ready(&self) -> bool {
        !self.closed && self.handshake.is_ready()
    }

    pub fn confirm_ack_written(&mut self) -> Result<(), ConnectionError> {
        if self.closed {
            return Err(ConnectionError::NotReady);
        }
        Ok(self.handshake.acknowledge_written()?)
    }

    /// This is not authorization or dispatch. Caller owns returned bytes and
    /// must bound its write queue. A subsequent write failure calls fail().
    pub fn prepare_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<OutboundRequest, ConnectionError> {
        if !self.is_ready() {
            return Err(ConnectionError::NotReady);
        }
        if matches!(method, "initialize" | "initialized") {
            return Err(ConnectionError::ReservedMethod);
        }
        let id = self.pending.reserve(method)?;
        let message = Message::Request(Request {
            id: id.clone(),
            method: method.into(),
            params,
            trace: None,
        });
        match message.encode_line(self.max_frame) {
            Ok(bytes) => Ok(OutboundRequest { id, bytes }),
            Err(_) => {
                self.fail();
                Err(ConnectionError::Encode)
            }
        }
    }

    pub fn mark_uncertain(&mut self, id: &RequestId) -> Result<bool, ConnectionError> {
        Ok(self.pending.mark_uncertain(id)?)
    }

    pub fn unresolved(&self) -> impl Iterator<Item = (i64, &PendingRequest)> {
        self.pending.unresolved()
    }

    /// One frame per call. Re-submit unconsumed suffix after handling the event.
    /// Epoch is host-attached, not taken from JSON. Mismatch consumes no bytes.
    pub fn receive(
        &mut self,
        epoch: &str,
        bytes: &[u8],
    ) -> Result<(usize, Option<Incoming>), ConnectionError> {
        if epoch != self.epoch {
            return Err(CorrelationError::EpochMismatch.into());
        }
        if self.closed {
            return Err(ConnectionError::NotReady);
        }
        let result = self.receive_inner(bytes);
        if result.is_err() {
            self.fail();
        }
        result
    }

    fn receive_inner(
        &mut self,
        bytes: &[u8],
    ) -> Result<(usize, Option<Incoming>), ConnectionError> {
        let (consumed, frame) = self.decoder.push(bytes)?;
        let Some(frame) = frame else {
            return Ok((consumed, None));
        };
        let message = Message::decode(&frame, self.max_frame)?;
        let routed = self.pending.receive(&self.epoch, message)?;
        let event = match routed {
            RoutedMessage::Matched { pending, reply } if pending.method() == "initialize" => {
                self.handshake.observe_reply(&reply)?;
                let ack = self.handshake.prepare_acknowledgement()?;
                Incoming::Acknowledgement(
                    ack.encode_line(self.max_frame)
                        .map_err(|_| ConnectionError::Encode)?,
                )
            }
            other => Incoming::Routed(other),
        };
        Ok((consumed, Some(event)))
    }

    /// Actual transport EOF only; no inference about tasks or child processes.
    pub fn eof(&mut self) -> Result<(), ConnectionError> {
        if self.closed {
            return Err(ConnectionError::NotReady);
        }
        let result = self.decoder.finish();
        self.fail();
        Ok(result?)
    }

    pub fn fail(&mut self) {
        self.closed = true;
        self.handshake.fail();
        self.pending.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_seal_retains_unanswered_initialize_without_claiming_dispatch() {
        let (connection, _unsent) =
            Connection::begin("epoch", 1, 4096, "fixture", "1", false).unwrap();
        let terminal = connection.into_terminal();
        assert_eq!(terminal.epoch(), "epoch");
        let entries: Vec<_> = terminal.unresolved().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, 1);
        assert_eq!(entries[0].1.method(), "initialize");
        assert!(entries[0].1.uncertain());
    }
    const INIT: &[u8] = b"{\"id\":1,\"result\":{\"userAgent\":\"test\",\"platformFamily\":\"unix\",\"platformOs\":\"linux\"}}\n";
    fn ready() -> Connection {
        let (mut connection, _) = Connection::begin("e1", 2, 512, "harness", "1", false).unwrap();
        assert!(matches!(
            connection.receive("e1", INIT).unwrap().1,
            Some(Incoming::Acknowledgement(_))
        ));
        connection.confirm_ack_written().unwrap();
        connection
    }

    #[test]
    fn composed_handshake_blocks_requests_until_ack_and_routes_replies() {
        let (mut connection, initial) =
            Connection::begin("e1", 2, 512, "harness", "1", false).unwrap();
        assert!(matches!(
            Message::decode(&initial, 512).unwrap(),
            Message::Request(_)
        ));
        assert!(connection.prepare_request("thread/read", None).is_err());
        for byte in &INIT[..INIT.len() - 1] {
            assert!(connection.receive("e1", &[*byte]).unwrap().1.is_none());
        }
        let Some(Incoming::Acknowledgement(ack)) = connection.receive("e1", b"\n").unwrap().1
        else {
            panic!("ack")
        };
        assert_eq!(ack, b"{\"method\":\"initialized\"}\n");
        assert!(!connection.is_ready());
        assert!(connection.prepare_request("thread/read", None).is_err());
        connection.confirm_ack_written().unwrap();
        assert!(matches!(
            connection.prepare_request("initialize", None),
            Err(ConnectionError::ReservedMethod)
        ));
        let request = connection.prepare_request("thread/read", None).unwrap();
        assert_eq!(request.id, RequestId::Integer(2));
        let Some(Incoming::Routed(RoutedMessage::Matched { pending, reply })) = connection
            .receive("e1", b"{\"id\":2,\"result\":null}\n")
            .unwrap()
            .1
        else {
            panic!("matched")
        };
        assert_eq!(pending.method(), "thread/read");
        assert!(matches!(reply, Message::Response(_)));
        assert_eq!(connection.unresolved().count(), 0);
    }

    #[test]
    fn stale_epoch_does_not_corrupt_partial_frame_and_server_request_is_not_a_reply() {
        let mut connection = ready();
        connection.prepare_request("thread/read", None).unwrap();
        connection.receive("e1", b"{\"id\":2,").unwrap();
        assert!(matches!(
            connection.receive("old", b"bad\n"),
            Err(ConnectionError::Correlation(
                CorrelationError::EpochMismatch
            ))
        ));
        let event = connection
            .receive("e1", b"\"method\":\"approval\"}\n")
            .unwrap()
            .1;
        assert!(matches!(
            event,
            Some(Incoming::Routed(RoutedMessage::ServerRequest(_)))
        ));
        assert_eq!(connection.unresolved().count(), 1);
        assert!(connection.is_ready());
    }

    #[test]
    fn protocol_failure_eof_and_encode_error_retain_uncertain_requests() {
        for invalid in [b"not json\n".to_vec(), vec![b'x'; 512]] {
            let mut connection = ready();
            connection.prepare_request("turn/start", None).unwrap();
            assert!(connection.receive("e1", &invalid).is_err());
            assert!(!connection.is_ready());
            assert!(
                connection
                    .unresolved()
                    .all(|(_, request)| request.uncertain())
            );
            assert!(connection.prepare_request("turn/start", None).is_err());
        }
        let mut connection = ready();
        connection.prepare_request("turn/start", None).unwrap();
        connection.receive("e1", b"{").unwrap();
        assert!(matches!(
            connection.eof(),
            Err(ConnectionError::Frame(FrameError::Truncated))
        ));
        assert_eq!(connection.unresolved().count(), 1);
        let mut connection = ready();
        assert!(matches!(
            connection.prepare_request(
                "turn/start",
                Some(serde_json::json!({"text":"x".repeat(1024)}))
            ),
            Err(ConnectionError::Encode)
        ));
        assert!(!connection.is_ready());
        assert_eq!(connection.unresolved().count(), 1);
    }
}

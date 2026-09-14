//! Local initialize sequencing, not authentication or effective capabilities.
use crate::rpc::{Message, Notification, Request, RequestId};
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum HandshakeError {
    #[error("invalid initialize identifier or client metadata")]
    Invalid,
    #[error("invalid handshake transition")]
    State,
    #[error("message is not the expected initialize reply")]
    Unexpected,
    #[error("server rejected initialize")]
    Rejected,
    #[error("initialize response metadata is malformed or unsupported")]
    Malformed,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerMetadata {
    user_agent: String,
    platform_family: String,
    platform_os: String,
}

impl ServerMetadata {
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }
    pub fn platform_family(&self) -> &str {
        &self.platform_family
    }
    pub fn platform_os(&self) -> &str {
        &self.platform_os
    }
}

enum State {
    WaitingReply,
    AwaitingAcknowledgement(ServerMetadata),
    AcknowledgementPrepared(ServerMetadata),
    Ready(ServerMetadata),
    Failed,
}

pub struct Handshake {
    request_id: RequestId,
    state: State,
}

impl Handshake {
    /// Host reserves this ID in its connection's PendingRequests before dispatch.
    /// Returned request is unsent; an ambiguous write must fail the handshake.
    pub fn begin(
        request_id: RequestId,
        name: &str,
        version: &str,
        experimental_api: bool,
    ) -> Result<(Self, Message), HandshakeError> {
        if !valid(name, 256)
            || !valid(version, 256)
            || matches!(&request_id, RequestId::String(id) if !valid(id, 256))
        {
            return Err(HandshakeError::Invalid);
        }
        let request = Message::Request(Request {
            id: request_id.clone(),
            method: "initialize".into(),
            trace: None,
            params: Some(serde_json::json!({
                "clientInfo":{"name":name,"version":version},
                "capabilities":{"experimentalApi":experimental_api,"requestAttestation":false}
            })),
        });
        Ok((
            Self {
                request_id,
                state: State::WaitingReply,
            },
            request,
        ))
    }

    /// Host must authenticate and correlate connection epoch first. Unknown
    /// messages leave state unchanged so the router can handle them separately.
    pub fn observe_reply(&mut self, message: &Message) -> Result<(), HandshakeError> {
        if !matches!(self.state, State::WaitingReply) {
            return Err(HandshakeError::State);
        }
        let result = match message {
            Message::Response(response) if response.id == self.request_id => &response.result,
            Message::Error(response) if response.id == self.request_id => {
                self.fail();
                return Err(HandshakeError::Rejected);
            }
            _ => return Err(HandshakeError::Unexpected),
        };
        // Deserialize by reference: unrelated response fields are not cloned.
        let metadata = ServerMetadata::deserialize(result).ok().filter(|metadata| {
            valid(&metadata.user_agent, 4096)
                && valid(&metadata.platform_family, 128)
                && valid(&metadata.platform_os, 128)
        });
        match metadata {
            Some(metadata) => {
                self.state = State::AwaitingAcknowledgement(metadata);
                Ok(())
            }
            None => {
                self.fail();
                Err(HandshakeError::Malformed)
            }
        }
    }

    /// Produce once. A successful encode alone must not call acknowledge_written.
    pub fn prepare_acknowledgement(&mut self) -> Result<Message, HandshakeError> {
        if !matches!(self.state, State::AwaitingAcknowledgement(_)) {
            return Err(HandshakeError::State);
        }
        let State::AwaitingAcknowledgement(metadata) =
            std::mem::replace(&mut self.state, State::Failed)
        else {
            return Err(HandshakeError::State);
        };
        self.state = State::AcknowledgementPrepared(metadata);
        Ok(Message::Notification(Notification {
            method: "initialized".into(),
            params: None,
        }))
    }

    /// Trusted host confirms the entire ack was written/flushed in order. This
    /// records a host observation, not remote receipt or authenticated readiness.
    pub fn acknowledge_written(&mut self) -> Result<(), HandshakeError> {
        if !matches!(self.state, State::AcknowledgementPrepared(_)) {
            return Err(HandshakeError::State);
        }
        let State::AcknowledgementPrepared(metadata) =
            std::mem::replace(&mut self.state, State::Failed)
        else {
            return Err(HandshakeError::State);
        };
        self.state = State::Ready(metadata);
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.state, State::Ready(_))
    }
    pub fn metadata(&self) -> Option<&ServerMetadata> {
        match &self.state {
            State::Ready(metadata) => Some(metadata),
            _ => None,
        }
    }
    /// Disconnect, timeout or failed write. No automatic reinitialize or retry.
    pub fn fail(&mut self) {
        self.state = State::Failed;
    }
}

fn valid(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit
}

#[cfg(test)]
mod tests {
    use super::*;
    fn begin() -> Handshake {
        Handshake::begin(RequestId::Integer(1), "harness", "1", false)
            .unwrap()
            .0
    }
    fn reply() -> Message {
        Message::decode(br#"{"id":1,"result":{"userAgent":"codex/test","platformFamily":"unix","platformOs":"linux","codexHome":"/do-not-read"}}"#, 1024).unwrap()
    }

    #[test]
    fn readiness_requires_typed_matching_reply_then_ack_write() {
        let mut handshake = begin();
        assert!(!handshake.is_ready());
        assert_eq!(handshake.acknowledge_written(), Err(HandshakeError::State));
        assert!(handshake.prepare_acknowledgement().is_err());
        let wrong = Message::decode(br#"{"id":"1","result":{}}"#, 128).unwrap();
        assert_eq!(
            handshake.observe_reply(&wrong),
            Err(HandshakeError::Unexpected)
        );
        handshake.observe_reply(&reply()).unwrap();
        assert!(!handshake.is_ready());
        assert!(handshake.metadata().is_none());
        let ack = handshake.prepare_acknowledgement().unwrap();
        assert_eq!(
            ack.encode_line(128).unwrap(),
            b"{\"method\":\"initialized\"}\n"
        );
        assert!(!handshake.is_ready());
        assert!(handshake.prepare_acknowledgement().is_err());
        handshake.acknowledge_written().unwrap();
        assert_eq!(handshake.metadata().unwrap().platform_os(), "linux");
        assert!(handshake.is_ready());
        assert_eq!(handshake.acknowledge_written(), Err(HandshakeError::State));
        assert_eq!(
            handshake.observe_reply(&reply()),
            Err(HandshakeError::State)
        );
        handshake.fail();
        assert!(!handshake.is_ready());
        assert!(handshake.metadata().is_none());
    }

    #[test]
    fn errors_and_malformed_metadata_cannot_recover_to_ready() {
        for raw in [
            r#"{"id":1,"result":{}}"#,
            r#"{"id":1,"result":{"userAgent":"x","platformFamily":"","platformOs":"linux"}}"#,
            r#"{"id":1,"error":{"code":-1,"message":"rejected"}}"#,
        ] {
            let mut handshake = begin();
            assert!(
                handshake
                    .observe_reply(&Message::decode(raw.as_bytes(), 1024).unwrap())
                    .is_err()
            );
            assert_eq!(
                handshake.observe_reply(&reply()),
                Err(HandshakeError::State)
            );
            assert!(handshake.prepare_acknowledgement().is_err());
            assert!(!handshake.is_ready());
        }
        let mut handshake = begin();
        handshake.observe_reply(&reply()).unwrap();
        handshake.prepare_acknowledgement().unwrap();
        handshake.fail(); // Simulated ambiguous write/timeout, not no-dispatch.
        assert_eq!(handshake.acknowledge_written(), Err(HandshakeError::State));
    }

    #[test]
    fn explicit_capabilities_do_not_suppress_notifications_or_enable_attestation() {
        for experimental in [false, true] {
            let (_, message) =
                Handshake::begin(RequestId::Integer(2), "harness", "1", experimental).unwrap();
            let value: serde_json::Value =
                serde_json::from_slice(&message.encode_line(512).unwrap()).unwrap();
            assert_eq!(
                value["params"]["capabilities"],
                serde_json::json!({"experimentalApi":experimental,"requestAttestation":false})
            );
            assert_eq!(
                value["params"]["clientInfo"],
                serde_json::json!({"name":"harness","version":"1"})
            );
        }
        assert!(Handshake::begin(RequestId::String("".into()), "harness", "1", false).is_err());
        assert!(Handshake::begin(RequestId::Integer(1), " ", "1", false).is_err());
    }
}

//! Untrusted Codex app-server envelopes, not full JSON-RPC 2.0.
//! Classification never answers a server request or changes task authority.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Integer(i64),
}

// No Debug derive on payload-bearing types: normal diagnostics should not dump
// arbitrary tool, account, trace or model content.
#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Notification(Notification),
    Response(Response),
    Error(ErrorResponse),
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Value>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Notification {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub id: RequestId,
    pub result: Value,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    pub id: RequestId,
    pub error: RpcError,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("RPC byte limit must be between 1 byte and 16 MiB")]
    InvalidLimit,
    #[error("RPC frame exceeds byte limit")]
    TooLarge,
    #[error("malformed or unsupported RPC envelope")]
    Malformed,
}

impl Message {
    /// Decode one already-delimited frame; bound bytes before JSON allocations.
    /// Unknown methods are preserved for explicit dispatch/rejection by the host.
    /// Direct serde deserialization bypasses this byte/identifier policy.
    pub fn decode(frame: &[u8], max_bytes: usize) -> Result<Self, DecodeError> {
        if !(1..=16 * 1024 * 1024).contains(&max_bytes) {
            return Err(DecodeError::InvalidLimit);
        }
        if frame.len() > max_bytes {
            return Err(DecodeError::TooLarge);
        }
        let message: Self = serde_json::from_slice(frame).map_err(|_| DecodeError::Malformed)?;
        message.validate_labels()?;
        Ok(message)
    }

    /// Encode before writing to the transport. Cap includes LF. No delivery,
    /// permission or method-specific correctness is implied by these bytes.
    pub fn encode_line(&self, max_bytes: usize) -> std::io::Result<Vec<u8>> {
        use std::io::{Error, ErrorKind};
        if !(1..=16 * 1024 * 1024).contains(&max_bytes) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "invalid RPC output byte limit",
            ));
        }
        self.validate_labels()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "invalid RPC identifier or method"))?;
        crate::output::json_line(self, max_bytes)
    }

    fn validate_labels(&self) -> Result<(), DecodeError> {
        let (id, method) = match self {
            Self::Request(request) => (Some(&request.id), Some(request.method.as_str())),
            Self::Notification(notification) => (None, Some(notification.method.as_str())),
            Self::Response(response) => (Some(&response.id), None),
            Self::Error(response) => (Some(&response.id), None),
        };
        let valid_label = |s: &str| !s.trim().is_empty() && s.len() <= 256;
        if method.is_some_and(|s| !valid_label(s))
            || matches!(id, Some(RequestId::String(s)) if !valid_label(s))
        {
            return Err(DecodeError::Malformed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outbound_roundtrip_is_one_bounded_frame_for_all_message_families() {
        use crate::framing::LineDecoder;
        for fixture in [
            serde_json::json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"harness","version":"1"}}}),
            serde_json::json!({"method":"initialized"}),
            serde_json::json!({"id":"1","result":null}),
            serde_json::json!({"id":2,"error":{"code":-1,"message":"Tiếng Việt 🌱\n\t\"\\"}}),
            serde_json::json!({"id":3,"method":"future","trace":{"opaque":true}}),
        ] {
            let message = Message::decode(&serde_json::to_vec(&fixture).unwrap(), 1024).unwrap();
            let mut expected = serde_json::to_vec(&message).unwrap();
            expected.push(b'\n');
            assert_eq!(message.encode_line(expected.len()).unwrap(), expected);
            assert!(message.encode_line(expected.len() - 1).is_err());
            assert_eq!(expected.iter().filter(|b| **b == b'\n').count(), 1);
            let mut decoder = LineDecoder::new(expected.len()).unwrap();
            let (consumed, frame) = decoder.push(&expected).unwrap();
            assert_eq!(consumed, expected.len());
            let decoded = Message::decode(&frame.unwrap(), 1024).unwrap();
            assert_eq!(serde_json::to_value(decoded).unwrap(), fixture);
            assert_eq!(decoder.finish(), Ok(()));
        }
    }

    #[test]
    fn outbound_policy_rejects_invalid_constructed_messages() {
        let bad = Message::Request(Request {
            id: RequestId::Integer(1),
            method: " ".into(),
            params: None,
            trace: None,
        });
        assert!(bad.encode_line(1024).is_err());
        let bad_id = Message::Response(Response {
            id: RequestId::String("x".repeat(257)),
            result: Value::Null,
        });
        assert!(bad_id.encode_line(1024).is_err());
        let valid = Message::Notification(Notification {
            method: "initialized".into(),
            params: None,
        });
        for limit in [0, 16 * 1024 * 1024 + 1, usize::MAX] {
            assert!(valid.encode_line(limit).is_err());
        }
    }

    #[test]
    fn preserves_message_kind_id_type_and_null_result() {
        let Message::Request(request) = Message::decode(br#"{"id":"1","method":"future/approval","params":{"command":"test"},"trace":{"traceparent":"opaque"}}"#, 512).unwrap() else { panic!("request") };
        assert_eq!(request.id, RequestId::String("1".into()));
        assert!(request.trace.is_some());
        let Message::Response(response) =
            Message::decode(br#"{"id":1,"result":null}"#, 512).unwrap()
        else {
            panic!("response")
        };
        assert_eq!(response.id, RequestId::Integer(1));
        assert!(response.result.is_null());
        assert!(matches!(
            Message::decode(br#"{"method":"future/event"}"#, 512).unwrap(),
            Message::Notification(_)
        ));
        let Message::Error(error) = Message::decode(
            br#"{"id":-1,"error":{"code":-32601,"message":"unknown","data":null}}"#,
            512,
        )
        .unwrap() else {
            panic!("error")
        };
        assert_eq!(error.error.code, -32601);
    }

    #[test]
    fn ambiguous_or_invalid_envelopes_never_downgrade() {
        for bytes in [
            r#"{"id":null,"method":"approval"}"#,
            r#"{"id":1.5,"method":"approval"}"#,
            r#"{"id":1,"method":"approval","result":{}}"#,
            r#"{"id":1,"result":{},"error":{"code":1,"message":"x"}}"#,
            r#"{"id":1}"#,
            r#"{"method":"x","method":"y"}"#,
            r#"{"id":1,"id":2,"result":null}"#,
            r#"{"id":"","method":"x"}"#,
            r#"{"method":" "}"#,
            r#"{"jsonrpc":"2.0","method":"x"}"#,
            r#"{"id":1,"error":{"code":"1","message":"x"}}"#,
            r#"{"method":"x"} {}"#,
            "[]",
            "null",
            "{}",
        ] {
            assert!(
                matches!(
                    Message::decode(bytes.as_bytes(), 1024),
                    Err(DecodeError::Malformed)
                ),
                "{bytes}"
            );
        }
    }

    #[test]
    fn byte_cap_precedes_parsing_and_errors_do_not_echo_payload() {
        let bytes = br#"{"method":"x"}"#;
        assert!(Message::decode(bytes, bytes.len()).is_ok());
        assert!(matches!(
            Message::decode(bytes, bytes.len() - 1),
            Err(DecodeError::TooLarge)
        ));
        assert!(matches!(
            Message::decode(bytes, 0),
            Err(DecodeError::InvalidLimit)
        ));
        assert!(matches!(
            Message::decode(&[255], 8),
            Err(DecodeError::Malformed)
        ));
        assert!(matches!(
            Message::decode(b"secret", 2),
            Err(DecodeError::TooLarge)
        ));
        assert_eq!(
            DecodeError::Malformed.to_string(),
            "malformed or unsupported RPC envelope"
        );
    }

    #[test]
    fn framed_interleaving_preserves_server_requests_and_response_ids() {
        use crate::framing::LineDecoder;
        let stream = concat!(
            "{\"id\":7,\"result\":{}}\n",
            "{\"id\":\"7\",\"method\":\"item/commandExecution/requestApproval\",\"params\":{}}\n",
            "{\"method\":\"thread/closed\",\"params\":{\"threadId\":\"child\"}}\n",
            "{\"id\":8,\"error\":{\"code\":-1,\"message\":\"failed\"}}\n"
        );
        for chunk_size in [1, 7, stream.len()] {
            let mut decoder = LineDecoder::new(256).unwrap();
            let mut kinds = Vec::new();
            for chunk in stream.as_bytes().chunks(chunk_size) {
                let mut rest = chunk;
                while !rest.is_empty() {
                    let (consumed, frame) = decoder.push(rest).unwrap();
                    rest = &rest[consumed..];
                    if let Some(frame) = frame {
                        kinds.push(match Message::decode(&frame, 256).unwrap() {
                            Message::Response(response) => {
                                assert_eq!(response.id, RequestId::Integer(7));
                                "response"
                            }
                            Message::Request(request) => {
                                assert_eq!(request.id, RequestId::String("7".into()));
                                "request"
                            }
                            Message::Notification(_) => "notification",
                            Message::Error(response) => {
                                assert_eq!(response.id, RequestId::Integer(8));
                                "error"
                            }
                        });
                    }
                }
            }
            assert_eq!(kinds, ["response", "request", "notification", "error"]);
            assert_eq!(decoder.finish(), Ok(()));
        }
    }
}

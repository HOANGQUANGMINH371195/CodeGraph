//! Connection-owned RPC accounting, not dispatch or authenticated authority.
use std::collections::BTreeMap;

use crate::rpc::{Message, RequestId};

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CorrelationError {
    #[error("invalid connection epoch, method or pending limit")]
    Invalid,
    #[error("request table is closed")]
    Closed,
    #[error("pending limit or request ID space exhausted")]
    Capacity,
    #[error("unknown pending request")]
    Unknown,
    #[error("message belongs to a different connection epoch")]
    EpochMismatch,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PendingRequest {
    method: String,
    uncertain: bool,
}

impl PendingRequest {
    pub fn method(&self) -> &str {
        &self.method
    }
    pub fn uncertain(&self) -> bool {
        self.uncertain
    }
}

/// Payloads deliberately have no Debug implementation.
pub enum RoutedMessage {
    Matched {
        pending: PendingRequest,
        reply: Message,
    },
    UnmatchedReply(Message),
    ServerRequest(Message),
    Notification(Message),
}

/// Never Clone or reuse an epoch for another connection. The host authenticates
/// the transport and attaches the epoch; it must not trust an epoch from JSON.
pub struct PendingRequests {
    epoch: String,
    limit: usize,
    next_id: Option<i64>,
    closed: bool,
    pending: BTreeMap<i64, PendingRequest>,
}

/// Immutable end-of-accounting snapshot. This does not prove process exit,
/// whether requests executed, or permission to retry. No wire payload is kept.
pub struct TerminalRequests {
    epoch: String,
    pending: BTreeMap<i64, PendingRequest>,
}

impl std::fmt::Debug for TerminalRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalRequests")
            .field("unresolved_count", &self.pending.len())
            .finish_non_exhaustive()
    }
}

impl TerminalRequests {
    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    /// Original IDs in ascending order; all entries are uncertain.
    pub fn unresolved(&self) -> impl ExactSizeIterator<Item = (i64, &PendingRequest)> {
        self.pending.iter().map(|(id, pending)| (*id, pending))
    }
}

impl PendingRequests {
    /// End correlation permanently. Buffered replies must be processed before
    /// sealing if they are to count; sealing never fabricates their outcomes.
    pub fn into_terminal(mut self) -> TerminalRequests {
        self.close();
        TerminalRequests {
            epoch: self.epoch,
            pending: self.pending,
        }
    }

    pub fn new(epoch: &str, limit: usize) -> Result<Self, CorrelationError> {
        if !valid_label(epoch) || !(1..=4096).contains(&limit) {
            return Err(CorrelationError::Invalid);
        }
        Ok(Self {
            epoch: epoch.into(),
            limit,
            next_id: Some(1),
            closed: false,
            pending: BTreeMap::new(),
        })
    }

    /// Reserve before writing. This does not send or authorize the request.
    pub fn reserve(&mut self, method: &str) -> Result<RequestId, CorrelationError> {
        if !valid_label(method) {
            return Err(CorrelationError::Invalid);
        }
        if self.closed {
            return Err(CorrelationError::Closed);
        }
        if self.pending.len() >= self.limit {
            return Err(CorrelationError::Capacity);
        }
        let id = self.next_id.ok_or(CorrelationError::Capacity)?;
        self.next_id = id.checked_add(1);
        self.pending.insert(
            id,
            PendingRequest {
                method: method.into(),
                uncertain: false,
            },
        );
        Ok(RequestId::Integer(id))
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Read-only unresolved metadata for reconciliation/checkpointing by host.
    /// Iterating does not prove whether the server executed any request.
    pub fn unresolved(&self) -> impl Iterator<Item = (i64, &PendingRequest)> {
        self.pending.iter().map(|(id, pending)| (*id, pending))
    }

    /// Timeout or ambiguous write: retain correlation, never infer no dispatch.
    pub fn mark_uncertain(&mut self, id: &RequestId) -> Result<bool, CorrelationError> {
        let RequestId::Integer(id) = id else {
            return Err(CorrelationError::Unknown);
        };
        let pending = self.pending.get_mut(id).ok_or(CorrelationError::Unknown)?;
        let changed = !pending.uncertain;
        pending.uncertain = true;
        Ok(changed)
    }

    /// Stop new requests, preserving unresolved records for host reconciliation.
    /// Buffered replies from this epoch may still arrive; no retry is performed.
    pub fn close(&mut self) {
        self.closed = true;
        for pending in self.pending.values_mut() {
            pending.uncertain = true;
        }
    }

    /// Removal completes RPC correlation only. Validate the reply against its
    /// method; neither a success envelope nor a matching ID proves task success.
    pub fn receive(
        &mut self,
        epoch: &str,
        message: Message,
    ) -> Result<RoutedMessage, CorrelationError> {
        if epoch != self.epoch {
            return Err(CorrelationError::EpochMismatch);
        }
        let id = match &message {
            Message::Request(_) => return Ok(RoutedMessage::ServerRequest(message)),
            Message::Notification(_) => return Ok(RoutedMessage::Notification(message)),
            Message::Response(response) => &response.id,
            Message::Error(response) => &response.id,
        };
        let pending = match id {
            RequestId::Integer(id) => self.pending.remove(id),
            RequestId::String(_) => None,
        };
        Ok(match pending {
            Some(pending) => RoutedMessage::Matched {
                pending,
                reply: message,
            },
            None => RoutedMessage::UnmatchedReply(message),
        })
    }
}

fn valid_label(label: &str) -> bool {
    !label.trim().is_empty() && label.len() <= 256
}

#[cfg(test)]
mod tests {
    use super::*;
    fn decode(text: &str) -> Message {
        Message::decode(text.as_bytes(), 1024).unwrap()
    }

    #[test]
    fn out_of_order_error_and_success_match_once_without_id_reuse() {
        let mut table = PendingRequests::new("connection-1", 2).unwrap();
        assert_eq!(table.reserve("initialize"), Ok(RequestId::Integer(1)));
        assert_eq!(table.reserve("thread/read"), Ok(RequestId::Integer(2)));
        let RoutedMessage::Matched { pending, reply } = table
            .receive(
                "connection-1",
                decode(r#"{"id":2,"error":{"code":-1,"message":"failed"}}"#),
            )
            .unwrap()
        else {
            panic!("match")
        };
        assert_eq!(pending.method(), "thread/read");
        assert!(!pending.uncertain());
        assert!(matches!(reply, Message::Error(_)));
        assert!(matches!(
            table
                .receive("connection-1", decode(r#"{"id":2,"result":{}}"#))
                .unwrap(),
            RoutedMessage::UnmatchedReply(_)
        ));
        assert_eq!(table.reserve("thread/start"), Ok(RequestId::Integer(3)));
        assert!(matches!(
            table
                .receive("connection-1", decode(r#"{"id":1,"result":{}}"#))
                .unwrap(),
            RoutedMessage::Matched { .. }
        ));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn server_requests_typed_id_collisions_and_stale_epochs_do_not_consume_pending() {
        let mut table = PendingRequests::new("current", 1).unwrap();
        table.reserve("initialize").unwrap();
        assert!(matches!(
            table
                .receive("current", decode(r#"{"id":1,"method":"approval"}"#))
                .unwrap(),
            RoutedMessage::ServerRequest(_)
        ));
        assert!(matches!(
            table
                .receive(
                    "current",
                    decode(r#"{"method":"thread/closed","params":{"threadId":"x"}}"#)
                )
                .unwrap(),
            RoutedMessage::Notification(_)
        ));
        assert!(matches!(
            table
                .receive("current", decode(r#"{"id":"1","result":null}"#))
                .unwrap(),
            RoutedMessage::UnmatchedReply(_)
        ));
        assert!(matches!(
            table.receive("old", decode(r#"{"id":1,"result":null}"#)),
            Err(CorrelationError::EpochMismatch)
        ));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn uncertain_and_closed_requests_retain_slots_until_late_reply() {
        let mut table = PendingRequests::new("current", 1).unwrap();
        let id = table.reserve("turn/start").unwrap();
        assert_eq!(table.mark_uncertain(&id), Ok(true));
        assert_eq!(table.mark_uncertain(&id), Ok(false));
        assert_eq!(table.reserve("turn/start"), Err(CorrelationError::Capacity));
        table.close();
        let unresolved: Vec<_> = table.unresolved().collect();
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].0, 1);
        assert_eq!(unresolved[0].1.method(), "turn/start");
        assert!(unresolved[0].1.uncertain());
        assert_eq!(table.reserve("turn/start"), Err(CorrelationError::Closed));
        assert_eq!(table.len(), 1);
        let RoutedMessage::Matched { pending, .. } = table
            .receive("current", decode(r#"{"id":1,"result":null}"#))
            .unwrap()
        else {
            panic!("match")
        };
        assert!(pending.uncertain());
        assert!(table.is_empty());
        assert_eq!(table.mark_uncertain(&id), Err(CorrelationError::Unknown));
    }

    #[test]
    fn invalid_inputs_and_id_exhaustion_fail_without_wraparound() {
        for (epoch, limit) in [("", 1), ("x", 0), ("x", 4097)] {
            assert!(matches!(
                PendingRequests::new(epoch, limit),
                Err(CorrelationError::Invalid)
            ));
        }
        let mut table = PendingRequests::new("current", 2).unwrap();
        assert_eq!(table.reserve(" "), Err(CorrelationError::Invalid));
        assert!(table.is_empty());
        table.next_id = Some(i64::MAX);
        assert_eq!(table.reserve("last"), Ok(RequestId::Integer(i64::MAX)));
        assert_eq!(table.reserve("overflow"), Err(CorrelationError::Capacity));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn terminal_seal_preserves_only_unresolved_ids_and_redacts_debug() {
        let mut table = PendingRequests::new("secret-epoch", 3).unwrap();
        table.reserve("secret-method").unwrap();
        table.reserve("finished").unwrap();
        table.reserve("third\nmethod").unwrap();
        table.close();
        table
            .receive("secret-epoch", decode(r#"{"id":2,"result":null}"#))
            .unwrap();
        let terminal = table.into_terminal();
        assert_eq!(terminal.epoch(), "secret-epoch");
        let entries: Vec<_> = terminal.unresolved().collect();
        assert_eq!(
            entries.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [1, 3]
        );
        assert_eq!(entries[0].1.method(), "secret-method");
        assert_eq!(entries[1].1.method(), "third\nmethod");
        assert!(entries.iter().all(|(_, pending)| pending.uncertain()));
        assert_eq!(
            format!("{terminal:?}"),
            "TerminalRequests { unresolved_count: 2, .. }"
        );
    }

    #[test]
    fn terminal_seal_handles_empty_capacity_and_maximum_id_without_reallocation_of_ids() {
        assert_eq!(
            PendingRequests::new("empty", 1)
                .unwrap()
                .into_terminal()
                .unresolved()
                .len(),
            0
        );
        let mut full = PendingRequests::new("full", 4096).unwrap();
        for _ in 0..4096 {
            full.reserve("request").unwrap();
        }
        let terminal = full.into_terminal();
        assert_eq!(terminal.unresolved().len(), 4096);
        assert!(terminal.unresolved().all(|(_, p)| p.uncertain()));
        assert_eq!(terminal.unresolved().last().unwrap().0, 4096);
        let mut last = PendingRequests::new("last", 1).unwrap();
        last.next_id = Some(i64::MAX);
        last.reserve("last").unwrap();
        assert_eq!(
            last.into_terminal().unresolved().next().unwrap().0,
            i64::MAX
        );
    }
}

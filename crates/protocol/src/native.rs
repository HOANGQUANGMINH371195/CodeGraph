//! Narrow, untrusted projection of Codex app-server lifecycle notifications.
//! Not authentication, a full JSON-RPC router, or permission to release slots.
//! The host must bound frames before parsing and reconcile runtime generations:
//! a closed thread can later resume with the same ID.
use serde::Deserialize;

/// A syntactically validated ID, not proof of native ownership or existence.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct NativeId(String);

impl TryFrom<String> for NativeId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() || value.len() > 256 {
            return Err("native identifier must be nonblank and at most 256 bytes");
        }
        Ok(Self(value))
    }
}

impl NativeId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Supported notification subset. Requests (with `id`) and unknown methods are
/// rejected, not consumed as lifecycle observations. Payload projections ignore
/// unrelated fields; they must never be serialized as lossless wire messages.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "method", content = "params", deny_unknown_fields)]
pub enum LifecycleNotification {
    #[serde(rename = "thread/closed")]
    ThreadClosed(ThreadIdentity),
    #[serde(rename = "thread/status/changed")]
    ThreadStatusChanged(ThreadStatusChanged),
    #[serde(rename = "turn/started")]
    TurnStarted(TurnStarted),
    #[serde(rename = "turn/completed")]
    TurnCompleted(TurnCompleted),
}

impl LifecycleNotification {
    pub fn thread_id(&self) -> &NativeId {
        match self {
            Self::ThreadClosed(event) => &event.thread_id,
            Self::ThreadStatusChanged(event) => &event.thread_id,
            Self::TurnStarted(event) => &event.thread_id,
            Self::TurnCompleted(event) => &event.thread_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadIdentity {
    pub thread_id: NativeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStatusChanged {
    pub thread_id: NativeId,
    pub status: ThreadStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ThreadStatus {
    NotLoaded,
    Idle,
    SystemError,
    #[serde(rename_all = "camelCase")]
    Active {
        active_flags: Vec<ActiveFlag>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActiveFlag {
    WaitingOnApproval,
    WaitingOnUserInput,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStarted {
    pub thread_id: NativeId,
    pub turn: StartedTurn,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct StartedTurn {
    pub id: NativeId,
    pub status: StartedStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartedStatus {
    InProgress,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCompleted {
    pub thread_id: NativeId,
    pub turn: CompletedTurn,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CompletedTurn {
    pub id: NativeId,
    pub status: CompletedStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompletedStatus {
    Completed,
    Interrupted,
    Failed,
}

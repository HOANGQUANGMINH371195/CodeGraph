//! Wire observations only; decoding never creates verified execution authority.
use crate::{ProtocolError, SCHEMA_VERSION, require_current_schema};
use graph_domain::execution as domain;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Exited,
    Cancelled,
    TimedOut,
    OutputLimit,
    SpawnFailed,
    HostError,
}

impl From<StopReason> for domain::StopReason {
    fn from(value: StopReason) -> Self {
        match value {
            StopReason::Exited => Self::Exited,
            StopReason::Cancelled => Self::Cancelled,
            StopReason::TimedOut => Self::TimedOut,
            StopReason::OutputLimit => Self::OutputLimit,
            StopReason::SpawnFailed => Self::SpawnFailed,
            StopReason::HostError => Self::HostError,
        }
    }
}
impl From<domain::StopReason> for StopReason {
    fn from(value: domain::StopReason) -> Self {
        match value {
            domain::StopReason::Exited => Self::Exited,
            domain::StopReason::Cancelled => Self::Cancelled,
            domain::StopReason::TimedOut => Self::TimedOut,
            domain::StopReason::OutputLimit => Self::OutputLimit,
            domain::StopReason::SpawnFailed => Self::SpawnFailed,
            domain::StopReason::HostError => Self::HostError,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamCompletion {
    Complete,
    Truncated,
    ReadFailed,
    Incomplete,
}

impl From<StreamCompletion> for domain::StreamCompletion {
    fn from(value: StreamCompletion) -> Self {
        match value {
            StreamCompletion::Complete => Self::Complete,
            StreamCompletion::Truncated => Self::Truncated,
            StreamCompletion::ReadFailed => Self::ReadFailed,
            StreamCompletion::Incomplete => Self::Incomplete,
        }
    }
}
impl From<domain::StreamCompletion> for StreamCompletion {
    fn from(value: domain::StreamCompletion) -> Self {
        match value {
            domain::StreamCompletion::Complete => Self::Complete,
            domain::StreamCompletion::Truncated => Self::Truncated,
            domain::StreamCompletion::ReadFailed => Self::ReadFailed,
            domain::StreamCompletion::Incomplete => Self::Incomplete,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeCleanup {
    Complete,
    Incomplete,
    Unverifiable,
}

impl From<ScopeCleanup> for domain::ScopeCleanup {
    fn from(value: ScopeCleanup) -> Self {
        match value {
            ScopeCleanup::Complete => Self::Complete,
            ScopeCleanup::Incomplete => Self::Incomplete,
            ScopeCleanup::Unverifiable => Self::Unverifiable,
        }
    }
}
impl From<domain::ScopeCleanup> for ScopeCleanup {
    fn from(value: domain::ScopeCleanup) -> Self {
        match value {
            domain::ScopeCleanup::Complete => Self::Complete,
            domain::ScopeCleanup::Incomplete => Self::Incomplete,
            domain::ScopeCleanup::Unverifiable => Self::Unverifiable,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChildCompletion {
    NotSpawned {},
    Unreaped {},
    Reaped { exit_code: Option<i32> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionCompletion {
    pub schema_version: u32,
    pub reason: StopReason,
    pub child: ChildCompletion,
    pub stdout: StreamCompletion,
    pub stderr: StreamCompletion,
    pub cleanup: ScopeCleanup,
}

impl ExecutionCompletion {
    pub fn try_into_domain(self) -> Result<domain::ExecutionCompletion, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(domain::ExecutionCompletion {
            reason: self.reason.into(),
            child: match self.child {
                ChildCompletion::NotSpawned {} => domain::ChildCompletion::NotSpawned,
                ChildCompletion::Unreaped {} => domain::ChildCompletion::Unreaped,
                ChildCompletion::Reaped { exit_code } => {
                    domain::ChildCompletion::Reaped { exit_code }
                }
            },
            stdout: self.stdout.into(),
            stderr: self.stderr.into(),
            cleanup: self.cleanup.into(),
        })
    }
}
impl From<&domain::ExecutionCompletion> for ExecutionCompletion {
    fn from(value: &domain::ExecutionCompletion) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            reason: value.reason.into(),
            child: match value.child {
                domain::ChildCompletion::NotSpawned => ChildCompletion::NotSpawned {},
                domain::ChildCompletion::Unreaped => ChildCompletion::Unreaped {},
                domain::ChildCompletion::Reaped { exit_code } => {
                    ChildCompletion::Reaped { exit_code }
                }
            },
            stdout: value.stdout.into(),
            stderr: value.stderr.into(),
            cleanup: value.cleanup.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn wire_rejects_missing_unknown_and_future_contracts() {
        let valid = json!({"schema_version":1, "reason":"exited",
            "child":{"state":"reaped","exit_code":0}, "stdout":"complete",
            "stderr":"complete","cleanup":"complete"});
        let wire: ExecutionCompletion = serde_json::from_value(valid.clone()).unwrap();
        let report = wire.try_into_domain().unwrap();
        assert_eq!(
            serde_json::to_value(ExecutionCompletion::from(&report)).unwrap(),
            valid
        );
        for key in [
            "schema_version",
            "reason",
            "child",
            "stdout",
            "stderr",
            "cleanup",
        ] {
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(
                serde_json::from_value::<ExecutionCompletion>(missing).is_err(),
                "{key}"
            );
        }
        let mut future: ExecutionCompletion = serde_json::from_value(valid.clone()).unwrap();
        future.schema_version = 2;
        assert!(future.try_into_domain().is_err());
        let mut extra = valid.clone();
        extra["verified"] = json!(true);
        assert!(serde_json::from_value::<ExecutionCompletion>(extra).is_err());
        for key in ["reason", "stdout", "stderr", "cleanup"] {
            let mut unknown = valid.clone();
            unknown[key] = json!("future_state");
            assert!(serde_json::from_value::<ExecutionCompletion>(unknown).is_err());
        }
        let mut malformed = valid.clone();
        malformed["child"] = json!({"state":"unreaped","exit_code":0});
        assert!(serde_json::from_value::<ExecutionCompletion>(malformed).is_err());
        // Omitted exit status is unknown, never an implicit zero.
        let mut unknown_exit = valid;
        unknown_exit["child"] = json!({"state":"reaped"});
        let wire: ExecutionCompletion = serde_json::from_value(unknown_exit).unwrap();
        assert_eq!(
            wire.try_into_domain().unwrap().reported_check_outcome(),
            graph_domain::CheckOutcome::Unknown
        );
    }
}

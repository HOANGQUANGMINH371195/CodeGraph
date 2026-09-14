use crate::{DomainError, RpcLaunchSpec};

/// Reported spawn-stage outcome, never current process state or terminal RPC success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RpcSpawnDisposition {
    CancelledBeforeSpawn,
    ExpiredBeforeSpawn,
    SpawnFailed,
    Spawned,
}
impl RpcSpawnDisposition {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CancelledBeforeSpawn => "cancelled_before_spawn",
            Self::ExpiredBeforeSpawn => "expired_before_spawn",
            Self::SpawnFailed => "spawn_failed",
            Self::Spawned => "spawned",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcSpawnObservation {
    launch: RpcLaunchSpec,
    observed_at_ms: i64,
    disposition: RpcSpawnDisposition,
    process_id: Option<u32>,
}
impl RpcSpawnObservation {
    /// Records one bounded observation of the spawn stage.
    ///
    /// # Errors
    /// Returns an error when the timestamp is negative or the disposition and
    /// optional process identifier disagree.
    pub fn new(
        launch: RpcLaunchSpec,
        observed_at_ms: i64,
        disposition: RpcSpawnDisposition,
        process_id: Option<u32>,
    ) -> Result<Self, DomainError> {
        if observed_at_ms < 0 {
            return Err(DomainError::Invalid(
                "spawn observation time must be nonnegative",
            ));
        }
        if match disposition {
            RpcSpawnDisposition::Spawned => !matches!(process_id, Some(pid) if pid > 0),
            _ => process_id.is_some(),
        } {
            return Err(DomainError::Invalid(
                "spawn disposition and process ID disagree",
            ));
        }
        Ok(Self {
            launch,
            observed_at_ms,
            disposition,
            process_id,
        })
    }
    #[must_use]
    pub fn launch(&self) -> &RpcLaunchSpec {
        &self.launch
    }
    #[must_use]
    pub fn observed_at_ms(&self) -> i64 {
        self.observed_at_ms
    }
    #[must_use]
    pub fn disposition(&self) -> RpcSpawnDisposition {
        self.disposition
    }
    /// Historical numeric hint only. Never use this alone for liveness, attach or kill.
    #[must_use]
    pub fn process_id(&self) -> Option<u32> {
        self.process_id
    }
}

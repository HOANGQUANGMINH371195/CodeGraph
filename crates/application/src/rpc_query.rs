use graph_domain::{DomainError, RpcLaunchSpec, RpcSpawnObservation, RpcTerminalReceipt, TaskSpec};
use std::error::Error;

/// Historical accounting only. Neither a missing nor a present claim establishes
/// process state, successful delivery, cleanup, current authority or safe retry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcLaunchLedgerSnapshot {
    spec: RpcLaunchSpec,
    claimed_at_ms: Option<i64>,
    spawn_observation: Option<RpcSpawnObservation>,
    terminal_receipt: Option<RpcTerminalReceipt>,
}
impl RpcLaunchLedgerSnapshot {
    pub fn new(
        spec: RpcLaunchSpec,
        claimed_at_ms: Option<i64>,
        spawn_observation: Option<RpcSpawnObservation>,
    ) -> Result<Self, DomainError> {
        if claimed_at_ms.is_some_and(|at| at < 0) {
            return Err(DomainError::Invalid(
                "RPC claim timestamp must be nonnegative",
            ));
        }
        if spawn_observation
            .as_ref()
            .is_some_and(|observation| claimed_at_ms.is_none() || observation.launch() != &spec)
        {
            return Err(DomainError::Invalid(
                "RPC spawn observation requires matching claimed launch",
            ));
        }
        Ok(Self {
            spec,
            claimed_at_ms,
            spawn_observation,
            terminal_receipt: None,
        })
    }
    pub fn spec(&self) -> &RpcLaunchSpec {
        &self.spec
    }
    pub fn claimed_at_ms(&self) -> Option<i64> {
        self.claimed_at_ms
    }
    /// Missing report is unknown, not evidence that no spawn occurred.
    pub fn spawn_observation(&self) -> Option<&RpcSpawnObservation> {
        self.spawn_observation.as_ref()
    }
    /// Attach historical terminal accounting from the same repository snapshot.
    /// This checks linkage, not blob availability or process liveness.
    pub fn with_terminal_receipt(
        mut self,
        receipt: Option<RpcTerminalReceipt>,
    ) -> Result<Self, DomainError> {
        if receipt
            .as_ref()
            .is_some_and(|r| Some(r.spawn()) != self.spawn_observation.as_ref())
        {
            return Err(DomainError::Invalid(
                "RPC terminal receipt requires matching spawn",
            ));
        }
        self.terminal_receipt = receipt;
        Ok(self)
    }
    pub fn terminal_receipt(&self) -> Option<&RpcTerminalReceipt> {
        self.terminal_receipt.as_ref()
    }
}

/// Queries must not register, claim, refresh a lease or reset one-shot accounting.
pub trait RpcLaunchQueryRepository {
    type Error: Error + Send + Sync + 'static;
    /// Exact complete task filtering; None also covers a different caller scope.
    /// The returned data must be from one consistent repository snapshot.
    /// Present terminal receipts require exact stored spawn, analysis run and
    /// artifact metadata linkage; corrupt references are errors, not absence.
    /// This contract does not verify blob contents.
    fn rpc_launch_snapshot(
        &self,
        id: &str,
        task: &TaskSpec,
    ) -> Result<Option<RpcLaunchLedgerSnapshot>, Self::Error>;
}

use crate::{Store, StoreError};
use graph_application::RpcSpawnObservationRepository;
use graph_domain::{EventKind, RpcLaunchSpec, RpcSpawnObservation};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

fn claim(connection: &Connection, id: &str) -> Result<Option<i64>, StoreError> {
    let at: Option<i64> = connection
        .query_row(include_str!("sql/select_rpc_launch_claim.sql"), [id], |r| {
            r.get(0)
        })
        .optional()?;
    if at.is_some_and(|at| at < 0) {
        return Err(StoreError::Corrupt("invalid RPC claim timestamp".into()));
    }
    Ok(at)
}

// Values must come from one repository snapshot: a joined statement or a read
// transaction. Invalid storage is never silently interpreted as no outcome.
pub(crate) fn decode(
    id: &str,
    at: i64,
    raw: &str,
    launch: &RpcLaunchSpec,
    claimed: Option<i64>,
) -> Result<RpcSpawnObservation, StoreError> {
    let corrupt = || StoreError::Corrupt("invalid RPC spawn observation or linkage".into());
    let wire: graph_protocol::RpcSpawnObservation =
        serde_json::from_str(raw).map_err(|_| corrupt())?;
    let observation = wire.try_into_domain().map_err(|_| corrupt())?;
    if observation.launch() != launch
        || observation.launch().id() != id
        || observation.observed_at_ms() != at
        || !claimed.is_some_and(|at| at >= 0)
    {
        return Err(corrupt());
    }
    Ok(observation)
}

// This multi-statement path requires the caller's transaction.
pub(crate) fn read(
    connection: &Connection,
    id: &str,
) -> Result<Option<RpcSpawnObservation>, StoreError> {
    let row: Option<(i64, String)> = connection
        .query_row(
            include_str!("sql/select_rpc_spawn_observation.sql"),
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(at, raw)| {
        let corrupt = || StoreError::Corrupt("invalid RPC spawn observation or linkage".into());
        let launch = crate::rpc_launch::read(connection, id)?.ok_or_else(corrupt)?;
        decode(id, at, &raw, &launch, claim(connection, id)?)
    })
    .transpose()
}

impl RpcSpawnObservationRepository for Store {
    type Error = StoreError;
    fn record_rpc_spawn_observation(
        &mut self,
        observation: &RpcSpawnObservation,
    ) -> Result<bool, StoreError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let launch = observation.launch();
        let stored = crate::rpc_launch::read(&tx, launch.id())?.ok_or(StoreError::Unavailable)?;
        if &stored != launch {
            return Err(StoreError::Conflict);
        }
        if claim(&tx, launch.id())?.is_none() {
            return Err(StoreError::Unavailable);
        }
        if let Some(previous) = read(&tx, launch.id())? {
            if previous != *observation {
                return Err(StoreError::Conflict);
            }
            tx.commit()?;
            return Ok(false);
        }
        let raw = serde_json::to_string(&graph_protocol::RpcSpawnObservation::from(observation))?;
        tx.execute(
            include_str!("sql/insert_rpc_spawn_observation.sql"),
            params![launch.id(), observation.observed_at_ms(), raw],
        )?;
        tx.execute(
            include_str!("sql/insert_task_event.sql"),
            params![
                launch.task().id().as_str(),
                EventKind::RpcSpawnObserved.as_str(),
                observation.observed_at_ms(),
                launch.id()
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }
    fn rpc_spawn_observation(
        &self,
        launch: &RpcLaunchSpec,
    ) -> Result<Option<RpcSpawnObservation>, StoreError> {
        // Read-only deferred transaction; no lease refresh or new claim.
        let tx = self.0.unchecked_transaction()?;
        let result = read(&tx, launch.id())?.filter(|value| value.launch() == launch);
        tx.commit()?;
        Ok(result)
    }
}

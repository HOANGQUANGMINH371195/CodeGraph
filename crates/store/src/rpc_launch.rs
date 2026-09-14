use crate::{Store, StoreError};
use graph_application::{RpcLaunchLedgerSnapshot, RpcLaunchQueryRepository, RpcLaunchRepository};
use graph_domain::{EventKind, RpcLaunchSpec, TaskSpec};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

type LaunchRow = (String, String, String, i64, String, Option<String>);

fn decode(
    id: &str,
    (task_id, host, epoch, fence, raw, task_raw): LaunchRow,
) -> Result<RpcLaunchSpec, StoreError> {
    let corrupt = || StoreError::Corrupt("invalid RPC launch descriptor or linkage".into());
    let wire: graph_protocol::RpcLaunchSpec = serde_json::from_str(&raw).map_err(|_| corrupt())?;
    let spec = wire.try_into_domain().map_err(|_| corrupt())?;
    let wire: graph_protocol::TaskSpec =
        serde_json::from_str(&task_raw.ok_or_else(corrupt)?).map_err(|_| corrupt())?;
    let task = wire.try_into_domain().map_err(|_| corrupt())?;
    if spec.id() != id
        || spec.task() != &task
        || task.id().as_str() != task_id
        || spec.host_id() != host
        || spec.connection().epoch() != epoch
        || spec.origin_lease().fencing_token() != fence
    {
        return Err(corrupt());
    }
    Ok(spec)
}

pub(crate) fn read(connection: &Connection, id: &str) -> Result<Option<RpcLaunchSpec>, StoreError> {
    let row: Option<LaunchRow> = connection
        .query_row(include_str!("sql/select_rpc_launch.sql"), [id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .optional()?;
    row.map(|row| decode(id, row)).transpose()
}

impl RpcLaunchQueryRepository for Store {
    type Error = StoreError;
    fn rpc_launch_snapshot(
        &self,
        id: &str,
        task: &TaskSpec,
    ) -> Result<Option<RpcLaunchLedgerSnapshot>, StoreError> {
        if id.trim().is_empty() {
            return Err(StoreError::Invalid("RPC launch ID is required"));
        }
        // All launch and terminal linkage reads share one deferred snapshot.
        // No lease, claim, event or receipt mutation is performed.
        let tx = self.0.unchecked_transaction()?;
        type SnapshotRow = (LaunchRow, Option<i64>, Option<(String, i64, String)>);
        let row: Option<SnapshotRow> = tx
            .query_row(
                include_str!("sql/select_rpc_launch_snapshot.sql"),
                [id],
                |r| {
                    Ok((
                        (
                            r.get(0)?,
                            r.get(1)?,
                            r.get(2)?,
                            r.get(3)?,
                            r.get(4)?,
                            r.get(5)?,
                        ),
                        r.get(6)?,
                        match r.get::<_, Option<String>>(7)? {
                            Some(id) => Some((id, r.get(8)?, r.get(9)?)),
                            None => None,
                        },
                    ))
                },
            )
            .optional()?;
        let terminal = crate::rpc_terminal::read(&tx, id)?;
        let result = row
            .map(|(row, claimed, spawn)| {
                let spec = decode(id, row)?;
                let observation = spawn
                    .map(|(spawn_id, at, raw)| {
                        crate::rpc_spawn::decode(&spawn_id, at, &raw, &spec, claimed)
                    })
                    .transpose()?;
                let snapshot = RpcLaunchLedgerSnapshot::new(spec, claimed, observation)
                    .and_then(|s| s.with_terminal_receipt(terminal))
                    .map_err(|_| {
                        StoreError::Corrupt("invalid RPC launch ledger snapshot".into())
                    })?;
                Ok::<_, StoreError>((snapshot.spec().task() == task).then_some(snapshot))
            })
            .transpose()
            .map(Option::flatten)?;
        tx.commit()?;
        Ok(result)
    }
}

fn current_origin(
    connection: &Connection,
    spec: &RpcLaunchSpec,
    now_ms: i64,
) -> Result<(), StoreError> {
    type Row = (String, String, Option<String>, i64, Option<i64>);
    let row: Row = connection
        .query_row(
            include_str!("sql/select_rpc_origin_lease.sql"),
            [spec.task().id().as_str()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?
        .ok_or(StoreError::Unavailable)?;
    let (raw, state, owner, fence, expires) = row;
    let corrupt = || StoreError::Corrupt("invalid RPC origin task".into());
    let wire: graph_protocol::TaskSpec = serde_json::from_str(&raw).map_err(|_| corrupt())?;
    let task = wire.try_into_domain().map_err(|_| corrupt())?;
    if &task != spec.task() {
        return Err(StoreError::Conflict);
    }
    let lease = spec.origin_lease();
    if state != "leased"
        || owner.as_deref() != Some(lease.owner().as_str())
        || fence != lease.fencing_token()
        || expires != Some(lease.expires_at_ms())
        || lease.expires_at_ms() <= now_ms
    {
        return Err(StoreError::Unavailable);
    }
    Ok(())
}

fn check_time(now_ms: i64) -> Result<(), StoreError> {
    if now_ms < 0 {
        Err(StoreError::Invalid("RPC ledger time must be nonnegative"))
    } else {
        Ok(())
    }
}

impl RpcLaunchRepository for Store {
    type Error = StoreError;
    fn register_rpc_launch(
        &mut self,
        spec: &RpcLaunchSpec,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        check_time(now_ms)?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = read(&tx, spec.id())? {
            if existing != *spec {
                return Err(StoreError::Conflict);
            }
            tx.commit()?;
            return Ok(false);
        }
        current_origin(&tx, spec, now_ms)?;
        let alias = tx
            .query_row(
                include_str!("sql/select_rpc_launch_alias.sql"),
                params![
                    spec.task().id().as_str(),
                    spec.origin_lease().fencing_token(),
                    spec.host_id(),
                    spec.connection().epoch()
                ],
                |_| Ok(()),
            )
            .optional()?;
        if alias.is_some() {
            return Err(StoreError::Conflict);
        }
        let descriptor = serde_json::to_string(&graph_protocol::RpcLaunchSpec::from(spec))?;
        tx.execute(
            include_str!("sql/insert_rpc_launch.sql"),
            params![
                spec.id(),
                spec.task().id().as_str(),
                spec.host_id(),
                spec.connection().epoch(),
                spec.origin_lease().fencing_token(),
                descriptor
            ],
        )?;
        tx.execute(
            include_str!("sql/insert_task_event.sql"),
            params![
                spec.task().id().as_str(),
                EventKind::RpcLaunchRegistered.as_str(),
                now_ms,
                spec.id()
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }
    fn rpc_launch(&self, id: &str, task: &TaskSpec) -> Result<Option<RpcLaunchSpec>, StoreError> {
        if id.trim().is_empty() {
            return Err(StoreError::Invalid("RPC launch ID is required"));
        }
        Ok(read(&self.0, id)?.filter(|spec| spec.task() == task))
    }
    fn claim_rpc_launch(&mut self, spec: &RpcLaunchSpec, now_ms: i64) -> Result<bool, StoreError> {
        check_time(now_ms)?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = read(&tx, spec.id())?.ok_or(StoreError::Unavailable)?;
        if existing != *spec {
            return Err(StoreError::Conflict);
        }
        let previous: Option<i64> = tx
            .query_row(
                include_str!("sql/select_rpc_launch_claim.sql"),
                [spec.id()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(at) = previous {
            if at < 0 {
                return Err(StoreError::Corrupt("invalid RPC claim timestamp".into()));
            }
            tx.commit()?;
            return Ok(false);
        }
        current_origin(&tx, spec, now_ms)?;
        tx.execute(
            include_str!("sql/insert_rpc_launch_claim.sql"),
            params![spec.id(), now_ms],
        )?;
        tx.execute(
            include_str!("sql/insert_task_event.sql"),
            params![
                spec.task().id().as_str(),
                EventKind::RpcLaunchClaimed.as_str(),
                now_ms,
                spec.id()
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }
}

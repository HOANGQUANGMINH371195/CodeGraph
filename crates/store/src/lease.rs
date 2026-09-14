use crate::{StoreError, domain_corruption};
use graph_domain::{EventKind, Lease, TaskId, WorkerId};
use rusqlite::{Transaction, params};

/// Caller owns the transaction and must commit only after every check succeeds.
pub(crate) fn issue(
    transaction: &Transaction<'_>,
    task_id: &TaskId,
    owner: &WorkerId,
    now_ms: i64,
    duration_ms: i64,
) -> Result<Lease, StoreError> {
    let expires_at_ms = now_ms
        .checked_add(duration_ms)
        .ok_or(StoreError::Invalid("lease overflow"))?;
    if now_ms < 0 || duration_ms <= 0 {
        return Err(StoreError::Invalid(
            "lease requires nonnegative time and positive duration",
        ));
    }
    let changed = transaction.execute(
        include_str!("sql/lease_task.sql"),
        params![task_id.as_str(), owner.as_str(), now_ms, expires_at_ms],
    )?;
    if changed != 1 {
        return Err(StoreError::Unavailable);
    }
    let fence = transaction.query_row(
        include_str!("sql/select_task_fence.sql"),
        [task_id.as_str()],
        |row| row.get(0),
    )?;
    let lease = Lease::issue(task_id.clone(), owner.clone(), fence, expires_at_ms)
        .map_err(domain_corruption)?;
    let payload = serde_json::to_string(&graph_protocol::Lease::from(&lease))?;
    transaction.execute(
        include_str!("sql/insert_task_event.sql"),
        params![
            task_id.as_str(),
            EventKind::Leased.as_str(),
            now_ms,
            payload
        ],
    )?;
    Ok(lease)
}

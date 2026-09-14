//! Cancellation is a durable control-plane transition, not process termination.

use graph_domain::{EventKind, TaskId, TaskState, WorkerId};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

use crate::{Store, StoreError, domain_corruption};

impl Store {
    pub(crate) fn cancel_task(
        &mut self,
        task_id: &TaskId,
        requested_by: &WorkerId,
        reason: &str,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        if reason.trim().is_empty() {
            return Err(StoreError::Invalid("cancellation reason is required"));
        }
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let target = tx
            .query_row(
                include_str!("sql/cancellation_target.sql"),
                [task_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or(StoreError::Unavailable)?;
        let state = TaskState::from_db(&target.0).map_err(domain_corruption)?;
        if state == TaskState::Cancelled {
            return Ok(false); // Original actor/reason/event remain authoritative.
        }
        if !state.can_cancel() {
            return Err(StoreError::Unavailable);
        }
        let changed = tx.execute(
            include_str!("sql/cancel_task.sql"),
            params![task_id.as_str(), state.as_db()],
        )?;
        if changed != 1 {
            return Err(StoreError::Unavailable);
        }
        let payload = serde_json::to_string(&serde_json::json!({
            "requested_by": requested_by.as_str(), "reason": reason,
            "previous_state": state.as_db(), "previous_owner": target.1,
            "fencing_token": target.2,
        }))?;
        tx.execute(
            include_str!("sql/insert_task_event.sql"),
            params![
                task_id.as_str(),
                EventKind::Cancelled.as_str(),
                now_ms,
                payload
            ],
        )?;
        // The existing outbox trigger fires in this transaction as well.
        tx.commit()?;
        Ok(true)
    }
}

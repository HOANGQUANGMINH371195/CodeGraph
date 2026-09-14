use crate::{Store, StoreError};
use graph_application::ExecutionLaunchRepository;
use graph_domain::{EventKind, ExecutionPlan};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

impl ExecutionLaunchRepository for Store {
    type Error = StoreError;
    fn claim_execution_launch(
        &mut self,
        plan: &ExecutionPlan,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        if now_ms < 0 {
            return Err(StoreError::Invalid("launch claim time must be nonnegative"));
        }
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let registered = crate::execution_plan::read(&tx, plan.binding().run_id())?
            .ok_or(StoreError::Unavailable)?;
        if registered != *plan {
            return Err(StoreError::Conflict);
        }
        let previous: Option<i64> = tx
            .query_row(
                include_str!("sql/select_execution_launch_claim.sql"),
                [plan.binding().run_id()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(at) = previous {
            if at < 0 {
                return Err(StoreError::Corrupt("invalid launch claim time".into()));
            }
            tx.commit()?;
            return Ok(false);
        }
        if tx
            .query_row(
                include_str!("sql/select_execution_receipt.sql"),
                [plan.binding().run_id()],
                |_| Ok(()),
            )
            .optional()?
            .is_some()
        {
            return Err(StoreError::Unavailable);
        }
        crate::execution_plan::validate_candidate(&tx, plan.binding())?;
        tx.execute(
            include_str!("sql/insert_execution_launch_claim.sql"),
            params![plan.binding().run_id(), now_ms],
        )?;
        tx.execute(
            include_str!("sql/insert_task_event.sql"),
            params![
                plan.binding().task().id().as_str(),
                EventKind::ExecutionLaunchClaimed.as_str(),
                now_ms,
                plan.binding().run_id()
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }
}

use crate::{Store, StoreError};
use graph_application::ExecutionReceiptRepository;
use graph_domain::{ExecutionReceipt, TaskSpec};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

pub(crate) fn decode(
    raw: &str,
    run_id: &str,
    task_id: &str,
) -> Result<ExecutionReceipt, StoreError> {
    let corrupt = || StoreError::Corrupt("invalid execution receipt identity or contract".into());
    let wire: graph_protocol::ExecutionReceipt =
        serde_json::from_str(raw).map_err(|_| corrupt())?;
    let receipt = wire.try_into_domain().map_err(|_| corrupt())?;
    if receipt.binding().run_id() != run_id || receipt.binding().task().id().as_str() != task_id {
        return Err(corrupt());
    }
    Ok(receipt)
}

pub(crate) fn task(raw: &str, id: &str) -> Result<TaskSpec, StoreError> {
    let corrupt = || StoreError::Corrupt("invalid receipt task linkage".into());
    let wire: graph_protocol::TaskSpec = serde_json::from_str(raw).map_err(|_| corrupt())?;
    let spec = wire.try_into_domain().map_err(|_| corrupt())?;
    if spec.id().as_str() != id {
        return Err(corrupt());
    }
    Ok(spec)
}

pub(crate) fn read(
    connection: &rusqlite::Connection,
    run_id: &str,
    expected: &TaskSpec,
) -> Result<Option<ExecutionReceipt>, StoreError> {
    let row: Option<(String, String, Option<String>)> = connection
        .query_row(
            include_str!("sql/select_execution_receipt_task.sql"),
            [run_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    row.map(|(task_id, raw, spec)| {
        let receipt = decode(&raw, run_id, &task_id)?;
        let stored = task(
            &spec.ok_or_else(|| StoreError::Corrupt("missing receipt task".into()))?,
            &task_id,
        )?;
        if receipt.binding().task() != &stored {
            return Err(StoreError::Corrupt(
                "receipt differs from stored task".into(),
            ));
        }
        Ok((stored == *expected).then_some(receipt))
    })
    .transpose()
    .map(Option::flatten)
}

impl ExecutionReceiptRepository for Store {
    type Error = StoreError;
    fn record_execution_receipt(&mut self, receipt: &ExecutionReceipt) -> Result<bool, StoreError> {
        let binding = receipt.binding();
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(plan) = crate::execution_plan::read(&tx, binding.run_id())? {
            if !plan.matches_receipt(receipt) {
                return Err(StoreError::Conflict);
            }
        }
        let raw: Option<String> = tx
            .query_row(
                include_str!("sql/select_task_snapshot.sql"),
                [binding.task().id().as_str()],
                |r| r.get(0),
            )
            .optional()?;
        let stored_task = task(
            &raw.ok_or(StoreError::Unavailable)?,
            binding.task().id().as_str(),
        )?;
        if stored_task != *binding.task() {
            return Err(StoreError::Conflict);
        }
        let existing: Option<(String, String)> = tx
            .query_row(
                include_str!("sql/select_execution_receipt.sql"),
                [binding.run_id()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((task_id, raw)) = existing {
            let previous = decode(&raw, binding.run_id(), &task_id)?;
            if previous != *receipt {
                return Err(StoreError::Conflict);
            }
            tx.commit()?;
            return Ok(false);
        }
        let descriptor = serde_json::to_string(&graph_protocol::ExecutionReceipt::from(receipt))?;
        tx.execute(
            include_str!("sql/insert_execution_receipt.sql"),
            params![binding.run_id(), binding.task().id().as_str(), descriptor],
        )?;
        tx.commit()?;
        Ok(true)
    }

    fn execution_receipt(
        &self,
        run_id: &str,
        expected: &TaskSpec,
    ) -> Result<Option<ExecutionReceipt>, StoreError> {
        if run_id.trim().is_empty() {
            return Err(StoreError::Invalid("check run ID is required"));
        }
        read(&self.0, run_id, expected)
    }
}

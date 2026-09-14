use crate::{Store, StoreError};
use graph_application::RpcTerminalReceiptRepository;
use graph_domain::{EventKind, RpcLaunchSpec, RpcTerminalReceipt};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

fn corrupt() -> StoreError {
    StoreError::Corrupt("invalid RPC terminal receipt or linkage".into())
}
fn project(raw: &str) -> Result<graph_domain::ProjectRef, StoreError> {
    let p: graph_protocol::ProjectRef = serde_json::from_str(raw).map_err(|_| corrupt())?;
    Ok(p.into())
}
// Caller holds a transaction for all referenced rows.
fn linked(c: &Connection, r: &RpcTerminalReceipt) -> Result<(), StoreError> {
    let spawn =
        crate::rpc_spawn::read(c, r.spawn().launch().id())?.ok_or(StoreError::Unavailable)?;
    if &spawn != r.spawn() {
        return Err(StoreError::Conflict);
    }
    let run = r.output_run();
    let (scope, graph, raw): (String, String, String) = c
        .query_row(
            include_str!("sql/select_rpc_terminal_run.sql"),
            [run.id()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or(StoreError::Unavailable)?;
    let wire: graph_protocol::AnalysisRun = serde_json::from_str(&raw).map_err(|_| corrupt())?;
    let stored = wire.try_into_domain().map_err(|_| corrupt())?;
    if stored.id() != run.id()
        || stored.project() != &project(&scope)?
        || stored.graph_version() != graph
    {
        return Err(corrupt());
    }
    if &stored != run {
        return Err(StoreError::Conflict);
    }
    for artifact in [r.stdout(), r.stderr()].into_iter().flatten() {
        let (scope, graph, analysis, raw): (String, String, String, String) = c
            .query_row(
                include_str!("sql/select_rpc_terminal_artifact.sql"),
                [artifact.id()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?
            .ok_or(StoreError::Unavailable)?;
        let wire: graph_protocol::Artifact = serde_json::from_str(&raw).map_err(|_| corrupt())?;
        let stored = wire.try_into_domain().map_err(|_| corrupt())?;
        if stored.id() != artifact.id()
            || stored.project() != &project(&scope)?
            || stored.graph_version() != graph
            || stored.analysis_run() != analysis
        {
            return Err(corrupt());
        }
        if &stored != artifact {
            return Err(StoreError::Conflict);
        }
    }
    Ok(())
}
type Row = (String, Option<String>, Option<String>, i64, String);
// Caller must hold a transaction across receipt and all referenced reads.
pub(crate) fn read(c: &Connection, id: &str) -> Result<Option<RpcTerminalReceipt>, StoreError> {
    let row: Option<Row> = c
        .query_row(include_str!("sql/select_rpc_terminal.sql"), [id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .optional()?;
    row.map(|(run, stdout, stderr, at, raw)| {
        let wire: graph_protocol::RpcTerminalReceipt =
            serde_json::from_str(&raw).map_err(|_| corrupt())?;
        let r = wire.try_into_domain().map_err(|_| corrupt())?;
        if r.spawn().launch().id() != id
            || r.output_run().id() != run
            || r.stdout().map(|a| a.id()) != stdout.as_deref()
            || r.stderr().map(|a| a.id()) != stderr.as_deref()
            || r.finished_at_ms() != at
        {
            return Err(corrupt());
        }
        linked(c, &r).map_err(|e| match e {
            StoreError::Conflict | StoreError::Unavailable => corrupt(),
            other => other,
        })?;
        Ok(r)
    })
    .transpose()
}
impl RpcTerminalReceiptRepository for Store {
    type Error = StoreError;
    fn record_rpc_terminal_receipt(&mut self, r: &RpcTerminalReceipt) -> Result<bool, StoreError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let launch = r.spawn().launch();
        if let Some(previous) = read(&tx, launch.id())? {
            if previous != *r {
                return Err(StoreError::Conflict);
            }
            tx.commit()?;
            return Ok(false);
        }
        linked(&tx, r)?;
        let raw = serde_json::to_string(&graph_protocol::RpcTerminalReceipt::from(r))?;
        tx.execute(
            include_str!("sql/insert_rpc_terminal.sql"),
            params![
                launch.id(),
                r.output_run().id(),
                r.stdout().map(|a| a.id()),
                r.stderr().map(|a| a.id()),
                r.finished_at_ms(),
                raw
            ],
        )?;
        tx.execute(
            include_str!("sql/insert_task_event.sql"),
            params![
                launch.task().id().as_str(),
                EventKind::RpcTerminalRecorded.as_str(),
                r.finished_at_ms(),
                launch.id()
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }
    fn rpc_terminal_receipt(
        &self,
        launch: &RpcLaunchSpec,
    ) -> Result<Option<RpcTerminalReceipt>, StoreError> {
        let tx = self.0.unchecked_transaction()?;
        let result = read(&tx, launch.id())?.filter(|r| r.spawn().launch() == launch);
        tx.commit()?;
        Ok(result)
    }
}

#[cfg(test)]
#[path = "rpc_terminal_tests.rs"]
mod tests;

use crate::{Store, StoreError};
use graph_application::ExecutionPlanRepository;
use graph_domain::{ExecutionPlan, TaskSpec};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

pub(crate) fn read(
    connection: &Connection,
    run_id: &str,
) -> Result<Option<ExecutionPlan>, StoreError> {
    let row: Option<(String, String, Option<String>)> = connection
        .query_row(
            include_str!("sql/select_execution_plan.sql"),
            [run_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    row.map(|(id, raw, spec)| {
        let corrupt = || StoreError::Corrupt("invalid execution plan or task linkage".into());
        let wire: graph_protocol::ExecutionPlan =
            serde_json::from_str(&raw).map_err(|_| corrupt())?;
        let plan = wire.try_into_domain().map_err(|_| corrupt())?;
        let wire: graph_protocol::TaskSpec =
            serde_json::from_str(&spec.ok_or_else(corrupt)?).map_err(|_| corrupt())?;
        let task = wire.try_into_domain().map_err(|_| corrupt())?;
        if plan.binding().run_id() != run_id
            || task.id().as_str() != id
            || plan.binding().task() != &task
        {
            return Err(corrupt());
        }
        Ok(plan)
    })
    .transpose()
}

pub(crate) fn validate_candidate(
    connection: &Connection,
    binding: &graph_domain::CheckRunBinding,
) -> Result<(), StoreError> {
    let submitted = crate::submission_query::read(connection, binding.task().id())?
        .ok_or(StoreError::Unavailable)?;
    if submitted.spec != *binding.task()
        || submitted.owner != *binding.origin_lease().owner()
        || submitted.fencing_token != binding.origin_lease().fencing_token()
        || submitted.submission_sequence != binding.submission_sequence()
        || submitted.artifact != binding.candidate().id()
    {
        return Err(StoreError::Conflict);
    }
    let (_, _, _, policy) =
        crate::check_policy::read(connection, binding.task())?.ok_or(StoreError::Unavailable)?;
    if policy.as_ref() != Some(binding.policy()) {
        return Err(StoreError::Conflict);
    }
    let project =
        serde_json::to_string(&graph_protocol::ProjectRef::from(binding.task().project()))?;
    let row: Option<(String, String)> = connection
        .query_row(
            include_str!("sql/select_artifact.sql"),
            params![
                binding.candidate().id(),
                project,
                binding.task().graph_version()
            ],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (raw, run) = row.ok_or(StoreError::Unavailable)?;
    let corrupt = || StoreError::Corrupt("invalid registered candidate descriptor".into());
    let wire: graph_protocol::Artifact = serde_json::from_str(&raw).map_err(|_| corrupt())?;
    let artifact = wire.try_into_domain().map_err(|_| corrupt())?;
    if artifact.analysis_run() != run {
        return Err(corrupt());
    }
    if &artifact != binding.candidate() {
        return Err(StoreError::Conflict);
    }
    let raw_run: Option<String> = connection
        .query_row(
            include_str!("sql/select_analysis_run.sql"),
            params![run, project, binding.task().graph_version()],
            |r| r.get(0),
        )
        .optional()?;
    let corrupt_run =
        || StoreError::Corrupt("candidate analysis registration missing or invalid".into());
    let wire: graph_protocol::AnalysisRun =
        serde_json::from_str(&raw_run.ok_or_else(corrupt_run)?).map_err(|_| corrupt_run())?;
    let analysis = wire.try_into_domain().map_err(|_| corrupt_run())?;
    if analysis.id() != artifact.analysis_run()
        || analysis.project() != artifact.project()
        || analysis.graph_version() != artifact.graph_version()
    {
        return Err(corrupt_run());
    }
    Ok(())
}

impl ExecutionPlanRepository for Store {
    type Error = StoreError;
    fn register_execution_plan(&mut self, plan: &ExecutionPlan) -> Result<bool, StoreError> {
        let binding = plan.binding();
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = read(&tx, binding.run_id())? {
            if existing != *plan {
                return Err(StoreError::Conflict);
            }
            tx.commit()?;
            return Ok(false);
        }
        let receipt_exists = tx
            .query_row(
                include_str!("sql/select_execution_receipt.sql"),
                [binding.run_id()],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if receipt_exists {
            return Err(StoreError::Conflict);
        }
        validate_candidate(&tx, binding)?;
        let descriptor = serde_json::to_string(&graph_protocol::ExecutionPlan::from(plan))?;
        tx.execute(
            include_str!("sql/insert_execution_plan.sql"),
            params![binding.run_id(), binding.task().id().as_str(), descriptor],
        )?;
        tx.commit()?;
        Ok(true)
    }
    fn execution_plan(
        &self,
        run_id: &str,
        task: &TaskSpec,
    ) -> Result<Option<ExecutionPlan>, StoreError> {
        if run_id.trim().is_empty() {
            return Err(StoreError::Invalid("check run ID is required"));
        }
        Ok(read(&self.0, run_id)?.filter(|plan| plan.binding().task() == task))
    }
}

use crate::{Store, StoreError};
use graph_application::CheckPolicyRepository;
use graph_application::PolicyLeaseRepository;
use graph_domain::{EventKind, RequiredChecks, TaskSpec};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

type PolicyRow = (TaskSpec, String, i64, Option<RequiredChecks>);

impl PolicyLeaseRepository for Store {
    type Error = StoreError;
    fn lease_with_policy(
        &mut self,
        task: &TaskSpec,
        policy: &RequiredChecks,
        owner: &graph_domain::WorkerId,
        now_ms: i64,
        duration_ms: i64,
    ) -> Result<graph_domain::Lease, StoreError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (stored, _, _, registered) = read(&tx, task)?.ok_or(StoreError::Unavailable)?;
        if stored != *task {
            return Err(StoreError::Conflict);
        }
        let registered = registered.ok_or(StoreError::Unavailable)?;
        if registered != *policy {
            return Err(StoreError::Conflict);
        }
        let lease = crate::lease::issue(&tx, task.id(), owner, now_ms, duration_ms)?;
        tx.commit()?;
        Ok(lease)
    }
}

pub(crate) fn read(
    connection: &Connection,
    task: &TaskSpec,
) -> Result<Option<PolicyRow>, StoreError> {
    let row = connection
        .query_row(
            include_str!("sql/select_task_check_policy.sql"),
            [task.id().as_str()],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?;
    row.map(|(spec, state, fence, policy)| {
        let corrupt = || StoreError::Corrupt("invalid task check-policy binding".into());
        let wire: graph_protocol::TaskSpec = serde_json::from_str(&spec).map_err(|_| corrupt())?;
        let spec = wire.try_into_domain().map_err(|_| corrupt())?;
        if spec.id() != task.id() {
            return Err(corrupt());
        }
        let policy = policy
            .map(|raw| {
                let wire: graph_protocol::RequiredChecks =
                    serde_json::from_str(&raw).map_err(|_| corrupt())?;
                wire.try_into_domain().map_err(|_| corrupt())
            })
            .transpose()?;
        Ok((spec, state, fence, policy))
    })
    .transpose()
}

impl CheckPolicyRepository for Store {
    type Error = StoreError;

    fn register_check_policy(
        &mut self,
        task: &TaskSpec,
        policy: &RequiredChecks,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        if now_ms < 0 {
            return Err(StoreError::Invalid(
                "policy registration time must be nonnegative",
            ));
        }
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (stored, state, fence, existing) = read(&tx, task)?.ok_or(StoreError::Unavailable)?;
        if stored != *task {
            return Err(StoreError::Conflict);
        }
        if let Some(existing) = existing {
            if existing != *policy {
                return Err(StoreError::Conflict);
            }
            tx.commit()?;
            return Ok(false);
        }
        if state != "queued" || fence != 0 {
            return Err(StoreError::Unavailable);
        }
        let descriptor = serde_json::to_string(&graph_protocol::RequiredChecks::from(policy))?;
        tx.execute(
            include_str!("sql/insert_task_check_policy.sql"),
            params![task.id().as_str(), descriptor],
        )?;
        tx.execute(
            include_str!("sql/insert_task_event.sql"),
            params![
                task.id().as_str(),
                EventKind::CheckPolicyRegistered.as_str(),
                now_ms,
                descriptor
            ],
        )?;
        // The outbox trigger runs inside this transaction, before commit.
        tx.commit()?;
        Ok(true)
    }

    fn check_policy(&self, task: &TaskSpec) -> Result<Option<RequiredChecks>, StoreError> {
        Ok(read(&self.0, task)?
            .and_then(|(stored, _, _, policy)| if stored == *task { policy } else { None }))
    }
}

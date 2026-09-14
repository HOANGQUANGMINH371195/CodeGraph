use crate::{Store, StoreError};
use graph_application::{SubmissionQueryRepository, SubmittedCandidate};
use graph_domain::{TaskId, WorkerId};
use rusqlite::OptionalExtension;

impl SubmissionQueryRepository for Store {
    type Error = StoreError;

    fn submitted_candidate(&self, id: &TaskId) -> Result<Option<SubmittedCandidate>, StoreError> {
        read(&self.0, id)
    }
}

pub(crate) fn read(
    connection: &rusqlite::Connection,
    id: &TaskId,
) -> Result<Option<SubmittedCandidate>, StoreError> {
    // One statement reads task identity and submission in the same snapshot.
    let row = connection
        .query_row(
            include_str!("sql/select_submitted_candidate.sql"),
            [id.as_str()],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            },
        )
        .optional()?;
    row.map(|(spec, owner, fence, seq, at, artifact, count)| {
        let corrupt = || StoreError::Corrupt("invalid submitted candidate ledger binding".into());
        if count != 1 {
            return Err(corrupt());
        }
        let wire: graph_protocol::TaskSpec = serde_json::from_str(&spec).map_err(|_| corrupt())?;
        let spec = wire.try_into_domain().map_err(|_| corrupt())?;
        let owner = WorkerId::new(owner.ok_or_else(corrupt)?).map_err(|_| corrupt())?;
        let sequence = seq.ok_or_else(corrupt)?;
        let at = at.ok_or_else(corrupt)?;
        let artifact = artifact.ok_or_else(corrupt)?;
        if spec.id() != id || fence <= 0 || sequence <= 0 || at < 0 || artifact.trim().is_empty() {
            return Err(corrupt());
        }
        Ok(SubmittedCandidate {
            spec,
            owner,
            fencing_token: fence,
            submission_sequence: sequence,
            submitted_at_ms: at,
            artifact,
        })
    })
    .transpose()
}

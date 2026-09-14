use crate::{Store, StoreError};
use graph_application::{TaskQueryRepository, TaskSnapshot};
use graph_domain::{TaskId, TaskState};
use rusqlite::OptionalExtension;

impl TaskQueryRepository for Store {
    type Error = StoreError;

    fn task_snapshot(&self, id: &TaskId) -> Result<Option<TaskSnapshot>, StoreError> {
        // One statement reads spec and state from the same SQLite snapshot.
        let row: Option<(String, String)> = self
            .0
            .query_row(
                include_str!("sql/select_task_snapshot.sql"),
                [id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        row.map(|(spec, state)| {
            let wire: graph_protocol::TaskSpec =
                serde_json::from_str(&spec).map_err(|e| StoreError::Corrupt(e.to_string()))?;
            let spec = wire
                .try_into_domain()
                .map_err(|e| StoreError::Corrupt(e.to_string()))?;
            if spec.id() != id {
                return Err(StoreError::Corrupt(
                    "task row/spec identity mismatch".into(),
                ));
            }
            let state =
                TaskState::from_db(&state).map_err(|e| StoreError::Corrupt(e.to_string()))?;
            Ok(TaskSnapshot { spec, state })
        })
        .transpose()
    }
}

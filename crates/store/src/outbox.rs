//! Durable, independent consumer cursors. Delivery is at least once: consumers
//! must apply events idempotently before acknowledging them. External side
//! effects and this SQLite cursor cannot form a single transaction.

use graph_domain::TaskEvent;
use rusqlite::{OptionalExtension, TransactionBehavior, params};

use crate::{Store, StoreError};

impl Store {
    pub fn pending_events(&self, consumer: &str, limit: u32) -> Result<Vec<TaskEvent>, StoreError> {
        validate_consumer(consumer)?;
        let cursor = self
            .0
            .query_row(
                "SELECT event_sequence FROM projection_cursors WHERE consumer=?1",
                [consumer],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0);
        self.task_events(cursor, limit)
    }

    /// Advance only to the next event. This prevents an out-of-order worker
    /// from acknowledging work another worker has not yet applied.
    pub fn acknowledge_event(&mut self, consumer: &str, sequence: i64) -> Result<(), StoreError> {
        validate_consumer(consumer)?;
        if sequence <= 0 {
            return Err(StoreError::Invalid("event sequence must be positive"));
        }
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let cursor = tx
            .query_row(
                "SELECT event_sequence FROM projection_cursors WHERE consumer=?1",
                [consumer],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0);
        if sequence <= cursor {
            return Ok(());
        }
        let next: Option<i64> = tx.query_row(
            "SELECT MIN(event_sequence) FROM event_outbox WHERE event_sequence>?1",
            [cursor],
            |row| row.get(0),
        )?;
        if next != Some(sequence) {
            return Err(StoreError::Invalid(
                "acknowledgement must follow event order",
            ));
        }
        tx.execute(
            "INSERT INTO projection_cursors(consumer,event_sequence) VALUES(?1,?2)
             ON CONFLICT(consumer) DO UPDATE SET event_sequence=excluded.event_sequence",
            params![consumer, sequence],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn validate_consumer(consumer: &str) -> Result<(), StoreError> {
    if consumer.trim().is_empty() {
        Err(StoreError::Invalid("consumer is required"))
    } else {
        Ok(())
    }
}

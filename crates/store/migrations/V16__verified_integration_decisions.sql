-- Immutable accepted-decision record. The transition itself stays in adapter
-- code so it can revalidate all durable bindings inside the same transaction.
CREATE TABLE verified_integration_decisions (
    task_id TEXT PRIMARY KEY REFERENCES tasks(id),
    descriptor TEXT NOT NULL,
    target_verification TEXT NOT NULL
);

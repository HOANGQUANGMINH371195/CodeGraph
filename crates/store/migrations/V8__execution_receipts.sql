-- Historical producer claims only: no execution authority or fabricated backfill.
CREATE TABLE execution_receipts (
    run_id TEXT PRIMARY KEY NOT NULL CHECK(length(trim(run_id)) > 0),
    task_id TEXT NOT NULL REFERENCES tasks(id),
    descriptor TEXT NOT NULL
);
CREATE INDEX execution_receipts_task ON execution_receipts(task_id);

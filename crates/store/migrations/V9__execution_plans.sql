-- No inferred plans for historical receipts; these do not grant launch authority.
CREATE TABLE execution_plans (
    run_id TEXT PRIMARY KEY NOT NULL CHECK(length(trim(run_id)) > 0),
    task_id TEXT NOT NULL REFERENCES tasks(id),
    descriptor TEXT NOT NULL
);
CREATE INDEX execution_plans_task ON execution_plans(task_id);

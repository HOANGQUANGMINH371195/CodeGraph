-- No defaults/backfill: existing tasks have unknown check policy.
CREATE TABLE task_check_policies (
    task_id TEXT PRIMARY KEY REFERENCES tasks(id),
    descriptor TEXT NOT NULL
);

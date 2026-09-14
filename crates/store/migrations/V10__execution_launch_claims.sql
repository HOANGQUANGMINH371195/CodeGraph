-- One-shot marker, not an expiring lease. Never auto-reclaim uncertain execution.
CREATE TABLE execution_launch_claims (
    run_id TEXT PRIMARY KEY NOT NULL REFERENCES execution_plans(run_id),
    claimed_at_ms INTEGER NOT NULL CHECK(claimed_at_ms >= 0)
);

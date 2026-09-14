-- Descriptions and one-shot accounting only; not host approval or OS execution.
CREATE TABLE rpc_launch_specs (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    host_id TEXT NOT NULL,
    connection_epoch TEXT NOT NULL,
    fencing_token INTEGER NOT NULL CHECK(fencing_token > 0),
    descriptor TEXT NOT NULL,
    UNIQUE(task_id, fencing_token),
    UNIQUE(host_id, connection_epoch)
);

CREATE TABLE rpc_launch_claims (
    launch_id TEXT PRIMARY KEY NOT NULL REFERENCES rpc_launch_specs(id),
    claimed_at_ms INTEGER NOT NULL CHECK(claimed_at_ms >= 0)
);

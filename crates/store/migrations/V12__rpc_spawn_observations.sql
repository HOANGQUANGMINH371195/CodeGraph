-- Historical host reports; neither a PID nor absence proves process state.
CREATE TABLE rpc_spawn_observations (
    launch_id TEXT PRIMARY KEY NOT NULL REFERENCES rpc_launch_claims(launch_id),
    observed_at_ms INTEGER NOT NULL CHECK(observed_at_ms >= 0),
    descriptor TEXT NOT NULL
);

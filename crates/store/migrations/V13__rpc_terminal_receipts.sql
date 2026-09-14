CREATE TABLE rpc_terminal_receipts (
    launch_id TEXT PRIMARY KEY NOT NULL REFERENCES rpc_spawn_observations(launch_id),
    output_run TEXT NOT NULL REFERENCES analysis_runs(id),
    stdout_id TEXT REFERENCES artifacts(id),
    stderr_id TEXT REFERENCES artifacts(id),
    finished_at_ms INTEGER NOT NULL CHECK(finished_at_ms >= 0),
    descriptor TEXT NOT NULL
);
CREATE TRIGGER rpc_terminal_no_update BEFORE UPDATE ON rpc_terminal_receipts
BEGIN SELECT RAISE(ABORT, 'RPC terminal receipt is immutable'); END;
CREATE TRIGGER rpc_terminal_no_delete BEFORE DELETE ON rpc_terminal_receipts
BEGIN SELECT RAISE(ABORT, 'RPC terminal receipt is immutable'); END;

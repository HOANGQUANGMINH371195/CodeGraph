-- Disposable in-memory test only: allow simulation of damaged persisted rows.
PRAGMA foreign_keys=OFF;
DROP TRIGGER rpc_terminal_no_update;
DROP TRIGGER analysis_runs_no_update;
DROP TRIGGER analysis_runs_no_delete;
DROP TRIGGER artifacts_no_update;
DROP TRIGGER artifacts_no_delete;

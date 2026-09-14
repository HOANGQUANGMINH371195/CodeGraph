-- Registration is provenance metadata, never an accepted fact or success receipt.
CREATE TABLE analysis_runs (
    id TEXT PRIMARY KEY NOT NULL,
    project TEXT NOT NULL,
    graph_version TEXT NOT NULL,
    descriptor TEXT NOT NULL
);
CREATE TRIGGER analysis_runs_no_update BEFORE UPDATE ON analysis_runs
BEGIN SELECT RAISE(ABORT, 'analysis run is immutable'); END;
CREATE TRIGGER analysis_runs_no_delete BEFORE DELETE ON analysis_runs
BEGIN SELECT RAISE(ABORT, 'analysis run is immutable'); END;

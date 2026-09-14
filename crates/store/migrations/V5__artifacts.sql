-- Metadata only. Blob existence/protection are not attested by registration.
CREATE UNIQUE INDEX analysis_runs_scope ON analysis_runs(id, project, graph_version);
CREATE TABLE artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    project TEXT NOT NULL,
    graph_version TEXT NOT NULL,
    analysis_run TEXT NOT NULL,
    descriptor TEXT NOT NULL,
    FOREIGN KEY (analysis_run, project, graph_version)
        REFERENCES analysis_runs(id, project, graph_version)
);
CREATE TRIGGER artifacts_no_update BEFORE UPDATE ON artifacts
BEGIN SELECT RAISE(ABORT, 'artifact is immutable'); END;
CREATE TRIGGER artifacts_no_delete BEFORE DELETE ON artifacts
BEGIN SELECT RAISE(ABORT, 'artifact is immutable'); END;

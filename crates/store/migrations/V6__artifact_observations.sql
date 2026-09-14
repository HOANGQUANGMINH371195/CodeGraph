-- Historical byte observations; not proof of current availability or execution.
CREATE UNIQUE INDEX artifacts_scope ON artifacts(id, project, graph_version);
CREATE TABLE artifact_observations (
    id TEXT PRIMARY KEY NOT NULL,
    artifact_id TEXT NOT NULL,
    project TEXT NOT NULL,
    graph_version TEXT NOT NULL,
    observed_at_ms INTEGER NOT NULL CHECK (observed_at_ms >= 0),
    verifier_version TEXT NOT NULL,
    FOREIGN KEY (artifact_id, project, graph_version)
        REFERENCES artifacts(id, project, graph_version)
);
CREATE TRIGGER artifact_observations_no_update BEFORE UPDATE ON artifact_observations
BEGIN SELECT RAISE(ABORT, 'artifact observation is immutable'); END;
CREATE TRIGGER artifact_observations_no_delete BEFORE DELETE ON artifact_observations
BEGIN SELECT RAISE(ABORT, 'artifact observation is immutable'); END;

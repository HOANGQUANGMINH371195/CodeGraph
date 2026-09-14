-- Citation records are inputs to verification, never accepted graph facts.
CREATE TABLE source_evidence (
    id TEXT PRIMARY KEY NOT NULL,
    project TEXT NOT NULL,
    graph_version TEXT NOT NULL,
    path TEXT NOT NULL,
    content_sha256 TEXT NOT NULL CHECK(length(content_sha256) = 64),
    start_line INTEGER NOT NULL CHECK(start_line > 0),
    end_line INTEGER NOT NULL CHECK(end_line >= start_line),
    analysis_run TEXT NOT NULL
);

CREATE TRIGGER source_evidence_no_update BEFORE UPDATE ON source_evidence
BEGIN
    SELECT RAISE(ABORT, 'source evidence is immutable');
END;

CREATE TRIGGER source_evidence_no_delete BEFORE DELETE ON source_evidence
BEGIN
    SELECT RAISE(ABORT, 'source evidence is immutable');
END;

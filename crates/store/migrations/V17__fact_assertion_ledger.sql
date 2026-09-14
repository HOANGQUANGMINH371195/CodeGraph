-- Candidate assertions and terminal decisions are separate immutable rows.
-- The normal graph view joins only accepted decisions; producer candidates are
-- never an implicit graph write.
CREATE TABLE fact_assertions (
    assertion_id TEXT NOT NULL,
    project TEXT NOT NULL,
    graph_version TEXT NOT NULL,
    descriptor TEXT NOT NULL,
    PRIMARY KEY (assertion_id, project, graph_version)
);

CREATE TRIGGER fact_assertions_no_update BEFORE UPDATE ON fact_assertions
BEGIN SELECT RAISE(ABORT, 'fact assertion is immutable'); END;
CREATE TRIGGER fact_assertions_no_delete BEFORE DELETE ON fact_assertions
BEGIN SELECT RAISE(ABORT, 'fact assertion is immutable'); END;

CREATE TABLE fact_decisions (
    assertion_id TEXT NOT NULL,
    project TEXT NOT NULL,
    graph_version TEXT NOT NULL,
    decision TEXT NOT NULL,
    receipt TEXT NOT NULL,
    PRIMARY KEY (assertion_id, project, graph_version),
    FOREIGN KEY (assertion_id, project, graph_version)
        REFERENCES fact_assertions(assertion_id, project, graph_version)
);

CREATE TRIGGER fact_decisions_no_update BEFORE UPDATE ON fact_decisions
BEGIN SELECT RAISE(ABORT, 'fact decision is immutable'); END;
CREATE TRIGGER fact_decisions_no_delete BEFORE DELETE ON fact_decisions
BEGIN SELECT RAISE(ABORT, 'fact decision is immutable'); END;

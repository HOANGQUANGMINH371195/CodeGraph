-- Durable task control plane. Never edit a migration after it has shipped;
-- write V{next}__description.sql instead so checksums and incident debugging
-- remain trustworthy.
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    spec TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN (
        'queued', 'leased', 'submitted', 'integrated', 'rejected', 'cancelled'
    )),
    owner TEXT,
    fence INTEGER NOT NULL DEFAULT 0,
    expires INTEGER
);

CREATE TABLE dependencies (
    task_id TEXT NOT NULL REFERENCES tasks(id),
    dependency_id TEXT NOT NULL REFERENCES tasks(id),
    PRIMARY KEY(task_id, dependency_id)
);

CREATE TABLE events (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    kind TEXT NOT NULL,
    at_ms INTEGER NOT NULL,
    payload TEXT NOT NULL
);

CREATE INDEX events_after_seq ON events(seq);
CREATE INDEX dependencies_by_task ON dependencies(task_id);

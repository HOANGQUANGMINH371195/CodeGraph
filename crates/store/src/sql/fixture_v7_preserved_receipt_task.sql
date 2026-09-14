-- Migration-test fixture, never used in the runtime write path.
INSERT INTO tasks(id, spec, state) VALUES ('preserved', 'fixture-only', 'queued');
INSERT INTO events(task_id, kind, at_ms, payload)
VALUES ('preserved', 'enqueued', 0, 'original');

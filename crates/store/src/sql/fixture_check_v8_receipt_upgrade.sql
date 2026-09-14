SELECT
    (SELECT count(*) FROM execution_receipts),
    (SELECT count(*) FROM tasks WHERE id = 'preserved' AND spec = 'fixture-only'),
    (SELECT count(*) FROM events WHERE task_id = 'preserved' AND payload = 'original'),
    (SELECT count(*) FROM event_outbox);

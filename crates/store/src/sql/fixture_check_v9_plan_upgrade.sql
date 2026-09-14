SELECT
    (SELECT count(*) FROM execution_plans),
    (SELECT count(*) FROM execution_receipts WHERE run_id = 'old-run' AND descriptor = 'legacy diagnostic'),
    (SELECT count(*) FROM tasks WHERE id = 'preserved' AND spec = 'fixture-only');

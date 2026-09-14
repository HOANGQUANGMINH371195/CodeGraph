SELECT (SELECT COUNT(*) FROM rpc_launch_specs),
       (SELECT COUNT(*) FROM rpc_launch_claims),
       (SELECT COUNT(*) FROM tasks WHERE id = 'preserved' AND spec = 'fixture-only'),
       (SELECT COUNT(*) FROM events WHERE task_id = 'preserved' AND payload = 'original'),
       (SELECT COUNT(*) FROM event_outbox);

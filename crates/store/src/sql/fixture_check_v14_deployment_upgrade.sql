SELECT
 (SELECT count(*) FROM deployment_headers),
 (SELECT count(*) FROM deployment_nodes),
 (SELECT count(*) FROM deployment_edges),
 (SELECT count(*) FROM deployment_unknowns),
 (SELECT count(*) FROM tasks WHERE id='preserved' AND spec='fixture-only' AND state='queued'),
 (SELECT count(*) FROM events WHERE task_id='preserved' AND kind='enqueued' AND at_ms=0 AND payload='original');

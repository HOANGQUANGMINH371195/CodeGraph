SELECT
    (SELECT count(*) FROM rpc_launch_specs),
    (SELECT count(*) FROM rpc_launch_claims),
    (SELECT count(*) FROM events),
    (SELECT count(*) FROM event_outbox);

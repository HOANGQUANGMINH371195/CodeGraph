SELECT (SELECT count(*) FROM rpc_spawn_observations),
       (SELECT count(*) FROM events), (SELECT count(*) FROM event_outbox),
       (SELECT count(*) FROM rpc_launch_claims), (SELECT count(*) FROM rpc_launch_specs);

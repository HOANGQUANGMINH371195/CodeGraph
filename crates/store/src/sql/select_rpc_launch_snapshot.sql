SELECT launch.task_id, launch.host_id, launch.connection_epoch,
       launch.fencing_token, launch.descriptor, task.spec, claim.claimed_at_ms,
       spawn.launch_id, spawn.observed_at_ms, spawn.descriptor
FROM rpc_launch_specs launch
LEFT JOIN tasks task ON task.id = launch.task_id
LEFT JOIN rpc_launch_claims claim ON claim.launch_id = launch.id
LEFT JOIN rpc_spawn_observations spawn ON spawn.launch_id = launch.id
WHERE launch.id = ?1;

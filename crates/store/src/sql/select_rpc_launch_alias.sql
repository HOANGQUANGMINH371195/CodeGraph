SELECT 1 FROM rpc_launch_specs
WHERE (task_id = ?1 AND fencing_token = ?2)
   OR (host_id = ?3 AND connection_epoch = ?4)
LIMIT 1;

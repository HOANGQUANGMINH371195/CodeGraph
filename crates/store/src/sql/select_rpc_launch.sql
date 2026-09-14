SELECT launch.task_id, launch.host_id, launch.connection_epoch,
       launch.fencing_token, launch.descriptor, task.spec
FROM rpc_launch_specs launch LEFT JOIN tasks task ON task.id = launch.task_id
WHERE launch.id = ?1;

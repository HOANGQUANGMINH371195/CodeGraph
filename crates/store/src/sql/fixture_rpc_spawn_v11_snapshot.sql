-- Compare complete row values, including descriptor bytes and event/outbox IDs.
SELECT json_array(
    (SELECT json_group_array(json_array(id,spec,state,owner,fence,expires)) FROM (SELECT * FROM tasks ORDER BY id)),
    (SELECT json_group_array(json_array(id,task_id,host_id,connection_epoch,fencing_token,descriptor)) FROM (SELECT * FROM rpc_launch_specs ORDER BY id)),
    (SELECT json_group_array(json_array(launch_id,claimed_at_ms)) FROM (SELECT * FROM rpc_launch_claims ORDER BY launch_id)),
    (SELECT json_group_array(json_array(seq,task_id,kind,at_ms,payload)) FROM (SELECT * FROM events ORDER BY seq)),
    (SELECT json_group_array(event_sequence) FROM (SELECT * FROM event_outbox ORDER BY event_sequence))
);

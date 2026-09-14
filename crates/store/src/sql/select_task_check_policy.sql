SELECT t.spec, t.state, t.fence, p.descriptor
FROM tasks t LEFT JOIN task_check_policies p ON p.task_id = t.id
WHERE t.id = ?1;

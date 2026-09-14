SELECT p.task_id, p.descriptor, t.spec
FROM execution_plans p LEFT JOIN tasks t ON t.id = p.task_id WHERE p.run_id = ?1;

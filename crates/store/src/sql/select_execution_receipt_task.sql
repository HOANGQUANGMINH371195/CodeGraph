SELECT r.task_id, r.descriptor, t.spec
FROM execution_receipts AS r LEFT JOIN tasks AS t ON t.id = r.task_id
WHERE r.run_id = ?1;

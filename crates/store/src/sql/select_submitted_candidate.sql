SELECT t.spec, t.owner, t.fence, e.seq, e.at_ms, e.payload,
       (SELECT count(*) FROM events s WHERE s.task_id = t.id AND s.kind = 'submitted')
FROM tasks t
LEFT JOIN events e ON e.task_id = t.id AND e.kind = 'submitted'
WHERE t.id = ?1 AND t.state = 'submitted';

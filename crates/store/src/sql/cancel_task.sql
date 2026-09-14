-- A terminal state and cleared owner revoke submission authority without
-- recycling the fence; any future retry must allocate a new attempt/fence.
UPDATE tasks SET state = 'cancelled', owner = NULL, expires = NULL
WHERE id = ?1 AND state = ?2;

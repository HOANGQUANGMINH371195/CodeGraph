UPDATE tasks SET state='leased', owner=?2, fence=fence+1, expires=?4
WHERE id=?1 AND (state='queued' OR (state='leased' AND expires<=?3))
  AND NOT EXISTS (
    SELECT 1 FROM dependencies dependency
    JOIN tasks parent ON parent.id=dependency.dependency_id
    WHERE dependency.task_id=?1 AND parent.state!='integrated'
  );

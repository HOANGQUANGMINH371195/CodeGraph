SELECT id FROM artifacts
WHERE project = ?1 AND graph_version = ?2 AND id > ?3
ORDER BY id ASC LIMIT ?4;

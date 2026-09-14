SELECT descriptor, analysis_run FROM artifacts
WHERE id = ?1 AND project = ?2 AND graph_version = ?3;

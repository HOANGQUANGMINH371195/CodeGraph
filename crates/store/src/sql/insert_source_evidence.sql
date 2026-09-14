INSERT INTO source_evidence(id, project, graph_version, path, content_sha256, start_line, end_line, analysis_run)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
ON CONFLICT(id) DO NOTHING

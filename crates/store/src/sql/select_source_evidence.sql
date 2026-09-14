SELECT path, content_sha256, start_line, end_line, analysis_run
FROM source_evidence
WHERE id = ?1 AND project = ?2 AND graph_version = ?3

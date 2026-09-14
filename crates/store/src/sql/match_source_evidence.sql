SELECT project = ?2 AND graph_version = ?3 AND path = ?4
   AND content_sha256 = ?5 AND start_line = ?6 AND end_line = ?7 AND analysis_run = ?8
FROM source_evidence WHERE id = ?1

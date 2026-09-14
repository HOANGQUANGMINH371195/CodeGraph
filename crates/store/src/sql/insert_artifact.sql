INSERT INTO artifacts(id, project, graph_version, analysis_run, descriptor)
VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(id) DO NOTHING;

INSERT INTO analysis_runs(id, project, graph_version, descriptor)
VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO NOTHING;

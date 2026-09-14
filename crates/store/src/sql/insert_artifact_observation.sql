INSERT INTO artifact_observations(id, artifact_id, project, graph_version, observed_at_ms, verifier_version)
VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(id) DO NOTHING;

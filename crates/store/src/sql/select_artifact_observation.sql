SELECT artifact_id, observed_at_ms, verifier_version FROM artifact_observations
WHERE id = ?1 AND project = ?2 AND graph_version = ?3;

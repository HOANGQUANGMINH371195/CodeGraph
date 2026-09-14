SELECT artifact_id = ?2 AND project = ?3 AND graph_version = ?4
    AND observed_at_ms = ?5 AND verifier_version = ?6
FROM artifact_observations WHERE id = ?1;

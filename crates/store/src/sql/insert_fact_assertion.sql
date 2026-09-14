INSERT INTO fact_assertions(assertion_id, project, graph_version, descriptor)
VALUES (?1, ?2, ?3, ?4)
ON CONFLICT(assertion_id, project, graph_version) DO NOTHING;

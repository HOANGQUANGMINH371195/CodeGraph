INSERT INTO fact_decisions(assertion_id, project, graph_version, decision, receipt)
VALUES (?1, ?2, ?3, ?4, ?5)
ON CONFLICT(assertion_id, project, graph_version) DO NOTHING;

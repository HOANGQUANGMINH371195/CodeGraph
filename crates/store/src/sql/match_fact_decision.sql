SELECT decision = ?4 AND receipt = ?5
FROM fact_decisions
WHERE assertion_id = ?1 AND project = ?2 AND graph_version = ?3;

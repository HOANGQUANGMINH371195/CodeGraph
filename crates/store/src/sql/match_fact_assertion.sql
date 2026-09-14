SELECT descriptor = ?4
FROM fact_assertions
WHERE assertion_id = ?1 AND project = ?2 AND graph_version = ?3;

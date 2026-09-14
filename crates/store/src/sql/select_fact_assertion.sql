SELECT a.descriptor, d.decision, d.receipt
FROM fact_assertions AS a
LEFT JOIN fact_decisions AS d
  ON d.assertion_id = a.assertion_id
 AND d.project = a.project
 AND d.graph_version = a.graph_version
WHERE a.assertion_id = ?1 AND a.project = ?2 AND a.graph_version = ?3;

INSERT INTO deployment_headers
(project,graph_version,path,adapter,generation,adapter_version,evidence_id,node_count,edge_count,unknown_count)
VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
ON CONFLICT(project,graph_version,path,adapter) DO UPDATE SET
generation=excluded.generation, adapter_version=excluded.adapter_version,
evidence_id=excluded.evidence_id,node_count=excluded.node_count,
edge_count=excluded.edge_count,unknown_count=excluded.unknown_count
RETURNING id;

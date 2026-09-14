SELECT id,generation,adapter_version,evidence_id,node_count,edge_count,unknown_count
FROM deployment_headers WHERE project=?1 AND graph_version=?2 AND path=?3 AND adapter=?4;

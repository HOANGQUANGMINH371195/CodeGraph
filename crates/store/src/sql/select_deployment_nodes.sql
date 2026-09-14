SELECT ordinal,node_id,kind,name,evidence_id FROM deployment_nodes WHERE header_id=?1 ORDER BY ordinal;

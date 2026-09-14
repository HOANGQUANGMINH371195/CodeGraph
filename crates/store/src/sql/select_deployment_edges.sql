SELECT ordinal,source,target,kind,mount_target,evidence_id FROM deployment_edges WHERE header_id=?1 ORDER BY ordinal;

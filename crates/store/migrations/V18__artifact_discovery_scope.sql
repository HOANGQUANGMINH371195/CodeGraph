-- Seek within the exact snapshot/graph before advancing the artifact ID cursor.
CREATE INDEX artifacts_discovery_scope ON artifacts(project, graph_version, id);

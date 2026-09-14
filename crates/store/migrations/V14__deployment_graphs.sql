CREATE TABLE deployment_headers (
 id INTEGER PRIMARY KEY,
 project TEXT NOT NULL,
 graph_version TEXT NOT NULL,
 path TEXT NOT NULL,
 adapter TEXT NOT NULL,
 generation INTEGER NOT NULL CHECK(generation > 0),
 adapter_version TEXT,
 evidence_id TEXT REFERENCES source_evidence(id),
 node_count INTEGER NOT NULL CHECK(node_count BETWEEN 0 AND 10000),
 edge_count INTEGER NOT NULL CHECK(edge_count BETWEEN 0 AND 50000),
 unknown_count INTEGER NOT NULL CHECK(unknown_count BETWEEN 0 AND 50000),
 CHECK((adapter_version IS NOT NULL AND evidence_id IS NOT NULL) OR
       (adapter_version IS NULL AND evidence_id IS NULL AND node_count=0 AND edge_count=0 AND unknown_count=0)),
 UNIQUE(project, graph_version, path, adapter)
);
CREATE TABLE deployment_nodes (
 header_id INTEGER NOT NULL REFERENCES deployment_headers(id),
 ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
 node_id TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('service','volume','network')),
 name TEXT NOT NULL,
 evidence_id TEXT NOT NULL REFERENCES source_evidence(id),
 PRIMARY KEY(header_id,node_id),
 UNIQUE(header_id,ordinal)
);
CREATE TABLE deployment_edges (
 header_id INTEGER NOT NULL REFERENCES deployment_headers(id),
 ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
 source TEXT NOT NULL,
 target TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('mounts','depends_on','attached_to')),
 mount_target TEXT,
 evidence_id TEXT NOT NULL REFERENCES source_evidence(id),
 PRIMARY KEY(header_id,ordinal),
 FOREIGN KEY(header_id,source) REFERENCES deployment_nodes(header_id,node_id),
 FOREIGN KEY(header_id,target) REFERENCES deployment_nodes(header_id,node_id)
);
CREATE TABLE deployment_unknowns (
 header_id INTEGER NOT NULL REFERENCES deployment_headers(id),
 ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
 reason TEXT NOT NULL,
 line INTEGER NOT NULL CHECK(line > 0),
 PRIMARY KEY(header_id,ordinal)
);

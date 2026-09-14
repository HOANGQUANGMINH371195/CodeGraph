CREATE TABLE sql_link_headers (
 id INTEGER PRIMARY KEY,
 project TEXT NOT NULL,
 graph_version TEXT NOT NULL,
 code_path TEXT NOT NULL,
 adapter TEXT NOT NULL,
 generation INTEGER NOT NULL CHECK(generation > 0),
 adapter_version TEXT,
 code_evidence_id TEXT REFERENCES source_evidence(id),
 link_count INTEGER NOT NULL CHECK(link_count BETWEEN 0 AND 10000),
 CHECK((adapter_version IS NOT NULL AND code_evidence_id IS NOT NULL) OR
       (adapter_version IS NULL AND code_evidence_id IS NULL AND link_count=0)),
 UNIQUE(project, graph_version, code_path, adapter)
);
CREATE TABLE sql_links (
 header_id INTEGER NOT NULL REFERENCES sql_link_headers(id),
 ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
 link_id TEXT NOT NULL,
 target_evidence_id TEXT NOT NULL REFERENCES source_evidence(id),
 candidate_path TEXT NOT NULL,
 start_offset INTEGER NOT NULL CHECK(start_offset >= 0),
 end_offset INTEGER NOT NULL CHECK(end_offset > start_offset),
 coordinate_encoding TEXT NOT NULL CHECK(coordinate_encoding='utf16_code_unit'),
 statement_count INTEGER NOT NULL CHECK(statement_count BETWEEN 0 AND 256),
 PRIMARY KEY(header_id, link_id),
 UNIQUE(header_id, ordinal)
);
CREATE TABLE sql_link_statements (
 header_id INTEGER NOT NULL,
 link_id TEXT NOT NULL,
 ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
 operation TEXT NOT NULL CHECK(operation IN ('query','insert','update','delete','create_table','begin','commit','rollback','other')),
 relation_count INTEGER NOT NULL CHECK(relation_count BETWEEN 0 AND 256),
 PRIMARY KEY(header_id, link_id, ordinal),
 FOREIGN KEY(header_id, link_id) REFERENCES sql_links(header_id, link_id)
);
CREATE TABLE sql_link_relations (
 header_id INTEGER NOT NULL,
 link_id TEXT NOT NULL,
 statement_ordinal INTEGER NOT NULL CHECK(statement_ordinal >= 0),
 ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
 relation TEXT NOT NULL,
 PRIMARY KEY(header_id, link_id, statement_ordinal, ordinal),
 FOREIGN KEY(header_id, link_id, statement_ordinal)
   REFERENCES sql_link_statements(header_id, link_id, ordinal)
);

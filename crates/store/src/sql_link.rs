//! Durable host-verified code-to-SQL candidate graphs. Parser/source I/O stay
//! outside this module; its transactions only persist an already validated
//! aggregate or a tombstone.

use graph_application::{SqlLinkRepository, SqlLinkSnapshot};
use graph_domain::{
    SourceEvidence,
    sql_link::{
        CoordinateEncoding, SqlLink, SqlLinkGraph, SqlLinkScope, SqlOperation, SqlStatement,
    },
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::{
    Store, StoreError, domain_corruption,
    evidence::{record_source_in_transaction, source_evidence_in_connection},
};

struct Header {
    id: i64,
    generation: u64,
    version: Option<String>,
    code: Option<String>,
    links: usize,
}

fn header(db: &Connection, scope: &SqlLinkScope) -> Result<Option<Header>, StoreError> {
    let project = serde_json::to_string(&graph_protocol::ProjectRef::from(scope.project()))?;
    db.query_row("SELECT id,generation,adapter_version,code_evidence_id,link_count FROM sql_link_headers WHERE project=?1 AND graph_version=?2 AND code_path=?3 AND adapter=?4", params![project,scope.graph_version(),scope.code_path(),scope.adapter()], |r| Ok(Header { id:r.get(0)?, generation:r.get(1)?, version:r.get(2)?, code:r.get(3)?, links:r.get(4)? })).optional().map_err(Into::into)
}

fn citation(db: &Connection, id: &str, scope: &SqlLinkScope) -> Result<SourceEvidence, StoreError> {
    source_evidence_in_connection(db, id, scope.project(), scope.graph_version())?
        .ok_or_else(|| StoreError::Corrupt("missing SQL-link evidence".into()))
}

fn operation(value: SqlOperation) -> &'static str {
    match value {
        SqlOperation::Query => "query",
        SqlOperation::Insert => "insert",
        SqlOperation::Update => "update",
        SqlOperation::Delete => "delete",
        SqlOperation::CreateTable => "create_table",
        SqlOperation::Begin => "begin",
        SqlOperation::Commit => "commit",
        SqlOperation::Rollback => "rollback",
        SqlOperation::Other => "other",
    }
}
fn decode_operation(value: &str) -> Option<SqlOperation> {
    Some(match value {
        "query" => SqlOperation::Query,
        "insert" => SqlOperation::Insert,
        "update" => SqlOperation::Update,
        "delete" => SqlOperation::Delete,
        "create_table" => SqlOperation::CreateTable,
        "begin" => SqlOperation::Begin,
        "commit" => SqlOperation::Commit,
        "rollback" => SqlOperation::Rollback,
        "other" => SqlOperation::Other,
        _ => return None,
    })
}

impl Store {
    fn write_sql_links(
        &mut self,
        scope: &SqlLinkScope,
        graph: Option<&SqlLinkGraph>,
        expected: u64,
    ) -> Result<u64, StoreError> {
        let next = expected
            .checked_add(1)
            .filter(|n| i64::try_from(*n).is_ok())
            .ok_or(StoreError::Invalid("SQL-link generation exhausted"))?;
        let project = serde_json::to_string(&graph_protocol::ProjectRef::from(scope.project()))?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if header(&tx, scope)?.as_ref().map_or(0, |h| h.generation) != expected {
            return Err(StoreError::Conflict);
        }
        if let Some(graph) = graph {
            record_source_in_transaction(&tx, graph.code())?;
            for link in graph.links() {
                record_source_in_transaction(&tx, &link.target)?;
            }
        }
        tx.execute("INSERT INTO sql_link_headers(project,graph_version,code_path,adapter,generation,adapter_version,code_evidence_id,link_count) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(project,graph_version,code_path,adapter) DO UPDATE SET generation=excluded.generation,adapter_version=excluded.adapter_version,code_evidence_id=excluded.code_evidence_id,link_count=excluded.link_count", params![project,scope.graph_version(),scope.code_path(),scope.adapter(),next,graph.map(SqlLinkGraph::adapter_version),graph.map(|g|g.code().id()),graph.map_or(0,|g|g.links().len())])?;
        let id = header(&tx, scope)?
            .ok_or(StoreError::Invalid("SQL-link header missing after upsert"))?
            .id;
        tx.execute("DELETE FROM sql_link_relations WHERE header_id=?1", [id])?;
        tx.execute("DELETE FROM sql_link_statements WHERE header_id=?1", [id])?;
        tx.execute("DELETE FROM sql_links WHERE header_id=?1", [id])?;
        if let Some(graph) = graph {
            for (ordinal, link) in graph.links().iter().enumerate() {
                tx.execute("INSERT INTO sql_links(header_id,ordinal,link_id,target_evidence_id,candidate_path,start_offset,end_offset,coordinate_encoding,statement_count) VALUES(?1,?2,?3,?4,?5,?6,?7,'utf16_code_unit',?8)", params![id,ordinal,link.id,link.target.id(),link.candidate_path,link.start,link.end,link.statements.len()])?;
                for statement in &link.statements {
                    tx.execute("INSERT INTO sql_link_statements(header_id,link_id,ordinal,operation,relation_count) VALUES(?1,?2,?3,?4,?5)",params![id,link.id,statement.ordinal,operation(statement.operation),statement.relations.len()])?;
                    for (relation_ordinal, relation) in statement.relations.iter().enumerate() {
                        tx.execute("INSERT INTO sql_link_relations(header_id,link_id,statement_ordinal,ordinal,relation) VALUES(?1,?2,?3,?4,?5)",params![id,link.id,statement.ordinal,relation_ordinal,relation])?;
                    }
                }
            }
        }
        tx.commit()?;
        Ok(next)
    }
}

impl SqlLinkRepository for Store {
    type Error = StoreError;
    fn replace_sql_links(
        &mut self,
        graph: &SqlLinkGraph,
        expected_generation: u64,
    ) -> Result<u64, StoreError> {
        self.write_sql_links(&graph.scope(), Some(graph), expected_generation)
    }
    fn invalidate_sql_links(
        &mut self,
        scope: &SqlLinkScope,
        expected_generation: u64,
    ) -> Result<u64, StoreError> {
        self.write_sql_links(scope, None, expected_generation)
    }
    fn sql_links(&self, scope: &SqlLinkScope) -> Result<Option<SqlLinkSnapshot>, StoreError> {
        let tx = self.0.unchecked_transaction()?;
        let Some(h) = header(&tx, scope)? else {
            tx.commit()?;
            return Ok(None);
        };
        if h.links > 10_000 {
            return Err(StoreError::Corrupt("invalid SQL-link count".into()));
        }
        let graph = match (h.version, h.code) {
            (None, None) => None,
            (Some(version), Some(code_id)) => {
                let code = citation(&tx, &code_id, scope)?;
                if code.path() != scope.code_path() {
                    return Err(StoreError::Corrupt(
                        "SQL-link code evidence path mismatch".into(),
                    ));
                };
                let mut links = Vec::new();
                let mut statement = tx.prepare("SELECT ordinal,link_id,target_evidence_id,candidate_path,start_offset,end_offset,coordinate_encoding,statement_count FROM sql_links WHERE header_id=?1 ORDER BY ordinal")?;
                let mut rows = statement.query([h.id])?;
                while let Some(row) = rows.next()? {
                    if row.get::<_, usize>(0)? != links.len() || links.len() >= h.links {
                        return Err(StoreError::Corrupt("invalid SQL-link ordering".into()));
                    };
                    let link_id: String = row.get(1)?;
                    let target = citation(&tx, &row.get::<_, String>(2)?, scope)?;
                    let encoding: String = row.get(6)?;
                    let statements = read_statements(&tx, h.id, &link_id, row.get(7)?)?;
                    links.push(SqlLink {
                        id: link_id,
                        target,
                        candidate_path: row.get(3)?,
                        start: row.get(4)?,
                        end: row.get(5)?,
                        coordinate_encoding: match encoding.as_str() {
                            "utf16_code_unit" => CoordinateEncoding::Utf16CodeUnit,
                            _ => {
                                return Err(StoreError::Corrupt(
                                    "invalid SQL-link encoding".into(),
                                ));
                            }
                        },
                        statements,
                    });
                }
                drop(rows);
                drop(statement);
                if links.len() != h.links {
                    return Err(StoreError::Corrupt("SQL-link count mismatch".into()));
                };
                Some(
                    SqlLinkGraph::new(scope.adapter().into(), version, code, links)
                        .map_err(domain_corruption)?,
                )
            }
            _ => return Err(StoreError::Corrupt("invalid SQL-link tombstone".into())),
        };
        tx.commit()?;
        Ok(Some(SqlLinkSnapshot {
            generation: h.generation,
            graph,
        }))
    }
}

fn read_statements(
    db: &Connection,
    header_id: i64,
    link_id: &str,
    expected: usize,
) -> Result<Vec<SqlStatement>, StoreError> {
    let mut statements = Vec::new();
    let mut statement = db.prepare("SELECT ordinal,operation,relation_count FROM sql_link_statements WHERE header_id=?1 AND link_id=?2 ORDER BY ordinal")?;
    let mut rows = statement.query(params![header_id, link_id])?;
    while let Some(row) = rows.next()? {
        let ordinal: u16 = row.get(0)?;
        if usize::from(ordinal) != statements.len() || statements.len() >= expected {
            return Err(StoreError::Corrupt(
                "invalid SQL-link statement ordering".into(),
            ));
        };
        let count: usize = row.get(2)?;
        let mut relations = Vec::new();
        let mut relation_statement = db.prepare("SELECT ordinal,relation FROM sql_link_relations WHERE header_id=?1 AND link_id=?2 AND statement_ordinal=?3 ORDER BY ordinal")?;
        let mut relation_rows = relation_statement.query(params![header_id, link_id, ordinal])?;
        while let Some(relation) = relation_rows.next()? {
            if relation.get::<_, usize>(0)? != relations.len() || relations.len() >= count {
                return Err(StoreError::Corrupt(
                    "invalid SQL-link relation ordering".into(),
                ));
            };
            relations.push(relation.get(1)?);
        }
        drop(relation_rows);
        drop(relation_statement);
        if relations.len() != count {
            return Err(StoreError::Corrupt(
                "SQL-link relation count mismatch".into(),
            ));
        };
        statements.push(SqlStatement {
            ordinal,
            operation: decode_operation(&row.get::<_, String>(1)?)
                .ok_or_else(|| StoreError::Corrupt("invalid SQL-link operation".into()))?,
            relations,
        });
    }
    drop(rows);
    drop(statement);
    if statements.len() != expected {
        return Err(StoreError::Corrupt(
            "SQL-link statement count mismatch".into(),
        ));
    };
    Ok(statements)
}

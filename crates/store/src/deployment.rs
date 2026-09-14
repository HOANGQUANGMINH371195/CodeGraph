//! Relational declared graphs. Transactions never cross parser or source I/O.
use graph_application::{DeploymentRepository, DeploymentSnapshot};
use graph_domain::{
    SourceEvidence,
    deployment::{DeploymentGraph, DeploymentScope, Edge, EdgeKind, Node, NodeKind, Unknown},
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
    evidence: Option<String>,
    nodes: usize,
    edges: usize,
    unknowns: usize,
}

fn header(db: &Connection, scope: &DeploymentScope) -> Result<Option<Header>, StoreError> {
    let project = serde_json::to_string(&graph_protocol::ProjectRef::from(scope.project()))?;
    Ok(db
        .query_row(
            include_str!("sql/select_deployment_header.sql"),
            params![
                project,
                scope.graph_version(),
                scope.path(),
                scope.adapter()
            ],
            |r| {
                Ok(Header {
                    id: r.get(0)?,
                    generation: r.get(1)?,
                    version: r.get(2)?,
                    evidence: r.get(3)?,
                    nodes: r.get(4)?,
                    edges: r.get(5)?,
                    unknowns: r.get(6)?,
                })
            },
        )
        .optional()?)
}

fn corrupt() -> StoreError {
    StoreError::Corrupt("invalid declared deployment graph".into())
}

fn citation(
    db: &Connection,
    id: &str,
    scope: &DeploymentScope,
) -> Result<SourceEvidence, StoreError> {
    let evidence = source_evidence_in_connection(db, id, scope.project(), scope.graph_version())?
        .ok_or_else(corrupt)?;
    if evidence.path() != scope.path() {
        return Err(corrupt());
    }
    Ok(evidence)
}

impl Store {
    fn write_deployment(
        &mut self,
        scope: &DeploymentScope,
        graph: Option<&DeploymentGraph>,
        expected: u64,
    ) -> Result<u64, StoreError> {
        let next = expected
            .checked_add(1)
            .filter(|n| i64::try_from(*n).is_ok())
            .ok_or(StoreError::Invalid("deployment generation exhausted"))?;
        let project = serde_json::to_string(&graph_protocol::ProjectRef::from(scope.project()))?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let old = header(&tx, scope)?;
        if old.as_ref().map_or(0, |h| h.generation) != expected {
            return Err(StoreError::Conflict);
        }
        if let Some(graph) = graph {
            record_source_in_transaction(&tx, graph.evidence())?;
            for node in graph.nodes() {
                record_source_in_transaction(&tx, &node.evidence)?;
            }
            for edge in graph.edges() {
                record_source_in_transaction(&tx, &edge.evidence)?;
            }
        }
        let id: i64 = tx.query_row(
            include_str!("sql/upsert_deployment_header.sql"),
            params![
                project,
                scope.graph_version(),
                scope.path(),
                scope.adapter(),
                next,
                graph.map(DeploymentGraph::adapter_version),
                graph.map(|g| g.evidence().id()),
                graph.map_or(0, |g| g.nodes().len()),
                graph.map_or(0, |g| g.edges().len()),
                graph.map_or(0, |g| g.unknowns().len()),
            ],
            |r| r.get(0),
        )?;
        tx.execute(include_str!("sql/delete_deployment_edges.sql"), [id])?;
        tx.execute(include_str!("sql/delete_deployment_nodes.sql"), [id])?;
        tx.execute(include_str!("sql/delete_deployment_unknowns.sql"), [id])?;
        if let Some(graph) = graph {
            let mut insert = tx.prepare(include_str!("sql/insert_deployment_node.sql"))?;
            for (ordinal, node) in graph.nodes().iter().enumerate() {
                let kind = match node.kind {
                    NodeKind::Service => "service",
                    NodeKind::Volume => "volume",
                    NodeKind::Network => "network",
                };
                insert.execute(params![
                    id,
                    ordinal,
                    node.id,
                    kind,
                    node.name,
                    node.evidence.id()
                ])?;
            }
            let mut insert = tx.prepare(include_str!("sql/insert_deployment_edge.sql"))?;
            for (ordinal, edge) in graph.edges().iter().enumerate() {
                let kind = match edge.kind {
                    EdgeKind::Mounts => "mounts",
                    EdgeKind::DependsOn => "depends_on",
                    EdgeKind::AttachedTo => "attached_to",
                };
                insert.execute(params![
                    id,
                    ordinal,
                    edge.source,
                    edge.target,
                    kind,
                    edge.mount_target,
                    edge.evidence.id()
                ])?;
            }
            let mut insert = tx.prepare(include_str!("sql/insert_deployment_unknown.sql"))?;
            for (ordinal, unknown) in graph.unknowns().iter().enumerate() {
                insert.execute(params![id, ordinal, unknown.reason, unknown.line])?;
            }
        }
        tx.commit()?;
        Ok(next)
    }
}

impl DeploymentRepository for Store {
    type Error = StoreError;

    fn replace_deployment(
        &mut self,
        graph: &DeploymentGraph,
        expected_generation: u64,
    ) -> Result<u64, StoreError> {
        self.write_deployment(&graph.scope(), Some(graph), expected_generation)
    }

    fn invalidate_deployment(
        &mut self,
        scope: &DeploymentScope,
        expected_generation: u64,
    ) -> Result<u64, StoreError> {
        self.write_deployment(scope, None, expected_generation)
    }

    fn deployment(
        &self,
        scope: &DeploymentScope,
    ) -> Result<Option<DeploymentSnapshot>, StoreError> {
        // unchecked_transaction takes &self, but SQLite rejects nesting. This
        // private connection has no externally exposed transaction capability.
        let tx = self.0.unchecked_transaction()?;
        let Some(h) = header(&tx, scope)? else {
            tx.commit()?;
            return Ok(None);
        };
        if h.generation == 0 || h.nodes > 10_000 || h.edges > 50_000 || h.unknowns > 50_000 {
            return Err(corrupt());
        }
        let mut nodes = Vec::new();
        let mut statement = tx.prepare(include_str!("sql/select_deployment_nodes.sql"))?;
        let mut rows = statement.query([h.id])?;
        while let Some(r) = rows.next()? {
            if r.get::<_, usize>(0)? != nodes.len() || nodes.len() >= h.nodes {
                return Err(corrupt());
            }
            let kind = match r.get::<_, String>(2)?.as_str() {
                "service" => NodeKind::Service,
                "volume" => NodeKind::Volume,
                "network" => NodeKind::Network,
                _ => return Err(corrupt()),
            };
            nodes.push(Node {
                id: r.get(1)?,
                kind,
                name: r.get(3)?,
                evidence: citation(&tx, &r.get::<_, String>(4)?, scope)?,
            });
        }
        drop(rows);
        drop(statement);
        let mut edges = Vec::new();
        let mut statement = tx.prepare(include_str!("sql/select_deployment_edges.sql"))?;
        let mut rows = statement.query([h.id])?;
        while let Some(r) = rows.next()? {
            if r.get::<_, usize>(0)? != edges.len() || edges.len() >= h.edges {
                return Err(corrupt());
            }
            let kind = match r.get::<_, String>(3)?.as_str() {
                "mounts" => EdgeKind::Mounts,
                "depends_on" => EdgeKind::DependsOn,
                "attached_to" => EdgeKind::AttachedTo,
                _ => return Err(corrupt()),
            };
            edges.push(Edge {
                source: r.get(1)?,
                target: r.get(2)?,
                kind,
                mount_target: r.get(4)?,
                evidence: citation(&tx, &r.get::<_, String>(5)?, scope)?,
            });
        }
        drop(rows);
        drop(statement);
        let mut unknowns = Vec::new();
        let mut statement = tx.prepare(include_str!("sql/select_deployment_unknowns.sql"))?;
        let mut rows = statement.query([h.id])?;
        while let Some(r) = rows.next()? {
            if r.get::<_, usize>(0)? != unknowns.len() || unknowns.len() >= h.unknowns {
                return Err(corrupt());
            }
            unknowns.push(Unknown {
                reason: r.get(1)?,
                line: r.get(2)?,
            });
        }
        drop(rows);
        drop(statement);
        if nodes.len() != h.nodes || edges.len() != h.edges || unknowns.len() != h.unknowns {
            return Err(corrupt());
        }
        let graph = match (h.version, h.evidence) {
            (Some(version), Some(id)) => Some(
                DeploymentGraph::new(
                    scope.adapter().into(),
                    version,
                    citation(&tx, &id, scope)?,
                    nodes,
                    edges,
                    unknowns,
                )
                .map_err(domain_corruption)?,
            ),
            (None, None) if nodes.is_empty() && edges.is_empty() && unknowns.is_empty() => None,
            _ => return Err(corrupt()),
        };
        tx.commit()?;
        Ok(Some(DeploymentSnapshot {
            generation: h.generation,
            graph,
        }))
    }
}

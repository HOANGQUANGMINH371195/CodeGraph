use graph_application::{DeploymentRepository, EvidenceRepository};
use graph_domain::{
    ProjectRef, SourceEvidence,
    deployment::{DeploymentGraph, Edge, EdgeKind, Node, NodeKind, Unknown},
};
use graph_store::{Store, StoreError};
use rusqlite::Connection;

fn evidence(id: &str, path: &str) -> SourceEvidence {
    SourceEvidence::new(
        id.into(),
        ProjectRef {
            repository_id: "r".into(),
            worktree_id: "w".into(),
            git_head: "h".into(),
            working_tree_fingerprint: "f".into(),
            config_hash: "c".into(),
            ignore_policy_version: "1".into(),
        },
        "g".into(),
        path.into(),
        "a".repeat(64),
        1,
        10,
        "run".into(),
    )
    .unwrap()
}

fn graph(id: &str) -> DeploymentGraph {
    let e = evidence(id, "compose.yaml");
    DeploymentGraph::new(
        "compose".into(),
        "1".into(),
        e.clone(),
        vec![
            Node {
                id: "service".into(),
                kind: NodeKind::Service,
                name: "app".into(),
                evidence: e.clone(),
            },
            Node {
                id: "volume".into(),
                kind: NodeKind::Volume,
                name: "data".into(),
                evidence: e.clone(),
            },
        ],
        vec![Edge {
            source: "service".into(),
            target: "volume".into(),
            kind: EdgeKind::Mounts,
            mount_target: Some("/data".into()),
            evidence: e,
        }],
        vec![Unknown {
            reason: "unmodeled field".into(),
            line: 2,
        }],
    )
    .unwrap()
}

#[test]
fn relational_corruption_is_rejected_not_returned_as_absence_or_partial_graph() {
    for sql in [
        include_str!("fixtures/deployment/delete_node.sql"),
        include_str!("fixtures/deployment/delete_edge.sql"),
        include_str!("fixtures/deployment/delete_unknown.sql"),
        include_str!("fixtures/deployment/wrong_endpoint.sql"),
        include_str!("fixtures/deployment/wrong_kind.sql"),
        include_str!("fixtures/deployment/wrong_citation.sql"),
        include_str!("fixtures/deployment/wrong_ordinal.sql"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("graph.db");
        let mut store = Store::open(&path).unwrap();
        let g = graph("root");
        store.replace_deployment(&g, 0).unwrap();
        store
            .record_source(&evidence("foreign", "foreign.yaml"))
            .unwrap();
        // Deliberately bypass runtime connection safeguards in this owned fixture.
        let fixture = Connection::open(&path).unwrap();
        fixture.pragma_update(None, "foreign_keys", false).unwrap();
        fixture.execute_batch(sql).unwrap();
        assert!(
            matches!(store.deployment(&g.scope()), Err(StoreError::Corrupt(_))),
            "{sql}"
        );
    }
}

#[test]
fn late_edge_failure_rolls_back_header_deletions_nodes_and_new_citation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("graph.db");
    let mut store = Store::open(&path).unwrap();
    let old = graph("old");
    store.replace_deployment(&old, 0).unwrap();
    let before = store.deployment(&old.scope()).unwrap();
    let fixture = Connection::open(&path).unwrap();
    fixture
        .execute_batch(include_str!("fixtures/deployment/reject_edge.sql"))
        .unwrap();
    let next = graph("new");
    assert!(matches!(
        store.replace_deployment(&next, 1),
        Err(StoreError::Sql(_))
    ));
    assert_eq!(store.deployment(&old.scope()).unwrap(), before);
    assert!(
        store
            .source_evidence("new", old.evidence().project(), "g")
            .unwrap()
            .is_none()
    );
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.deployment(&old.scope()).unwrap(), before);
    fixture
        .execute_batch(include_str!("fixtures/deployment/remove_reject_edge.sql"))
        .unwrap();
    assert_eq!(store.replace_deployment(&next, 1).unwrap(), 2);
    assert_eq!(
        store.deployment(&next.scope()).unwrap().unwrap().graph,
        Some(next)
    );
}

#[test]
fn generation_overflow_never_wraps_or_mutates_existing_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("graph.db");
    let mut store = Store::open(&path).unwrap();
    let g = graph("root");
    store.replace_deployment(&g, 0).unwrap();
    Connection::open(&path)
        .unwrap()
        .execute_batch(include_str!("fixtures/deployment/max_generation.sql"))
        .unwrap();
    let before = store.deployment(&g.scope()).unwrap();
    for generation in [i64::MAX as u64, u64::MAX] {
        assert!(matches!(
            store.replace_deployment(&g, generation),
            Err(StoreError::Invalid(_))
        ));
        assert!(matches!(
            store.invalidate_deployment(&g.scope(), generation),
            Err(StoreError::Invalid(_))
        ));
    }
    assert_eq!(store.deployment(&g.scope()).unwrap(), before);
}

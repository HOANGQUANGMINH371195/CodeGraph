use graph_application::SqlLinkRepository;
use graph_domain::sql_link::{
    CoordinateEncoding, SqlLink, SqlLinkGraph, SqlOperation, SqlStatement,
};
use graph_domain::{ProjectRef, SourceEvidence};
use graph_store::{Store, StoreError};

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn project() -> ProjectRef {
    ProjectRef {
        repository_id: "repo".into(),
        worktree_id: "main".into(),
        git_head: "head".into(),
        working_tree_fingerprint: "tree".into(),
        config_hash: "config".into(),
        ignore_policy_version: "ignore".into(),
    }
}
fn evidence(id: &str, path: &str) -> SourceEvidence {
    SourceEvidence::new(
        id.into(),
        project(),
        "g1".into(),
        path.into(),
        HASH.into(),
        1,
        10,
        "run".into(),
    )
    .unwrap()
}
fn graph(version: &str) -> SqlLinkGraph {
    let code = evidence("code", "src/store.ts");
    SqlLinkGraph::new(
        "codegraph-file-reads".into(),
        version.into(),
        code,
        vec![SqlLink {
            id: "link".into(),
            target: evidence("sql", "sql/orders.sql"),
            candidate_path: "sql/orders.sql".into(),
            start: 1,
            end: 4,
            coordinate_encoding: CoordinateEncoding::Utf16CodeUnit,
            statements: vec![SqlStatement {
                ordinal: 0,
                operation: SqlOperation::Insert,
                relations: vec!["orders".into()],
            }],
        }],
    )
    .unwrap()
}

#[test]
fn roundtrip_cas_tombstone_and_reopen_preserve_candidate_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("links.db");
    let first = graph("1");
    let scope = first.scope();
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.sql_links(&scope).unwrap(), None);
    assert_eq!(store.replace_sql_links(&first, 0).unwrap(), 1);
    assert!(matches!(
        store.replace_sql_links(&graph("2"), 0),
        Err(StoreError::Conflict)
    ));
    assert_eq!(store.invalidate_sql_links(&scope, 1).unwrap(), 2);
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.sql_links(&scope).unwrap().unwrap().graph, None);
    assert_eq!(store.replace_sql_links(&graph("2"), 2).unwrap(), 3);
    let snapshot = store.sql_links(&scope).unwrap().unwrap();
    assert_eq!(snapshot.generation, 3);
    assert_eq!(snapshot.graph, Some(graph("2")));
}

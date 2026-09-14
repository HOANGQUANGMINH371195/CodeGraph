use graph_domain::sql_link::{
    CoordinateEncoding, SqlLink, SqlLinkGraph, SqlOperation, SqlStatement,
};
use graph_domain::{DomainError, ProjectRef, SourceEvidence};

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn project() -> ProjectRef {
    ProjectRef {
        repository_id: "repo".into(),
        worktree_id: "main".into(),
        git_head: "head".into(),
        working_tree_fingerprint: "tree".into(),
        config_hash: "config".into(),
        ignore_policy_version: "ignore-v1".into(),
    }
}

fn evidence(id: &str, path: &str, start: u32, end: u32) -> SourceEvidence {
    SourceEvidence::new(
        id.into(),
        project(),
        "graph-v1".into(),
        path.into(),
        HASH.into(),
        start,
        end,
        "run-1".into(),
    )
    .unwrap()
}

fn link(id: &str) -> SqlLink {
    SqlLink {
        id: id.into(),
        target: evidence("sql-1", "sql/orders.sql", 1, 4),
        candidate_path: "sql/orders.sql".into(),
        start: 10,
        end: 35,
        coordinate_encoding: CoordinateEncoding::Utf16CodeUnit,
        statements: vec![SqlStatement {
            ordinal: 0,
            operation: SqlOperation::Insert,
            relations: vec!["orders".into()],
        }],
    }
}

fn graph(links: Vec<SqlLink>) -> Result<SqlLinkGraph, DomainError> {
    SqlLinkGraph::new(
        "codegraph-file-reads".into(),
        "1".into(),
        evidence("code-1", "src/store.ts", 1, 80),
        links,
    )
}

#[test]
fn candidate_graph_is_scoped_and_explicitly_structural() {
    let graph = graph(vec![link("read-orders")]).unwrap();
    assert_eq!(graph.scope().adapter(), "codegraph-file-reads");
    assert_eq!(graph.scope().code_path(), "src/store.ts");
    assert_eq!(graph.code().id(), "code-1");
    assert_eq!(graph.links()[0].target.path(), "sql/orders.sql");
    assert_eq!(
        graph.links()[0].statements[0].operation,
        SqlOperation::Insert
    );
}

#[test]
fn rejects_non_full_sources_mixed_snapshot_duplicate_ids_and_invalid_extent() {
    assert!(
        SqlLinkGraph::new(
            "a".into(),
            "v".into(),
            evidence("code", "src/a.ts", 2, 3),
            vec![]
        )
        .is_err()
    );

    let mut mixed = link("one");
    mixed.target = SourceEvidence::new(
        "sql-2".into(),
        project(),
        "other".into(),
        "sql/x.sql".into(),
        HASH.into(),
        1,
        2,
        "run-1".into(),
    )
    .unwrap();
    mixed.candidate_path = "sql/x.sql".into();
    assert!(graph(vec![mixed]).is_err());

    assert!(graph(vec![link("same"), link("same")]).is_err());
    let mut invalid = link("extent");
    invalid.end = invalid.start;
    assert!(graph(vec![invalid]).is_err());
}

#[test]
fn rejects_target_path_mismatch_partial_target_and_statement_gap() {
    let mut mismatch = link("path");
    mismatch.candidate_path = "sql/not-orders.sql".into();
    assert!(graph(vec![mismatch]).is_err());

    let mut partial = link("partial");
    partial.target = evidence("sql-1", "sql/orders.sql", 2, 4);
    assert!(graph(vec![partial]).is_err());

    let mut statement_gap = link("statements");
    statement_gap.statements[0].ordinal = 1;
    assert!(graph(vec![statement_gap]).is_err());
}

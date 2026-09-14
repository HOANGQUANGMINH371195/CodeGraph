use graph_application::{DeploymentRepository, EvidenceRepository};
use graph_domain::deployment::{
    DeploymentGraph, DeploymentScope, Edge, EdgeKind, Node, NodeKind, Unknown,
};
use graph_domain::{ProjectRef, SourceEvidence};
use graph_store::{Store, StoreError};
use std::sync::{Arc, Barrier};
use std::thread;

const GRAPH_VERSION: &str = "graph-v1";
const SOURCE_PATH: &str = "compose.yaml";
const SOURCE_HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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

fn evidence(
    id: &str,
    project: ProjectRef,
    graph_version: &str,
    path: &str,
    hash: &str,
) -> SourceEvidence {
    SourceEvidence::new(
        id.into(),
        project,
        graph_version.into(),
        path.into(),
        hash.into(),
        1,
        10,
        "run-1".into(),
    )
    .expect("test source evidence must be valid")
}

fn source(id: &str) -> SourceEvidence {
    evidence(id, project(), GRAPH_VERSION, SOURCE_PATH, SOURCE_HASH)
}

fn graph_with_source(
    source: SourceEvidence,
    adapter_version: &str,
    node_evidence: Option<SourceEvidence>,
) -> DeploymentGraph {
    let service_evidence = node_evidence.unwrap_or_else(|| source.clone());
    DeploymentGraph::new(
        "compose".into(),
        adapter_version.into(),
        source.clone(),
        vec![
            Node {
                id: "service-api".into(),
                kind: NodeKind::Service,
                name: "api".into(),
                evidence: service_evidence,
            },
            Node {
                id: "volume-data".into(),
                kind: NodeKind::Volume,
                name: "data".into(),
                evidence: source.clone(),
            },
            Node {
                id: "network-front".into(),
                kind: NodeKind::Network,
                name: "front".into(),
                evidence: source.clone(),
            },
            Node {
                id: "service-db".into(),
                kind: NodeKind::Service,
                name: "db".into(),
                evidence: source.clone(),
            },
        ],
        vec![
            Edge {
                source: "service-api".into(),
                target: "volume-data".into(),
                kind: EdgeKind::Mounts,
                mount_target: Some("/var/lib/data".into()),
                evidence: source.clone(),
            },
            Edge {
                source: "service-api".into(),
                target: "service-db".into(),
                kind: EdgeKind::DependsOn,
                mount_target: None,
                evidence: source.clone(),
            },
            Edge {
                source: "service-api".into(),
                target: "network-front".into(),
                kind: EdgeKind::AttachedTo,
                mount_target: None,
                evidence: source.clone(),
            },
        ],
        vec![Unknown {
            reason: "unresolved extension".into(),
            line: 10,
        }],
    )
    .expect("test deployment graph must be valid")
}

fn graph(id: &str, adapter_version: &str) -> DeploymentGraph {
    graph_with_source(source(id), adapter_version, None)
}

fn empty_graph() -> DeploymentGraph {
    DeploymentGraph::new(
        "compose".into(),
        "2.0".into(),
        source("empty-root"),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("empty deployment graph must be valid")
}

fn scope_for(
    adapter: &str,
    project: ProjectRef,
    graph_version: &str,
    path: &str,
) -> DeploymentScope {
    DeploymentScope::new(
        adapter.into(),
        &evidence("scope", project, graph_version, path, SOURCE_HASH),
    )
    .expect("test deployment scope must be valid")
}

#[test]
fn full_graph_round_trips_with_all_kinds_and_unknowns_after_reopen() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("deployment.sqlite");
    let expected = graph("full-root", "2.0");
    let scope = expected.scope();

    assert_eq!(scope.adapter(), "compose");
    assert_eq!(scope.project(), expected.evidence().project());
    assert_eq!(scope.graph_version(), GRAPH_VERSION);
    assert_eq!(scope.path(), SOURCE_PATH);
    assert_eq!(
        scope,
        DeploymentScope::new("compose".into(), expected.evidence()).unwrap()
    );

    let mut store = Store::open(&path).expect("open store");
    assert_eq!(store.deployment(&scope).unwrap(), None);
    assert_eq!(store.replace_deployment(&expected, 0).unwrap(), 1);
    drop(store);

    let store = Store::open(&path).expect("reopen store");
    let snapshot = store
        .deployment(&scope)
        .expect("read deployment")
        .expect("deployment exists after reopen");
    assert_eq!(snapshot.generation, 1);
    assert_eq!(snapshot.graph, Some(expected));
}

#[test]
fn adapter_version_is_not_scope_and_replacement_preserves_generation_cas() {
    let mut store = Store::open(":memory:").expect("open in-memory store");
    let first = graph("version-root", "2.0");
    let upgraded = graph("version-root", "3.0");
    let scope = first.scope();

    assert_eq!(store.replace_deployment(&first, 0).unwrap(), 1);
    assert_eq!(store.replace_deployment(&upgraded, 1).unwrap(), 2);
    let snapshot = store.deployment(&scope).unwrap().unwrap();
    assert_eq!(snapshot.generation, 2);
    assert_eq!(snapshot.graph, Some(upgraded));
}

#[test]
fn empty_graph_and_invalidation_tombstone_are_distinct_and_recoverable() {
    let mut store = Store::open(":memory:").expect("open in-memory store");
    let empty = empty_graph();
    let scope = empty.scope();

    assert_eq!(store.deployment(&scope).unwrap(), None);
    assert_eq!(store.replace_deployment(&empty, 0).unwrap(), 1);
    assert_eq!(
        store.deployment(&scope).unwrap(),
        Some(graph_application::DeploymentSnapshot {
            generation: 1,
            graph: Some(empty.clone()),
        })
    );

    assert_eq!(store.invalidate_deployment(&scope, 1).unwrap(), 2);
    assert_eq!(
        store.deployment(&scope).unwrap(),
        Some(graph_application::DeploymentSnapshot {
            generation: 2,
            graph: None,
        })
    );

    let recovered = graph("recovered-root", "2.0");
    assert_eq!(store.replace_deployment(&recovered, 2).unwrap(), 3);
    assert_eq!(
        store.deployment(&scope).unwrap(),
        Some(graph_application::DeploymentSnapshot {
            generation: 3,
            graph: Some(recovered),
        })
    );
}

#[test]
fn stale_replace_and_invalidate_are_conflicts_without_mutating_snapshot() {
    let mut store = Store::open(":memory:").expect("open in-memory store");
    let first = graph("cas-root", "2.0");
    let replacement = graph("cas-replacement", "2.0");
    let scope = first.scope();

    assert_eq!(store.replace_deployment(&first, 0).unwrap(), 1);
    let before = store.deployment(&scope).unwrap();
    assert!(matches!(
        store.replace_deployment(&replacement, 0),
        Err(StoreError::Conflict)
    ));
    assert_eq!(store.deployment(&scope).unwrap(), before);
    assert!(matches!(
        store.invalidate_deployment(&scope, 0),
        Err(StoreError::Conflict)
    ));
    assert_eq!(store.deployment(&scope).unwrap(), before);
}

#[test]
fn every_owner_dimension_isolation_prevents_cross_scope_reads() {
    let mut store = Store::open(":memory:").expect("open in-memory store");
    let expected = graph("isolation-root", "2.0");
    let scope = expected.scope();
    store.replace_deployment(&expected, 0).unwrap();

    let mut variants = vec![(
        "adapter",
        scope_for("other-adapter", project(), GRAPH_VERSION, SOURCE_PATH),
    )];
    let mut changed_project = project();
    changed_project.repository_id.push_str("-other");
    variants.push((
        "repository_id",
        scope_for("compose", changed_project, GRAPH_VERSION, SOURCE_PATH),
    ));
    let mut changed_project = project();
    changed_project.worktree_id.push_str("-other");
    variants.push((
        "worktree_id",
        scope_for("compose", changed_project, GRAPH_VERSION, SOURCE_PATH),
    ));
    let mut changed_project = project();
    changed_project.git_head.push_str("-other");
    variants.push((
        "git_head",
        scope_for("compose", changed_project, GRAPH_VERSION, SOURCE_PATH),
    ));
    let mut changed_project = project();
    changed_project.working_tree_fingerprint.push_str("-other");
    variants.push((
        "working_tree_fingerprint",
        scope_for("compose", changed_project, GRAPH_VERSION, SOURCE_PATH),
    ));
    let mut changed_project = project();
    changed_project.config_hash.push_str("-other");
    variants.push((
        "config_hash",
        scope_for("compose", changed_project, GRAPH_VERSION, SOURCE_PATH),
    ));
    let mut changed_project = project();
    changed_project.ignore_policy_version.push_str("-other");
    variants.push((
        "ignore_policy_version",
        scope_for("compose", changed_project, GRAPH_VERSION, SOURCE_PATH),
    ));
    variants.push((
        "graph_version",
        scope_for("compose", project(), "graph-v2", SOURCE_PATH),
    ));
    variants.push((
        "path",
        scope_for("compose", project(), GRAPH_VERSION, "other-compose.yaml"),
    ));

    let mut foreign = Vec::new();
    for (label, variant) in variants {
        assert_eq!(store.deployment(&variant).unwrap(), None, "{label} leaked");
        let source = evidence(
            &format!("foreign-{label}"),
            variant.project().clone(),
            variant.graph_version(),
            variant.path(),
            SOURCE_HASH,
        );
        let candidate = graph_with_source(source, "1", None);
        let candidate = DeploymentGraph::new(
            variant.adapter().into(),
            "1".into(),
            candidate.evidence().clone(),
            candidate.nodes().to_vec(),
            candidate.edges().to_vec(),
            candidate.unknowns().to_vec(),
        )
        .unwrap();
        assert_eq!(store.replace_deployment(&candidate, 0).unwrap(), 1);
        foreign.push((variant, candidate));
    }
    assert_eq!(store.invalidate_deployment(&scope, 1).unwrap(), 2);
    for (variant, candidate) in foreign {
        let saved = store.deployment(&variant).unwrap().unwrap();
        assert_eq!(saved.generation, 1);
        assert_eq!(saved.graph, Some(candidate));
    }
}

#[test]
fn empty_replacement_and_invalidation_retract_rows_but_keep_historical_citations() {
    let mut store = Store::open(":memory:").unwrap();
    let original = graph("original", "1");
    let scope = original.scope();
    assert_eq!(store.replace_deployment(&original, 0).unwrap(), 1);
    let empty = empty_graph();
    assert_eq!(store.replace_deployment(&empty, 1).unwrap(), 2);
    assert_eq!(
        store.deployment(&scope).unwrap().unwrap().graph,
        Some(empty)
    );
    assert_eq!(store.replace_deployment(&original, 2).unwrap(), 3);
    assert_eq!(store.invalidate_deployment(&scope, 3).unwrap(), 4);
    assert_eq!(store.deployment(&scope).unwrap().unwrap().graph, None);
    assert_eq!(
        store
            .source_evidence("original", original.evidence().project(), GRAPH_VERSION)
            .unwrap(),
        Some(original.evidence().clone())
    );
    let absent = scope_for("absent", project(), GRAPH_VERSION, SOURCE_PATH);
    assert_eq!(store.invalidate_deployment(&absent, 0).unwrap(), 1);
    let tombstone = store.deployment(&absent).unwrap().unwrap();
    assert_eq!(tombstone.generation, 1);
    assert_eq!(tombstone.graph, None);
}

#[test]
fn citation_conflict_rolls_back_new_root_and_graph_rows() {
    let mut store = Store::open(":memory:").expect("open in-memory store");
    let conflicting = evidence(
        "node-collision",
        project(),
        GRAPH_VERSION,
        "foreign.yaml",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    assert!(store.record_source(&conflicting).unwrap());

    let root = source("new-root");
    let node_citation = source("node-collision");
    let candidate = graph_with_source(root.clone(), "2.0", Some(node_citation));
    let scope = candidate.scope();

    assert!(matches!(
        store.replace_deployment(&candidate, 0),
        Err(StoreError::Conflict)
    ));
    assert_eq!(store.deployment(&scope).unwrap(), None);
    assert_eq!(
        store
            .source_evidence("new-root", root.project(), root.graph_version())
            .unwrap(),
        None
    );
    assert_eq!(
        store
            .source_evidence(
                conflicting.id(),
                conflicting.project(),
                conflicting.graph_version(),
            )
            .unwrap(),
        Some(conflicting)
    );
}

#[test]
fn concurrent_generation_zero_writers_have_exactly_one_success() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("concurrent-deployment.sqlite");
    let first = graph("concurrent-first", "2.0");
    let second = graph("concurrent-second", "2.0");
    let expected_first = first.clone();
    let expected_second = second.clone();
    let scope = first.scope();
    let barrier = Arc::new(Barrier::new(2));

    let first_store = Store::open(&path).expect("open first writer");
    let second_store = Store::open(&path).expect("open second writer");
    let first_barrier = Arc::clone(&barrier);
    let first_handle = thread::spawn(move || {
        let mut store = first_store;
        first_barrier.wait();
        store.replace_deployment(&first, 0)
    });
    let second_barrier = Arc::clone(&barrier);
    let second_handle = thread::spawn(move || {
        let mut store = second_store;
        second_barrier.wait();
        store.replace_deployment(&second, 0)
    });

    let outcomes = [
        first_handle
            .join()
            .expect("first writer thread")
            .map(|_| ()),
        second_handle
            .join()
            .expect("second writer thread")
            .map(|_| ()),
    ];
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(StoreError::Conflict)))
            .count(),
        1
    );

    let store = Store::open(&path).expect("reopen concurrent store");
    let snapshot = store.deployment(&scope).unwrap().unwrap();
    assert_eq!(snapshot.generation, 1);
    assert!(snapshot.graph == Some(expected_first) || snapshot.graph == Some(expected_second));
}

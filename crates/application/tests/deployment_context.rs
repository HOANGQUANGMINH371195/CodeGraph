use graph_application::deployment_context::{ContextLimits, Direction, select};
use graph_domain::deployment::{DeploymentGraph, Edge, EdgeKind, Node, NodeKind};
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

fn evidence() -> SourceEvidence {
    SourceEvidence::new(
        "compose-source".into(),
        project(),
        "graph-v1".into(),
        "compose.yaml".into(),
        HASH.into(),
        1,
        32,
        "analysis-1".into(),
    )
    .expect("test evidence must satisfy the public constructor")
}

fn node(id: &str, kind: NodeKind, evidence: &SourceEvidence) -> Node {
    Node {
        id: id.into(),
        kind,
        name: id.into(),
        evidence: evidence.clone(),
    }
}

fn edge(
    source: &str,
    target: &str,
    kind: EdgeKind,
    mount_target: Option<&str>,
    evidence: &SourceEvidence,
) -> Edge {
    Edge {
        source: source.into(),
        target: target.into(),
        kind,
        mount_target: mount_target.map(Into::into),
        evidence: evidence.clone(),
    }
}

fn graph() -> DeploymentGraph {
    let evidence = evidence();
    DeploymentGraph::new(
        "compose".into(),
        "2.0".into(),
        evidence.clone(),
        vec![
            node("api", NodeKind::Service, &evidence),
            node("db", NodeKind::Service, &evidence),
            node("cache", NodeKind::Service, &evidence),
            node("data", NodeKind::Volume, &evidence),
            node("front", NodeKind::Network, &evidence),
            node("worker", NodeKind::Service, &evidence),
            node("isolated", NodeKind::Volume, &evidence),
        ],
        vec![
            edge("api", "db", EdgeKind::DependsOn, None, &evidence),
            edge("db", "cache", EdgeKind::DependsOn, None, &evidence),
            edge("api", "cache", EdgeKind::DependsOn, None, &evidence),
            edge(
                "api",
                "data",
                EdgeKind::Mounts,
                Some("/var/lib/data"),
                &evidence,
            ),
            edge("api", "front", EdgeKind::AttachedTo, None, &evidence),
            edge("cache", "api", EdgeKind::DependsOn, None, &evidence),
            edge("worker", "worker", EdgeKind::DependsOn, None, &evidence),
        ],
        Vec::new(),
    )
    .expect("test deployment graph must satisfy domain validation")
}

fn limits(depth: u32, nodes: usize, edges: usize) -> ContextLimits {
    ContextLimits::new(depth, nodes, edges).expect("test limits must be valid")
}

fn assert_selection_is_safe(
    graph: &DeploymentGraph,
    selection: &graph_application::deployment_context::Selection,
) {
    assert!(
        selection
            .nodes
            .iter()
            .all(|&index| index < graph.nodes().len())
    );
    assert!(
        selection
            .edges
            .iter()
            .all(|&index| index < graph.edges().len())
    );

    for &edge_index in &selection.edges {
        let edge = &graph.edges()[edge_index];
        let source = graph
            .nodes()
            .iter()
            .position(|node| node.id == edge.source)
            .expect("validated edge source must exist");
        let target = graph
            .nodes()
            .iter()
            .position(|node| node.id == edge.target)
            .expect("validated edge target must exist");
        assert!(selection.nodes.contains(&source));
        assert!(selection.nodes.contains(&target));
    }
}

#[test]
fn selects_exact_directional_neighborhoods_in_bfs_edge_order() {
    let graph = graph();
    let outgoing = select(&graph, "api", Direction::Outgoing, limits(16, 100, 100))
        .expect("outgoing selection should succeed");
    let incoming = select(&graph, "cache", Direction::Incoming, limits(16, 100, 100))
        .expect("incoming selection should succeed");
    let both = select(&graph, "api", Direction::Both, limits(16, 100, 100))
        .expect("bidirectional selection should succeed");

    assert_eq!(outgoing.nodes, vec![0, 1, 2, 3, 4]);
    assert_eq!(outgoing.edges, vec![0, 2, 3, 4, 1, 5]);
    assert!(!outgoing.depth_limited);
    assert!(!outgoing.budget_limited);

    assert_eq!(incoming.nodes, vec![2, 1, 0]);
    assert_eq!(incoming.edges, vec![1, 2, 0, 5]);
    assert!(!incoming.depth_limited);
    assert!(!incoming.budget_limited);

    assert_eq!(both.nodes, vec![0, 1, 2, 3, 4]);
    assert_eq!(both.edges, vec![0, 2, 3, 4, 5, 1]);
    assert!(!both.depth_limited);
    assert!(!both.budget_limited);

    for selection in [&outgoing, &incoming, &both] {
        assert_selection_is_safe(&graph, selection);
    }
    assert_eq!(graph.edges()[incoming.edges[0]].source, "db");
    assert_eq!(graph.edges()[incoming.edges[0]].target, "cache");
    assert_eq!(graph.edges()[incoming.edges[1]].source, "api");
    assert_eq!(graph.edges()[incoming.edges[1]].target, "cache");
}

#[test]
fn depth_frontier_omissions_set_only_depth_flag() {
    let graph = graph();
    let zero = select(&graph, "api", Direction::Outgoing, limits(0, 100, 100))
        .expect("depth zero should still select the seed");
    let frontier = select(&graph, "api", Direction::Outgoing, limits(1, 100, 100))
        .expect("frontier-limited selection should succeed");

    assert_eq!(zero.nodes, vec![0]);
    assert!(zero.edges.is_empty());
    assert!(zero.depth_limited);
    assert!(!zero.budget_limited);

    assert_eq!(frontier.nodes, vec![0, 1, 2, 3, 4]);
    assert_eq!(frontier.edges, vec![0, 2, 3, 4]);
    assert!(frontier.depth_limited);
    assert!(!frontier.budget_limited);
    assert_selection_is_safe(&graph, &frontier);
}

#[test]
fn zero_edge_budget_is_seed_only_and_sets_budget_flag() {
    let graph = graph();
    let selection = select(&graph, "api", Direction::Both, limits(16, 100, 0))
        .expect("zero edge budget should still select the seed");

    assert_eq!(selection.nodes, vec![0]);
    assert!(selection.edges.is_empty());
    assert!(!selection.depth_limited);
    assert!(selection.budget_limited);
    assert_selection_is_safe(&graph, &selection);
}

#[test]
fn node_and_edge_caps_require_both_caps_for_new_endpoints() {
    let graph = graph();
    let node_limited = select(&graph, "api", Direction::Outgoing, limits(16, 4, 100))
        .expect("node-limited selection should succeed");
    let edge_limited = select(&graph, "api", Direction::Outgoing, limits(16, 100, 3))
        .expect("edge-limited selection should succeed");

    assert_eq!(node_limited.nodes, vec![0, 1, 2, 3]);
    assert_eq!(node_limited.edges, vec![0, 2, 3, 1, 5]);
    assert!(node_limited.budget_limited);
    assert!(!node_limited.depth_limited);
    assert_selection_is_safe(&graph, &node_limited);

    assert_eq!(edge_limited.nodes, vec![0, 1, 2, 3]);
    assert_eq!(edge_limited.edges, vec![0, 2, 3]);
    assert!(edge_limited.budget_limited);
    assert!(!edge_limited.depth_limited);
    assert_selection_is_safe(&graph, &edge_limited);
}

#[test]
fn exact_and_over_caps_include_the_full_reachable_component_without_flags() {
    let graph = graph();
    let exact = select(&graph, "api", Direction::Outgoing, limits(16, 5, 6))
        .expect("exact caps should permit the full component");
    let over = select(&graph, "api", Direction::Outgoing, limits(16, 6, 7))
        .expect("over-provisioned caps should permit the full component");

    assert_eq!(exact.nodes, vec![0, 1, 2, 3, 4]);
    assert_eq!(exact.edges, vec![0, 2, 3, 4, 1, 5]);
    assert!(!exact.depth_limited);
    assert!(!exact.budget_limited);
    assert_eq!(over, exact);
    assert_selection_is_safe(&graph, &exact);
}

#[test]
fn cycles_and_self_links_are_emitted_once_and_disconnected_facts_are_excluded() {
    let graph = graph();
    let cycle = select(&graph, "api", Direction::Outgoing, limits(16, 100, 100))
        .expect("cyclic selection should terminate");
    let self_link = select(&graph, "worker", Direction::Both, limits(16, 100, 100))
        .expect("self-link selection should terminate");

    assert_eq!(cycle.nodes, vec![0, 1, 2, 3, 4]);
    assert_eq!(cycle.edges, vec![0, 2, 3, 4, 1, 5]);
    assert!(!cycle.nodes.contains(&5));
    assert!(!cycle.nodes.contains(&6));
    assert_eq!(self_link.nodes, vec![5]);
    assert_eq!(self_link.edges, vec![6]);
    assert_selection_is_safe(&graph, &cycle);
    assert_selection_is_safe(&graph, &self_link);
}

#[test]
fn invalid_limits_and_unknown_seed_return_domain_errors() {
    assert!(matches!(
        ContextLimits::new(17, 1, 0),
        Err(DomainError::Invalid(_))
    ));
    assert!(matches!(
        ContextLimits::new(16, 0, 0),
        Err(DomainError::Invalid(_))
    ));
    assert!(matches!(
        ContextLimits::new(16, 1001, 0),
        Err(DomainError::Invalid(_))
    ));
    assert!(matches!(
        ContextLimits::new(16, 1, 5001),
        Err(DomainError::Invalid(_))
    ));

    let graph = graph();
    assert!(matches!(
        select(&graph, "missing", Direction::Outgoing, limits(16, 100, 100),),
        Err(DomainError::Invalid(_))
    ));
}

#[test]
fn repeated_selection_is_deterministic() {
    let graph = graph();
    let first = select(&graph, "api", Direction::Both, limits(2, 4, 4))
        .expect("first selection should succeed");
    let second = select(&graph, "api", Direction::Both, limits(2, 4, 4))
        .expect("second selection should succeed");

    assert_eq!(first, second);
    assert_selection_is_safe(&graph, &first);
}

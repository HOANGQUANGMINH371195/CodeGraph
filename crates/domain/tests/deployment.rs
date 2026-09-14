use graph_domain::deployment::{DeploymentGraph, Edge, EdgeKind, Node, NodeKind, Unknown};
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

fn citation(
    id: &str,
    project: ProjectRef,
    graph_version: &str,
    path: &str,
    hash: &str,
    start_line: u32,
    end_line: u32,
    analysis_run: &str,
) -> SourceEvidence {
    SourceEvidence::new(
        id.into(),
        project,
        graph_version.into(),
        path.into(),
        hash.into(),
        start_line,
        end_line,
        analysis_run.into(),
    )
    .expect("test citation must satisfy the existing constructor")
}

fn source() -> SourceEvidence {
    citation(
        "root",
        project(),
        "graph-v1",
        "compose.yaml",
        HASH,
        1,
        10,
        "run-1",
    )
}

fn service(evidence: &SourceEvidence) -> Node {
    Node {
        id: "service-api".into(),
        kind: NodeKind::Service,
        name: "api".into(),
        evidence: evidence.clone(),
    }
}

fn volume(evidence: &SourceEvidence) -> Node {
    Node {
        id: "volume-data".into(),
        kind: NodeKind::Volume,
        name: "data".into(),
        evidence: evidence.clone(),
    }
}

fn network(evidence: &SourceEvidence) -> Node {
    Node {
        id: "network-front".into(),
        kind: NodeKind::Network,
        name: "front".into(),
        evidence: evidence.clone(),
    }
}

fn database(evidence: &SourceEvidence) -> Node {
    Node {
        id: "service-db".into(),
        kind: NodeKind::Service,
        name: "db".into(),
        evidence: evidence.clone(),
    }
}

fn mounts(evidence: &SourceEvidence) -> Edge {
    Edge {
        source: "service-api".into(),
        target: "volume-data".into(),
        kind: EdgeKind::Mounts,
        mount_target: Some("/var/lib/data".into()),
        evidence: evidence.clone(),
    }
}

fn depends_on(evidence: &SourceEvidence) -> Edge {
    Edge {
        source: "service-api".into(),
        target: "service-db".into(),
        kind: EdgeKind::DependsOn,
        mount_target: None,
        evidence: evidence.clone(),
    }
}

fn attached_to(evidence: &SourceEvidence) -> Edge {
    Edge {
        source: "service-api".into(),
        target: "network-front".into(),
        kind: EdgeKind::AttachedTo,
        mount_target: None,
        evidence: evidence.clone(),
    }
}

fn unknown() -> Unknown {
    Unknown {
        reason: "unresolved extension".into(),
        line: 10,
    }
}

fn valid_graph() -> DeploymentGraph {
    let evidence = source();
    DeploymentGraph::new(
        "compose".into(),
        "2.0".into(),
        evidence.clone(),
        vec![
            service(&evidence),
            volume(&evidence),
            network(&evidence),
            database(&evidence),
        ],
        vec![
            mounts(&evidence),
            depends_on(&evidence),
            attached_to(&evidence),
        ],
        vec![unknown()],
    )
    .expect("baseline deployment graph must be valid")
}

fn graph_with_single_node(node_evidence: SourceEvidence) -> Result<DeploymentGraph, DomainError> {
    let evidence = source();
    DeploymentGraph::new(
        "compose".into(),
        "2.0".into(),
        evidence,
        vec![Node {
            id: "service-api".into(),
            kind: NodeKind::Service,
            name: "api".into(),
            evidence: node_evidence,
        }],
        Vec::new(),
        Vec::new(),
    )
}

fn assert_rejected<T: std::fmt::Debug>(result: Result<T, DomainError>, label: &str) {
    assert!(result.is_err(), "{label} must be rejected");
}

fn assert_rejected_without_echo<T: std::fmt::Debug>(result: Result<T, DomainError>, secret: &str) {
    let error = result.expect_err("invalid graph must be rejected");
    let display = error.to_string();
    let debug = format!("{error:?}");
    assert!(
        !display.contains(secret),
        "display leaked the secret: {display}"
    );
    assert!(!debug.contains(secret), "debug leaked the secret: {debug}");
}

#[test]
fn valid_graph_exposes_immutable_domain_data() {
    let graph = valid_graph();
    let evidence = source();

    assert_eq!(graph.adapter(), "compose");
    assert_eq!(graph.adapter_version(), "2.0");
    assert_eq!(graph.evidence(), &evidence);
    assert_eq!(graph.nodes().len(), 4);
    assert_eq!(
        graph.edges(),
        &[
            mounts(&evidence),
            depends_on(&evidence),
            attached_to(&evidence)
        ]
    );
    assert_eq!(graph.unknowns(), &[unknown()]);
}

#[test]
fn empty_graph_is_a_valid_explicit_replacement() {
    let evidence = source();
    let graph = DeploymentGraph::new(
        "compose".into(),
        "2.0".into(),
        evidence.clone(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("an explicit empty replacement is valid");

    assert!(graph.nodes().is_empty());
    assert!(graph.edges().is_empty());
    assert!(graph.unknowns().is_empty());
    assert_eq!(graph.evidence(), &evidence);
}

#[test]
fn rejects_empty_control_and_oversized_adapter_fields() {
    let evidence = source();
    for (label, adapter, version) in [
        ("empty adapter", String::new(), "2.0".to_owned()),
        ("whitespace adapter", " \t".into(), "2.0".into()),
        ("control adapter", "compose\nsecret".into(), "2.0".into()),
        ("oversized adapter", "a".repeat(129), "2.0".into()),
        ("empty version", "compose".into(), String::new()),
        ("control version", "compose".into(), "2.0\nsecret".into()),
        ("oversized version", "compose".into(), "v".repeat(129)),
    ] {
        assert_rejected(
            DeploymentGraph::new(
                adapter,
                version,
                evidence.clone(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            label,
        );
    }
}

#[test]
fn rejects_invalid_node_and_unknown_text() {
    let evidence = source();
    for (label, node) in [
        (
            "empty node id",
            Node {
                id: String::new(),
                kind: NodeKind::Service,
                name: "api".into(),
                evidence: evidence.clone(),
            },
        ),
        (
            "control node name",
            Node {
                id: "service-api".into(),
                kind: NodeKind::Service,
                name: "api\nsecret".into(),
                evidence: evidence.clone(),
            },
        ),
        (
            "oversized node id",
            Node {
                id: "i".repeat(513),
                kind: NodeKind::Service,
                name: "api".into(),
                evidence: evidence.clone(),
            },
        ),
        (
            "oversized node name",
            Node {
                id: "service-api".into(),
                kind: NodeKind::Service,
                name: "n".repeat(1025),
                evidence: evidence.clone(),
            },
        ),
    ] {
        assert_rejected(
            DeploymentGraph::new(
                "compose".into(),
                "2.0".into(),
                evidence.clone(),
                vec![node],
                Vec::new(),
                Vec::new(),
            ),
            label,
        );
    }

    for reason in [String::new(), "unknown\nsecret".into(), "r".repeat(513)] {
        assert_rejected(
            DeploymentGraph::new(
                "compose".into(),
                "2.0".into(),
                evidence.clone(),
                Vec::new(),
                Vec::new(),
                vec![Unknown { reason, line: 1 }],
            ),
            "invalid unknown reason",
        );
    }
}

#[test]
fn rejects_collection_budget_overflows() {
    let assert_rejected = |result: Result<DeploymentGraph, DomainError>, label: &str| {
        assert!(
            matches!(
                result,
                Err(DomainError::Invalid("deployment graph exceeds item budget"))
            ),
            "{label} must hit the item budget, not duplicate validation"
        );
    };
    let evidence = source();
    let node = service(&evidence);
    let volume = volume(&evidence);
    let edge = mounts(&evidence);

    assert_rejected(
        DeploymentGraph::new(
            "compose".into(),
            "2.0".into(),
            evidence.clone(),
            vec![node; 10_001],
            Vec::new(),
            Vec::new(),
        ),
        "node budget",
    );
    assert_rejected(
        DeploymentGraph::new(
            "compose".into(),
            "2.0".into(),
            evidence.clone(),
            vec![service(&evidence), volume],
            vec![edge; 50_001],
            Vec::new(),
        ),
        "edge budget",
    );
    assert_rejected(
        DeploymentGraph::new(
            "compose".into(),
            "2.0".into(),
            evidence,
            Vec::new(),
            Vec::new(),
            vec![unknown(); 50_001],
        ),
        "unknown budget",
    );
}

#[test]
fn exact_collection_limits_accept_distinct_valid_graph_data() {
    let evidence = source();
    let nodes = (0..10_000)
        .map(|index| Node {
            id: format!("service-{index}"),
            ..service(&evidence)
        })
        .collect();
    assert_eq!(
        DeploymentGraph::new(
            "compose".into(),
            "1".into(),
            evidence.clone(),
            nodes,
            vec![],
            vec![]
        )
        .unwrap()
        .nodes()
        .len(),
        10_000
    );

    let edges = (0..50_000)
        .map(|index| Edge {
            mount_target: Some(format!("/data/{index}")),
            ..mounts(&evidence)
        })
        .collect();
    assert_eq!(
        DeploymentGraph::new(
            "compose".into(),
            "1".into(),
            evidence.clone(),
            vec![service(&evidence), volume(&evidence)],
            edges,
            vec![]
        )
        .unwrap()
        .edges()
        .len(),
        50_000
    );

    assert_eq!(
        DeploymentGraph::new(
            "compose".into(),
            "1".into(),
            evidence,
            vec![],
            vec![],
            vec![unknown(); 50_000]
        )
        .unwrap()
        .unknowns()
        .len(),
        50_000
    );
}

#[test]
fn edge_citations_cannot_cross_scope_or_reuse_node_identity() {
    let evidence = source();
    let foreign = citation(
        "edge",
        project(),
        "graph-v1",
        "compose.yaml",
        &"b".repeat(64),
        2,
        2,
        "run-1",
    );
    let error = DeploymentGraph::new(
        "compose".into(),
        "1".into(),
        evidence.clone(),
        vec![service(&evidence), volume(&evidence)],
        vec![Edge {
            evidence: foreign,
            ..mounts(&evidence)
        }],
        vec![],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DomainError::Invalid("deployment citation differs from source scope")
    ));

    let node_evidence = citation(
        "shared",
        project(),
        "graph-v1",
        "compose.yaml",
        HASH,
        2,
        2,
        "run-1",
    );
    let edge_evidence = citation(
        "shared",
        project(),
        "graph-v1",
        "compose.yaml",
        HASH,
        3,
        3,
        "run-1",
    );
    let error = DeploymentGraph::new(
        "compose".into(),
        "1".into(),
        evidence.clone(),
        vec![service(&node_evidence), volume(&evidence)],
        vec![Edge {
            evidence: edge_evidence,
            ..mounts(&evidence)
        }],
        vec![],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DomainError::Invalid("deployment citation identity has conflicting content")
    ));
}

#[test]
fn rejects_duplicate_node_ids() {
    let evidence = source();
    let duplicate = Node {
        id: "service-api".into(),
        kind: NodeKind::Volume,
        name: "different-kind".into(),
        evidence: evidence.clone(),
    };

    assert_rejected(
        DeploymentGraph::new(
            "compose".into(),
            "2.0".into(),
            evidence.clone(),
            vec![service(&evidence), duplicate],
            Vec::new(),
            Vec::new(),
        ),
        "duplicate node identity",
    );
}

#[test]
fn accepts_only_the_three_declared_edge_shapes() {
    let graph = valid_graph();
    assert_eq!(graph.edges()[0].kind, EdgeKind::Mounts);
    assert_eq!(
        graph.edges()[0].mount_target.as_deref(),
        Some("/var/lib/data")
    );
    assert_eq!(graph.edges()[1].kind, EdgeKind::DependsOn);
    assert_eq!(graph.edges()[1].mount_target, None);
    assert_eq!(graph.edges()[2].kind, EdgeKind::AttachedTo);
    assert_eq!(graph.edges()[2].mount_target, None);
}

#[test]
fn rejects_missing_and_mistyped_edge_endpoints() {
    let evidence = source();
    let nodes = vec![
        service(&evidence),
        volume(&evidence),
        network(&evidence),
        database(&evidence),
    ];
    let cases = [
        Edge {
            source: "missing-service".into(),
            target: "volume-data".into(),
            kind: EdgeKind::Mounts,
            mount_target: Some("/data".into()),
            evidence: evidence.clone(),
        },
        Edge {
            source: "service-api".into(),
            target: "missing-volume".into(),
            kind: EdgeKind::Mounts,
            mount_target: Some("/data".into()),
            evidence: evidence.clone(),
        },
        Edge {
            source: "volume-data".into(),
            target: "volume-data".into(),
            kind: EdgeKind::Mounts,
            mount_target: Some("/data".into()),
            evidence: evidence.clone(),
        },
        Edge {
            source: "service-api".into(),
            target: "volume-data".into(),
            kind: EdgeKind::DependsOn,
            mount_target: None,
            evidence: evidence.clone(),
        },
        Edge {
            source: "service-api".into(),
            target: "service-db".into(),
            kind: EdgeKind::AttachedTo,
            mount_target: None,
            evidence,
        },
    ];

    for edge in cases {
        assert_rejected(
            DeploymentGraph::new(
                "compose".into(),
                "2.0".into(),
                source(),
                nodes.clone(),
                vec![edge],
                Vec::new(),
            ),
            "missing or mistyped edge endpoint",
        );
    }
}

#[test]
fn rejects_invalid_mount_targets_and_mount_metadata_on_other_edges() {
    let evidence = source();
    let nodes = vec![
        service(&evidence),
        volume(&evidence),
        network(&evidence),
        database(&evidence),
    ];
    let cases = [
        Edge {
            source: "service-api".into(),
            target: "volume-data".into(),
            kind: EdgeKind::Mounts,
            mount_target: None,
            evidence: evidence.clone(),
        },
        Edge {
            source: "service-api".into(),
            target: "volume-data".into(),
            kind: EdgeKind::Mounts,
            mount_target: Some("relative/data".into()),
            evidence: evidence.clone(),
        },
        Edge {
            source: "service-api".into(),
            target: "volume-data".into(),
            kind: EdgeKind::Mounts,
            mount_target: Some("/$DATA".into()),
            evidence: evidence.clone(),
        },
        Edge {
            source: "service-api".into(),
            target: "service-db".into(),
            kind: EdgeKind::DependsOn,
            mount_target: Some("/should-be-none".into()),
            evidence: evidence.clone(),
        },
        Edge {
            source: "service-api".into(),
            target: "network-front".into(),
            kind: EdgeKind::AttachedTo,
            mount_target: Some("/should-be-none".into()),
            evidence,
        },
    ];

    for edge in cases {
        assert_rejected(
            DeploymentGraph::new(
                "compose".into(),
                "2.0".into(),
                source(),
                nodes.clone(),
                vec![edge],
                Vec::new(),
            ),
            "invalid mount metadata",
        );
    }
}

#[test]
fn rejects_duplicate_edges_using_the_full_edge_identity() {
    let evidence = source();
    let duplicate = mounts(&evidence);
    assert_rejected(
        DeploymentGraph::new(
            "compose".into(),
            "2.0".into(),
            evidence.clone(),
            vec![service(&evidence), volume(&evidence)],
            vec![duplicate.clone(), duplicate],
            Vec::new(),
        ),
        "duplicate edge identity",
    );

    let second_evidence = citation(
        "edge-2",
        project(),
        "graph-v1",
        "compose.yaml",
        HASH,
        2,
        2,
        "run-1",
    );
    let distinct_citation = Edge {
        evidence: second_evidence,
        ..mounts(&evidence)
    };
    let graph = DeploymentGraph::new(
        "compose".into(),
        "2.0".into(),
        evidence.clone(),
        vec![service(&evidence), volume(&evidence)],
        vec![mounts(&evidence), distinct_citation],
        Vec::new(),
    )
    .expect("edge evidence id is part of duplicate identity");
    assert_eq!(graph.edges().len(), 2);
}

#[test]
fn rejects_citations_outside_every_source_scope_dimension() {
    let mut cases = Vec::new();

    for (label, mut changed) in [
        ("repository", project()),
        ("worktree", project()),
        ("git head", project()),
        ("working tree", project()),
        ("config hash", project()),
        ("ignore policy", project()),
    ] {
        match label {
            "repository" => changed.repository_id = "other-repo".into(),
            "worktree" => changed.worktree_id = "other-worktree".into(),
            "git head" => changed.git_head = "other-head".into(),
            "working tree" => changed.working_tree_fingerprint = "other-tree".into(),
            "config hash" => changed.config_hash = "other-config".into(),
            "ignore policy" => changed.ignore_policy_version = "other-ignore".into(),
            _ => unreachable!("all scope cases are listed above"),
        }
        cases.push((
            label,
            citation(
                "node",
                changed,
                "graph-v1",
                "compose.yaml",
                HASH,
                2,
                2,
                "run-1",
            ),
        ));
    }
    cases.extend([
        (
            "graph version",
            citation(
                "node",
                project(),
                "graph-v2",
                "compose.yaml",
                HASH,
                2,
                2,
                "run-1",
            ),
        ),
        (
            "path",
            citation(
                "node",
                project(),
                "graph-v1",
                "other.yaml",
                HASH,
                2,
                2,
                "run-1",
            ),
        ),
        (
            "content hash",
            citation(
                "node",
                project(),
                "graph-v1",
                "compose.yaml",
                &"b".repeat(64),
                2,
                2,
                "run-1",
            ),
        ),
        (
            "analysis run",
            citation(
                "node",
                project(),
                "graph-v1",
                "compose.yaml",
                HASH,
                2,
                2,
                "run-2",
            ),
        ),
        (
            "range ends after source",
            citation(
                "node",
                project(),
                "graph-v1",
                "compose.yaml",
                HASH,
                2,
                11,
                "run-1",
            ),
        ),
    ]);

    for (label, changed) in cases {
        assert_rejected(graph_with_single_node(changed), label);
    }

    let non_full_source = citation(
        "root",
        project(),
        "graph-v1",
        "compose.yaml",
        HASH,
        2,
        10,
        "run-1",
    );
    assert_rejected(
        DeploymentGraph::new(
            "compose".into(),
            "2.0".into(),
            non_full_source,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
        "root source starts after line one",
    );
}

#[test]
fn rejects_conflicting_content_for_a_citation_id_including_the_root() {
    let conflicting_root_id = citation(
        "root",
        project(),
        "graph-v1",
        "compose.yaml",
        HASH,
        2,
        3,
        "run-1",
    );
    assert_rejected(
        graph_with_single_node(conflicting_root_id),
        "root citation identity conflict",
    );
}

#[test]
fn accepts_nested_citation_ranges_but_rejects_unknown_lines_outside_root() {
    let nested = citation(
        "node",
        project(),
        "graph-v1",
        "compose.yaml",
        HASH,
        2,
        9,
        "run-1",
    );
    let graph = graph_with_single_node(nested).expect("nested citation is within full source");
    assert_eq!(graph.nodes()[0].evidence.start_line(), 2);
    assert_eq!(graph.nodes()[0].evidence.end_line(), 9);

    let evidence = source();
    for line in [0, 11] {
        assert_rejected(
            DeploymentGraph::new(
                "compose".into(),
                "2.0".into(),
                evidence.clone(),
                Vec::new(),
                Vec::new(),
                vec![Unknown {
                    reason: "unresolved".into(),
                    line,
                }],
            ),
            "unknown line outside root citation",
        );
    }
}

#[test]
fn invalid_errors_are_static_and_do_not_echo_untrusted_identifiers() {
    let secret = "super-secret-runtime-token";
    let evidence = source();
    let missing_target = Edge {
        source: "service-api".into(),
        target: secret.into(),
        kind: EdgeKind::Mounts,
        mount_target: Some("/data".into()),
        evidence: evidence.clone(),
    };

    assert_rejected_without_echo(
        DeploymentGraph::new(
            "compose".into(),
            "2.0".into(),
            evidence,
            vec![service(&source())],
            vec![missing_target],
            Vec::new(),
        ),
        secret,
    );
}

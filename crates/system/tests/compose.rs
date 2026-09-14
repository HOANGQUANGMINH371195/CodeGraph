use std::{error::Error, fmt};

use graph_application::{SourceLimits, SourceReader, SourceVerificationError, verify_source};
use graph_domain::{ProjectRef, SourceEvidence};
use graph_system::{EdgeKind, NodeKind, analyze_compose};
use sha2::{Digest, Sha256};

const ORDERS_COMPOSE: &str = include_str!("../../../fixtures/orders/compose.yaml");
const ORDERS_HASH: &str = "a2a29afb353a588cc28759f281819f5cfda82fd5044f8fd5e38247959fa37701";

#[derive(Debug)]
struct MemorySourceError;

impl fmt::Display for MemorySourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("in-memory source exceeded its read budget")
    }
}

impl Error for MemorySourceError {}

struct MemorySource {
    bytes: Vec<u8>,
}

impl SourceReader for MemorySource {
    type Error = MemorySourceError;

    fn read_source(
        &self,
        _evidence: &SourceEvidence,
        limit: usize,
    ) -> Result<Vec<u8>, Self::Error> {
        if self.bytes.len() > limit {
            return Err(MemorySourceError);
        }
        Ok(self.bytes.clone())
    }
}

fn project(repository_id: &str, worktree_id: &str) -> ProjectRef {
    ProjectRef {
        repository_id: repository_id.into(),
        worktree_id: worktree_id.into(),
        git_head: "head-1".into(),
        working_tree_fingerprint: "tree-1".into(),
        config_hash: "config-1".into(),
        ignore_policy_version: "ignore-1".into(),
    }
}

fn evidence(
    bytes: &[u8],
    repository_id: &str,
    worktree_id: &str,
    path: &str,
    start_line: u32,
    end_line: u32,
) -> SourceEvidence {
    SourceEvidence::new(
        "evidence-1".into(),
        project(repository_id, worktree_id),
        "graph-1".into(),
        path.into(),
        format!("{:x}", Sha256::digest(bytes)),
        start_line,
        end_line,
        "run-1".into(),
    )
    .unwrap()
}

fn verified(
    text: &str,
    repository_id: &str,
    worktree_id: &str,
    path: &str,
    start_line: u32,
    end_line: u32,
) -> graph_application::SourceSlice {
    let bytes = text.as_bytes();
    let source = MemorySource {
        bytes: bytes.to_vec(),
    };
    verify_source(
        &source,
        &evidence(
            bytes,
            repository_id,
            worktree_id,
            path,
            start_line,
            end_line,
        ),
        SourceLimits::new(1024 * 1024, 1024 * 1024).unwrap(),
    )
    .unwrap()
}

fn verified_with_limits(text: &str, max_source_bytes: usize) -> graph_application::SourceSlice {
    let bytes = text.as_bytes();
    let source = MemorySource {
        bytes: bytes.to_vec(),
    };
    let line_count = text.lines().count() as u32;
    verify_source(
        &source,
        &evidence(
            bytes,
            "repo-a",
            "worktree-a",
            "deploy/compose.yaml",
            1,
            line_count,
        ),
        SourceLimits::new(max_source_bytes, max_source_bytes).unwrap(),
    )
    .unwrap()
}

fn full_verified(text: &str) -> graph_application::SourceSlice {
    let line_count = text.lines().count() as u32;
    verified(
        text,
        "repo-a",
        "worktree-a",
        "deploy/compose.yaml",
        1,
        line_count,
    )
}

fn projection_debug(text: &str) -> String {
    format!("{:?}", analyze_compose(&full_verified(text)).unwrap())
}

#[test]
fn orders_fixture_projects_declared_nodes_edges_and_real_evidence() {
    let projection = analyze_compose(&full_verified(ORDERS_COMPOSE)).unwrap();

    assert_eq!(projection.nodes.len(), 4);
    assert_eq!(
        projection
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, NodeKind::Service))
            .count(),
        2
    );
    assert_eq!(
        projection
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, NodeKind::Volume))
            .count(),
        2
    );
    assert_eq!(
        projection
            .edges
            .iter()
            .filter(|edge| matches!(edge.kind, EdgeKind::Mounts))
            .count(),
        2
    );
    assert!(!projection.unknowns.is_empty());

    let orders = projection
        .nodes
        .iter()
        .find(|node| node.name == "orders")
        .unwrap();
    assert_eq!(orders.evidence.start_line(), 2);
    assert_eq!(orders.evidence.end_line(), 2);
    assert_eq!(orders.evidence.content_sha256(), ORDERS_HASH);
    assert!(projection.unknowns.iter().any(|unknown| {
        ["build", "environment", "ports"]
            .iter()
            .any(|field| unknown.reason.contains(field))
    }));
}

#[test]
fn declaration_ids_are_namespaced_but_not_line_identity() {
    let first = full_verified(ORDERS_COMPOSE);
    let moved =
        ORDERS_COMPOSE.replace("services:\n", "# harmless declaration comment\nservices:\n");
    let second = full_verified(&moved);
    let first_projection = analyze_compose(&first).unwrap();
    let second_projection = analyze_compose(&second).unwrap();

    let first_ids: Vec<_> = first_projection
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect();
    let second_ids: Vec<_> = second_projection
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect();
    assert_eq!(first_ids, second_ids);
    assert_ne!(
        first_projection.nodes[0].evidence.content_sha256(),
        second_projection.nodes[0].evidence.content_sha256()
    );

    let other_scope = verified(
        ORDERS_COMPOSE,
        "repo-b",
        "worktree-b",
        "other/compose.yaml",
        1,
        20,
    );
    let other_projection = analyze_compose(&other_scope).unwrap();
    assert_ne!(
        first_ids,
        other_projection
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn mounts_only_target_declared_volumes() {
    let projection = analyze_compose(&full_verified(ORDERS_COMPOSE)).unwrap();
    let volume_ids: Vec<_> = projection
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Volume))
        .map(|node| node.id.clone())
        .collect();
    for edge in projection
        .edges
        .iter()
        .filter(|edge| matches!(edge.kind, EdgeKind::Mounts))
    {
        assert!(volume_ids.contains(&edge.target));
        assert!(edge.mount_target.is_some());
    }
}

#[test]
fn depends_on_is_declaration_not_traffic_and_network_forms_are_mapped() {
    let text = "services:\n  api:\n    depends_on: [db]\n    networks: [front]\n  db:\n    networks:\n      back: {}\nnetworks:\n  front: {}\n  back: {}\n";
    let projection = analyze_compose(&full_verified(text)).unwrap();

    assert!(
        projection
            .edges
            .iter()
            .any(|edge| matches!(edge.kind, EdgeKind::DependsOn))
    );
    assert_eq!(
        projection
            .edges
            .iter()
            .filter(|edge| matches!(edge.kind, EdgeKind::AttachedTo))
            .count(),
        2
    );
    assert!(
        projection
            .nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::Network) && node.name == "front")
    );
    assert!(
        projection
            .nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::Network) && node.name == "back")
    );
    assert!(
        !projection
            .edges
            .iter()
            .any(|edge| format!("{:?}", edge.kind).contains("Traffic"))
    );
}

#[test]
fn interpolation_is_not_resolved_and_secrets_are_not_projected() {
    let secret = "super-secret-do-not-leak";
    let text = format!(
        "services:\n  api:\n    image: ${{IMAGE_NAME}}\n    environment:\n      TOKEN: {secret}\n"
    );
    let debug = projection_debug(&text);
    assert!(!debug.contains(secret));
    assert!(debug.contains("interpol"));
}

#[test]
fn duplicate_keys_malformed_documents_and_aliases_are_errors_without_secrets() {
    for text in [
        "services:\n  api: {}\n  api: {}\n",
        "services:\n  api: [\n",
        "services:\n  api: {}\n---\nservices:\n  db: {}\n",
        "defaults: &defaults\n  image: alpine\nservices:\n  api:\n    <<: *defaults\n",
    ] {
        let result = analyze_compose(&full_verified(text));
        assert!(result.is_err(), "expected an error for {text:?}");
    }

    let secret = "parse-error-secret";
    let text = format!("services:\n  api: [${secret}\n");
    let error = analyze_compose(&full_verified(&text)).unwrap_err();
    assert!(!format!("{error:?}").contains(secret));
    assert!(!error.to_string().contains(secret));
}

#[test]
fn depth_over_64_and_one_mib_source_budget_fail_closed() {
    let mut deep = String::from("services:\n  api:\n    x:");
    for _ in 0..65 {
        deep.push_str("\n      x:");
    }
    deep.push_str(" value\n");
    assert!(analyze_compose(&full_verified(&deep)).is_err());

    let nested_flow = format!(
        "services:\n  api:\n    image: {}{}\n",
        "[".repeat(66),
        "]".repeat(66)
    );
    assert!(analyze_compose(&full_verified(&nested_flow)).is_err());

    let oversized = vec![b'x'; 1024 * 1024 + 1];
    let source = MemorySource {
        bytes: oversized.clone(),
    };
    let citation = evidence(&oversized, "repo-a", "worktree-a", "huge.yaml", 1, 1);
    assert!(matches!(
        verify_source(
            &source,
            &citation,
            SourceLimits::new(1024 * 1024, 1024 * 1024).unwrap()
        ),
        Err(SourceVerificationError::Reader(_)) | Err(SourceVerificationError::SourceTooLarge)
    ));

    let oversized_valid = format!(
        "services:\n  api:\n    image: {}\n",
        "a".repeat(1024 * 1024)
    );
    assert!(analyze_compose(&verified_with_limits(&oversized_valid, 2 * 1024 * 1024)).is_err());
}

#[test]
fn parser_rejects_a_partial_verified_slice_even_when_its_source_hash_is_valid() {
    let partial = verified(
        ORDERS_COMPOSE,
        "repo-a",
        "worktree-a",
        "compose.yaml",
        1,
        10,
    );
    assert!(analyze_compose(&partial).is_err());
}

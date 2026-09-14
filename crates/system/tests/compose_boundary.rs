use graph_application::{SourceLimits, SourceReader, SourceSlice, verify_source};
use graph_domain::{ProjectRef, SourceEvidence};
use graph_system::{EdgeKind, NodeKind, analyze_compose, analyze_compose_graph};
use sha2::{Digest, Sha256};

struct Memory(Vec<u8>);
impl SourceReader for Memory {
    type Error = std::io::Error;
    fn read_source(&self, _: &SourceEvidence, limit: usize) -> Result<Vec<u8>, Self::Error> {
        if self.0.len() > limit {
            return Err(std::io::Error::other("limit"));
        }
        Ok(self.0.clone())
    }
}

fn input(text: &str, repo: &str, worktree: &str) -> SourceSlice {
    let citation = SourceEvidence::new(
        "caller-supplied-id".into(),
        ProjectRef {
            repository_id: repo.into(),
            worktree_id: worktree.into(),
            git_head: "head".into(),
            working_tree_fingerprint: "tree".into(),
            config_hash: "config".into(),
            ignore_policy_version: "ignore".into(),
        },
        "graph".into(),
        "compose.yaml".into(),
        format!("{:x}", Sha256::digest(text.as_bytes())),
        1,
        u32::try_from(text.lines().count()).unwrap(),
        "run".into(),
    )
    .unwrap();
    verify_source(
        &Memory(text.as_bytes().to_vec()),
        &citation,
        SourceLimits::new(2 * 1024 * 1024, 2 * 1024 * 1024).unwrap(),
    )
    .unwrap()
}

#[test]
fn identities_cannot_collide_through_delimiters_or_changed_source() {
    let text = "services: {api: {image: alpine}}\n";
    let first = analyze_compose(&input(text, "repo:a", "b")).unwrap();
    let other = analyze_compose(&input(text, "repo", "a:b")).unwrap();
    assert_ne!(first.nodes[0].id, other.nodes[0].id);
    assert_ne!(first.nodes[0].evidence.id(), other.nodes[0].evidence.id());
    let changed =
        analyze_compose(&input(&text.replace("alpine", "ubuntu"), "repo:a", "b")).unwrap();
    assert_eq!(first.nodes[0].id, changed.nodes[0].id);
    assert_ne!(first.nodes[0].evidence.id(), changed.nodes[0].evidence.id());
}

#[test]
fn references_cite_their_actual_items_in_block_yaml() {
    let text = "services:\n  api:\n    depends_on:\n      - db\n    networks:\n      front: {}\n  db: {}\nnetworks:\n  front: {}\n";
    let result = analyze_compose(&input(text, "r", "w")).unwrap();
    let dependency = result
        .edges
        .iter()
        .find(|edge| edge.kind == EdgeKind::DependsOn)
        .unwrap();
    let network = result
        .edges
        .iter()
        .find(|edge| edge.kind == EdgeKind::AttachedTo)
        .unwrap();
    assert_eq!(dependency.evidence.start_line(), 4);
    assert_eq!(network.evidence.start_line(), 6);
}

#[test]
fn cr_and_crlf_declaration_citations_can_be_read_back() {
    for (text, expected_line) in [
        ("services:\r  api: {}\r", 1),
        ("services:\r\n  api: {}\r\n", 2),
    ] {
        let result = analyze_compose(&input(text, "r", "w")).unwrap();
        let citation = &result.nodes[0].evidence;
        assert_eq!(citation.start_line(), expected_line);
        let slice = verify_source(
            &Memory(text.as_bytes().to_vec()),
            citation,
            SourceLimits::new(1024, 1024).unwrap(),
        )
        .unwrap();
        assert!(slice.text().contains("api"));
    }
}

#[test]
fn validated_batch_retains_producer_source_and_empty_replacement() {
    let text = "services: {api: {volumes: ['data:/data']}}\nvolumes: {data: {}}\n";
    let source = input(text, "r", "w");
    let graph = analyze_compose_graph(&source).unwrap();
    assert_eq!(graph.adapter(), "compose");
    assert_eq!(graph.adapter_version(), "1");
    assert_eq!(graph.evidence(), source.evidence());
    assert_eq!(graph.nodes().len(), 2);
    assert_eq!(graph.edges().len(), 1);
    assert!(
        graph
            .edges()
            .iter()
            .all(|edge| graph.nodes().iter().any(|node| node.id == edge.target))
    );

    let empty = analyze_compose_graph(&input("services: {}\n", "r", "w")).unwrap();
    assert!(empty.nodes().is_empty());
    assert!(empty.edges().is_empty());
    assert!(empty.unknowns().is_empty());
}

#[test]
fn duplicate_reference_candidates_cannot_bypass_graph_validation() {
    let source = input(
        "services: {api: {networks: [front, front]}}\nnetworks: {front: {}}\n",
        "r",
        "w",
    );
    assert_eq!(analyze_compose(&source).unwrap().edges.len(), 2);
    let error = analyze_compose_graph(&source).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("duplicate deployment edge identity")
    );
}

#[test]
fn valid_short_and_long_named_mounts_keep_declared_edges() {
    for mount in [
        "      - data:/data:ro\n",
        "      - data:/data:rw\n",
        "      - type: volume\n        source: data\n        target: /data\n        read_only: true\n",
        "      - type: volume\n        source: data\n        target: /data\n        read_only: false\n",
    ] {
        let text = format!("services:\n  api:\n    volumes:\n{mount}volumes:\n  data: {{}}\n");
        let result = analyze_compose(&input(&text, "r", "w")).unwrap();
        let mounts: Vec<_> = result
            .edges
            .iter()
            .filter(|edge| edge.kind == EdgeKind::Mounts)
            .collect();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].mount_target.as_deref(), Some("/data"));
        assert!(
            result
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::Volume && node.id == mounts[0].target)
        );
    }
}

#[test]
fn unsupported_and_dynamic_mounts_are_unknown_not_fabricated_or_fatal() {
    for mount in [
        "data:${TARGET}",
        "data:relative",
        "missing:/data",
        "./local:/data",
        "/data",
        "data:/data:secret-mode",
    ] {
        let text =
            format!("services:\n  api:\n    volumes:\n      - '{mount}'\nvolumes:\n  data: {{}}\n");
        let result = analyze_compose(&input(&text, "r", "w")).unwrap();
        assert!(result.edges.is_empty(), "unsupported mount emitted an edge");
        assert!(result.unknowns.iter().any(|unknown| unknown.line == 4));
        assert!(!format!("{result:?}").contains("${TARGET}"));
        assert!(!format!("{result:?}").contains("secret-mode"));
    }
}

#[test]
fn unresolved_network_and_service_references_preserve_other_declarations() {
    let text = "services:\n  api:\n    depends_on: [missing]\n    networks: [missing]\n";
    let result = analyze_compose(&input(text, "r", "w")).unwrap();
    assert_eq!(result.nodes.len(), 1);
    assert!(result.edges.is_empty());
    assert!(result.unknowns.iter().any(|unknown| unknown.line == 3));
    assert!(result.unknowns.iter().any(|unknown| unknown.line == 4));
}

#[test]
fn unmodeled_fields_and_resource_options_are_visible_without_values() {
    let text = "services:\n  api:\n    image: image-private-value\n    command: command-private-value\nvolumes:\n  data:\n    driver_opts: {token: token-private-value}\nnetworks:\n  front:\n    external: true\n";
    let result = analyze_compose(&input(text, "r", "w")).unwrap();
    for line in [3, 4, 7, 10] {
        assert!(
            result.unknowns.iter().any(|unknown| unknown.line == line),
            "missing field diagnostic at {line}"
        );
    }
    let debug = format!("{result:?}");
    for value in [
        "image-private-value",
        "command-private-value",
        "token-private-value",
    ] {
        assert!(!debug.contains(value));
    }
}

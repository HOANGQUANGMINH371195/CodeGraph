//! Application-side derivation of stable graph IDs.
//!
//! The domain owns the identity inputs and rendered-ID invariants. Hashing is
//! kept here because the architecture intentionally keeps `graph-domain`
//! free of crypto and other infrastructure dependencies.

use graph_domain::{
    EDGE_IDENTITY_VERSION, EdgeId, EdgeIdentityInput, NODE_IDENTITY_VERSION, NodeId,
    NodeIdentityInput,
};
use sha2::{Digest, Sha256};

/// SHA-256 of arbitrary source/semantic bytes, rendered as lowercase hex.
#[must_use]
pub fn sha256_fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Derive the stable logical ID of a code symbol.
pub fn derive_node_id(input: &NodeIdentityInput) -> Result<NodeId, graph_domain::DomainError> {
    let digest = canonical_digest(
        b"project-graph/node",
        NODE_IDENTITY_VERSION,
        &input.canonical_fields(),
        &input.canonical_anchor(),
    );
    NodeId::from_digest(digest)
}

/// Derive the stable logical ID of a graph relation.
pub fn derive_edge_id(input: &EdgeIdentityInput) -> Result<EdgeId, graph_domain::DomainError> {
    let fields = [
        input.source().as_str(),
        input.target().as_str(),
        input.kind(),
        input.discriminator(),
    ];
    let digest = canonical_digest(b"project-graph/edge", EDGE_IDENTITY_VERSION, &fields, &[]);
    EdgeId::from_digest(digest)
}

fn canonical_digest(domain: &[u8], version: u32, fields: &[&str], numbers: &[u64]) -> String {
    let mut hasher = Sha256::new();
    append_bytes(&mut hasher, domain);
    hasher.update(version.to_be_bytes());
    hasher.update(
        u64::try_from(fields.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for field in fields {
        append_bytes(&mut hasher, field.as_bytes());
    }
    hasher.update(
        u64::try_from(numbers.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for number in numbers {
        hasher.update(number.to_be_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn append_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
    hasher.update(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_domain::{NodeIdentityInput, ProjectRef};

    fn project(head: &str, working: &str, config: &str) -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "worktree".into(),
            git_head: head.into(),
            working_tree_fingerprint: working.into(),
            config_hash: config.into(),
            ignore_policy_version: "ignore-v1".into(),
        }
    }

    fn input(project: &ProjectRef) -> NodeIdentityInput {
        input_with(
            project,
            "function",
            "orders::create",
            "a".repeat(64),
            "src/orders.ts",
            100,
            240,
            "b".repeat(64),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn input_with(
        project: &ProjectRef,
        kind: &str,
        qualified_name: &str,
        signature_fingerprint: String,
        declaration_path: &str,
        declaration_start_byte: u64,
        declaration_end_byte: u64,
        content_fingerprint: String,
    ) -> NodeIdentityInput {
        NodeIdentityInput::new(
            project,
            "typescript".into(),
            kind.into(),
            qualified_name.into(),
            signature_fingerprint,
            declaration_path.into(),
            declaration_start_byte,
            declaration_end_byte,
            content_fingerprint,
        )
        .expect("valid identity fixture")
    }

    #[test]
    fn node_id_is_byte_stable_and_snapshot_independent() {
        let first = derive_node_id(&input(&project("head-a", "dirty-a", "config-a"))).unwrap();
        let second = derive_node_id(&input(&project("head-b", "dirty-b", "config-b"))).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.as_str().len(), "node:v1:".len() + 64);
        assert_eq!(NodeId::parse(first.as_str()).unwrap(), first);
    }

    #[test]
    fn node_id_changes_for_symbol_identity_fields_but_not_body() {
        let base = input(&project("head", "dirty", "config"));
        let changed_path = input_with(
            &project("head", "dirty", "config"),
            "function",
            "orders::create",
            "a".repeat(64),
            "src/other.ts",
            100,
            240,
            "b".repeat(64),
        );
        let changed_signature = input_with(
            &project("head", "dirty", "config"),
            "function",
            "orders::create",
            sha256_fingerprint(b"(id: string)"),
            "src/orders.ts",
            100,
            240,
            "b".repeat(64),
        );
        let changed_anchor = input_with(
            &project("head", "dirty", "config"),
            "function",
            "orders::create",
            "a".repeat(64),
            "src/orders.ts",
            101,
            240,
            "b".repeat(64),
        );
        let changed_body = input_with(
            &project("head", "dirty", "config"),
            "function",
            "orders::create",
            "a".repeat(64),
            "src/orders.ts",
            100,
            240,
            sha256_fingerprint(b"body-v2"),
        );
        assert_ne!(
            derive_node_id(&base).unwrap(),
            derive_node_id(&changed_path).unwrap()
        );
        assert_ne!(
            derive_node_id(&base).unwrap(),
            derive_node_id(&changed_signature).unwrap()
        );
        assert_ne!(
            derive_node_id(&base).unwrap(),
            derive_node_id(&changed_anchor).unwrap()
        );
        assert_eq!(
            derive_node_id(&base).unwrap(),
            derive_node_id(&changed_body).unwrap()
        );
    }

    #[test]
    fn length_prefixing_prevents_delimiter_field_collisions() {
        let first = input_with(
            &project("head", "dirty", "config"),
            "ab",
            "c",
            "a".repeat(64),
            "src/orders.ts",
            100,
            240,
            "b".repeat(64),
        );
        let second = input_with(
            &project("head", "dirty", "config"),
            "a",
            "bc",
            "a".repeat(64),
            "src/orders.ts",
            100,
            240,
            "b".repeat(64),
        );
        assert_ne!(
            derive_node_id(&first).unwrap(),
            derive_node_id(&second).unwrap()
        );
    }

    #[test]
    fn edge_id_is_relation_scoped_and_discriminator_aware() {
        let source = NodeId::from_digest("a".repeat(64)).unwrap();
        let target = NodeId::from_digest("b".repeat(64)).unwrap();
        let first = EdgeIdentityInput::new(
            source.clone(),
            target.clone(),
            "calls".into(),
            "parameter:0".into(),
        )
        .unwrap();
        let second =
            EdgeIdentityInput::new(source, target, "calls".into(), "parameter:1".into()).unwrap();
        assert_ne!(
            derive_edge_id(&first).unwrap(),
            derive_edge_id(&second).unwrap()
        );
        assert_eq!(
            derive_edge_id(&first).unwrap(),
            derive_edge_id(&first).unwrap()
        );
    }
}

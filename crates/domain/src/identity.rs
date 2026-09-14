//! Stable graph identity value objects.
//!
//! IDs are scoped to a repository/worktree and derived from canonical,
//! length-prefixed fields.  They identify a logical relation, not an evidence
//! location or a producer row.  Source coordinates and content fingerprints
//! remain inputs to extraction/evidence and change detection respectively.

use std::fmt;

use crate::{DomainError, ProjectRef, validate_text};

/// Version of the canonical node identity material and rendered ID prefix.
pub const NODE_IDENTITY_VERSION: u32 = 1;
/// Version of the canonical edge identity material and rendered ID prefix.
pub const EDGE_IDENTITY_VERSION: u32 = 1;

const MAX_IDENTITY_TEXT_BYTES: usize = 16 * 1024;
const NODE_ID_PREFIX: &str = "node:v1:";
const EDGE_ID_PREFIX: &str = "edge:v1:";

/// The stable scope portion of a graph identity.
///
/// Revision, dirty fingerprint, configuration and ignore policy are excluded:
/// they describe a snapshot and must invalidate/rebuild evidence without
/// changing the logical namespace of the worktree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdentityNamespace {
    repository_id: String,
    worktree_id: String,
}

impl IdentityNamespace {
    /// Construct a namespace from the stable project scope fields.
    pub fn new(repository_id: String, worktree_id: String) -> Result<Self, DomainError> {
        validate_identity_text("repository identity", &repository_id)?;
        validate_identity_text("worktree identity", &worktree_id)?;
        Ok(Self {
            repository_id,
            worktree_id,
        })
    }

    /// Return the repository identity used by the canonical material.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Return the worktree identity used by the canonical material.
    #[must_use]
    pub fn worktree_id(&self) -> &str {
        &self.worktree_id
    }
}

/// Inputs needed to derive a logical code-symbol identity.
///
/// `declaration_start_byte`/`declaration_end_byte` are an anchor for distinct
/// declarations with otherwise identical names.  They are not line-only
/// identity: line/column evidence is intentionally absent from this type.
/// `content_fingerprint` is validated and carried for change detection, but
/// is not in the logical ID material, so a body edit does not silently turn
/// one symbol into another symbol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeIdentityInput {
    namespace: IdentityNamespace,
    language: String,
    kind: String,
    qualified_name: String,
    signature_fingerprint: String,
    declaration_path: String,
    declaration_start_byte: u64,
    declaration_end_byte: u64,
    content_fingerprint: String,
}

impl NodeIdentityInput {
    /// Construct and validate symbol identity inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project: &ProjectRef,
        language: String,
        kind: String,
        qualified_name: String,
        signature_fingerprint: String,
        declaration_path: String,
        declaration_start_byte: u64,
        declaration_end_byte: u64,
        content_fingerprint: String,
    ) -> Result<Self, DomainError> {
        project.validate()?;
        let namespace =
            IdentityNamespace::new(project.repository_id.clone(), project.worktree_id.clone())?;
        for (name, value) in [
            ("node language", &language),
            ("node kind", &kind),
            ("node qualified name", &qualified_name),
        ] {
            validate_identity_text(name, value)?;
        }
        validate_repository_path(&declaration_path)?;
        validate_sha256("node signature fingerprint", &signature_fingerprint)?;
        validate_sha256("node content fingerprint", &content_fingerprint)?;
        if declaration_end_byte <= declaration_start_byte {
            return Err(DomainError::Invalid("node declaration byte span"));
        }
        Ok(Self {
            namespace,
            language,
            kind,
            qualified_name,
            signature_fingerprint,
            declaration_path,
            declaration_start_byte,
            declaration_end_byte,
            content_fingerprint,
        })
    }

    /// Stable repository/worktree namespace.
    #[must_use]
    pub fn namespace(&self) -> &IdentityNamespace {
        &self.namespace
    }

    /// Language tag supplied by the extractor.
    #[must_use]
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Symbol kind supplied by the extractor.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Canonical qualified symbol name.
    #[must_use]
    pub fn qualified_name(&self) -> &str {
        &self.qualified_name
    }

    /// Fingerprint of the normalized declaration signature.
    #[must_use]
    pub fn signature_fingerprint(&self) -> &str {
        &self.signature_fingerprint
    }

    /// Repository-relative declaration path.
    #[must_use]
    pub fn declaration_path(&self) -> &str {
        &self.declaration_path
    }

    /// Start byte of the declaration anchor.
    #[must_use]
    pub fn declaration_start_byte(&self) -> u64 {
        self.declaration_start_byte
    }

    /// Exclusive end byte of the declaration anchor.
    #[must_use]
    pub fn declaration_end_byte(&self) -> u64 {
        self.declaration_end_byte
    }

    /// Fingerprint of semantic/body content used for change detection.
    #[must_use]
    pub fn content_fingerprint(&self) -> &str {
        &self.content_fingerprint
    }

    /// Return the canonical text fields in their identity order.
    #[must_use]
    pub fn canonical_fields(&self) -> [&str; 7] {
        [
            self.namespace.repository_id(),
            self.namespace.worktree_id(),
            self.language(),
            self.kind(),
            self.qualified_name(),
            self.signature_fingerprint(),
            self.declaration_path(),
        ]
    }

    /// Return the numeric declaration anchor in its identity order.
    #[must_use]
    pub fn canonical_anchor(&self) -> [u64; 2] {
        [self.declaration_start_byte, self.declaration_end_byte]
    }
}

/// Stable opaque identity of a logical code symbol.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(String);

impl NodeId {
    /// Construct an ID from a validated lowercase SHA-256 digest.
    pub fn from_digest(digest: impl Into<String>) -> Result<Self, DomainError> {
        let digest = digest.into();
        validate_sha256("node id digest", &digest)?;
        Ok(Self(format!("node:v{NODE_IDENTITY_VERSION}:{digest}")))
    }

    /// Parse an ID received from a protocol or persistence boundary.
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        validate_rendered_id("node id", &value, NODE_ID_PREFIX)?;
        Ok(Self(value))
    }

    /// Return the canonical rendered ID.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Inputs needed to derive one relation identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeIdentityInput {
    source: NodeId,
    target: NodeId,
    kind: String,
    discriminator: String,
}

impl EdgeIdentityInput {
    /// Construct a relation key. Empty discriminator means the relation has no
    /// extra semantic lane; call-site evidence is not part of this key.
    pub fn new(
        source: NodeId,
        target: NodeId,
        kind: String,
        discriminator: String,
    ) -> Result<Self, DomainError> {
        validate_identity_text("edge kind", &kind)?;
        validate_optional_identity_text("edge discriminator", &discriminator)?;
        Ok(Self {
            source,
            target,
            kind,
            discriminator,
        })
    }

    /// Source symbol ID.
    #[must_use]
    pub fn source(&self) -> &NodeId {
        &self.source
    }

    /// Target symbol ID.
    #[must_use]
    pub fn target(&self) -> &NodeId {
        &self.target
    }

    /// Relation kind, such as `calls` or `imports`.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Optional semantic lane, such as a parameter index or route verb.
    #[must_use]
    pub fn discriminator(&self) -> &str {
        &self.discriminator
    }
}

/// Stable opaque identity of one graph relation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(String);

impl EdgeId {
    /// Construct an ID from a validated lowercase SHA-256 digest.
    pub fn from_digest(digest: impl Into<String>) -> Result<Self, DomainError> {
        let digest = digest.into();
        validate_sha256("edge id digest", &digest)?;
        Ok(Self(format!("edge:v{EDGE_IDENTITY_VERSION}:{digest}")))
    }

    /// Parse an ID received from a protocol or persistence boundary.
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        validate_rendered_id("edge id", &value, EDGE_ID_PREFIX)?;
        Ok(Self(value))
    }

    /// Return the canonical rendered ID.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

fn validate_identity_text(name: &'static str, value: &str) -> Result<(), DomainError> {
    validate_text(name, value)?;
    if value.len() > MAX_IDENTITY_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(DomainError::Invalid("identity text"));
    }
    Ok(())
}

fn validate_optional_identity_text(name: &'static str, value: &str) -> Result<(), DomainError> {
    if value.is_empty() {
        return Ok(());
    }
    validate_identity_text(name, value)
}

fn validate_repository_path(path: &str) -> Result<(), DomainError> {
    validate_identity_text("node declaration path", path)?;
    if path.contains(['\\', ':'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(DomainError::Invalid(
            "node declaration path must be repository relative",
        ));
    }
    Ok(())
}

fn validate_sha256(name: &'static str, value: &str) -> Result<(), DomainError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DomainError::Invalid(name));
    }
    Ok(())
}

fn validate_rendered_id(name: &'static str, value: &str, prefix: &str) -> Result<(), DomainError> {
    if !value.starts_with(prefix) {
        return Err(DomainError::Invalid(name));
    }
    let digest = &value[prefix.len()..];
    validate_sha256(name, digest)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        NodeIdentityInput::new(
            project,
            "typescript".into(),
            "function".into(),
            "orders::create".into(),
            "a".repeat(64),
            "src/orders.ts".into(),
            100,
            240,
            "b".repeat(64),
        )
        .expect("valid identity fixture")
    }

    #[test]
    fn identity_material_is_snapshot_independent_and_not_line_only() {
        let first = input(&project("head-a", "dirty-a", "config-a"));
        let second = input(&project("head-b", "dirty-b", "config-b"));
        assert_eq!(first.canonical_fields(), second.canonical_fields());
        assert_eq!(first.canonical_anchor(), second.canonical_anchor());
        assert_eq!(first.canonical_fields()[4], "orders::create");
        assert_eq!(first.canonical_anchor(), [100, 240]);
    }

    #[test]
    fn identity_material_exposes_path_signature_and_body_separately() {
        let base = input(&project("head", "dirty", "config"));
        assert_eq!(base.declaration_path(), "src/orders.ts");
        assert_eq!(base.signature_fingerprint(), "a".repeat(64));
        assert_eq!(base.content_fingerprint(), "b".repeat(64));
        assert_ne!(base.declaration_start_byte(), base.declaration_end_byte());
    }

    #[test]
    fn invalid_node_inputs_and_ids_fail_closed() {
        let project = project("head", "dirty", "config");
        assert!(
            NodeIdentityInput::new(
                &project,
                "typescript".into(),
                "function".into(),
                "f".into(),
                "A".repeat(64),
                "src/../f.ts".into(),
                1,
                2,
                "a".repeat(64),
            )
            .is_err()
        );
        assert!(NodeId::parse("node:v1:ABC").is_err());
        assert!(NodeId::parse("node:v2:".to_owned() + &"a".repeat(64)).is_err());
        assert!(NodeId::parse("node:v1:".to_owned() + &"a".repeat(63)).is_err());
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
        .expect("valid edge fixture");
        let second = EdgeIdentityInput::new(source, target, "calls".into(), "parameter:1".into())
            .expect("valid edge fixture");
        assert_ne!(first.discriminator(), second.discriminator());
    }

    #[test]
    fn edge_identity_rejects_empty_kind_and_control_text() {
        let node = NodeId::from_digest("a".repeat(64)).unwrap();
        assert!(EdgeIdentityInput::new(node.clone(), node.clone(), "".into(), "".into()).is_err());
        assert!(
            EdgeIdentityInput::new(node.clone(), node, "calls".into(), "bad\nvalue".into(),)
                .is_err()
        );
        assert!(EdgeId::parse("edge:v1:".to_owned() + &"a".repeat(64)).is_ok());
    }
}

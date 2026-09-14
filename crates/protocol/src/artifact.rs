use crate::{ProjectRef, ProtocolError, SCHEMA_VERSION, require_current_schema};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRetention {
    Ephemeral,
    Evidence,
    UserPinned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactProtection {
    Unreviewed,
    Redacted,
    Encrypted,
    RedactedAndEncrypted,
}

/// Untrusted metadata, including the producer's protection declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub schema_version: u32,
    pub id: String,
    pub project: ProjectRef,
    pub graph_version: String,
    pub analysis_run: String,
    pub content_sha256: String,
    pub byte_length: u64,
    pub kind: String,
    pub retention: ArtifactRetention,
    pub declared_protection: ArtifactProtection,
}

impl Artifact {
    pub fn try_into_domain(self) -> Result<graph_domain::Artifact, ProtocolError> {
        require_current_schema(self.schema_version)?;
        let retention = match self.retention {
            ArtifactRetention::Ephemeral => graph_domain::ArtifactRetention::Ephemeral,
            ArtifactRetention::Evidence => graph_domain::ArtifactRetention::Evidence,
            ArtifactRetention::UserPinned => graph_domain::ArtifactRetention::UserPinned,
        };
        let protection = match self.declared_protection {
            ArtifactProtection::Unreviewed => graph_domain::ArtifactProtection::Unreviewed,
            ArtifactProtection::Redacted => graph_domain::ArtifactProtection::Redacted,
            ArtifactProtection::Encrypted => graph_domain::ArtifactProtection::Encrypted,
            ArtifactProtection::RedactedAndEncrypted => {
                graph_domain::ArtifactProtection::RedactedAndEncrypted
            }
        };
        Ok(graph_domain::Artifact::new(
            self.id,
            self.project.into(),
            self.graph_version,
            self.analysis_run,
            self.content_sha256,
            self.byte_length,
            self.kind,
            retention,
            protection,
        )?)
    }
}

impl From<&graph_domain::Artifact> for Artifact {
    fn from(value: &graph_domain::Artifact) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: value.id().into(),
            project: value.project().into(),
            graph_version: value.graph_version().into(),
            analysis_run: value.analysis_run().into(),
            content_sha256: value.content_sha256().into(),
            byte_length: value.byte_length(),
            kind: value.kind().into(),
            retention: match value.retention() {
                graph_domain::ArtifactRetention::Ephemeral => ArtifactRetention::Ephemeral,
                graph_domain::ArtifactRetention::Evidence => ArtifactRetention::Evidence,
                graph_domain::ArtifactRetention::UserPinned => ArtifactRetention::UserPinned,
            },
            declared_protection: match value.declared_protection() {
                graph_domain::ArtifactProtection::Unreviewed => ArtifactProtection::Unreviewed,
                graph_domain::ArtifactProtection::Redacted => ArtifactProtection::Redacted,
                graph_domain::ArtifactProtection::Encrypted => ArtifactProtection::Encrypted,
                graph_domain::ArtifactProtection::RedactedAndEncrypted => {
                    ArtifactProtection::RedactedAndEncrypted
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Artifact {
        Artifact {
            schema_version: 1,
            id: "a1".into(),
            project: ProjectRef {
                repository_id: "repo".into(),
                worktree_id: "w".into(),
                git_head: "h".into(),
                working_tree_fingerprint: "s".into(),
                config_hash: "c".into(),
                ignore_policy_version: "1".into(),
            },
            graph_version: "g1".into(),
            analysis_run: "r1".into(),
            content_sha256: "a".repeat(64),
            byte_length: 0,
            kind: "stdout".into(),
            retention: ArtifactRetention::Evidence,
            declared_protection: ArtifactProtection::Unreviewed,
        }
    }

    #[test]
    fn artifact_roundtrip_preserves_all_policy_declarations_and_empty_bytes() {
        for retention in [
            ArtifactRetention::Ephemeral,
            ArtifactRetention::Evidence,
            ArtifactRetention::UserPinned,
        ] {
            for protection in [
                ArtifactProtection::Unreviewed,
                ArtifactProtection::Redacted,
                ArtifactProtection::Encrypted,
                ArtifactProtection::RedactedAndEncrypted,
            ] {
                let mut wire = fixture();
                wire.retention = retention;
                wire.declared_protection = protection;
                let domain = wire.clone().try_into_domain().unwrap();
                assert_eq!(Artifact::from(&domain), wire);
                assert_eq!(domain.byte_length(), 0);
            }
        }
    }

    #[test]
    fn artifact_rejects_future_schema_bad_hash_overflow_and_missing_scope() {
        let mut wire = fixture();
        wire.schema_version = 2;
        assert!(matches!(
            wire.try_into_domain(),
            Err(ProtocolError::UnsupportedSchema(2))
        ));
        for hash in [
            "a".repeat(63),
            "A".repeat(64),
            "g".repeat(64),
            "é".repeat(32),
        ] {
            let mut wire = fixture();
            wire.content_sha256 = hash;
            assert!(wire.try_into_domain().is_err());
        }
        let mut wire = fixture();
        wire.byte_length = u64::MAX;
        assert!(wire.try_into_domain().is_err());
        for field in 0..10 {
            let mut wire = fixture();
            let target = match field {
                0 => &mut wire.id,
                1 => &mut wire.graph_version,
                2 => &mut wire.analysis_run,
                3 => &mut wire.kind,
                4 => &mut wire.project.repository_id,
                5 => &mut wire.project.worktree_id,
                6 => &mut wire.project.git_head,
                7 => &mut wire.project.working_tree_fingerprint,
                8 => &mut wire.project.config_hash,
                _ => &mut wire.project.ignore_policy_version,
            };
            *target = " ".into();
            assert!(wire.try_into_domain().is_err(), "field {field}");
        }
    }
}

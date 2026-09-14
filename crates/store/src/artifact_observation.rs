use crate::{Store, StoreError};
use graph_application::{
    ARTIFACT_CONTENT_VERIFIER_VERSION, ArtifactContentObservation, ArtifactObservationRepository,
    ArtifactRepository, VerifiedArtifactContent,
};
use graph_domain::ProjectRef;
use rusqlite::{OptionalExtension, TransactionBehavior, params};

impl ArtifactObservationRepository for Store {
    type Error = StoreError;
    fn record_artifact_observation(
        &mut self,
        id: &str,
        content: &VerifiedArtifactContent,
        observed_at_ms: i64,
    ) -> Result<bool, StoreError> {
        if id.trim().is_empty() || observed_at_ms < 0 {
            return Err(StoreError::Invalid(
                "observation id and nonnegative timestamp required",
            ));
        }
        let artifact = content.artifact();
        if self
            .artifact(artifact.id(), artifact.project(), artifact.graph_version())?
            .as_ref()
            != Some(artifact)
        {
            return Err(StoreError::Invalid(
                "verified artifact must exactly match registered metadata",
            ));
        }
        let project = serde_json::to_string(&graph_protocol::ProjectRef::from(artifact.project()))?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            include_str!("sql/insert_artifact_observation.sql"),
            params![
                id,
                artifact.id(),
                project,
                artifact.graph_version(),
                observed_at_ms,
                ARTIFACT_CONTENT_VERIFIER_VERSION
            ],
        )?;
        if inserted == 0 {
            let same: bool = tx.query_row(
                include_str!("sql/match_artifact_observation.sql"),
                params![
                    id,
                    artifact.id(),
                    project,
                    artifact.graph_version(),
                    observed_at_ms,
                    ARTIFACT_CONTENT_VERIFIER_VERSION
                ],
                |r| r.get(0),
            )?;
            if !same {
                return Err(StoreError::Conflict);
            }
        }
        tx.commit()?;
        Ok(inserted == 1)
    }

    fn artifact_observation(
        &self,
        id: &str,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<Option<ArtifactContentObservation>, StoreError> {
        project
            .validate()
            .map_err(|_| StoreError::Invalid("invalid project scope"))?;
        if id.trim().is_empty() || graph_version.trim().is_empty() {
            return Err(StoreError::Invalid(
                "observation id and graph version required",
            ));
        }
        let project_json = serde_json::to_string(&graph_protocol::ProjectRef::from(project))?;
        let row: Option<(String, i64, String)> = self
            .0
            .query_row(
                include_str!("sql/select_artifact_observation.sql"),
                params![id, project_json, graph_version],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        row.map(|(artifact_id, observed_at_ms, verifier_version)| {
            if observed_at_ms < 0 || verifier_version != ARTIFACT_CONTENT_VERIFIER_VERSION {
                return Err(StoreError::Corrupt(
                    "invalid or unsupported artifact observation".into(),
                ));
            }
            let artifact = self
                .artifact(&artifact_id, project, graph_version)?
                .ok_or_else(|| StoreError::Corrupt("observation artifact missing".into()))?;
            Ok(ArtifactContentObservation {
                id: id.into(),
                artifact,
                observed_at_ms,
                verifier_version,
            })
        })
        .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_application::{AnalysisRepository, ArtifactReader, verify_artifact};
    use graph_domain::{AnalysisRun, Artifact, ArtifactProtection, ArtifactRetention};
    use std::io::{self, Read};

    fn run() -> AnalysisRun {
        AnalysisRun::new(
            "r1".into(),
            ProjectRef {
                repository_id: "repo".into(),
                worktree_id: "w".into(),
                git_head: "h".into(),
                working_tree_fingerprint: "s".into(),
                config_hash: "c".into(),
                ignore_policy_version: "1".into(),
            },
            "g1".into(),
            "fixture".into(),
            "1".into(),
            "a".repeat(64),
            "b".repeat(64),
        )
        .unwrap()
    }
    fn artifact() -> Artifact {
        Artifact::new(
            "a1".into(),
            run().project().clone(),
            "g1".into(),
            "r1".into(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            3,
            "stdout".into(),
            ArtifactRetention::Evidence,
            ArtifactProtection::Unreviewed,
        )
        .unwrap()
    }
    struct Reader;
    impl ArtifactReader for Reader {
        fn open_artifact(&self, _: &Artifact) -> io::Result<Box<dyn Read>> {
            Ok(Box::new(io::Cursor::new(b"abc")))
        }
    }

    #[test]
    fn observation_requires_exact_registration_and_replays_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.db");
        let mut store = Store::open(&path).unwrap();
        let content = verify_artifact(&Reader, &artifact(), 3).unwrap();
        assert!(
            store
                .record_artifact_observation("o1", &content, 10)
                .is_err()
        );
        store.record_analysis_run(&run()).unwrap();
        store.record_artifact(&artifact()).unwrap();
        assert!(
            store
                .record_artifact_observation("o1", &content, 10)
                .unwrap()
        );
        assert!(
            !store
                .record_artifact_observation("o1", &content, 10)
                .unwrap()
        );
        assert!(matches!(
            store.record_artifact_observation("o1", &content, 11),
            Err(StoreError::Conflict)
        ));
        assert!(
            store
                .record_artifact_observation(" ", &content, 10)
                .is_err()
        );
        assert!(
            store
                .record_artifact_observation("o2", &content, -1)
                .is_err()
        );
        let mut changed = graph_protocol::Artifact::from(&artifact());
        changed.kind = "different".into();
        let changed_content =
            verify_artifact(&Reader, &changed.try_into_domain().unwrap(), 3).unwrap();
        assert!(
            store
                .record_artifact_observation("o2", &changed_content, 10)
                .is_err()
        );
        assert!(
            store
                .0
                .execute("UPDATE artifact_observations SET observed_at_ms=99", [])
                .is_err()
        );
        assert!(
            store
                .0
                .execute("DELETE FROM artifact_observations", [])
                .is_err()
        );
        drop(store);
        let store = Store::open(&path).unwrap();
        let found = store
            .artifact_observation("o1", run().project(), "g1")
            .unwrap()
            .unwrap();
        assert_eq!(found.artifact, artifact());
        assert_eq!(found.observed_at_ms, 10);
        assert_eq!(found.verifier_version, ARTIFACT_CONTENT_VERIFIER_VERSION);
        assert!(
            store
                .artifact_observation("missing", run().project(), "g1")
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .artifact_observation("o1", run().project(), "g2")
                .unwrap()
                .is_none()
        );
        for index in 0..6 {
            let mut scope = run().project().clone();
            let field = match index {
                0 => &mut scope.repository_id,
                1 => &mut scope.worktree_id,
                2 => &mut scope.git_head,
                3 => &mut scope.working_tree_fingerprint,
                4 => &mut scope.config_hash,
                _ => &mut scope.ignore_policy_version,
            };
            *field = "other".into();
            assert!(
                store
                    .artifact_observation("o1", &scope, "g1")
                    .unwrap()
                    .is_none()
            );
        }
        assert!(store.pending_events("projector", 100).unwrap().is_empty());
    }

    #[test]
    fn v5_upgrade_preserves_artifacts_without_fabricated_observations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("upgrade.db");
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        crate::embedded::migrations::runner()
            .set_target(refinery::Target::Version(5))
            .run(&mut conn)
            .unwrap();
        let mut old = Store(conn);
        old.record_analysis_run(&run()).unwrap();
        old.record_artifact(&artifact()).unwrap();
        drop(old);
        let store = Store::open(&path).unwrap();
        assert_eq!(
            store.artifact("a1", run().project(), "g1").unwrap(),
            Some(artifact())
        );
        assert!(
            store
                .artifact_observation("o1", run().project(), "g1")
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .0
                .execute(
                    include_str!("sql/insert_artifact_observation.sql"),
                    params![
                        "bad",
                        "a1",
                        "other-snapshot",
                        "g1",
                        1,
                        ARTIFACT_CONTENT_VERIFIER_VERSION
                    ]
                )
                .is_err()
        );
    }
}

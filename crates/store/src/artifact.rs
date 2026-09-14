use crate::{Store, StoreError};
use graph_application::{AnalysisRepository, ArtifactRepository};
use graph_domain::{Artifact, ProjectRef};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

impl ArtifactRepository for Store {
    type Error = StoreError;

    fn record_artifact(&mut self, artifact: &Artifact) -> Result<bool, StoreError> {
        // Runs are immutable; the composite FK also enforces scope at insertion.
        if self
            .analysis_run(
                artifact.analysis_run(),
                artifact.project(),
                artifact.graph_version(),
            )?
            .is_none()
        {
            return Err(StoreError::Invalid(
                "artifact requires a registered analysis run in the same snapshot",
            ));
        }
        let project = serde_json::to_string(&graph_protocol::ProjectRef::from(artifact.project()))?;
        let descriptor = serde_json::to_string(&graph_protocol::Artifact::from(artifact))?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            include_str!("sql/insert_artifact.sql"),
            params![
                artifact.id(),
                project,
                artifact.graph_version(),
                artifact.analysis_run(),
                descriptor
            ],
        )?;
        if inserted == 0 {
            let equal: bool = tx.query_row(
                include_str!("sql/match_artifact.sql"),
                params![
                    artifact.id(),
                    project,
                    artifact.graph_version(),
                    artifact.analysis_run(),
                    descriptor
                ],
                |r| r.get(0),
            )?;
            if !equal {
                return Err(StoreError::Conflict);
            }
        }
        tx.commit()?;
        Ok(inserted == 1)
    }

    fn artifact(
        &self,
        id: &str,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<Option<Artifact>, StoreError> {
        project
            .validate()
            .map_err(|_| StoreError::Invalid("invalid project scope"))?;
        if id.trim().is_empty() || graph_version.trim().is_empty() {
            return Err(StoreError::Invalid(
                "artifact id and graph version are required",
            ));
        }
        let project_json = serde_json::to_string(&graph_protocol::ProjectRef::from(project))?;
        let raw: Option<(String, String)> = self
            .0
            .query_row(
                include_str!("sql/select_artifact.sql"),
                params![id, project_json, graph_version],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        raw.map(|(json, run_id)| {
            let wire: graph_protocol::Artifact = serde_json::from_str(&json)?;
            let artifact = wire
                .try_into_domain()
                .map_err(|e| StoreError::Corrupt(e.to_string()))?;
            if artifact.id() != id
                || artifact.project() != project
                || artifact.graph_version() != graph_version
                || artifact.analysis_run() != run_id
            {
                return Err(StoreError::Corrupt(
                    "artifact descriptor scope mismatch".into(),
                ));
            }
            if self
                .analysis_run(&run_id, project, graph_version)?
                .is_none()
            {
                return Err(StoreError::Corrupt(
                    "artifact analysis registration missing".into(),
                ));
            }
            Ok(artifact)
        })
        .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_domain::{AnalysisRun, ArtifactProtection, ArtifactRetention};

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
            "c".repeat(64),
            0,
            "stdout".into(),
            ArtifactRetention::Evidence,
            ArtifactProtection::Unreviewed,
        )
        .unwrap()
    }

    #[test]
    fn artifact_registration_requires_scope_and_replays_immutably_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifacts.db");
        let mut store = Store::open(&path).unwrap();
        let original = artifact();
        assert!(store.record_artifact(&original).is_err());
        assert!(
            store
                .artifact("a1", run().project(), "g1")
                .unwrap()
                .is_none()
        );
        store.record_analysis_run(&run()).unwrap();
        assert!(store.record_artifact(&original).unwrap());
        assert!(!store.record_artifact(&original).unwrap());
        let mut conflict = graph_protocol::Artifact::from(&original);
        conflict.byte_length = 12;
        assert!(matches!(
            store.record_artifact(&conflict.try_into_domain().unwrap()),
            Err(StoreError::Conflict)
        ));
        assert!(
            store
                .0
                .execute("UPDATE artifacts SET descriptor='{}'", [])
                .is_err()
        );
        assert!(store.0.execute("DELETE FROM artifacts", []).is_err());
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(
            store.artifact("a1", run().project(), "g1").unwrap(),
            Some(original.clone())
        );
        for index in 0..7 {
            let mut wire = graph_protocol::Artifact::from(&original);
            let field = match index {
                0 => &mut wire.project.repository_id,
                1 => &mut wire.project.worktree_id,
                2 => &mut wire.project.git_head,
                3 => &mut wire.project.working_tree_fingerprint,
                4 => &mut wire.project.config_hash,
                5 => &mut wire.project.ignore_policy_version,
                _ => &mut wire.graph_version,
            };
            *field = "different".into();
            let different = wire.try_into_domain().unwrap();
            assert!(
                store
                    .artifact("a1", different.project(), different.graph_version())
                    .unwrap()
                    .is_none()
            );
            assert!(store.record_artifact(&different).is_err());
        }
        assert!(
            store
                .artifact("missing", run().project(), "g1")
                .unwrap()
                .is_none()
        );
        assert!(store.pending_events("projector", 100).unwrap().is_empty());
    }

    #[test]
    fn v4_upgrade_keeps_runs_and_foreign_key_rejects_cross_scope_raw_insert() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("upgrade.db");
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        crate::embedded::migrations::runner()
            .set_target(refinery::Target::Version(4))
            .run(&mut conn)
            .unwrap();
        let mut old = Store(conn);
        old.record_analysis_run(&run()).unwrap();
        drop(old);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(
            store.analysis_run("r1", run().project(), "g1").unwrap(),
            Some(run())
        );
        assert!(
            store
                .0
                .execute(
                    include_str!("sql/insert_artifact.sql"),
                    params!["bad", "different", "g1", "r1", "{}"]
                )
                .is_err()
        );
        assert!(store.record_artifact(&artifact()).unwrap());
    }
}

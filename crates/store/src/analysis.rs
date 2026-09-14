use crate::{Store, StoreError};
use graph_application::AnalysisRepository;
use graph_domain::{AnalysisRun, ProjectRef};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

impl AnalysisRepository for Store {
    type Error = StoreError;
    fn record_analysis_run(&mut self, run: &AnalysisRun) -> Result<bool, StoreError> {
        let project = serde_json::to_string(&graph_protocol::ProjectRef::from(run.project()))?;
        let descriptor = serde_json::to_string(&graph_protocol::AnalysisRun::from(run))?;
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            include_str!("sql/insert_analysis_run.sql"),
            params![run.id(), project, run.graph_version(), descriptor],
        )?;
        if inserted == 0 {
            let equal: bool = tx.query_row(
                include_str!("sql/match_analysis_run.sql"),
                params![run.id(), project, run.graph_version(), descriptor],
                |r| r.get(0),
            )?;
            if !equal {
                return Err(StoreError::Conflict);
            }
        }
        tx.commit()?;
        Ok(inserted == 1)
    }
    fn analysis_run(
        &self,
        id: &str,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<Option<AnalysisRun>, StoreError> {
        project
            .validate()
            .map_err(|_| StoreError::Invalid("invalid project scope"))?;
        if id.trim().is_empty() || graph_version.trim().is_empty() {
            return Err(StoreError::Invalid(
                "analysis id and graph version are required",
            ));
        }
        let project_json = serde_json::to_string(&graph_protocol::ProjectRef::from(project))?;
        let raw: Option<String> = self
            .0
            .query_row(
                include_str!("sql/select_analysis_run.sql"),
                params![id, project_json, graph_version],
                |r| r.get(0),
            )
            .optional()?;
        raw.map(|json| {
            let wire: graph_protocol::AnalysisRun = serde_json::from_str(&json)?;
            let run = wire
                .try_into_domain()
                .map_err(|e| StoreError::Corrupt(e.to_string()))?;
            if run.id() != id || run.project() != project || run.graph_version() != graph_version {
                return Err(StoreError::Corrupt(
                    "analysis descriptor scope mismatch".into(),
                ));
            }
            Ok(run)
        })
        .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_application::EvidenceRepository;
    use graph_domain::SourceEvidence;

    fn project() -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: "head".into(),
            working_tree_fingerprint: "snapshot".into(),
            config_hash: "cfg".into(),
            ignore_policy_version: "1".into(),
        }
    }
    fn run() -> AnalysisRun {
        AnalysisRun::new(
            "run1".into(),
            project(),
            "g1".into(),
            "codegraph".into(),
            "revision1".into(),
            "a".repeat(64),
            "b".repeat(64),
        )
        .unwrap()
    }
    fn citation() -> SourceEvidence {
        SourceEvidence::new(
            "e1".into(),
            project(),
            "g1".into(),
            "src/main.rs".into(),
            "c".repeat(64),
            1,
            2,
            "run1".into(),
        )
        .unwrap()
    }

    #[test]
    fn registration_replays_immutably_and_links_only_matching_scope() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("runs.db");
        let mut store = Store::open(&path).unwrap();
        store.record_source(&citation()).unwrap();
        assert!(store.source_analysis_run(&citation()).unwrap().is_none());
        let original = run();
        assert!(store.record_analysis_run(&original).unwrap());
        assert!(!store.record_analysis_run(&original).unwrap());
        let mut changed = graph_protocol::AnalysisRun::from(&original);
        changed.analyzer_version = "other".into();
        assert!(matches!(
            store.record_analysis_run(&changed.try_into_domain().unwrap()),
            Err(StoreError::Conflict)
        ));
        assert!(
            store
                .0
                .execute("UPDATE analysis_runs SET descriptor='{}'", [])
                .is_err()
        );
        assert!(store.0.execute("DELETE FROM analysis_runs", []).is_err());
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_eq!(
            store.source_analysis_run(&citation()).unwrap(),
            Some(original)
        );
        assert!(
            store
                .analysis_run("run1", &project(), "g2")
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .analysis_run("absent", &project(), "g1")
                .unwrap()
                .is_none()
        );
        for index in 0..6 {
            let mut scope = project();
            let field = match index {
                0 => &mut scope.repository_id,
                1 => &mut scope.worktree_id,
                2 => &mut scope.git_head,
                3 => &mut scope.working_tree_fingerprint,
                4 => &mut scope.config_hash,
                _ => &mut scope.ignore_policy_version,
            };
            *field = "other".into();
            assert!(store.analysis_run("run1", &scope, "g1").unwrap().is_none());
        }
        // Registration alone neither accepts a fact nor creates task events.
        assert!(store.pending_events("projector", 100).unwrap().is_empty());
    }

    #[test]
    fn v3_upgrade_preserves_unregistered_citations_without_fabricating_runs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("upgrade.db");
        let mut connection = rusqlite::Connection::open(&path).unwrap();
        crate::embedded::migrations::runner()
            .set_target(refinery::Target::Version(3))
            .run(&mut connection)
            .unwrap();
        let mut old = Store(connection);
        old.record_source(&citation()).unwrap();
        drop(old);
        let upgraded = Store::open(&path).unwrap();
        assert_eq!(
            upgraded.source_evidence("e1", &project(), "g1").unwrap(),
            Some(citation())
        );
        assert!(upgraded.source_analysis_run(&citation()).unwrap().is_none());
    }

    #[test]
    fn run_wire_roundtrips_and_rejects_invalid_or_future_contracts() {
        let expected = run();
        let wire = graph_protocol::AnalysisRun::from(&expected);
        let json = serde_json::to_string(&wire).unwrap();
        let decoded: graph_protocol::AnalysisRun = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.try_into_domain().unwrap(), expected);
        let mut future = wire.clone();
        future.schema_version = 2;
        assert!(future.try_into_domain().is_err());
        for hash in ["", "abc", &"A".repeat(64)] {
            let mut bad = wire.clone();
            bad.input_manifest_sha256 = hash.into();
            assert!(bad.try_into_domain().is_err());
        }
        let mut bad = wire.clone();
        bad.analyzer = " ".into();
        assert!(bad.try_into_domain().is_err());
        let mut bad = wire;
        bad.configuration_sha256 = "z".repeat(64);
        assert!(bad.try_into_domain().is_err());
    }
}

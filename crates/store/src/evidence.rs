use graph_application::EvidenceRepository;
use graph_domain::{ProjectRef, SourceEvidence};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

use crate::{Store, StoreError, domain_corruption};

impl EvidenceRepository for Store {
    type Error = StoreError;

    fn record_source(&mut self, evidence: &SourceEvidence) -> Result<bool, StoreError> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = record_source_in_transaction(&tx, evidence)?;
        tx.commit()?;
        Ok(inserted)
    }

    fn source_evidence(
        &self,
        id: &str,
        project: &ProjectRef,
        graph_version: &str,
    ) -> Result<Option<SourceEvidence>, StoreError> {
        source_evidence_in_connection(&self.0, id, project, graph_version)
    }
}

pub(crate) fn record_source_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    evidence: &SourceEvidence,
) -> Result<bool, StoreError> {
    let project = serde_json::to_string(&graph_protocol::ProjectRef::from(evidence.project()))?;
    let inserted = tx.execute(
        include_str!("sql/insert_source_evidence.sql"),
        params![
            evidence.id(),
            project,
            evidence.graph_version(),
            evidence.path(),
            evidence.content_sha256(),
            evidence.start_line(),
            evidence.end_line(),
            evidence.analysis_run()
        ],
    )?;
    if inserted == 0 {
        let equal: bool = tx.query_row(
            include_str!("sql/match_source_evidence.sql"),
            params![
                evidence.id(),
                project,
                evidence.graph_version(),
                evidence.path(),
                evidence.content_sha256(),
                evidence.start_line(),
                evidence.end_line(),
                evidence.analysis_run()
            ],
            |row| row.get(0),
        )?;
        if !equal {
            return Err(StoreError::Conflict);
        }
    }
    Ok(inserted == 1)
}

pub(crate) fn source_evidence_in_connection(
    connection: &rusqlite::Connection,
    id: &str,
    project: &ProjectRef,
    graph_version: &str,
) -> Result<Option<SourceEvidence>, StoreError> {
    project
        .validate()
        .map_err(|_| StoreError::Invalid("invalid project scope"))?;
    if id.trim().is_empty() || graph_version.trim().is_empty() {
        return Err(StoreError::Invalid(
            "evidence id and graph version are required",
        ));
    }
    let project_json = serde_json::to_string(&graph_protocol::ProjectRef::from(project))?;
    let raw = connection
        .query_row(
            include_str!("sql/select_source_evidence.sql"),
            params![id, project_json, graph_version],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, u32>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    raw.map(|(path, hash, start, end, run)| {
        SourceEvidence::new(
            id.into(),
            project.clone(),
            graph_version.into(),
            path,
            hash,
            start,
            end,
            run,
        )
        .map_err(domain_corruption)
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "main".into(),
            git_head: "head".into(),
            working_tree_fingerprint: "snapshot".into(),
            config_hash: "config".into(),
            ignore_policy_version: "1".into(),
        }
    }

    fn evidence(
        path: &str,
        hash: &str,
        start: u32,
        end: u32,
    ) -> Result<SourceEvidence, graph_domain::DomainError> {
        SourceEvidence::new(
            "e1".into(),
            project(),
            "g1".into(),
            path.into(),
            hash.into(),
            start,
            end,
            "run1".into(),
        )
    }

    #[test]
    fn evidence_replays_immutably_and_isolated_by_complete_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.db");
        let original = evidence("src/main.rs", &"a".repeat(64), 1, 5).unwrap();
        let mut store = Store::open(&path).unwrap();
        assert!(store.record_source(&original).unwrap());
        assert!(!store.record_source(&original).unwrap());
        let changed = evidence("src/main.rs", &"b".repeat(64), 1, 5).unwrap();
        assert!(matches!(
            store.record_source(&changed),
            Err(StoreError::Conflict)
        ));
        assert!(
            store
                .0
                .execute("UPDATE source_evidence SET path='other'", [])
                .is_err()
        );
        assert!(store.0.execute("DELETE FROM source_evidence", []).is_err());
        drop(store);

        let store = Store::open(&path).unwrap();
        assert_eq!(
            store.source_evidence("e1", &project(), "g1").unwrap(),
            Some(original)
        );
        assert_eq!(store.source_evidence("e1", &project(), "g2").unwrap(), None);
        assert_eq!(
            store.source_evidence("unknown", &project(), "g1").unwrap(),
            None
        );
        for field in 0..6 {
            let mut scope = project();
            let value = match field {
                0 => &mut scope.repository_id,
                1 => &mut scope.worktree_id,
                2 => &mut scope.git_head,
                3 => &mut scope.working_tree_fingerprint,
                4 => &mut scope.config_hash,
                _ => &mut scope.ignore_policy_version,
            };
            *value = "different".into();
            assert_eq!(store.source_evidence("e1", &scope, "g1").unwrap(), None);
        }
    }

    #[test]
    fn malformed_source_citations_are_rejected_before_storage() {
        let hash = "a".repeat(64);
        for path in [
            "/etc/passwd",
            "../outside",
            "src/../outside",
            "C:/outside",
            "src\\main.rs",
            "src//main.rs",
            "./main.rs",
            "bad\0path",
        ] {
            assert!(evidence(path, &hash, 1, 1).is_err(), "{path:?}");
        }
        assert!(evidence("src/main.rs", "fake-hash", 1, 1).is_err());
        assert!(evidence("src/main.rs", &hash, 0, 1).is_err());
        assert!(evidence("src/main.rs", &hash, 5, 4).is_err());
    }
}

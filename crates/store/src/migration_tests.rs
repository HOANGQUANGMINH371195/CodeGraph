use super::*;

fn scalar(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

#[test]
fn v13_upgrade_preserves_history_and_existing_data_without_inventing_graphs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v13.sqlite");
    let mut db = Connection::open(&path).unwrap();
    embedded::migrations::runner()
        .set_target(refinery::Target::Version(13))
        .run(&mut db)
        .unwrap();
    let before = embedded::migrations::runner()
        .get_applied_migrations(&mut db)
        .unwrap();
    db.execute_batch(include_str!("sql/fixture_v7_preserved_receipt_task.sql"))
        .unwrap();
    drop(db);
    for _ in 0..2 {
        let mut store = Store::open(&path).unwrap();
        let after = embedded::migrations::runner()
            .get_applied_migrations(&mut store.0)
            .unwrap();
        assert_eq!(&after[..13], before.as_slice());
        assert_eq!(after.len(), 18);
        let counts: (i64, i64, i64, i64, i64, i64) = store
            .0
            .query_row(
                include_str!("sql/fixture_check_v14_deployment_upgrade.sql"),
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(counts, (0, 0, 0, 0, 1, 1));
    }
}

#[test]
fn v7_upgrade_adds_empty_receipt_ledger_without_rewriting_prior_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v7.sqlite");
    let mut connection = Connection::open(&path).unwrap();
    embedded::migrations::runner()
        .set_target(refinery::Target::Version(7))
        .run(&mut connection)
        .unwrap();
    let before = embedded::migrations::runner()
        .get_applied_migrations(&mut connection)
        .unwrap();
    connection
        .execute_batch(include_str!("sql/fixture_v7_preserved_receipt_task.sql"))
        .unwrap();
    drop(connection);
    drop(Store::open(&path).unwrap());
    let mut connection = Connection::open(&path).unwrap();
    let after = embedded::migrations::runner()
        .get_applied_migrations(&mut connection)
        .unwrap();
    assert_eq!(&after[..7], before.as_slice());
    assert_eq!(after.len(), 18);
    let counts: (i64, i64, i64, i64) = connection
        .query_row(
            include_str!("sql/fixture_check_v8_receipt_upgrade.sql"),
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(counts, (0, 1, 1, 1));
}

#[test]
fn nonzero_user_version_is_rejected_before_journal_or_schema_changes() {
    for version in [1_i64, 99, -1] {
        for has_history in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("legacy.db");
            if has_history {
                drop(Store::open(&path).unwrap());
            }
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch(
                    "PRAGMA journal_mode=DELETE;
                 CREATE TABLE preserved_payload(value TEXT);
                 INSERT INTO preserved_payload VALUES('original');",
                )
                .unwrap();
            connection
                .pragma_update(None, "user_version", version)
                .unwrap();
            drop(connection);
            let bytes = std::fs::read(&path).unwrap();
            for _ in 0..2 {
                assert!(
                    matches!(Store::open(&path), Err(StoreError::UnsupportedUserVersion(v)) if v == version)
                );
            }
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            assert!(!path.with_extension("db-wal").exists());
            let connection = Connection::open(&path).unwrap();
            assert_eq!(scalar(&connection, "PRAGMA user_version"), version);
            assert_eq!(
                scalar(
                    &connection,
                    "SELECT count(*) FROM preserved_payload WHERE value='original'"
                ),
                1
            );
            assert_eq!(
                scalar(
                    &connection,
                    "SELECT count(*) FROM sqlite_master WHERE name='refinery_schema_history'"
                ),
                i64::from(has_history)
            );
            let mode: String = connection
                .pragma_query_value(None, "journal_mode", |r| r.get(0))
                .unwrap();
            assert_eq!(mode, "delete");
        }
    }
}

#[test]
fn v9_upgrade_preserves_plan_without_inventing_launch_claim() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v9.sqlite");
    let mut connection = Connection::open(&path).unwrap();
    embedded::migrations::runner()
        .set_target(refinery::Target::Version(9))
        .run(&mut connection)
        .unwrap();
    connection
        .execute_batch(include_str!("sql/fixture_v7_preserved_receipt_task.sql"))
        .unwrap();
    connection
        .execute(
            include_str!("sql/insert_execution_plan.sql"),
            params!["old-run", "preserved", "legacy plan"],
        )
        .unwrap();
    let before = embedded::migrations::runner()
        .get_applied_migrations(&mut connection)
        .unwrap();
    drop(connection);
    drop(Store::open(&path).unwrap());
    let mut connection = Connection::open(&path).unwrap();
    let after = embedded::migrations::runner()
        .get_applied_migrations(&mut connection)
        .unwrap();
    assert_eq!(&after[..9], before.as_slice());
    assert_eq!(after.len(), 18);
    let counts: (i64, i64) = connection
        .query_row(
            include_str!("sql/fixture_check_v10_launch_upgrade.sql"),
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(counts, (0, 1));
}

#[test]
fn v8_upgrade_preserves_receipt_bytes_without_fabricating_execution_plan() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v8.sqlite");
    let mut connection = Connection::open(&path).unwrap();
    embedded::migrations::runner()
        .set_target(refinery::Target::Version(8))
        .run(&mut connection)
        .unwrap();
    connection
        .execute_batch(include_str!("sql/fixture_v7_preserved_receipt_task.sql"))
        .unwrap();
    connection
        .execute(
            include_str!("sql/insert_execution_receipt.sql"),
            params!["old-run", "preserved", "legacy diagnostic"],
        )
        .unwrap();
    let before = embedded::migrations::runner()
        .get_applied_migrations(&mut connection)
        .unwrap();
    drop(connection);
    drop(Store::open(&path).unwrap());
    let mut connection = Connection::open(&path).unwrap();
    let after = embedded::migrations::runner()
        .get_applied_migrations(&mut connection)
        .unwrap();
    assert_eq!(&after[..8], before.as_slice());
    assert_eq!(after.len(), 18);
    let counts: (i64, i64, i64) = connection
        .query_row(
            include_str!("sql/fixture_check_v9_plan_upgrade.sql"),
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(counts, (0, 1, 1));
}

#[test]
fn future_binary_history_is_rejected_without_rewriting_schema_or_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.db");
    drop(Store::open(&path).unwrap());
    let mut connection = Connection::open(&path).unwrap();
    let mut migrations = embedded::migrations::runner().get_migrations().clone();
    let future_version = migrations
        .iter()
        .map(|migration| migration.version())
        .max()
        .unwrap()
        + 1;
    migrations.push(
        refinery::Migration::unapplied(
            &format!("V{future_version}__future_fixture"),
            "CREATE TABLE future_payload(id INTEGER PRIMARY KEY, value TEXT);
         INSERT INTO future_payload VALUES(1, 'preserve me');",
        )
        .unwrap(),
    );
    refinery::Runner::new(&migrations)
        .set_grouped(true)
        .run(&mut connection)
        .unwrap();
    let before = refinery::Runner::new(&migrations)
        .get_applied_migrations(&mut connection)
        .unwrap();
    drop(connection);
    for _ in 0..2 {
        assert!(matches!(Store::open(&path), Err(StoreError::Migration(_))));
    }
    let mut reopened = Connection::open(&path).unwrap();
    assert_eq!(
        scalar(&reopened, "SELECT count(*) FROM refinery_schema_history"),
        before.len() as i64
    );
    assert_eq!(
        scalar(
            &reopened,
            "SELECT count(*) FROM future_payload WHERE value='preserve me'"
        ),
        1
    );
    let after = refinery::Runner::new(&migrations)
        .get_applied_migrations(&mut reopened)
        .unwrap();
    assert_eq!(before, after);
    let integrity: String = reopened
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
}

#[test]
fn v10_upgrade_preserves_history_and_does_not_fabricate_rpc_launches() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v10.sqlite");
    let mut connection = Connection::open(&path).unwrap();
    embedded::migrations::runner()
        .set_target(refinery::Target::Version(10))
        .run(&mut connection)
        .unwrap();
    connection
        .execute_batch(include_str!("sql/fixture_v7_preserved_receipt_task.sql"))
        .unwrap();
    let before = embedded::migrations::runner()
        .get_applied_migrations(&mut connection)
        .unwrap();
    drop(connection);
    drop(Store::open(&path).unwrap());
    let mut connection = Connection::open(&path).unwrap();
    let after = embedded::migrations::runner()
        .get_applied_migrations(&mut connection)
        .unwrap();
    assert_eq!(&after[..10], before.as_slice());
    assert_eq!(after.len(), 18);
    let counts: (i64, i64, i64, i64, i64) = connection
        .query_row(
            include_str!("sql/fixture_check_v11_rpc_upgrade.sql"),
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(counts, (0, 0, 1, 1, 1));
}

#[test]
fn failed_grouped_upgrade_rolls_back_prior_pending_versions_and_can_retry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("upgrade.db");
    let mut connection = Connection::open(&path).unwrap();
    embedded::migrations::runner()
        .set_target(refinery::Target::Version(1))
        .run(&mut connection)
        .unwrap();
    connection
        .execute_batch(
            "INSERT INTO tasks(id,spec,state) VALUES('preserved','fixture-only','queued');
         INSERT INTO events(task_id,kind,at_ms,payload) VALUES('preserved','enqueued',0,'original');
         CREATE TABLE source_evidence(sentinel TEXT);
         INSERT INTO source_evidence VALUES('user data');",
        )
        .unwrap();
    drop(connection);
    // V2 succeeds inside the group; V3 collides with a pre-existing table.
    // Store's bounded retry must not leave V2 committed or forge V3 history.
    assert!(matches!(Store::open(&path), Err(StoreError::Migration(_))));
    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        scalar(&connection, "SELECT count(*) FROM refinery_schema_history"),
        1
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM sqlite_master WHERE name='event_outbox'"
        ),
        0
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM tasks WHERE id='preserved'"
        ),
        1
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM events WHERE payload='original'"
        ),
        1
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM source_evidence WHERE sentinel='user data'"
        ),
        1
    );
    // Fixture-only explicit recovery preserves the conflicting data under a new name.
    // Production Store never renames or deletes user tables on its own.
    connection
        .execute_batch("ALTER TABLE source_evidence RENAME TO preserved_fixture_table;")
        .unwrap();
    drop(connection);
    drop(Store::open(&path).unwrap());
    let reopened = Connection::open(&path).unwrap();
    assert_eq!(
        scalar(&reopened, "SELECT count(*) FROM refinery_schema_history"),
        18
    );
    assert_eq!(scalar(&reopened, "SELECT count(*) FROM event_outbox"), 1);
    assert_eq!(
        scalar(
            &reopened,
            "SELECT count(*) FROM preserved_fixture_table WHERE sentinel='user data'"
        ),
        1
    );
}

#[test]
fn v11_upgrade_preserves_valid_rpc_claim_and_history_without_inventing_observation() {
    use graph_application::{
        RpcLaunchQueryRepository, RpcLaunchRepository, RpcSpawnObservationRepository,
    };
    use graph_domain::{ProjectRef, RpcConnectionSpec, RpcLaunchSpec, RpcProcessSpec};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v11.sqlite");
    let mut connection = Connection::open(&path).unwrap();
    embedded::migrations::runner()
        .set_target(refinery::Target::Version(11))
        .run(&mut connection)
        .unwrap();
    let before = embedded::migrations::runner()
        .get_applied_migrations(&mut connection)
        .unwrap();
    assert_eq!(before.len(), 11);
    // Exercise the real V11 APIs without Store::open upgrading the fixture first.
    let mut old = Store(connection);
    let task = TaskSpec::new(
        TaskId::new("preserved").unwrap(),
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "source".into(),
            git_head: "head".into(),
            working_tree_fingerprint: "dirty-source".into(),
            config_hash: "config".into(),
            ignore_policy_version: "1".into(),
        },
        "graph".into(),
        "reader".into(),
        "native".into(),
        vec!["src".into()],
        vec![],
        "context".into(),
        vec!["report".into()],
        100,
    )
    .unwrap();
    old.enqueue(&task, 0).unwrap();
    let lease = old
        .lease(task.id(), &WorkerId::new("worker").unwrap(), 10, 100)
        .unwrap();
    let mut snapshot = task.project().clone();
    snapshot.worktree_id = "execution".into();
    let spec = RpcLaunchSpec::new(
        "preserved-launch".into(),
        task,
        lease,
        "host".into(),
        "unverified-approval".into(),
        snapshot,
        RpcProcessSpec::new(
            "/fixture/never-executed".into(),
            vec!["".into(), "a b".into(), "tiếng Việt".into()],
            "src".into(),
            "a".repeat(64),
            "b".repeat(64),
            1000,
            200,
            4096,
            8192,
        )
        .unwrap(),
        RpcConnectionSpec::new(
            "epoch".into(),
            "fixture".into(),
            "1.2".into(),
            true,
            7,
            16384,
        )
        .unwrap(),
    )
    .unwrap();
    assert!(old.register_rpc_launch(&spec, 11).unwrap());
    assert!(old.claim_rpc_launch(&spec, 12).unwrap());
    let preserved: String = old
        .0
        .query_row(
            include_str!("sql/fixture_rpc_spawn_v11_snapshot.sql"),
            [],
            |r| r.get(0),
        )
        .unwrap();
    let events = old.events(0, 100).unwrap();
    assert_eq!(events.len(), 4);
    drop(old);

    for _ in 0..2 {
        let mut reopened = Store::open(&path).unwrap();
        let after = embedded::migrations::runner()
            .get_applied_migrations(&mut reopened.0)
            .unwrap();
        assert_eq!(&after[..11], before.as_slice());
        assert_eq!(after.len(), 18);
        let actual: String = reopened
            .0
            .query_row(
                include_str!("sql/fixture_rpc_spawn_v11_snapshot.sql"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(actual, preserved);
        assert_eq!(reopened.events(0, 100).unwrap(), events);
        assert_eq!(
            reopened.rpc_launch(spec.id(), spec.task()).unwrap(),
            Some(spec.clone())
        );
        assert_eq!(
            reopened
                .rpc_launch_snapshot(spec.id(), spec.task())
                .unwrap()
                .unwrap()
                .claimed_at_ms(),
            Some(12)
        );
        assert!(reopened.rpc_spawn_observation(&spec).unwrap().is_none());
        assert_eq!(
            scalar(&reopened.0, include_str!("sql/fixture_rpc_spawn_empty.sql")),
            0
        );
        assert!(!reopened.claim_rpc_launch(&spec, 13).unwrap());
        assert_eq!(reopened.events(0, 100).unwrap(), events);
    }
}

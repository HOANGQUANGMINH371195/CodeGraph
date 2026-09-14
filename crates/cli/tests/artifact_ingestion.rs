use graph_application::{
    AnalysisRepository, ArtifactIngestionError, ArtifactRepository, ArtifactVerificationError,
    ingest_artifact,
};
use graph_source::DirectoryArtifacts;
use graph_store::Store;
use serde_json::json;
use std::io::{self, Read};

struct MustNotRead;
impl Read for MustNotRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("must reject before consuming input")
    }
}

#[test]
fn ingestion_registers_before_bytes_and_recovers_failed_write_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("blobs");
    std::fs::create_dir(&root).unwrap();
    let path = dir.path().join("state.db");
    let mut store = Store::open(&path).unwrap();
    let project = json!({"repository_id":"repo","worktree_id":"w","git_head":"h",
        "working_tree_fingerprint":"s","config_hash":"c","ignore_policy_version":"1"});
    let descriptor: graph_protocol::Artifact =
        serde_json::from_value(json!({"schema_version":1,"id":"a1",
        "project":project,"graph_version":"g1","analysis_run":"r1",
        "content_sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "byte_length":3,"kind":"stdout","retention":"evidence","declared_protection":"unreviewed"}))
        .unwrap();
    let artifact = descriptor.clone().try_into_domain().unwrap();
    let writer = DirectoryArtifacts::open(&root, artifact.project().clone(), "g1".into()).unwrap();
    assert!(matches!(
        ingest_artifact(&mut store, &writer, &artifact, &mut MustNotRead, 2),
        Err(ArtifactIngestionError::TooLarge)
    ));
    assert!(matches!(
        ingest_artifact(&mut store, &writer, &artifact, &mut MustNotRead, 3),
        Err(ArtifactIngestionError::Registration(_))
    ));
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    let run: graph_protocol::AnalysisRun =
        serde_json::from_value(json!({"schema_version":1,"id":"r1",
        "project":project,"graph_version":"g1","analyzer":"fixture","analyzer_version":"1",
        "configuration_sha256":"a".repeat(64),"input_manifest_sha256":"b".repeat(64)}))
        .unwrap();
    store
        .record_analysis_run(&run.try_into_domain().unwrap())
        .unwrap();
    assert!(matches!(
        ingest_artifact(&mut store, &writer, &artifact, &mut b"abd".as_slice(), 3),
        Err(ArtifactIngestionError::Content {
            metadata_inserted: true,
            source: ArtifactVerificationError::HashMismatch
        })
    ));
    assert_eq!(
        store.artifact("a1", artifact.project(), "g1").unwrap(),
        Some(artifact.clone())
    );
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    drop(store);
    let mut store = Store::open(&path).unwrap();
    let success =
        ingest_artifact(&mut store, &writer, &artifact, &mut b"abc".as_slice(), 3).unwrap();
    assert!(!success.metadata_inserted);
    assert!(success.blob_inserted);
    assert_eq!(success.content.artifact(), &artifact);
    let replay =
        ingest_artifact(&mut store, &writer, &artifact, &mut b"abc".as_slice(), 3).unwrap();
    assert!(!replay.metadata_inserted && !replay.blob_inserted);
    let mut conflicting = descriptor;
    conflicting.kind = "different".into();
    assert!(matches!(
        ingest_artifact(
            &mut store,
            &writer,
            &conflicting.try_into_domain().unwrap(),
            &mut MustNotRead,
            3
        ),
        Err(ArtifactIngestionError::Registration(_))
    ));
    assert_eq!(
        std::fs::read(root.join(artifact.content_sha256())).unwrap(),
        b"abc"
    );
    assert!(store.pending_events("projector", 100).unwrap().is_empty());
}

use graph_application::{ArtifactReader, ArtifactVerificationError, verify_artifact};
use graph_domain::{Artifact, ArtifactProtection, ArtifactRetention, ProjectRef};
use graph_source::DirectoryArtifacts;
use sha2::{Digest, Sha256};
use std::io::{self, Read};

fn artifact(bytes: &[u8]) -> Artifact {
    Artifact::new(
        "a1".into(),
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "w".into(),
            git_head: "h".into(),
            working_tree_fingerprint: "s".into(),
            config_hash: "c".into(),
            ignore_policy_version: "1".into(),
        },
        "g1".into(),
        "r1".into(),
        format!("{:x}", Sha256::digest(bytes)),
        bytes.len() as u64,
        "stdout".into(),
        ArtifactRetention::Evidence,
        ArtifactProtection::Unreviewed,
    )
    .unwrap()
}

#[test]
fn ingest_publishes_verified_content_replays_and_preserves_bad_existing_blob() {
    let dir = tempfile::tempdir().unwrap();
    for bytes in [vec![], vec![0, 255, 1], vec![42; 100_000]] {
        let descriptor = artifact(&bytes);
        let store = DirectoryArtifacts::open(dir.path(), descriptor.project().clone(), "g1".into())
            .unwrap();
        assert!(
            store
                .ingest(&descriptor, bytes.as_slice(), bytes.len() as u64)
                .unwrap()
        );
        assert!(
            !store
                .ingest(&descriptor, bytes.as_slice(), bytes.len() as u64)
                .unwrap()
        );
        assert_eq!(
            std::fs::read(dir.path().join(descriptor.content_sha256())).unwrap(),
            bytes
        );
        verify_artifact(&store, &descriptor, 200_000).unwrap();
    }
    let descriptor = artifact(b"abc");
    let path = dir.path().join(descriptor.content_sha256());
    std::fs::write(&path, b"BAD").unwrap();
    let store =
        DirectoryArtifacts::open(dir.path(), descriptor.project().clone(), "g1".into()).unwrap();
    assert!(matches!(
        store.ingest(&descriptor, b"abc".as_slice(), 3),
        Err(ArtifactVerificationError::HashMismatch)
    ));
    assert_eq!(std::fs::read(path).unwrap(), b"BAD");
    assert!(std::fs::read_dir(dir.path()).unwrap().all(|e| {
        !e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".ingest-")
    }));
}

#[test]
fn ingest_failures_cleanup_only_owned_staging_and_never_publish() {
    let dir = tempfile::tempdir().unwrap();
    let descriptor = artifact(b"abc");
    let store =
        DirectoryArtifacts::open(dir.path(), descriptor.project().clone(), "g1".into()).unwrap();
    let sentinel = dir.path().join(".ingest-other-owner");
    std::fs::write(&sentinel, b"keep").unwrap();
    for bytes in [b"ab".as_slice(), b"abcd", b"abd"] {
        assert!(store.ingest(&descriptor, bytes, 3).is_err());
        assert!(!dir.path().join(descriptor.content_sha256()).exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    struct Fails;
    impl Read for Fails {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("injected reader failure"))
        }
    }
    assert!(store.ingest(&descriptor, Fails, 3).is_err());
    assert!(matches!(
        store.ingest(&descriptor, Fails, 2),
        Err(ArtifactVerificationError::TooLarge)
    ));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read(sentinel).unwrap(), b"keep");
}

#[test]
fn concurrent_ingestion_publishes_once_without_partial_blob_or_staging_leaks() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = vec![7; 100_000];
    let descriptor = artifact(&bytes);
    let barrier = std::sync::Barrier::new(4);
    let outcomes = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let store = DirectoryArtifacts::open(
                        dir.path(),
                        descriptor.project().clone(),
                        "g1".into(),
                    )
                    .unwrap();
                    barrier.wait();
                    store
                        .ingest(&descriptor, bytes.as_slice(), 100_000)
                        .unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(outcomes.iter().filter(|&&inserted| inserted).count(), 1);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    assert_eq!(
        std::fs::read(dir.path().join(descriptor.content_sha256())).unwrap(),
        bytes
    );
}

#[test]
fn streaming_blob_verification_checks_binary_empty_truncation_and_tampering() {
    let dir = tempfile::tempdir().unwrap();
    for bytes in [vec![], vec![0, 255, 10], vec![42; 100_000]] {
        let descriptor = artifact(&bytes);
        let path = dir.path().join(descriptor.content_sha256());
        let reader =
            DirectoryArtifacts::open(dir.path(), descriptor.project().clone(), "g1".into())
                .unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(
            verify_artifact(&reader, &descriptor, bytes.len() as u64)
                .unwrap()
                .artifact(),
            &descriptor
        );
        std::fs::write(&path, [&bytes[..], &[1]].concat()).unwrap();
        assert!(matches!(
            verify_artifact(&reader, &descriptor, 200_000),
            Err(ArtifactVerificationError::LengthMismatch)
        ));
        if !bytes.is_empty() {
            std::fs::write(&path, &bytes[..bytes.len() - 1]).unwrap();
            assert!(matches!(
                verify_artifact(&reader, &descriptor, 200_000),
                Err(ArtifactVerificationError::LengthMismatch)
            ));
            let mut tampered = bytes.clone();
            tampered[0] ^= 1;
            std::fs::write(&path, &tampered).unwrap();
            assert!(matches!(
                verify_artifact(&reader, &descriptor, 200_000),
                Err(ArtifactVerificationError::HashMismatch)
            ));
        }
    }
}

#[test]
fn budget_checked_before_open_and_scope_before_file_lookup() {
    struct MustNotOpen;
    impl ArtifactReader for MustNotOpen {
        fn open_artifact(&self, _: &Artifact) -> io::Result<Box<dyn Read>> {
            panic!("budget must precede open")
        }
    }
    let descriptor = artifact(b"abc");
    assert!(matches!(
        verify_artifact(&MustNotOpen, &descriptor, 2),
        Err(ArtifactVerificationError::TooLarge)
    ));
    let dir = tempfile::tempdir().unwrap();
    let reader =
        DirectoryArtifacts::open(dir.path(), descriptor.project().clone(), "g2".into()).unwrap();
    assert!(
        matches!(verify_artifact(&reader, &descriptor, 3), Err(ArtifactVerificationError::Read(e))
        if e.kind() == io::ErrorKind::PermissionDenied)
    );
    let reader =
        DirectoryArtifacts::open(dir.path(), descriptor.project().clone(), "g1".into()).unwrap();
    std::fs::create_dir(dir.path().join(descriptor.content_sha256())).unwrap();
    assert!(
        matches!(verify_artifact(&reader, &descriptor, 3), Err(ArtifactVerificationError::Read(e))
        if e.kind() == io::ErrorKind::InvalidInput)
    );
}

#[cfg(unix)]
#[test]
fn blob_symlink_cannot_escape_host_selected_root() {
    let dir = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let descriptor = artifact(b"secret");
    let outside = external.path().join("secret");
    std::fs::write(&outside, b"secret").unwrap();
    std::os::unix::fs::symlink(&outside, dir.path().join(descriptor.content_sha256())).unwrap();
    let reader =
        DirectoryArtifacts::open(dir.path(), descriptor.project().clone(), "g1".into()).unwrap();
    assert!(verify_artifact(&reader, &descriptor, 6).is_err());
}

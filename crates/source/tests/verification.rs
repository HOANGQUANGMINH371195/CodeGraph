use graph_application::{SourceLimits, SourceReader, SourceVerificationError, verify_source};
use graph_domain::{ProjectRef, SourceEvidence};
use graph_source::{DirectorySource, SourceReadError};
use sha2::{Digest, Sha256};

fn project() -> ProjectRef {
    ProjectRef {
        repository_id: "fixture".into(),
        worktree_id: "main".into(),
        git_head: "head".into(),
        working_tree_fingerprint: "snapshot".into(),
        config_hash: "cfg".into(),
        ignore_policy_version: "1".into(),
    }
}

fn citation(path: &str, bytes: &[u8], start: u32, end: u32) -> SourceEvidence {
    SourceEvidence::new(
        "e1".into(),
        project(),
        "g1".into(),
        path.into(),
        format!("{:x}", Sha256::digest(bytes)),
        start,
        end,
        "run1".into(),
    )
    .unwrap()
}

fn limits() -> SourceLimits {
    SourceLimits::new(1024, 256).unwrap()
}

#[test]
fn long_file_returns_only_requested_lines_within_output_budget() {
    let dir = tempfile::tempdir().unwrap();
    let content = format!(
        "{}target_one\ntarget_two\n{}",
        "irrelevant\n".repeat(1000),
        "more\n".repeat(1000)
    );
    std::fs::write(dir.path().join("long.rs"), &content).unwrap();
    let reader = DirectorySource::open(dir.path(), project(), "g1".into()).unwrap();
    let checked = verify_source(
        &reader,
        &citation("long.rs", content.as_bytes(), 1001, 1002),
        SourceLimits::new(32 * 1024, 32).unwrap(),
    )
    .unwrap();
    assert_eq!(checked.text(), "target_one\ntarget_two\n");
    assert!(checked.text().len() <= 32);
}

#[test]
fn returns_exact_unicode_crlf_slice_and_detects_changes_outside_the_slice() {
    let dir = tempfile::tempdir().unwrap();
    let original = "header\r\nxin chào\r\nlast".as_bytes();
    std::fs::write(dir.path().join("main.rs"), original).unwrap();
    let source = DirectorySource::open(dir.path(), project(), "g1".into()).unwrap();
    let evidence = citation("main.rs", original, 2, 2);
    let checked = verify_source(&source, &evidence, limits()).unwrap();
    assert_eq!(checked.text(), "xin chào\r\n");
    assert_eq!(checked.evidence(), &evidence);
    std::fs::write(dir.path().join("main.rs"), "changed\r\nxin chào\r\nlast").unwrap();
    assert!(matches!(
        verify_source(&source, &evidence, limits()),
        Err(SourceVerificationError::HashMismatch)
    ));
    // A previously checked slice owns its bytes; later edits cannot change it.
    assert_eq!(checked.text(), "xin chào\r\n");
}

#[test]
fn rejects_phantom_eof_lines_binary_text_and_oversized_output() {
    let dir = tempfile::tempdir().unwrap();
    let source = DirectorySource::open(dir.path(), project(), "g1".into()).unwrap();
    for bytes in [b"".as_slice(), b"abc\n", b"abc"] {
        std::fs::write(dir.path().join("main.rs"), bytes).unwrap();
        let line = if bytes.is_empty() { 1 } else { 2 };
        assert!(matches!(
            verify_source(&source, &citation("main.rs", bytes, line, line), limits()),
            Err(SourceVerificationError::InvalidRange)
        ));
    }
    for bytes in [b"\xff".as_slice(), b"abc\0def"] {
        std::fs::write(dir.path().join("main.rs"), bytes).unwrap();
        assert!(matches!(
            verify_source(&source, &citation("main.rs", bytes, 1, 1), limits()),
            Err(SourceVerificationError::InvalidText)
        ));
    }
    std::fs::write(dir.path().join("main.rs"), b"abc").unwrap();
    let evidence = citation("main.rs", b"abc", 1, 1);
    // Independent known SHA-256 test vector.
    assert_eq!(
        evidence.content_sha256(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(matches!(
        verify_source(&source, &evidence, SourceLimits::new(3, 2).unwrap()),
        Err(SourceVerificationError::SliceTooLarge)
    ));
    assert!(matches!(
        source.read_source(&evidence, 2),
        Err(SourceReadError::TooLarge)
    ));
    assert_eq!(
        verify_source(&source, &evidence, SourceLimits::new(3, 3).unwrap())
            .unwrap()
            .text(),
        "abc"
    );
}

#[test]
fn scope_is_checked_before_access_and_non_files_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut scope = project();
    scope.worktree_id = "other".into();
    let source = DirectorySource::open(dir.path(), scope, "g1".into()).unwrap();
    let evidence = citation("missing.rs", b"abc", 1, 1);
    assert!(matches!(
        source.read_source(&evidence, 100),
        Err(SourceReadError::ScopeMismatch)
    ));
    let source = DirectorySource::open(dir.path(), project(), "g2".into()).unwrap();
    assert!(matches!(
        source.read_source(&evidence, 100),
        Err(SourceReadError::ScopeMismatch)
    ));
    let source = DirectorySource::open(dir.path(), project(), "g1".into()).unwrap();
    std::fs::create_dir(dir.path().join("missing.rs")).unwrap();
    assert!(matches!(
        source.read_source(&evidence, 100),
        Err(SourceReadError::NotRegularFile)
    ));
}

#[cfg(unix)]
#[test]
fn symlink_escape_is_denied_but_internal_links_work() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret"), b"secret").unwrap();
    symlink(outside.path(), root.path().join("outside")).unwrap();
    let source = DirectorySource::open(root.path(), project(), "g1".into()).unwrap();
    assert!(
        source
            .read_source(&citation("outside/secret", b"secret", 1, 1), 100)
            .is_err()
    );
    std::fs::write(root.path().join("local"), b"local").unwrap();
    symlink("local", root.path().join("inside")).unwrap();
    assert_eq!(
        source
            .read_source(&citation("inside", b"local", 1, 1), 100)
            .unwrap(),
        b"local"
    );
}

#[cfg(unix)]
#[test]
fn fifo_is_rejected_without_waiting_for_a_writer() {
    use rustix::fs::{CWD, FileType, Mode, mknodat};
    let root = tempfile::tempdir().unwrap();
    mknodat(
        CWD,
        root.path().join("pipe"),
        FileType::Fifo,
        Mode::RUSR | Mode::WUSR,
        0,
    )
    .unwrap();
    let source = DirectorySource::open(root.path(), project(), "g1".into()).unwrap();
    assert!(matches!(
        source.read_source(&citation("pipe", b"", 1, 1), 100),
        Err(SourceReadError::NotRegularFile)
    ));
}

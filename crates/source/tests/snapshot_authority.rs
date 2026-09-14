use std::{fs, path::Path, process::Command, time::Duration};

use graph_application::{SourceReader, SourceSnapshotAuthority};
use graph_source::{GitSnapshotAuthority, GitSnapshotAuthorityError};
use sha2::{Digest, Sha256};

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git must be installed for the snapshot authority fixture");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn init_repo(root: &Path) {
    fs::create_dir_all(root).unwrap();
    git(root, &["init", "--quiet"]);
    git(root, &["config", "user.name", "snapshot-test"]);
    git(
        root,
        &["config", "user.email", "snapshot-test@example.invalid"],
    );
    fs::write(root.join("tracked.txt"), "tracked\n").unwrap();
    git(root, &["add", "tracked.txt"]);
    git(root, &["commit", "--quiet", "-m", "initial"]);
}

#[test]
fn git_target_head_verifier_rejects_project_drift() {
    use graph_application::TargetHeadVerifier;
    use graph_domain::{TaskId, TaskSpec};
    use graph_source::GitTargetHeadVerifier;
    let root = tempfile::tempdir().unwrap();
    init_repo(root.path());
    let authority = authority(root.path(), 4096);
    let project = authority.current_project().unwrap();
    let task = TaskSpec::new(
        TaskId::new("task-target").unwrap(),
        project.clone(),
        "g".into(),
        "prompt".into(),
        "native".into(),
        vec!["tracked.txt".into()],
        vec![],
        "context".into(),
        vec!["patch".into()],
        100,
    )
    .unwrap();
    let verifier = GitTargetHeadVerifier::new(authority, "git-target/v1".into()).unwrap();
    let now = || {
        i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
    };
    for _ in 0..2 {
        let before = now();
        let observation = verifier.verify_target_head(&task).unwrap();
        assert!((before..=now()).contains(&observation.observed_at_ms()));
    }
    let observed = verifier.verify_target_head(&task).unwrap();
    assert_eq!(observed.observed_target(), &project);
    for field in 0..6 {
        let mut changed = project.clone();
        match field {
            0 => changed.repository_id.push_str("-other"),
            1 => changed.worktree_id.push_str("-other"),
            2 => changed.git_head = "f".repeat(40),
            3 => changed.working_tree_fingerprint.push_str("-other"),
            4 => changed.config_hash.push_str("-other"),
            _ => changed.ignore_policy_version.push_str("-other"),
        }
        let drifted = TaskSpec::new(
            TaskId::new("task-drift").unwrap(),
            changed,
            "g".into(),
            "prompt".into(),
            "native".into(),
            vec!["tracked.txt".into()],
            vec![],
            "context".into(),
            vec!["patch".into()],
            100,
        )
        .unwrap();
        assert!(matches!(
            verifier.verify_target_head(&drifted),
            Err(GitSnapshotAuthorityError::ProjectMismatch)
        ));
    }
    fs::write(root.path().join("tracked.txt"), "changed\n").unwrap();
    assert!(matches!(
        verifier.verify_target_head(&task),
        Err(GitSnapshotAuthorityError::ProjectMismatch)
    ));
    fs::write(root.path().join("tracked.txt"), "tracked\n").unwrap();
    assert!(verifier.verify_target_head(&task).is_ok());
    git(
        root.path(),
        &["commit", "--allow-empty", "--quiet", "-m", "new head"],
    );
    assert_eq!(
        fs::read(root.path().join("tracked.txt")).unwrap(),
        b"tracked\n"
    );
    assert!(matches!(
        verifier.verify_target_head(&task),
        Err(GitSnapshotAuthorityError::ProjectMismatch)
    ));
}

fn authority(root: &Path, max_total_bytes: u64) -> GitSnapshotAuthority {
    GitSnapshotAuthority::new(
        root,
        "repository-fixture".into(),
        "worktree-fixture".into(),
        "config-fixture".into(),
        "ignore-v1".into(),
        max_total_bytes,
    )
    .unwrap()
}

#[cfg(unix)]
#[test]
fn configured_canonical_root_cannot_be_retargeted_with_a_symlink() {
    let parent = tempfile::tempdir().unwrap();
    let original = parent.path().join("repo");
    let moved = parent.path().join("moved");
    init_repo(&original);
    let admitted = authority(&original, 4096);
    let project = admitted.current_project().unwrap();
    fs::rename(&original, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &original).unwrap();
    assert!(matches!(
        admitted.current_project(),
        Err(GitSnapshotAuthorityError::RootMismatch)
    ));
    assert_eq!(authority(&moved, 4096).current_project().unwrap(), project);
}

fn evidence(
    project: &graph_domain::ProjectRef,
    path: &str,
    bytes: &[u8],
) -> graph_domain::SourceEvidence {
    graph_domain::SourceEvidence::new(
        format!("evidence-{path}"),
        project.clone(),
        "graph-v1".into(),
        path.into(),
        format!("{:x}", Sha256::digest(bytes)),
        1,
        1,
        "run-1".into(),
    )
    .unwrap()
}

#[test]
fn clean_snapshot_is_stable_and_binding_is_exact() {
    let root = tempfile::tempdir().unwrap();
    init_repo(root.path());
    let authority = authority(root.path(), 1024);

    let project = authority.current_project().unwrap();
    assert_eq!(authority.current_project().unwrap(), project);
    let binding = authority
        .bind_source_snapshot(root.path(), &project, "graph-v1")
        .unwrap();
    assert_eq!(binding.project(), &project);
    assert_eq!(binding.graph_version(), "graph-v1");
    assert!(binding.binding_id().starts_with("git:"));
}

#[cfg(unix)]
#[test]
fn fingerprint_changes_for_content_untracked_deletion_mode_and_symlink() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let root = tempfile::tempdir().unwrap();
    init_repo(root.path());
    symlink("tracked.txt", root.path().join("tracked-link")).unwrap();
    git(root.path(), &["add", "tracked-link"]);
    git(root.path(), &["commit", "--quiet", "-m", "link"]);
    let authority = authority(root.path(), 1024);
    let clean = authority.current_project().unwrap();

    fs::write(root.path().join("tracked.txt"), "changed\n").unwrap();
    assert_ne!(authority.current_project().unwrap(), clean);
    fs::write(root.path().join("tracked.txt"), "tracked\n").unwrap();
    assert_eq!(authority.current_project().unwrap(), clean);

    fs::write(root.path().join("untracked.txt"), "untracked\n").unwrap();
    assert_ne!(authority.current_project().unwrap(), clean);
    fs::remove_file(root.path().join("untracked.txt")).unwrap();
    assert_eq!(authority.current_project().unwrap(), clean);

    fs::remove_file(root.path().join("tracked.txt")).unwrap();
    assert_ne!(authority.current_project().unwrap(), clean);
    fs::write(root.path().join("tracked.txt"), "tracked\n").unwrap();
    assert_eq!(authority.current_project().unwrap(), clean);

    let path = root.path().join("tracked.txt");
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(permissions.mode() | 0o100);
    fs::set_permissions(&path, permissions).unwrap();
    assert_ne!(authority.current_project().unwrap(), clean);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(permissions.mode() & !0o111);
    fs::set_permissions(&path, permissions).unwrap();
    assert_eq!(authority.current_project().unwrap(), clean);

    fs::remove_file(root.path().join("tracked-link")).unwrap();
    symlink("other-target", root.path().join("tracked-link")).unwrap();
    assert_ne!(authority.current_project().unwrap(), clean);
    fs::remove_file(root.path().join("tracked-link")).unwrap();
    symlink("tracked.txt", root.path().join("tracked-link")).unwrap();
    assert_eq!(authority.current_project().unwrap(), clean);
}

#[test]
fn authority_rejects_root_project_graph_and_budget_mismatches() {
    let root = tempfile::tempdir().unwrap();
    init_repo(root.path());
    let configured = authority(root.path(), 1024);
    let project = configured.current_project().unwrap();

    let other_root = tempfile::tempdir().unwrap();
    init_repo(other_root.path());
    assert!(matches!(
        configured.bind_source_snapshot(other_root.path(), &project, "graph-v1"),
        Err(GitSnapshotAuthorityError::RootMismatch)
    ));

    for mutate in [
        |wrong: &mut graph_domain::ProjectRef| wrong.repository_id = "other-repository".into(),
        |wrong: &mut graph_domain::ProjectRef| wrong.git_head.push('0'),
        |wrong: &mut graph_domain::ProjectRef| wrong.working_tree_fingerprint.push('0'),
    ] {
        let mut wrong_project = project.clone();
        mutate(&mut wrong_project);
        assert!(matches!(
            configured.bind_source_snapshot(root.path(), &wrong_project, "graph-v1"),
            Err(GitSnapshotAuthorityError::ProjectMismatch)
        ));
    }
    assert!(matches!(
        configured.bind_source_snapshot(root.path(), &project, " "),
        Err(GitSnapshotAuthorityError::InvalidMetadata(_))
    ));

    let too_small = authority(root.path(), 1);
    assert!(matches!(
        too_small.current_project(),
        Err(GitSnapshotAuthorityError::SourceTooLarge)
    ));
}

#[test]
fn non_git_root_and_invalid_host_metadata_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let authority = authority(root.path(), 1024);
    assert!(matches!(
        authority.current_project(),
        Err(GitSnapshotAuthorityError::GitFailed)
    ));

    assert!(matches!(
        GitSnapshotAuthority::new(
            root.path(),
            " ".into(),
            "worktree".into(),
            "config".into(),
            "ignore".into(),
            1024,
        ),
        Err(GitSnapshotAuthorityError::InvalidMetadata(_))
    ));
    assert!(matches!(
        GitSnapshotAuthority::with_command_timeout(
            root.path(),
            "repository".into(),
            "worktree".into(),
            "config".into(),
            "ignore".into(),
            1024,
            Duration::ZERO,
        ),
        Err(GitSnapshotAuthorityError::InvalidMetadata(_))
    ));
}

#[test]
fn unsupported_index_entries_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    init_repo(root.path());
    fs::create_dir(root.path().join("unsupported")).unwrap();
    let head = String::from_utf8(git(root.path(), &["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .to_owned();
    let cacheinfo = format!("160000,{head},unsupported");
    git(
        root.path(),
        &["update-index", "--add", "--cacheinfo", &cacheinfo],
    );
    assert!(matches!(
        authority(root.path(), 1024).current_project(),
        Err(GitSnapshotAuthorityError::UnsupportedEntry)
    ));
}

#[cfg(unix)]
#[test]
fn linked_worktree_resolves_its_git_common_directory() {
    let container = tempfile::tempdir().unwrap();
    let main = container.path().join("main");
    let linked = container.path().join("linked");
    init_repo(&main);
    git(
        &main,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "linked",
            linked.to_str().unwrap(),
            "HEAD",
        ],
    );

    let authority = authority(&linked, 1024);
    let project = authority.current_project().unwrap();
    assert!(
        authority
            .bind_source_snapshot(&linked, &project, "graph-v1")
            .is_ok()
    );
}

#[test]
fn invalid_roots_are_rejected_before_git_probe() {
    let missing = tempfile::tempdir().unwrap().path().join("missing");
    assert!(matches!(
        GitSnapshotAuthority::new(
            &missing,
            "repository".into(),
            "worktree".into(),
            "config".into(),
            "ignore".into(),
            1024,
        ),
        Err(GitSnapshotAuthorityError::RootUnavailable(_))
    ));
}

#[test]
fn materialized_source_is_detached_from_live_worktree_and_cleans_up() {
    let root = tempfile::tempdir().unwrap();
    init_repo(root.path());
    let authority = authority(root.path(), 1024);
    let project = authority.current_project().unwrap();
    let materialized = authority
        .materialize_source_snapshot(root.path(), &project, "graph-v1")
        .unwrap();
    let materialized_root = materialized.root().to_owned();
    let citation = evidence(&project, "tracked.txt", b"tracked\n");

    fs::write(root.path().join("tracked.txt"), "mutated\n").unwrap();
    assert_eq!(
        materialized.source().read_source(&citation, 1024).unwrap(),
        b"tracked\n"
    );
    fs::remove_file(root.path().join("tracked.txt")).unwrap();
    assert_eq!(
        materialized.source().read_source(&citation, 1024).unwrap(),
        b"tracked\n"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&materialized_root)
                .unwrap()
                .permissions()
                .mode()
                & 0o222,
            0
        );
        assert_eq!(
            fs::metadata(materialized_root.join("tracked.txt"))
                .unwrap()
                .permissions()
                .mode()
                & 0o222,
            0
        );
    }

    drop(materialized);
    assert!(!materialized_root.exists());
}

#[cfg(unix)]
#[test]
fn materialized_source_normalizes_internal_links_and_rejects_external_links() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    init_repo(root.path());
    symlink("tracked.txt", root.path().join("tracked-link")).unwrap();
    git(root.path(), &["add", "tracked-link"]);
    git(root.path(), &["commit", "--quiet", "-m", "link"]);
    let authority = authority(root.path(), 4096);
    let project = authority.current_project().unwrap();
    let materialized = authority
        .materialize_source_snapshot(root.path(), &project, "graph-v1")
        .unwrap();
    let link = materialized.root().join("tracked-link");
    assert!(fs::symlink_metadata(&link).unwrap().is_file());
    assert_eq!(
        materialized
            .source()
            .read_source(&evidence(&project, "tracked-link", b"tracked\n"), 1024)
            .unwrap(),
        b"tracked\n"
    );
    drop(materialized);

    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "secret\n").unwrap();
    fs::remove_file(root.path().join("tracked-link")).unwrap();
    symlink(
        outside.path().join("secret"),
        root.path().join("tracked-link"),
    )
    .unwrap();
    git(root.path(), &["add", "tracked-link"]);
    let external_project = authority.current_project().unwrap();
    assert!(
        authority
            .materialize_source_snapshot(root.path(), &external_project, "graph-v1")
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn materializer_rejects_link_bytes_absent_from_snapshot_identity() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    init_repo(root.path());
    fs::write(root.path().join(".gitignore"), "ignored.txt\n").unwrap();
    fs::write(root.path().join("ignored.txt"), "first").unwrap();
    symlink("ignored.txt", root.path().join("alias.txt")).unwrap();
    let authority = authority(root.path(), 4096);
    let project = authority.current_project().unwrap();
    assert!(
        authority
            .materialize_source_snapshot(root.path(), &project, "g1")
            .is_err()
    );
    fs::write(root.path().join("ignored.txt"), "other").unwrap();
    assert_eq!(project, authority.current_project().unwrap());
    assert!(
        authority
            .materialize_source_snapshot(root.path(), &project, "g1")
            .is_err()
    );
}

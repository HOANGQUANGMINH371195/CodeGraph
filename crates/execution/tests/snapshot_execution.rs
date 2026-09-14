#![cfg(target_os = "linux")]
use graph_application::environment_fingerprint;
use graph_domain::{
    CheckCommand,
    execution::{ChildCompletion, StopReason, StreamCompletion},
};
use graph_execution::run_trusted_fixture;
use graph_source::GitSnapshotAuthority;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path, process::Command, sync::atomic::AtomicBool};

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn reaped_child_reads_materialized_bytes_after_live_source_deletion() {
    let live = tempfile::tempdir().unwrap();
    git(live.path(), &["init", "--quiet"]);
    git(live.path(), &["config", "user.name", "fixture"]);
    git(
        live.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    fs::write(live.path().join("source.txt"), b"original\n").unwrap();
    git(live.path(), &["add", "."]);
    git(live.path(), &["commit", "--quiet", "-m", "source"]);
    let authority = GitSnapshotAuthority::new(
        live.path(),
        "r".into(),
        "w".into(),
        "c".into(),
        "i".into(),
        4096,
    )
    .unwrap();
    let project = authority.current_project().unwrap();
    let snapshot = authority
        .materialize_source_snapshot(live.path(), &project, "g")
        .unwrap();
    assert_eq!(snapshot.snapshot().project(), &project);
    assert_eq!(snapshot.snapshot().graph_version(), "g");
    let snapshot_path = snapshot.root().to_owned();
    fs::write(live.path().join("source.txt"), b"changed\n").unwrap();
    fs::remove_file(live.path().join("source.txt")).unwrap();
    let exe = Path::new(env!("CARGO_BIN_EXE_graph-execution-fixture"))
        .canonicalize()
        .unwrap();
    let environment = BTreeMap::new();
    let command = CheckCommand::new(
        exe.to_str().unwrap().into(),
        vec!["snapshot-source".into()],
        ".".into(),
        format!("{:x}", Sha256::digest(fs::read(&exe).unwrap())),
        environment_fingerprint(&environment).unwrap(),
        5000,
        1000,
        4096,
        4096,
    )
    .unwrap();
    let run = run_trusted_fixture(
        &command,
        &exe,
        snapshot.root(),
        &environment,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(run.stdout, b"original\n");
    assert!(run.stderr.is_empty());
    assert_eq!(run.completion.reason, StopReason::Exited);
    assert_eq!(
        run.completion.child,
        ChildCompletion::Reaped { exit_code: Some(0) }
    );
    assert_eq!(run.completion.stdout, StreamCompletion::Complete);
    assert_eq!(run.completion.stderr, StreamCompletion::Complete);
    assert!(run.unreaped_child.is_none());
    drop(snapshot);
    assert!(!snapshot_path.exists());
}

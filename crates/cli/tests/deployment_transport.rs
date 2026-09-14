//! A failed stdout receipt is not proof that a database transaction failed.
#![cfg(target_os = "linux")]

use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

fn command(db: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"));
    cmd.arg("--database").arg(db).args(args);
    cmd
}

fn json_output(db: &Path, args: &[&str]) -> Value {
    let output = command(db, args).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn failed_stdout_after_mutation_stops_watch_and_keeps_committed_generation() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db.sqlite");
    let source = dir.path().join("compose.yaml");
    fs::write(
        &source,
        include_bytes!("../../../fixtures/orders/compose.yaml"),
    )
    .unwrap();
    let project = json!({"repository_id":"repo","worktree_id":"main","git_head":"head",
        "working_tree_fingerprint":"snapshot","config_hash":"cfg","ignore_policy_version":"1"});
    let citation = json!({"schema_version":1,"id":"source","project":project,"graph_version":"g",
        "path":"compose.yaml","content_sha256":"a2a29afb353a588cc28759f281819f5cfda82fd5044f8fd5e38247959fa37701",
        "start_line":1,"end_line":20,"analysis_run":"run"});
    let task = json!({"schema_version":1,"id":"task","project":project,"graph_version":"g",
        "role":"reader","account_lane":"local","scope":["src"],"dependencies":[],
        "context_ref":"ctx","expected_artifacts":["report"],"token_budget":100});
    let citation_path = dir.path().join("source.json");
    let task_path = dir.path().join("task.json");
    fs::write(&citation_path, serde_json::to_vec(&citation).unwrap()).unwrap();
    fs::write(&task_path, serde_json::to_vec(&task).unwrap()).unwrap();
    json_output(&db, &["record-evidence", citation_path.to_str().unwrap()]);
    let root = dir.path().to_str().unwrap();
    let task_arg = task_path.to_str().unwrap();
    let publish = [
        "publish-compose",
        "source",
        task_arg,
        root,
        "--expected-generation",
        "0",
    ];
    let invalidate = [
        "invalidate-compose",
        "source",
        task_arg,
        "--expected-generation",
        "1",
    ];
    let reindex = [
        "reindex-compose",
        "source",
        task_arg,
        root,
        "--analysis-run",
        "capture-run",
        "--expected-generation",
        "2",
    ];
    let watch = [
        "watch-compose",
        "source",
        task_arg,
        root,
        "--analysis-run",
        "watch-run",
        "--expected-generation",
        "3",
        "--max-cycles",
        "3",
        "--interval-ms",
        "10",
    ];
    for (args, generation) in [
        (publish.as_slice(), 1),
        (invalidate.as_slice(), 2),
        (reindex.as_slice(), 3),
        (watch.as_slice(), 4),
    ] {
        let full = fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap();
        let output = command(&db, args)
            .stdout(Stdio::from(full))
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!output.stderr.is_empty());
        let snapshot = json_output(&db, &["deployment", "source", task_arg]);
        assert_eq!(snapshot["generation"], generation);
        assert_eq!(snapshot["graph"].is_null(), generation == 2);
        assert_eq!(snapshot["source_bytes_verified"], false);
        // Replaying the original expectation must not make another mutation.
        let stale = command(&db, args).output().unwrap();
        assert!(!stale.status.success());
        assert!(stale.stdout.is_empty());
        assert_eq!(
            json_output(&db, &["deployment", "source", task_arg]),
            snapshot
        );
    }
}

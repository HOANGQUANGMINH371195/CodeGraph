use serde_json::json;
use std::process::Command;

#[test]
fn oversized_json_is_rejected_before_enqueue_but_exact_limit_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let path = dir.path().join("task.json");
    let task = json!({"schema_version":1,"id":"t1",
        "project":{"repository_id":"repo","worktree_id":"w","git_head":"h",
            "working_tree_fingerprint":"s","config_hash":"c","ignore_policy_version":"1"},
        "graph_version":"g1","role":"reader","account_lane":"local","scope":["src"],
        "dependencies":[],"context_ref":"ctx","expected_artifacts":["report"],"token_budget":10});
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
            .arg("--database")
            .arg(&db)
            .args(args)
            .output()
            .unwrap()
    };
    let mut bytes = serde_json::to_vec(&task).unwrap();
    bytes.resize(8 * 1024 * 1024 + 1, b' ');
    std::fs::write(&path, &bytes).unwrap();
    for command in [
        "enqueue",
        "record-analysis-run",
        "record-artifact",
        "record-evidence",
    ] {
        let rejected = invoke(&[command, path.to_str().unwrap()]);
        assert!(!rejected.status.success());
        assert!(rejected.stdout.is_empty());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("JSON input exceeds 8 MiB"));
    }
    let events = invoke(&["events"]);
    for args in [
        vec![
            "deployment-diagram",
            "missing",
            path.to_str().unwrap(),
            "--node",
            "seed",
        ],
        vec![
            "deployment-context",
            "missing",
            path.to_str().unwrap(),
            "--node",
            "seed",
        ],
        vec![
            "reindex-compose",
            "missing",
            path.to_str().unwrap(),
            dir.path().to_str().unwrap(),
            "--analysis-run",
            "run",
            "--expected-generation",
            "0",
        ],
        vec![
            "watch-compose",
            "missing",
            path.to_str().unwrap(),
            dir.path().to_str().unwrap(),
            "--analysis-run",
            "run",
            "--expected-generation",
            "0",
            "--max-cycles",
            "1",
        ],
        vec![
            "analyze-compose",
            "missing",
            path.to_str().unwrap(),
            dir.path().to_str().unwrap(),
        ],
        vec![
            "publish-compose",
            "missing",
            path.to_str().unwrap(),
            dir.path().to_str().unwrap(),
            "--expected-generation",
            "0",
        ],
        vec!["deployment", "missing", path.to_str().unwrap()],
        vec![
            "invalidate-compose",
            "missing",
            path.to_str().unwrap(),
            "--expected-generation",
            "0",
        ],
    ] {
        let rejected = invoke(&args);
        assert!(!rejected.status.success());
        assert!(rejected.stdout.is_empty());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("JSON input exceeds 8 MiB"));
    }
    assert!(events.status.success());
    assert!(events.stdout.is_empty());
    bytes.pop();
    std::fs::write(&path, &bytes).unwrap();
    let accepted = invoke(&["enqueue", path.to_str().unwrap()]);
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&accepted.stdout).unwrap()["inserted"],
        true
    );
}

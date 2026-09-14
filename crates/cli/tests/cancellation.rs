use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command, Output},
};

fn run(database: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .arg("--database")
        .arg(database)
        .args(args)
        .output()
        .unwrap()
}

fn parse(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn cancellation_cli_replays_once_and_rejects_late_worker() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let task_path = dir.path().join("task.json");
    let lease_path = dir.path().join("lease.json");
    let task = json!({"schema_version":1,"id":"t1",
        "project":{"repository_id":"repo","worktree_id":"main","git_head":"head",
            "working_tree_fingerprint":"snapshot","config_hash":"cfg","ignore_policy_version":"1"},
        "graph_version":"g1","role":"worker","account_lane":"local","scope":["src"],
        "dependencies":[],"context_ref":"ctx","expected_artifacts":["patch"],"token_budget":100});
    std::fs::write(&task_path, task.to_string()).unwrap();
    parse(run(&db, &["enqueue", task_path.to_str().unwrap()]));
    let query = ["task", "t1"];
    let snapshot = parse(run(&db, &query));
    let baseline = run(&db, &query);
    let exact_budget = baseline.stdout.len().to_string();
    let exact = run(&db, &["task", "t1", "--max-output-bytes", &exact_budget]);
    assert!(exact.status.success());
    assert_eq!(exact.stdout, baseline.stdout);
    for budget in [
        "0".to_owned(),
        (baseline.stdout.len() - 1).to_string(),
        "16777217".to_owned(),
    ] {
        let rejected = run(&db, &["task", "t1", "--max-output-bytes", &budget]);
        assert!(!rejected.status.success());
        assert!(rejected.stdout.is_empty());
        assert!(!rejected.stderr.is_empty());
    }
    assert_eq!(snapshot["task"]["spec"], task);
    assert_eq!(snapshot["task"]["state"], "queued");
    assert_eq!(snapshot["execution_authority_granted"], false);
    assert_eq!(snapshot["agent_liveness"], "unknown");
    assert!(parse(run(&db, &["task", "missing"]))["task"].is_null());
    let scoped = ["task", "t1", "--scope", task_path.to_str().unwrap()];
    assert_eq!(parse(run(&db, &scoped))["task"], snapshot["task"]);
    assert_eq!(
        parse(run(&db, &scoped))["snapshot_filter"],
        "caller_supplied"
    );
    for field in [
        "repository_id",
        "worktree_id",
        "git_head",
        "working_tree_fingerprint",
        "config_hash",
        "ignore_policy_version",
    ] {
        let mut other = task.clone();
        other["project"][field] = json!("other");
        std::fs::write(&task_path, other.to_string()).unwrap();
        assert!(parse(run(&db, &scoped))["task"].is_null());
    }
    let mut other = task.clone();
    other["graph_version"] = json!("other");
    std::fs::write(&task_path, other.to_string()).unwrap();
    assert!(parse(run(&db, &scoped))["task"].is_null());
    // Scope identity is snapshot-based, not equality of two task IDs.
    other = task.clone();
    other["id"] = json!("scope-carrier");
    std::fs::write(&task_path, other.to_string()).unwrap();
    assert_eq!(parse(run(&db, &scoped))["task"], snapshot["task"]);
    std::fs::write(&task_path, "{}").unwrap();
    let invalid_scope = run(&db, &scoped);
    assert!(!invalid_scope.status.success());
    assert!(invalid_scope.stdout.is_empty());
    std::fs::write(&task_path, task.to_string()).unwrap();
    let lease = parse(run(
        &db,
        &["lease", "t1", "worker", "--duration-ms", "600000"],
    ));
    std::fs::write(&lease_path, lease.to_string()).unwrap();
    assert_eq!(parse(run(&db, &query))["task"]["state"], "leased");
    parse(run(
        &db,
        &["submit", lease_path.to_str().unwrap(), "candidate.patch"],
    ));
    let integration = run(&db, &["integrate", "t1", "all checks passed"]);
    assert_eq!(parse(run(&db, &query))["task"]["state"], "submitted");
    assert!(!integration.status.success());
    assert!(integration.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&integration.stderr)
            .contains("string-based integration is disabled")
    );
    let cancel = [
        "cancel",
        "t1",
        "--requested-by",
        "operator",
        "--reason",
        "stop this task",
    ];
    assert_eq!(
        parse(run(&db, &cancel)),
        json!({"schema_version":1,"task_id":"t1",
        "state":"cancelled","changed":true,"process_termination_confirmed":false})
    );
    let retry = parse(run(&db, &cancel));
    assert_eq!(parse(run(&db, &query))["task"]["state"], "cancelled");
    assert_eq!(retry["changed"], false);
    assert_eq!(retry["process_termination_confirmed"], false);
    for args in [
        vec!["submit", lease_path.to_str().unwrap(), "late.patch"],
        vec!["lease", "t1", "other"],
        vec!["integrate", "t1", "untrusted receipt"],
    ] {
        let output = run(&db, &args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let output = run(&db, &["events"]);
    assert!(output.status.success());
    let events: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 4);
    assert_eq!(events.last().unwrap()["kind"], "cancelled");
}

#[test]
fn cancellation_cli_errors_do_not_print_success_json() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    for args in [
        vec!["cancel", "missing"],
        vec![
            "cancel",
            "missing",
            "--requested-by",
            "operator",
            "--reason",
            "stop",
        ],
        vec![
            "cancel",
            "missing",
            "--requested-by",
            "",
            "--reason",
            "stop",
        ],
        vec![
            "cancel",
            "missing",
            "--requested-by",
            "operator",
            "--reason",
            " ",
        ],
    ] {
        let output = run(&db, &args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let help = run(&db, &["cancel", "--help"]);
    assert!(help.status.success());
    assert!(
        String::from_utf8_lossy(&help.stdout).contains("does not acknowledge process termination")
    );
}

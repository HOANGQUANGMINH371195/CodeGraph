use serde_json::{Value, json};
use std::process::{Command, Output};

#[test]
fn evidence_cli_roundtrips_rejects_conflicts_and_hides_other_snapshots() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.db");
    let citation_path = dir.path().join("citation.json");
    let task_path = dir.path().join("task.json");
    let project = json!({"repository_id":"repo", "worktree_id":"main", "git_head":"head",
        "working_tree_fingerprint":"snapshot", "config_hash":"cfg", "ignore_policy_version":"1"});
    let mut citation = json!({"schema_version":1, "id":"e1", "project":project, "graph_version":"g1",
        "path":"src/main.rs", "content_sha256":"a".repeat(64), "start_line":1, "end_line":3,
        "analysis_run":"run1"});
    let mut task = json!({"schema_version":1, "id":"t1", "project":project, "graph_version":"g1",
        "role":"reader", "account_lane":"local", "scope":["src"], "dependencies":[],
        "context_ref":"ctx", "expected_artifacts":["report"], "token_budget":100});
    std::fs::write(&citation_path, citation.to_string()).unwrap();
    std::fs::write(&task_path, task.to_string()).unwrap();
    let run = |args: &[&str]| -> Output {
        Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
            .arg("--database")
            .arg(&database)
            .args(args)
            .output()
            .unwrap()
    };
    let parse = |out: Output| -> Value {
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    };
    let path = citation_path.to_str().unwrap();
    assert_eq!(
        parse(run(&["record-evidence", path])),
        json!({"inserted":true,"verified":false})
    );
    assert_eq!(parse(run(&["record-evidence", path]))["inserted"], false);
    assert_eq!(
        parse(run(&["evidence", "e1", task_path.to_str().unwrap()])),
        citation
    );
    let mut verified_citation = citation.clone();
    verified_citation["id"] = json!("e2");
    verified_citation["end_line"] = json!(1);
    verified_citation["content_sha256"] =
        json!("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    std::fs::write(&citation_path, verified_citation.to_string()).unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), b"abc").unwrap();
    parse(run(&["record-evidence", path]));
    let verified = parse(run(&[
        "verify-evidence",
        "e2",
        task_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    ]));
    assert_eq!(verified["source"], "abc");
    assert_eq!(verified["source_is_untrusted"], true);
    assert_eq!(verified["content_hash_and_lines_verified"], true);
    assert_eq!(verified["snapshot_binding"], "caller_supplied");
    assert_eq!(verified["relationship_verified"], false);
    assert_eq!(verified["analysis_run_verified"], false);
    let budget_error = run(&[
        "verify-evidence",
        "e2",
        task_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        "--max-slice-bytes",
        "2",
    ]);
    assert!(!budget_error.status.success());
    assert!(budget_error.stdout.is_empty());
    std::fs::write(dir.path().join("src/main.rs"), b"changed").unwrap();
    let stale = run(&[
        "verify-evidence",
        "e2",
        task_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    ]);
    assert!(!stale.status.success());
    assert!(stale.stdout.is_empty());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("SHA-256"));
    task["project"]["working_tree_fingerprint"] = json!("changed");
    std::fs::write(&task_path, task.to_string()).unwrap();
    assert!(parse(run(&["evidence", "e1", task_path.to_str().unwrap()])).is_null());
    citation["end_line"] = json!(4);
    std::fs::write(&citation_path, citation.to_string()).unwrap();
    assert!(!run(&["record-evidence", path]).status.success());
    citation["schema_version"] = json!(2);
    std::fs::write(&citation_path, citation.to_string()).unwrap();
    assert!(!run(&["record-evidence", path]).status.success());
}

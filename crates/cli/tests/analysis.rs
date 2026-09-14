use serde_json::{Value, json};
use std::process::{Command, Output};

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
fn registration_is_immutable_scoped_and_never_execution_proof() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.db");
    let run_path = dir.path().join("run.json");
    let task_path = dir.path().join("task.json");
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
            .arg("--database")
            .arg(&database)
            .args(args)
            .output()
            .unwrap()
    };
    let project = json!({"repository_id":"repo", "worktree_id":"main", "git_head":"head",
        "working_tree_fingerprint":"snapshot", "config_hash":"cfg", "ignore_policy_version":"1"});
    let descriptor = json!({"schema_version":1,"id":"run1","project":project,
        "graph_version":"g1","analyzer":"fixture","analyzer_version":"1",
        "configuration_sha256":"a".repeat(64),"input_manifest_sha256":"b".repeat(64)});
    let task = json!({"schema_version":1,"id":"t1","project":project,"graph_version":"g1",
        "role":"reader","account_lane":"local","scope":["src"],"dependencies":[],
        "context_ref":"ctx","expected_artifacts":["report"],"token_budget":100});
    std::fs::write(&run_path, descriptor.to_string()).unwrap();
    std::fs::write(&task_path, task.to_string()).unwrap();
    let record = ["record-analysis-run", run_path.to_str().unwrap()];
    let query = ["analysis-run", "run1", task_path.to_str().unwrap()];
    let missing = json!({"schema_version":1,"run":null,"execution_verified":false});
    assert_eq!(parse(invoke(&query)), missing);
    for inserted in [true, false] {
        assert_eq!(
            parse(invoke(&record)),
            json!({"schema_version":1,"id":"run1",
            "inserted":inserted,"execution_verified":false})
        );
    }
    let registered = json!({"schema_version":1,"run":descriptor,"execution_verified":false});
    assert_eq!(parse(invoke(&query)), registered);
    for field in [
        "repository_id",
        "worktree_id",
        "git_head",
        "working_tree_fingerprint",
        "config_hash",
        "ignore_policy_version",
    ] {
        let mut different = task.clone();
        different["project"][field] = json!("different");
        std::fs::write(&task_path, different.to_string()).unwrap();
        assert_eq!(parse(invoke(&query)), missing, "{field}");
    }
    let mut different = task.clone();
    different["graph_version"] = json!("g2");
    std::fs::write(&task_path, different.to_string()).unwrap();
    assert_eq!(parse(invoke(&query)), missing);
    std::fs::write(&task_path, task.to_string()).unwrap();
    assert_eq!(
        parse(invoke(&[
            "analysis-run",
            "unknown",
            task_path.to_str().unwrap()
        ])),
        missing
    );

    for (field, value) in [
        ("analyzer_version", json!("2")),
        ("schema_version", json!(2)),
        ("configuration_sha256", json!("bad")),
        ("input_manifest_sha256", json!("bad")),
        ("analyzer", json!(" ")),
        ("unexpected", json!(true)),
    ] {
        let mut invalid = descriptor.clone();
        invalid[field] = value;
        std::fs::write(&run_path, invalid.to_string()).unwrap();
        let output = invoke(&record);
        assert!(!output.status.success(), "{field}");
        assert!(output.stdout.is_empty(), "{field}");
        assert!(!output.stderr.is_empty(), "{field}");
        assert_eq!(parse(invoke(&query)), registered);
    }
    // Registration must not promote a matching citation's execution trust.
    let citation_path = dir.path().join("citation.json");
    let citation = json!({"schema_version":1,"id":"e1","project":project,"graph_version":"g1",
        "path":"source.txt","content_sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "start_line":1,"end_line":1,"analysis_run":"run1"});
    std::fs::write(&citation_path, citation.to_string()).unwrap();
    std::fs::write(dir.path().join("source.txt"), "abc").unwrap();
    parse(invoke(&[
        "record-evidence",
        citation_path.to_str().unwrap(),
    ]));
    let verified = parse(invoke(&[
        "verify-evidence",
        "e1",
        task_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    ]));
    assert_eq!(verified["content_hash_and_lines_verified"], true);
    assert_eq!(verified["analysis_run_verified"], false);
    assert_eq!(verified["relationship_verified"], false);
    assert!(invoke(&["events"]).stdout.is_empty());
}

#[test]
fn analysis_help_does_not_create_database_or_claim_execution() {
    let dir = tempfile::tempdir().unwrap();
    for command in ["record-analysis-run", "analysis-run"] {
        let output = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
            .current_dir(dir.path())
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("does not execute") || help.contains("not execution proof"));
        assert!(!dir.path().join("harness.db").exists());
    }
}

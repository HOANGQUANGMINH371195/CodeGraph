use serde_json::{Value, json};
use std::process::{Command, Output};

fn parse(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    serde_json::from_slice(&out.stdout).unwrap()
}

fn rejected(out: Output) {
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(!out.stderr.is_empty());
}

#[test]
fn artifact_cli_requires_run_replays_and_keeps_trust_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.db");
    let artifact_path = dir.path().join("artifact.json");
    let task_path = dir.path().join("task.json");
    let run_path = dir.path().join("run.json");
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
            .arg("--database")
            .arg(&database)
            .args(args)
            .output()
            .unwrap()
    };
    let project = json!({"repository_id":"repo","worktree_id":"w","git_head":"h",
        "working_tree_fingerprint":"s","config_hash":"c","ignore_policy_version":"1"});
    let artifact = json!({"schema_version":1,"id":"a1","project":project,
        "graph_version":"g1","analysis_run":"r1","content_sha256":"a".repeat(64),
        "byte_length":100,"kind":"stdout","retention":"evidence",
        "declared_protection":"redacted_and_encrypted"});
    let run = json!({"schema_version":1,"id":"r1","project":project,"graph_version":"g1",
        "analyzer":"fixture","analyzer_version":"1","configuration_sha256":"b".repeat(64),
        "input_manifest_sha256":"c".repeat(64)});
    let task = json!({"schema_version":1,"id":"t1","project":project,"graph_version":"g1",
        "role":"reader","account_lane":"local","scope":["src"],"dependencies":[],
        "context_ref":"ctx","expected_artifacts":["report"],"token_budget":100});
    std::fs::write(&artifact_path, artifact.to_string()).unwrap();
    std::fs::write(&run_path, run.to_string()).unwrap();
    std::fs::write(&task_path, task.to_string()).unwrap();
    let record = ["record-artifact", artifact_path.to_str().unwrap()];
    let query = ["artifact", "a1", task_path.to_str().unwrap()];
    rejected(invoke(&record));
    let missing = json!({"schema_version":1,"artifact":null,"content_verified":false,
        "protection_verified":false,"execution_verified":false});
    assert_eq!(parse(invoke(&query)), missing);
    parse(invoke(&["record-analysis-run", run_path.to_str().unwrap()]));
    for inserted in [true, false] {
        assert_eq!(
            parse(invoke(&record)),
            json!({"schema_version":1,"id":"a1","inserted":inserted,
            "content_verified":false,"protection_verified":false,"execution_verified":false})
        );
    }
    let registered = json!({"schema_version":1,"artifact":artifact,"content_verified":false,
        "protection_verified":false,"execution_verified":false});
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
            "artifact",
            "unknown",
            task_path.to_str().unwrap()
        ])),
        missing
    );
    for (field, value) in [
        ("byte_length", json!(101)),
        ("analysis_run", json!("unknown")),
        ("graph_version", json!("g2")),
        ("schema_version", json!(2)),
        ("content_sha256", json!("bad")),
        ("reference_count", json!(0)),
        ("verified", json!(true)),
    ] {
        let mut invalid = artifact.clone();
        invalid[field] = value;
        std::fs::write(&artifact_path, invalid.to_string()).unwrap();
        rejected(invoke(&record));
        assert_eq!(parse(invoke(&query)), registered, "{field}");
    }
    let events = invoke(&["events"]);
    assert!(events.status.success());
    assert!(events.stdout.is_empty());
}

#[test]
fn artifact_help_is_read_only_and_states_metadata_limitations() {
    let dir = tempfile::tempdir().unwrap();
    for command in [
        "record-artifact",
        "artifact",
        "verify-artifact",
        "ingest-artifact",
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
            .current_dir(dir.path())
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(out.stderr.is_empty());
        let help = String::from_utf8(out.stdout).unwrap();
        assert!(help.contains("does not"));
        assert!(help.contains("bytes"));
        assert!(!dir.path().join("harness.db").exists());
    }
}

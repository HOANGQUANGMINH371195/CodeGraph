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

#[test]
fn verify_artifact_checks_current_bytes_without_persisting_trust_or_exposing_content() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("blobs");
    std::fs::create_dir(&root).unwrap();
    let db = dir.path().join("state.db");
    let task_path = dir.path().join("task.json");
    let run_path = dir.path().join("run.json");
    let artifact_path = dir.path().join("artifact.json");
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
            .arg("--database")
            .arg(&db)
            .args(args)
            .output()
            .unwrap()
    };
    let project = json!({"repository_id":"repo","worktree_id":"w","git_head":"h",
        "working_tree_fingerprint":"s","config_hash":"c","ignore_policy_version":"1"});
    let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let artifact = json!({"schema_version":1,"id":"a1","project":project,"graph_version":"g1",
        "analysis_run":"r1","content_sha256":hash,"byte_length":3,"kind":"stdout",
        "retention":"evidence","declared_protection":"redacted"});
    let run = json!({"schema_version":1,"id":"r1","project":project,"graph_version":"g1",
        "analyzer":"fixture","analyzer_version":"1","configuration_sha256":"a".repeat(64),
        "input_manifest_sha256":"b".repeat(64)});
    let mut task = json!({"schema_version":1,"id":"t1","project":project,"graph_version":"g1",
        "role":"reader","account_lane":"local","scope":["src"],"dependencies":[],
        "context_ref":"ctx","expected_artifacts":["report"],"token_budget":100});
    std::fs::write(&task_path, task.to_string()).unwrap();
    std::fs::write(&run_path, run.to_string()).unwrap();
    std::fs::write(&artifact_path, artifact.to_string()).unwrap();
    parse(invoke(&["record-analysis-run", run_path.to_str().unwrap()]));
    parse(invoke(&[
        "record-artifact",
        artifact_path.to_str().unwrap(),
    ]));
    let args = [
        "verify-artifact",
        "a1",
        task_path.to_str().unwrap(),
        root.to_str().unwrap(),
    ];
    let fail = |out: Output| {
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    };
    fail(invoke(&args)); // Metadata exists, bytes do not.
    let blob = root.join(hash);
    std::fs::write(&blob, b"abc").unwrap();
    let verified = parse(invoke(&args));
    assert_eq!(
        verified,
        json!({"schema_version":1,"artifact":artifact,"content_verified":true,
        "protection_verified":false,"execution_verified":false,"snapshot_binding":"caller_supplied",
        "content_is_untrusted":true,"receipt_persisted":false})
    );
    let mut bounded = args.to_vec();
    bounded.extend(["--max-bytes", "2"]);
    fail(invoke(&bounded));
    for bytes in [b"abd".as_slice(), b"ab", b"abcd"] {
        std::fs::write(&blob, bytes).unwrap();
        fail(invoke(&args));
    }
    // Verification must not upgrade the immutable metadata lookup.
    assert_eq!(
        parse(invoke(&["artifact", "a1", task_path.to_str().unwrap()]))["content_verified"],
        false
    );
    std::fs::write(&blob, b"abc").unwrap();
    task["project"]["working_tree_fingerprint"] = json!("other");
    std::fs::write(&task_path, task.to_string()).unwrap();
    let wrong_scope = invoke(&args);
    assert!(
        String::from_utf8_lossy(&wrong_scope.stderr)
            .contains("artifact not found for this snapshot")
    );
    fail(wrong_scope);
    let events = invoke(&["events"]);
    assert!(events.status.success());
    assert!(events.stdout.is_empty());
}

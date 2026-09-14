use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

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
fn piped_ingestion_retries_content_failure_and_never_exposes_raw_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("blobs");
    std::fs::create_dir(&root).unwrap();
    let database = dir.path().join("state.db");
    let run_path = dir.path().join("run.json");
    let descriptor_path = dir.path().join("artifact.json");
    let task_path = dir.path().join("task.json");
    let project = json!({"repository_id":"repo","worktree_id":"w","git_head":"h",
        "working_tree_fingerprint":"s","config_hash":"c","ignore_policy_version":"1"});
    let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let descriptor = json!({"schema_version":1,"id":"a1","project":project,"graph_version":"g1",
        "analysis_run":"r1","content_sha256":hash,"byte_length":3,"kind":"stdout",
        "retention":"evidence","declared_protection":"redacted"});
    let run = json!({"schema_version":1,"id":"r1","project":project,"graph_version":"g1",
        "analyzer":"fixture","analyzer_version":"1","configuration_sha256":"a".repeat(64),
        "input_manifest_sha256":"b".repeat(64)});
    let task = json!({"schema_version":1,"id":"t1","project":project,"graph_version":"g1",
        "role":"reader","account_lane":"local","scope":["src"],"dependencies":[],
        "context_ref":"ctx","expected_artifacts":["report"],"token_budget":100});
    std::fs::write(&run_path, run.to_string()).unwrap();
    std::fs::write(&descriptor_path, descriptor.to_string()).unwrap();
    std::fs::write(&task_path, task.to_string()).unwrap();
    let invoke = |args: &[&str], bytes: &[u8]| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
            .arg("--database")
            .arg(&database)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(bytes).unwrap();
        child.wait_with_output().unwrap()
    };
    let args = [
        "ingest-artifact",
        descriptor_path.to_str().unwrap(),
        root.to_str().unwrap(),
    ];
    let reject = |out: Output| {
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    };
    reject(invoke(&args, b"abc")); // No run, no blob.
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    parse(invoke(
        &["record-analysis-run", run_path.to_str().unwrap()],
        b"",
    ));
    let mut limited = args.to_vec();
    limited.extend(["--max-bytes", "2"]);
    reject(invoke(&limited, b"abc"));
    assert!(
        parse(invoke(
            &["artifact", "a1", task_path.to_str().unwrap()],
            b""
        ))["artifact"]
            .is_null()
    );
    let failed = invoke(&args, b"abd");
    assert!(String::from_utf8_lossy(&failed.stderr).contains("metadata is registered"));
    reject(failed);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    for blob_inserted in [true, false] {
        assert_eq!(
            parse(invoke(&args, b"abc")),
            json!({"schema_version":1,"artifact":descriptor,
            "metadata_inserted":false,"blob_inserted":blob_inserted,"content_verified":true,
            "protection_verified":false,"execution_verified":false,"snapshot_binding":"caller_supplied",
            "content_is_untrusted":true,"receipt_persisted":false})
        );
    }
    assert_eq!(std::fs::read(root.join(hash)).unwrap(), b"abc");
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(
        parse(invoke(
            &[
                "verify-artifact",
                "a1",
                task_path.to_str().unwrap(),
                root.to_str().unwrap()
            ],
            b""
        ))["content_verified"],
        true
    );
    for bytes in [b"ab".as_slice(), b"abcd"] {
        reject(invoke(&args, bytes));
    }
    let mut conflict = descriptor.clone();
    conflict["kind"] = json!("stderr");
    std::fs::write(&descriptor_path, conflict.to_string()).unwrap();
    reject(invoke(&args, b"abc"));
    assert_eq!(std::fs::read(root.join(hash)).unwrap(), b"abc");
    assert_eq!(
        parse(invoke(
            &["artifact", "a1", task_path.to_str().unwrap()],
            b""
        ))["content_verified"],
        false
    );
    let mut verify = vec![
        "verify-artifact",
        "a1",
        task_path.to_str().unwrap(),
        root.to_str().unwrap(),
        "--observation-id",
        "check-1",
    ];
    assert_eq!(parse(invoke(&verify, b""))["receipt_persisted"], true);
    let query = [
        "artifact-observation",
        "check-1",
        task_path.to_str().unwrap(),
    ];
    let historical = parse(invoke(&query, b""));
    assert_eq!(historical["observation"]["artifact"], descriptor);
    assert_eq!(
        historical["observation"]["verifier_version"],
        "sha256-length-v1"
    );
    assert!(
        historical["observation"]["observed_at_ms"]
            .as_i64()
            .unwrap()
            > 0
    );
    assert_eq!(historical["historical_only"], true);
    assert_eq!(historical["content_verified"], false);
    assert_eq!(historical["execution_verified"], false);
    assert_eq!(historical["protection_verified"], false);
    // A new process reads history without requiring the blob to remain valid.
    std::fs::write(root.join(hash), b"abd").unwrap();
    assert_eq!(parse(invoke(&query, b"")), historical);
    verify[5] = "check-failed";
    reject(invoke(&verify, b""));
    assert!(
        parse(invoke(
            &[
                "artifact-observation",
                "check-failed",
                task_path.to_str().unwrap()
            ],
            b""
        ))["observation"]
            .is_null()
    );
    verify[5] = "   ";
    reject(invoke(&verify, b""));
    for field in [
        "repository_id",
        "worktree_id",
        "git_head",
        "working_tree_fingerprint",
        "config_hash",
        "ignore_policy_version",
    ] {
        let mut other = task.clone();
        other["project"][field] = json!("different");
        std::fs::write(&task_path, other.to_string()).unwrap();
        assert!(parse(invoke(&query, b""))["observation"].is_null());
    }
    let mut other = task.clone();
    other["graph_version"] = json!("different");
    std::fs::write(&task_path, other.to_string()).unwrap();
    assert!(parse(invoke(&query, b""))["observation"].is_null());
}

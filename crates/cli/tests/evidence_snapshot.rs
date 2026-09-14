use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use graph_source::GitSnapshotAuthority;
use serde_json::{Value, json};

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn invoke(database: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .arg("--database")
        .arg(database)
        .args(args)
        .output()
        .unwrap()
}

fn project_value(project: &graph_domain::ProjectRef) -> Value {
    json!({
        "repository_id": project.repository_id,
        "worktree_id": project.worktree_id,
        "git_head": project.git_head,
        "working_tree_fingerprint": project.working_tree_fingerprint,
        "config_hash": project.config_hash,
        "ignore_policy_version": project.ignore_policy_version,
    })
}

fn task_value(project: &Value) -> Value {
    json!({
        "schema_version": 1,
        "id": "task",
        "project": project,
        "graph_version": "graph-v1",
        "role": "reader",
        "account_lane": "local",
        "scope": ["src"],
        "dependencies": [],
        "context_ref": "ctx",
        "expected_artifacts": ["report"],
        "token_budget": 100,
    })
}

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

#[test]
fn opt_in_snapshot_config_binds_git_and_requires_registered_analysis_run() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    fs::create_dir_all(root.join("src")).unwrap();
    git(&root, &["init", "--quiet"]);
    git(&root, &["config", "user.name", "cli-snapshot-test"]);
    git(
        &root,
        &["config", "user.email", "cli-snapshot-test@example.invalid"],
    );
    fs::write(root.join("src/main.rs"), "abc").unwrap();
    git(&root, &["add", "src/main.rs"]);
    git(&root, &["commit", "--quiet", "-m", "initial"]);

    let authority = GitSnapshotAuthority::new(
        &root,
        "repository-fixture".into(),
        "worktree-fixture".into(),
        "config-fixture".into(),
        "ignore-v1".into(),
        1024,
    )
    .unwrap();
    let project = authority.current_project().unwrap();
    let project_json = project_value(&project);
    let task_path = directory.path().join("task.json");
    write_json(&task_path, &task_value(&project_json));
    let evidence_path = directory.path().join("evidence.json");
    write_json(
        &evidence_path,
        &json!({
            "schema_version": 1,
            "id": "e1",
            "project": project_json,
            "graph_version": "graph-v1",
            "path": "src/main.rs",
            "content_sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "start_line": 1,
            "end_line": 1,
            "analysis_run": "run1",
        }),
    );
    let run_path = directory.path().join("run.json");
    write_json(
        &run_path,
        &json!({
            "schema_version": 1,
            "id": "run1",
            "project": project_value(&project),
            "graph_version": "graph-v1",
            "analyzer": "fixture",
            "analyzer_version": "1",
            "configuration_sha256": "a".repeat(64),
            "input_manifest_sha256": "b".repeat(64),
        }),
    );
    let config_path = directory.path().join("snapshot.json");
    write_json(
        &config_path,
        &json!({
            "schema_version": 1,
            "root": root,
            "repository_id": "repository-fixture",
            "worktree_id": "worktree-fixture",
            "config_hash": "config-fixture",
            "ignore_policy_version": "ignore-v1",
            "max_total_bytes": 1024,
            "command_timeout_ms": 5000,
        }),
    );
    let database = directory.path().join("state.db");
    let evidence_arg = evidence_path.to_str().unwrap();
    let task_arg = task_path.to_str().unwrap();
    let root_arg = root.to_str().unwrap();
    let config_arg = config_path.to_str().unwrap();
    let run_arg = run_path.to_str().unwrap();

    let recorded = invoke(&database, &["record-evidence", evidence_arg]);
    assert!(
        recorded.status.success(),
        "{}",
        String::from_utf8_lossy(&recorded.stderr)
    );
    let legacy = invoke(&database, &["verify-evidence", "e1", task_arg, root_arg]);
    assert!(
        legacy.status.success(),
        "{}",
        String::from_utf8_lossy(&legacy.stderr)
    );
    let legacy_json: Value = serde_json::from_slice(&legacy.stdout).unwrap();
    assert_eq!(legacy_json["snapshot_binding"], "caller_supplied");
    assert_eq!(legacy_json["analysis_run_verified"], false);

    let missing_run = invoke(
        &database,
        &[
            "verify-evidence",
            "e1",
            task_arg,
            root_arg,
            "--snapshot-config",
            config_arg,
        ],
    );
    assert!(!missing_run.status.success());
    assert!(missing_run.stdout.is_empty());

    let unavailable_root = directory.path().join("not-created");
    let rejected_before_source = invoke(
        &database,
        &[
            "verify-evidence",
            "e1",
            task_arg,
            unavailable_root.to_str().unwrap(),
            "--snapshot-config",
            config_arg,
        ],
    );
    assert!(!rejected_before_source.status.success());
    assert!(rejected_before_source.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&rejected_before_source.stderr)
            .contains("analysis run unavailable before source materialization")
    );

    let recorded_run = invoke(&database, &["record-analysis-run", run_arg]);
    assert!(
        recorded_run.status.success(),
        "{}",
        String::from_utf8_lossy(&recorded_run.stderr)
    );
    let bound = invoke(
        &database,
        &[
            "verify-evidence",
            "e1",
            task_arg,
            root_arg,
            "--snapshot-config",
            config_arg,
        ],
    );
    assert!(
        bound.status.success(),
        "{}",
        String::from_utf8_lossy(&bound.stderr)
    );
    let bound_json: Value = serde_json::from_slice(&bound.stdout).unwrap();
    assert_eq!(bound_json["source"], "abc");
    assert_eq!(bound_json["snapshot_binding"], "git_authority");
    assert!(
        bound_json["snapshot_binding_id"]
            .as_str()
            .unwrap()
            .starts_with("git:")
    );
    assert_eq!(bound_json["analysis_run_verified"], true);
    assert_eq!(bound_json["analysis_run"], "run1");
    assert_eq!(bound_json["relationship_verified"], false);

    let mut invalid_config =
        serde_json::from_slice::<Value>(&fs::read(&config_path).unwrap()).unwrap();
    invalid_config["unexpected"] = json!(true);
    write_json(&config_path, &invalid_config);
    let rejected_config = invoke(
        &database,
        &[
            "verify-evidence",
            "e1",
            task_arg,
            root_arg,
            "--snapshot-config",
            config_arg,
        ],
    );
    assert!(!rejected_config.status.success());
    assert!(rejected_config.stdout.is_empty());

    write_json(
        &config_path,
        &json!({
            "schema_version": 1,
            "root": root,
            "repository_id": "repository-fixture",
            "worktree_id": "worktree-fixture",
            "config_hash": "config-fixture",
            "ignore_policy_version": "ignore-v1",
            "max_total_bytes": 1024,
            "command_timeout_ms": 5000,
        }),
    );
    let valid_config = serde_json::from_slice::<Value>(&fs::read(&config_path).unwrap()).unwrap();
    for (field, value) in [
        ("max_total_bytes", json!(0)),
        ("command_timeout_ms", json!(60_001)),
        ("root", json!(directory.path())),
    ] {
        let mut invalid = valid_config.clone();
        invalid[field] = value;
        write_json(&config_path, &invalid);
        let rejected_limits = invoke(
            &database,
            &[
                "verify-evidence",
                "e1",
                task_arg,
                root_arg,
                "--snapshot-config",
                config_arg,
            ],
        );
        assert!(!rejected_limits.status.success(), "{field}");
        assert!(rejected_limits.stdout.is_empty(), "{field}");
    }

    fs::write(root.join("src/main.rs"), "changed").unwrap();
    let stale = invoke(
        &database,
        &[
            "verify-evidence",
            "e1",
            task_arg,
            root_arg,
            "--snapshot-config",
            config_arg,
        ],
    );
    assert!(!stale.status.success());
    assert!(stale.stdout.is_empty());
}

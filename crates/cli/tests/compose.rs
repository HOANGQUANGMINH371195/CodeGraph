use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const ORDERS_COMPOSE: &[u8] = include_bytes!("../../../fixtures/orders/compose.yaml");
const ORDERS_HASH: &str = "a2a29afb353a588cc28759f281819f5cfda82fd5044f8fd5e38247959fa37701";
const MALFORMED_HASH: &str = "3dd52bb536076fca8573dd4d021e390b38d16d27fc2d5c204527d344d6a6e4f3";
const SECRET_HASH: &str = "d75c56e96b73490261bda5d8ae789a975cbe3031444064a93431e1d4796213e2";

struct Fixture {
    _dir: tempfile::TempDir,
    database: PathBuf,
    root: PathBuf,
    task_path: PathBuf,
    citation: Value,
    task: Value,
}

fn project(snapshot: &str) -> Value {
    json!({
        "repository_id":"repo",
        "worktree_id":"main",
        "git_head":"head",
        "working_tree_fingerprint":snapshot,
        "config_hash":"cfg",
        "ignore_policy_version":"1"
    })
}

fn task(project: &Value) -> Value {
    json!({
        "schema_version":1,
        "id":"task",
        "project":project,
        "graph_version":"g1",
        "role":"reader",
        "account_lane":"local",
        "scope":["src"],
        "dependencies":[],
        "context_ref":"ctx",
        "expected_artifacts":["report"],
        "token_budget":100
    })
}

fn fixture(source: &[u8], hash: &str, path: &str, start_line: u32, end_line: u32) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.db");
    let root = dir.path().join("root");
    fs::create_dir_all(&root).unwrap();
    write_source(&root, path, source);

    let project = project("snapshot");
    let citation = json!({
        "schema_version":1,
        "id":"e1",
        "project":project,
        "graph_version":"g1",
        "path":path,
        "content_sha256":hash,
        "start_line":start_line,
        "end_line":end_line,
        "analysis_run":"run1"
    });
    let task = task(&project);
    let citation_path = dir.path().join("citation.json");
    let task_path = dir.path().join("task.json");
    write_json(&citation_path, &citation);
    write_json(&task_path, &task);

    let recorded = invoke(
        &database,
        ["record-evidence", citation_path.to_str().unwrap()],
    );
    assert_eq!(
        success_json(recorded),
        json!({"inserted":true,"verified":false})
    );

    Fixture {
        _dir: dir,
        database,
        root,
        task_path,
        citation,
        task,
    }
}

fn orders_fixture() -> Fixture {
    fixture(ORDERS_COMPOSE, ORDERS_HASH, "compose.yaml", 1, 20)
}

fn write_source(root: &Path, relative_path: &str, source: &[u8]) {
    let path = root.join(relative_path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn invoke<I, S>(database: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .arg("--database")
        .arg(database)
        .args(args)
        .output()
        .unwrap()
}

fn success_json(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn rejected(output: Output) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

fn rejected_with_message(output: Output, message: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    rejected(output);
    assert!(
        stderr.contains(message),
        "missing {message:?} in {stderr:?}"
    );
}

fn analyze(fixture: &Fixture) -> Output {
    invoke(
        &fixture.database,
        [
            "analyze-compose",
            "e1",
            fixture.task_path.to_str().unwrap(),
            fixture.root.to_str().unwrap(),
        ],
    )
}

#[test]
fn orders_fixture_cli_returns_declared_candidate_projection_without_source_values() {
    let fixture = orders_fixture();
    let output = analyze(&fixture);
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let projection = success_json(output);

    let mut keys: Vec<_> = projection.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "analysis_run_verified",
            "candidate_only",
            "content_hash_and_lines_verified",
            "edges",
            "evidence",
            "kind",
            "nodes",
            "persisted",
            "provenance",
            "relationship_verified",
            "schema_version",
            "snapshot_binding",
            "source_is_untrusted",
            "unknowns",
        ]
    );
    assert_eq!(projection["schema_version"], 1);
    assert_eq!(projection["kind"], "compose_projection");
    assert_eq!(projection["provenance"], "iac_declared");
    assert_eq!(projection["evidence"], fixture.citation);
    assert_eq!(projection["candidate_only"], true);
    assert_eq!(projection["persisted"], false);
    assert_eq!(projection["source_is_untrusted"], true);
    assert_eq!(projection["content_hash_and_lines_verified"], true);
    assert_eq!(projection["snapshot_binding"], "caller_supplied");
    assert_eq!(projection["analysis_run_verified"], false);
    assert_eq!(projection["relationship_verified"], false);

    let nodes = projection["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 4);
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node["kind"] == "service")
            .count(),
        2
    );
    assert_eq!(
        nodes.iter().filter(|node| node["kind"] == "volume").count(),
        2
    );
    assert!(nodes.iter().all(|node| {
        node.as_object().unwrap().keys().collect::<Vec<_>>()
            == ["evidence", "id", "kind", "name"]
                .iter()
                .collect::<Vec<_>>()
    }));
    for name in [
        "orders",
        "notifications",
        "orders-data",
        "notification-data",
    ] {
        assert!(nodes.iter().any(|node| node["name"] == name));
    }

    let edges = projection["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().all(|edge| edge["kind"] == "mounts"));
    assert!(edges.iter().all(|edge| edge["mount_target"] == "/data"));
    assert!(edges.iter().all(|edge| {
        edge.as_object().unwrap().keys().collect::<Vec<_>>()
            == ["evidence", "kind", "mount_target", "source", "target"]
                .iter()
                .collect::<Vec<_>>()
    }));
    let node_ids: Vec<_> = nodes
        .iter()
        .map(|node| node["id"].as_str().unwrap())
        .collect();
    assert!(edges.iter().all(|edge| {
        node_ids.contains(&edge["source"].as_str().unwrap())
            && node_ids.contains(&edge["target"].as_str().unwrap())
    }));

    let unknowns = projection["unknowns"].as_array().unwrap();
    assert!(!unknowns.is_empty());
    assert!(unknowns.iter().all(|unknown| {
        unknown.as_object().unwrap().keys().collect::<Vec<_>>()
            == ["line", "reason"].iter().collect::<Vec<_>>()
    }));
    for reason in ["build", "environment", "ports"] {
        assert!(
            unknowns
                .iter()
                .any(|unknown| { unknown["reason"].as_str().unwrap().contains(reason) })
        );
    }

    for forbidden in [
        "\"DB_PATH\"",
        "/data/orders.db",
        "0.0.0.0",
        "127.0.0.1",
        "\"SERVICE\"",
        "\"BIND\"",
    ] {
        assert!(
            !stdout.contains(forbidden),
            "leaked source value {forbidden}"
        );
    }

    let mut derived_ids = Vec::new();
    derived_ids.extend(
        nodes
            .iter()
            .map(|node| node["evidence"]["id"].as_str().unwrap().to_owned()),
    );
    derived_ids.extend(
        edges
            .iter()
            .map(|edge| edge["evidence"]["id"].as_str().unwrap().to_owned()),
    );
    derived_ids.sort();
    derived_ids.dedup();
    for id in derived_ids {
        let query = invoke(
            &fixture.database,
            ["evidence", &id, fixture.task_path.to_str().unwrap()],
        );
        assert_eq!(
            success_json(query),
            Value::Null,
            "derived evidence was persisted"
        );
    }
    assert_eq!(
        success_json(invoke(
            &fixture.database,
            ["evidence", "e1", fixture.task_path.to_str().unwrap()],
        )),
        fixture.citation
    );
}

#[test]
fn output_budget_is_exact_including_newline_and_caps_are_rejected() {
    let fixture = orders_fixture();
    let baseline = analyze(&fixture);
    let baseline_bytes = baseline.stdout.clone();
    let _ = success_json(baseline);

    let exact = baseline_bytes.len().to_string();
    let exact_output = invoke(
        &fixture.database,
        [
            "analyze-compose",
            "e1",
            fixture.task_path.to_str().unwrap(),
            fixture.root.to_str().unwrap(),
            "--max-output-bytes",
            &exact,
        ],
    );
    assert!(exact_output.status.success());
    assert!(exact_output.stderr.is_empty());
    assert_eq!(exact_output.stdout, baseline_bytes);

    let under = (exact.parse::<usize>().unwrap() - 1).to_string();
    rejected(invoke(
        &fixture.database,
        [
            "analyze-compose",
            "e1",
            fixture.task_path.to_str().unwrap(),
            fixture.root.to_str().unwrap(),
            "--max-output-bytes",
            &under,
        ],
    ));
    rejected(invoke(
        &fixture.database,
        [
            "analyze-compose",
            "e1",
            fixture.task_path.to_str().unwrap(),
            fixture.root.to_str().unwrap(),
            "--max-output-bytes",
            "16777217",
        ],
    ));
}

#[test]
fn source_budget_is_enforced_at_one_mib_cli_cap() {
    let fixture = orders_fixture();
    let exact_source_bytes = ORDERS_COMPOSE.len();
    assert_eq!(exact_source_bytes, 457);
    let exact_source_limit = exact_source_bytes.to_string();
    assert_eq!(
        success_json(invoke(
            &fixture.database,
            [
                "analyze-compose",
                "e1",
                fixture.task_path.to_str().unwrap(),
                fixture.root.to_str().unwrap(),
                "--max-source-bytes",
                &exact_source_limit,
            ],
        ))["kind"],
        "compose_projection"
    );

    let below_source_limit = (exact_source_bytes - 1).to_string();
    rejected_with_message(
        invoke(
            &fixture.database,
            [
                "analyze-compose",
                "e1",
                fixture.task_path.to_str().unwrap(),
                fixture.root.to_str().unwrap(),
                "--max-source-bytes",
                &below_source_limit,
            ],
        ),
        "source file exceeds the read byte budget",
    );
    rejected(invoke(
        &fixture.database,
        [
            "analyze-compose",
            "e1",
            fixture.task_path.to_str().unwrap(),
            fixture.root.to_str().unwrap(),
            "--max-source-bytes",
            "1048577",
        ],
    ));
}

#[test]
fn stale_hash_snapshot_mismatch_malformed_and_partial_source_fail_closed() {
    let stale = orders_fixture();
    fs::write(stale.root.join("compose.yaml"), b"services:\n  stale: {}\n").unwrap();
    rejected_with_message(analyze(&stale), "SHA-256");

    let snapshot = orders_fixture();
    let mut changed_task = snapshot.task.clone();
    changed_task["project"]["working_tree_fingerprint"] = json!("changed");
    write_json(&snapshot.task_path, &changed_task);
    rejected_with_message(analyze(&snapshot), "evidence not found for this snapshot");

    let malformed = fixture(
        b"services:\n  api: [\n",
        MALFORMED_HASH,
        "compose.yaml",
        1,
        2,
    );
    let verified_malformed = success_json(invoke(
        &malformed.database,
        [
            "verify-evidence",
            "e1",
            malformed.task_path.to_str().unwrap(),
            malformed.root.to_str().unwrap(),
        ],
    ));
    assert_eq!(verified_malformed["source"], "services:\n  api: [\n");
    assert_eq!(verified_malformed["content_hash_and_lines_verified"], true);
    rejected_with_message(analyze(&malformed), "invalid Compose YAML");

    let partial = fixture(ORDERS_COMPOSE, ORDERS_HASH, "compose.yaml", 2, 20);
    rejected_with_message(analyze(&partial), "full-file citation starting at line one");
}

#[cfg(unix)]
#[test]
fn source_root_symlink_escape_is_rejected() {
    use std::os::unix::fs::symlink;

    let fixture = fixture(ORDERS_COMPOSE, ORDERS_HASH, "nested/compose.yaml", 1, 20);
    let outside = tempfile::tempdir().unwrap();
    write_source(outside.path(), "compose.yaml", ORDERS_COMPOSE);
    fs::remove_dir_all(fixture.root.join("nested")).unwrap();
    symlink(outside.path(), fixture.root.join("nested")).unwrap();
    rejected(analyze(&fixture));
}

#[test]
fn unknown_environment_values_are_not_serialized() {
    let fixture = fixture(
        b"services:\n  api:\n    environment:\n      API_TOKEN: super-secret-value\n",
        SECRET_HASH,
        "compose.yaml",
        1,
        4,
    );
    let output = analyze(&fixture);
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let projection = success_json(output);
    assert!(
        projection["unknowns"]
            .as_array()
            .unwrap()
            .iter()
            .any(|unknown| unknown["reason"] == "environment is not modeled")
    );
    assert!(!stdout.contains("API_TOKEN"));
    assert!(!stdout.contains("super-secret-value"));
}

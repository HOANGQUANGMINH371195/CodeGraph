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

struct Fixture {
    _dir: tempfile::TempDir,
    database: PathBuf,
    root: PathBuf,
    task_path: PathBuf,
    citation: Value,
}

fn project(snapshot: &str) -> Value {
    json!({
        "repository_id": "repo",
        "worktree_id": "main",
        "git_head": "head",
        "working_tree_fingerprint": snapshot,
        "config_hash": "cfg",
        "ignore_policy_version": "1"
    })
}

fn task(project: &Value) -> Value {
    json!({
        "schema_version": 1,
        "id": "task",
        "project": project,
        "graph_version": "g1",
        "role": "reader",
        "account_lane": "local",
        "scope": ["src"],
        "dependencies": [],
        "context_ref": "ctx",
        "expected_artifacts": ["report"],
        "token_budget": 100
    })
}

fn fixture(source: &[u8], hash: &str) -> Fixture {
    fixture_with_range(source, hash, 20)
}

fn fixture_with_range(source: &[u8], hash: &str, end_line: u32) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.db");
    let root = dir.path().join("root");
    fs::create_dir_all(&root).unwrap();
    write_source(&root, "compose.yaml", source);

    let project = project("snapshot");
    let citation = json!({
        "schema_version": 1,
        "id": "e1",
        "project": project,
        "graph_version": "g1",
        "path": "compose.yaml",
        "content_sha256": hash,
        "start_line": 1,
        "end_line": end_line,
        "analysis_run": "run1"
    });
    let task = task(&project);
    let citation_path = dir.path().join("citation.json");
    let task_path = dir.path().join("task.json");
    write_json(&citation_path, &citation);
    write_json(&task_path, &task);

    assert_eq!(
        success_json(invoke(
            &database,
            ["record-evidence", citation_path.to_str().unwrap()]
        )),
        json!({"inserted": true, "verified": false})
    );

    Fixture {
        _dir: dir,
        database,
        root,
        task_path,
        citation,
    }
}

fn orders_fixture() -> Fixture {
    fixture(ORDERS_COMPOSE, ORDERS_HASH)
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

fn publish(fixture: &Fixture, expected_generation: u64, extra: &[String]) -> Output {
    let mut args = vec![
        "publish-compose".to_owned(),
        "e1".to_owned(),
        fixture.task_path.to_str().unwrap().to_owned(),
        fixture.root.to_str().unwrap().to_owned(),
        "--expected-generation".to_owned(),
        expected_generation.to_string(),
    ];
    args.extend(extra.iter().cloned());
    invoke(&fixture.database, args)
}

fn deployment(fixture: &Fixture) -> Output {
    invoke(
        &fixture.database,
        ["deployment", "e1", fixture.task_path.to_str().unwrap()],
    )
}

fn invalidate(fixture: &Fixture, expected_generation: u64) -> Output {
    invalidate_with_extra(fixture, expected_generation, &[])
}

fn invalidate_with_extra(fixture: &Fixture, expected_generation: u64, extra: &[String]) -> Output {
    let mut args = vec![
        "invalidate-compose".to_owned(),
        "e1".to_owned(),
        fixture.task_path.to_str().unwrap().to_owned(),
        "--expected-generation".to_owned(),
        expected_generation.to_string(),
    ];
    args.extend(extra.iter().cloned());
    invoke(&fixture.database, args)
}

fn deployment_with_extra(fixture: &Fixture, extra: &[String]) -> Output {
    let mut args = vec![
        "deployment".to_owned(),
        "e1".to_owned(),
        fixture.task_path.to_str().unwrap().to_owned(),
    ];
    args.extend(extra.iter().cloned());
    invoke(&fixture.database, args)
}

fn mutation(operation: &str, generation: u64, source_verified: bool) -> Value {
    json!({
        "schema_version": 1,
        "kind": "deployment_mutation",
        "operation": operation,
        "generation": generation,
        "persisted": true,
        "source_bytes_verified_during_operation": source_verified,
        "snapshot_binding": "caller_supplied",
        "relationship_verified": false
    })
}

fn assert_snapshot_metadata(snapshot: &Value, generation: u64) {
    assert_eq!(
        snapshot,
        &json!({
            "schema_version": 1,
            "kind": "deployment_snapshot",
            "generation": generation,
            "historical": true,
            "source_bytes_verified": false,
            "relationship_verified": false,
            "analysis_run_verified": false,
            "snapshot_binding": "caller_supplied",
            "provenance": "iac_declared",
            "graph": snapshot["graph"]
        })
    );
}

fn assert_historical_graph(snapshot: &Value, fixture: &Fixture, generation: u64) {
    assert_snapshot_metadata(snapshot, generation);
    assert_eq!(snapshot["schema_version"], 1);
    assert_eq!(snapshot["kind"], "deployment_snapshot");
    assert_eq!(snapshot["generation"], generation);
    assert_eq!(snapshot["historical"], true);
    assert_eq!(snapshot["source_bytes_verified"], false);
    assert_eq!(snapshot["relationship_verified"], false);
    assert_eq!(snapshot["analysis_run_verified"], false);
    assert_eq!(snapshot["snapshot_binding"], "caller_supplied");
    assert_eq!(snapshot["provenance"], "iac_declared");

    let graph = &snapshot["graph"];
    assert_eq!(graph["adapter"], "compose");
    assert_eq!(graph["adapter_version"], "1");
    assert_eq!(graph["evidence"], fixture.citation);

    let nodes = graph["nodes"].as_array().unwrap();
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
        node["evidence"]["project"] == fixture.citation["project"]
            && node["evidence"]["graph_version"] == fixture.citation["graph_version"]
            && node["evidence"]["path"] == fixture.citation["path"]
            && node["evidence"]["content_sha256"] == fixture.citation["content_sha256"]
            && node["evidence"]["analysis_run"] == fixture.citation["analysis_run"]
            && node["evidence"]["start_line"] == node["evidence"]["end_line"]
    }));
    for name in [
        "orders",
        "notifications",
        "orders-data",
        "notification-data",
    ] {
        assert!(nodes.iter().any(|node| node["name"] == name));
    }
    for (name, line) in [
        ("orders", 2),
        ("notifications", 11),
        ("orders-data", 19),
        ("notification-data", 20),
    ] {
        let node = nodes.iter().find(|node| node["name"] == name).unwrap();
        assert_eq!(node["evidence"]["start_line"], line);
        assert_ne!(node["evidence"]["id"], fixture.citation["id"]);
    }

    let edges = graph["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().all(|edge| edge["kind"] == "mounts"));
    assert!(edges.iter().all(|edge| edge["mount_target"] == "/data"));
    assert!(edges.iter().all(|edge| {
        edge["evidence"]["project"] == fixture.citation["project"]
            && edge["evidence"]["graph_version"] == fixture.citation["graph_version"]
            && edge["evidence"]["path"] == fixture.citation["path"]
            && edge["evidence"]["content_sha256"] == fixture.citation["content_sha256"]
            && edge["evidence"]["analysis_run"] == fixture.citation["analysis_run"]
            && edge["evidence"]["start_line"] == edge["evidence"]["end_line"]
    }));
    for edge in edges {
        let owner = nodes
            .iter()
            .find(|node| node["id"] == edge["source"])
            .unwrap();
        let target = nodes
            .iter()
            .find(|node| node["id"] == edge["target"])
            .unwrap();
        let (line, target_name) = if owner["name"] == "orders" {
            (10, "orders-data")
        } else {
            (17, "notification-data")
        };
        assert_eq!(edge["evidence"]["start_line"], line);
        assert_eq!(target["name"], target_name);
        assert_ne!(edge["evidence"]["id"], fixture.citation["id"]);
    }

    let unknowns = graph["unknowns"].as_array().unwrap();
    assert!(!unknowns.is_empty());
    for reason in ["build", "environment", "ports"] {
        assert!(
            unknowns
                .iter()
                .any(|unknown| unknown["reason"].as_str().unwrap().contains(reason))
        );
    }
}

#[test]
fn orders_cli_lifecycle_is_publish_query_invalidate_republish_across_processes() {
    let fixture = orders_fixture();

    let candidate = success_json(analyze(&fixture));
    assert_eq!(candidate["kind"], "compose_projection");
    assert_eq!(candidate["candidate_only"], true);
    assert_eq!(candidate["persisted"], false);
    assert_eq!(success_json(deployment(&fixture)), Value::Null);

    assert_eq!(
        success_json(publish(&fixture, 0, &[])),
        mutation("publish", 1, true)
    );
    let published = success_json(deployment(&fixture));
    assert_historical_graph(&published, &fixture, 1);

    assert_eq!(
        success_json(invalidate(&fixture, 1)),
        mutation("invalidate", 2, false)
    );
    let invalidated = success_json(deployment(&fixture));
    assert_eq!(invalidated["graph"], Value::Null);
    assert_snapshot_metadata(&invalidated, 2);

    assert_eq!(
        success_json(publish(&fixture, 2, &[])),
        mutation("publish", 3, true)
    );
    assert_historical_graph(&success_json(deployment(&fixture)), &fixture, 3);
}

#[test]
fn published_graph_is_historical_after_source_root_deleted() {
    let fixture = orders_fixture();
    assert_eq!(
        success_json(publish(&fixture, 0, &[])),
        mutation("publish", 1, true)
    );
    fs::remove_dir_all(&fixture.root).unwrap();

    assert_historical_graph(&success_json(deployment(&fixture)), &fixture, 1);
}

#[test]
fn absent_owner_query_returns_null_without_source_root() {
    let fixture = orders_fixture();
    fs::remove_dir_all(&fixture.root).unwrap();

    assert_eq!(success_json(deployment(&fixture)), Value::Null);
}

#[test]
fn stale_generations_do_not_mutate_existing_snapshot() {
    let fixture = orders_fixture();
    assert_eq!(
        success_json(publish(&fixture, 0, &[])),
        mutation("publish", 1, true)
    );
    let before = success_json(deployment(&fixture));

    rejected_with_message(
        publish(&fixture, 0, &[]),
        "deployment generation or immutable citation conflict",
    );
    assert_eq!(success_json(deployment(&fixture)), before);

    rejected_with_message(
        invalidate(&fixture, 0),
        "deployment generation or immutable citation conflict",
    );
    assert_eq!(success_json(deployment(&fixture)), before);
}

#[test]
fn mutation_output_budget_counts_exact_newline_and_rejects_before_write() {
    let baseline = orders_fixture();
    let baseline_output = publish(&baseline, 0, &[]);
    let expected_bytes = baseline_output.stdout.clone();
    assert_eq!(success_json(baseline_output), mutation("publish", 1, true));

    let exact = orders_fixture();
    let exact_limit = expected_bytes.len().to_string();
    let exact_output = publish(&exact, 0, &["--max-output-bytes".into(), exact_limit]);
    assert!(exact_output.status.success());
    assert!(exact_output.stderr.is_empty());
    assert_eq!(exact_output.stdout, expected_bytes);
    assert_historical_graph(&success_json(deployment(&exact)), &exact, 1);

    let under = orders_fixture();
    let under_limit = (expected_bytes.len() - 1).to_string();
    rejected(publish(
        &under,
        0,
        &["--max-output-bytes".into(), under_limit],
    ));
    assert_eq!(success_json(deployment(&under)), Value::Null);

    let invalidated = orders_fixture();
    assert_eq!(
        success_json(publish(&invalidated, 0, &[])),
        mutation("publish", 1, true)
    );
    let before_invalidation = success_json(deployment(&invalidated));
    rejected(invalidate_with_extra(
        &invalidated,
        1,
        &["--max-output-bytes".into(), "0".into()],
    ));
    assert_eq!(success_json(deployment(&invalidated)), before_invalidation);

    let query_budget = orders_fixture();
    assert_eq!(
        success_json(publish(&query_budget, 0, &[])),
        mutation("publish", 1, true)
    );
    let query_output = deployment(&query_budget);
    let query_bytes = query_output.stdout.clone();
    let query_value = success_json(query_output);
    let exact_query = deployment_with_extra(
        &query_budget,
        &["--max-output-bytes".into(), query_bytes.len().to_string()],
    );
    assert!(exact_query.status.success());
    assert!(exact_query.stderr.is_empty());
    assert_eq!(exact_query.stdout, query_bytes);
    rejected(deployment_with_extra(
        &query_budget,
        &[
            "--max-output-bytes".into(),
            (query_bytes.len() - 1).to_string(),
        ],
    ));
    assert_eq!(success_json(deployment(&query_budget)), query_value);
}

#[test]
fn source_budget_rejects_before_publish_mutation() {
    let fixture = orders_fixture();
    let source_limit = (ORDERS_COMPOSE.len() - 1).to_string();

    rejected_with_message(
        publish(&fixture, 0, &["--max-source-bytes".into(), source_limit]),
        "source file exceeds the read byte budget",
    );
    assert_eq!(success_json(deployment(&fixture)), Value::Null);
}

#[test]
fn source_failures_and_mixed_snapshot_leave_previous_state_unchanged() {
    let retained = orders_fixture();
    assert_eq!(
        success_json(publish(&retained, 0, &[])),
        mutation("publish", 1, true)
    );
    let before = success_json(deployment(&retained));
    fs::write(
        retained.root.join("compose.yaml"),
        b"services:\n  changed: {}\n",
    )
    .unwrap();
    rejected_with_message(publish(&retained, 1, &[]), "SHA-256");
    assert_eq!(success_json(deployment(&retained)), before);

    let malformed = fixture_with_range(b"services:\n  api: [\n", MALFORMED_HASH, 2);
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
    rejected_with_message(publish(&malformed, 0, &[]), "invalid Compose YAML");
    assert_eq!(success_json(deployment(&malformed)), Value::Null);

    let mixed = orders_fixture();
    let original_task = fs::read(&mixed.task_path).unwrap();
    let mut changed_task: Value = serde_json::from_slice(&original_task).unwrap();
    changed_task["project"]["working_tree_fingerprint"] = json!("changed");
    write_json(&mixed.task_path, &changed_task);
    rejected_with_message(
        publish(&mixed, 0, &[]),
        "evidence not found for this snapshot",
    );
    write_json(
        &mixed.task_path,
        &serde_json::from_slice(&original_task).unwrap(),
    );
    assert_eq!(success_json(deployment(&mixed)), Value::Null);
}

#[test]
fn analyze_output_is_candidate_only_and_does_not_persist() {
    let fixture = orders_fixture();
    let output = analyze(&fixture);
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let candidate = success_json(output);
    assert_eq!(candidate["candidate_only"], true);
    assert_eq!(candidate["persisted"], false);
    assert_eq!(candidate["source_is_untrusted"], true);
    assert_eq!(candidate["relationship_verified"], false);
    assert_eq!(candidate["analysis_run_verified"], false);
    assert!(!stdout.contains("deployment_mutation"));
    assert_eq!(success_json(deployment(&fixture)), Value::Null);
}

#[test]
fn invalid_deployment_task_input_is_static_and_nonmutating() {
    let fixture = orders_fixture();
    let secret = "do-not-echo-this-task-secret";
    fs::write(&fixture.task_path, format!("{{\"secret\":\"{secret}\"}}\n")).unwrap();

    let output = invoke(
        &fixture.database,
        [
            "analyze-compose",
            "e1",
            fixture.task_path.to_str().unwrap(),
            fixture.root.to_str().unwrap(),
        ],
    );
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    rejected_with_message(output, "invalid deployment task input");
    assert!(!stderr.contains(secret));
}

#[test]
fn bounded_context_uses_only_stored_relationships_and_deduplicated_citations() {
    let fixture = orders_fixture();
    let mut source = ORDERS_COMPOSE.to_vec();
    source.extend_from_slice(
        "# irrelevant long source padding; never context instructions\n"
            .repeat(4000)
            .as_bytes(),
    );
    fs::write(fixture.root.join("compose.yaml"), &source).unwrap();
    success_json(invoke(
        &fixture.database,
        [
            "reindex-compose",
            "e1",
            fixture.task_path.to_str().unwrap(),
            fixture.root.to_str().unwrap(),
            "--analysis-run",
            "long-context",
            "--expected-generation",
            "0",
        ],
    ));
    let saved = success_json(deployment(&fixture));
    let seed = saved["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "orders")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    fs::remove_dir_all(&fixture.root).unwrap();
    let call = |extra: &[&str]| {
        let mut args = vec![
            "deployment-context",
            "e1",
            fixture.task_path.to_str().unwrap(),
            "--node",
            seed,
        ];
        args.extend_from_slice(extra);
        invoke(&fixture.database, args)
    };
    let result = call(&[]);
    let bytes = result.stdout.clone();
    let pack = success_json(result);
    assert!(bytes.len() < source.len() / 5);
    assert!(!String::from_utf8_lossy(&bytes).contains("irrelevant long source padding"));
    assert_eq!(pack["kind"], "deployment_context");
    for field in ["historical", "source_is_untrusted"] {
        assert_eq!(pack[field], true);
    }
    for field in [
        "source_bytes_verified",
        "relationship_verified",
        "depth_limited",
        "budget_limited",
    ] {
        assert_eq!(pack[field], false);
    }
    assert_eq!(pack["provenance"], "iac_declared");
    assert_eq!(pack["generation"], 1);
    assert_eq!(pack["total_nodes"], 4);
    assert_eq!(pack["omitted_nodes"], 2);
    assert_eq!(pack["total_edges"], 2);
    assert_eq!(pack["omitted_edges"], 1);
    assert!(pack["unknown_count"].as_u64().unwrap() > 0);
    let nodes = pack["nodes"].as_array().unwrap();
    let edges = pack["edges"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["source"], seed);
    assert_eq!(edges[0]["kind"], "mounts");
    assert_eq!(edges[0]["mount_target"], "/data");
    assert!(
        nodes
            .iter()
            .any(|n| n["id"] == edges[0]["target"] && n["name"] == "orders-data")
    );
    let citations = pack["citations"].as_array().unwrap();
    let ids: std::collections::HashSet<_> = citations
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), citations.len());
    assert_eq!(citations.len(), 4);
    assert!(ids.contains(pack["source_evidence_id"].as_str().unwrap()));
    for item in nodes.iter().chain(edges) {
        let id = item["evidence_id"].as_str().unwrap();
        assert!(ids.contains(id));
        let stored = success_json(invoke(
            &fixture.database,
            ["evidence", id, fixture.task_path.to_str().unwrap()],
        ));
        assert_eq!(&stored, citations.iter().find(|c| c["id"] == id).unwrap());
    }
    assert_eq!(
        success_json(call(&["--max-output-bytes", &bytes.len().to_string()])),
        pack
    );
    rejected(call(&[
        "--max-output-bytes",
        &(bytes.len() - 1).to_string(),
    ]));
    let capped = success_json(call(&["--max-nodes", "1"]));
    assert_eq!(capped["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(capped["edges"], json!([]));
    assert_eq!(capped["budget_limited"], true);
    let zero = success_json(call(&["--depth", "0"]));
    assert_eq!(zero["depth_limited"], true);
    assert_eq!(zero["edges"], json!([]));
    let reverse = success_json(call(&["--direction", "incoming"]));
    assert_eq!(reverse["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(reverse["edges"], json!([]));
    let edge_cap = success_json(call(&["--max-edges", "0"]));
    assert_eq!(edge_cap["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(edge_cap["edges"], json!([]));
    assert_eq!(edge_cap["budget_limited"], true);
    for args in [
        ["--max-nodes", "0"],
        ["--depth", "17"],
        ["--max-edges", "5001"],
        ["--direction", "diagonal"],
        ["--max-output-bytes", "16777217"],
    ] {
        rejected(call(&args));
    }
    assert_eq!(success_json(deployment(&fixture)), saved);
}

#[test]
fn context_rejects_absent_invalidated_or_unknown_seed_without_fake_architecture() {
    let fixture = orders_fixture();
    let call = || {
        invoke(
            &fixture.database,
            [
                "deployment-context",
                "e1",
                fixture.task_path.to_str().unwrap(),
                "--node",
                "missing",
            ],
        )
    };
    rejected_with_message(call(), "no graph");
    success_json(publish(&fixture, 0, &[]));
    rejected_with_message(call(), "seed not found");
    success_json(invalidate(&fixture, 1));
    rejected_with_message(call(), "invalidated");
    assert_eq!(success_json(deployment(&fixture))["generation"], 2);
}

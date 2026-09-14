use serde_json::{Value, json};
use std::{
    collections::HashSet,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const ORDERS_COMPOSE: &[u8] = include_bytes!("../../../fixtures/orders/compose.yaml");
const ORDERS_HASH: &str = "a2a29afb353a588cc28759f281819f5cfda82fd5044f8fd5e38247959fa37701";
const UNICODE_HASH: &str = "3adc79fb672547d443bbe6b6429e15a72de9e7ba6fe78ea101ec622b58a521aa";

struct Fixture {
    _dir: tempfile::TempDir,
    database: PathBuf,
    root: PathBuf,
    task_path: PathBuf,
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

fn fixture(source: &[u8], hash: &str, end_line: u32) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.db");
    let root = dir.path().join("root");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("compose.yaml"), source).unwrap();

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
    let citation_path = dir.path().join("citation.json");
    let task_path = dir.path().join("task.json");
    write_json(&citation_path, &citation);
    write_json(&task_path, &task(&project));

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
    }
}

fn orders_fixture() -> Fixture {
    fixture(ORDERS_COMPOSE, ORDERS_HASH, 20)
}

fn unicode_fixture() -> Fixture {
    let name = "界".repeat(33);
    let source = format!("services:\n  {name}:\n    networks: [front]\nnetworks:\n  front: {{}}\n");
    fixture(source.as_bytes(), UNICODE_HASH, 5)
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
        "missing {message:?} in stderr {stderr:?}"
    );
}

fn publish(fixture: &Fixture) {
    assert_eq!(
        success_json(invoke(
            &fixture.database,
            [
                "publish-compose",
                "e1",
                fixture.task_path.to_str().unwrap(),
                fixture.root.to_str().unwrap(),
                "--expected-generation",
                "0",
            ],
        ))["generation"],
        1
    );
}

fn invalidate(fixture: &Fixture) {
    assert_eq!(
        success_json(invoke(
            &fixture.database,
            [
                "invalidate-compose",
                "e1",
                fixture.task_path.to_str().unwrap(),
                "--expected-generation",
                "1",
            ],
        ))["generation"],
        2
    );
}

fn deployment(fixture: &Fixture) -> Output {
    invoke(
        &fixture.database,
        ["deployment", "e1", fixture.task_path.to_str().unwrap()],
    )
}

fn context(fixture: &Fixture, seed: &str, extra: &[String]) -> Output {
    let mut args = vec![
        "deployment-context".to_owned(),
        "e1".to_owned(),
        fixture.task_path.to_str().unwrap().to_owned(),
        "--node".to_owned(),
        seed.to_owned(),
    ];
    args.extend(extra.iter().cloned());
    invoke(&fixture.database, args)
}

fn diagram(fixture: &Fixture, seed: &str, extra: &[String]) -> Output {
    let mut args = vec![
        "deployment-diagram".to_owned(),
        "e1".to_owned(),
        fixture.task_path.to_str().unwrap().to_owned(),
        "--node".to_owned(),
        seed.to_owned(),
    ];
    args.extend(extra.iter().cloned());
    invoke(&fixture.database, args)
}

fn node_id_by_name(snapshot: &Value, name: &str) -> String {
    snapshot["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["name"] == name)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn short_label(name: &str) -> String {
    let mut chars = name.chars();
    let mut label: String = chars.by_ref().take(32).collect();
    if chars.next().is_some() {
        label.push('…');
    }
    label
}

fn node_kind(value: &Value) -> &str {
    value["kind"].as_str().unwrap()
}

fn edge_kind(value: &Value) -> &str {
    value["kind"].as_str().unwrap()
}

fn assert_diagram_contract(diagram: &Value) {
    assert_eq!(diagram["schema_version"], 1);
    assert_eq!(diagram["kind"], "deployment_diagram");

    let architecture = &diagram["diagram"];
    assert_eq!(architecture["schema_version"], 1);
    assert_eq!(architecture["diagram_type"], "architecture");
    assert!(architecture["meta"]["title"].as_str().unwrap().len() > 0);
    assert!(architecture["meta"].get("repository").is_none());
    assert_eq!(architecture["layout"]["mode"], "grid");
    assert!(
        architecture["cards"]
            .as_array()
            .is_some_and(|cards| !cards.is_empty())
    );

    let card_text = architecture["cards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| {
            let items = card["items"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} {items}", card["title"].as_str().unwrap())
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    assert!(card_text.contains("coverage"));
    assert!(card_text.contains("historical"));

    let manifest = &diagram["evidence_manifest"];
    assert_eq!(manifest["kind"], "deployment_context");
    assert_eq!(manifest["historical"], true);
    assert_eq!(manifest["source_bytes_verified"], false);
    assert_eq!(manifest["relationship_verified"], false);
    assert_eq!(manifest["analysis_run_verified"], false);
    assert_eq!(manifest["provenance"], "iac_declared");

    let citations = manifest["citations"].as_array().unwrap();
    let citation_ids = citations
        .iter()
        .map(|citation| citation["id"].as_str().unwrap())
        .collect::<HashSet<_>>();
    assert_eq!(citation_ids.len(), citations.len());
}

#[test]
fn published_orders_diagram_maps_declared_context_without_fabrication() {
    let fixture = orders_fixture();
    publish(&fixture);

    let snapshot = success_json(deployment(&fixture));
    let seed = node_id_by_name(&snapshot, "orders");
    let expected_context = success_json(context(&fixture, &seed, &[]));
    let output = diagram(&fixture, &seed, &[]);
    let output_bytes = output.stdout.clone();
    let report = success_json(output);
    assert_diagram_contract(&report);

    assert_eq!(report["evidence_manifest"], expected_context);
    assert_eq!(report["evidence_manifest"]["generation"], 1);
    assert_eq!(report["evidence_manifest"]["direction"], "both");
    assert_eq!(report["evidence_manifest"]["depth"], 2);

    let context_nodes = report["evidence_manifest"]["nodes"].as_array().unwrap();
    let context_edges = report["evidence_manifest"]["edges"].as_array().unwrap();
    let components = report["diagram"]["components"].as_array().unwrap();
    let connections = report["diagram"]["connections"].as_array().unwrap();
    let component_bindings = report["component_bindings"].as_array().unwrap();
    let connection_bindings = report["connection_bindings"].as_array().unwrap();
    assert_eq!(components.len(), context_nodes.len());
    assert_eq!(connections.len(), context_edges.len());
    assert_eq!(component_bindings.len(), context_nodes.len());
    assert_eq!(connection_bindings.len(), context_edges.len());

    for (index, binding) in component_bindings.iter().enumerate() {
        assert_eq!(binding["diagram_id"], format!("n{index}"));
        assert_eq!(binding["context_node_index"], index);
    }
    for (index, binding) in connection_bindings.iter().enumerate() {
        assert_eq!(binding["diagram_id"], format!("e{index}"));
        assert_eq!(binding["context_edge_index"], index);
    }

    let graph_nodes = snapshot["graph"]["nodes"].as_array().unwrap();
    let graph_edges = snapshot["graph"]["edges"].as_array().unwrap();
    for (index, (component, context_node)) in components.iter().zip(context_nodes).enumerate() {
        assert_eq!(component["id"], format!("n{index}"));
        assert_eq!(component["type"], "external");
        assert_eq!(
            component["label"],
            short_label(context_node["name"].as_str().unwrap())
        );
        assert_eq!(component["sublabel"], node_kind(context_node));
        assert_eq!(component["tag"], "declared");
        assert!(component.get("sources").is_none());
        assert!(component.get("repository").is_none());

        let original = graph_nodes
            .iter()
            .find(|node| node["id"] == context_node["id"])
            .unwrap();
        assert_eq!(context_node["kind"], original["kind"]);
        assert_eq!(context_node["name"], original["name"]);
        assert_eq!(context_node["evidence_id"], original["evidence"]["id"]);
    }

    for (index, (connection, context_edge)) in connections.iter().zip(context_edges).enumerate() {
        assert_eq!(connection["id"], format!("e{index}"));
        assert_eq!(
            connection["from"],
            format!(
                "n{}",
                context_node_index(context_nodes, &context_edge["source"])
            )
        );
        assert_eq!(
            connection["to"],
            format!(
                "n{}",
                context_node_index(context_nodes, &context_edge["target"])
            )
        );
        assert_eq!(connection["label"], edge_kind(context_edge));
        assert_eq!(connection["variant"], "dashed");
        assert!(connection.get("source").is_none());
        assert!(connection.get("repository").is_none());
        assert!(connection.get("network").is_none());

        let original = graph_edges
            .iter()
            .find(|edge| {
                edge["source"] == context_edge["source"]
                    && edge["target"] == context_edge["target"]
                    && edge["kind"] == context_edge["kind"]
            })
            .unwrap();
        assert_eq!(context_edge["mount_target"], original["mount_target"]);
        assert_eq!(context_edge["evidence_id"], original["evidence"]["id"]);
    }

    let manifest_citations = report["evidence_manifest"]["citations"].as_array().unwrap();
    for item in context_nodes.iter().chain(context_edges) {
        assert!(
            manifest_citations
                .iter()
                .any(|citation| citation["id"] == item["evidence_id"])
        );
    }
    assert!(!output_bytes.is_empty());
}

fn context_node_index(nodes: &[Value], id: &Value) -> usize {
    nodes.iter().position(|node| node["id"] == *id).unwrap()
}

#[test]
fn unicode_labels_truncate_by_characters_and_keep_networks_neutral() {
    let fixture = unicode_fixture();
    publish(&fixture);

    let snapshot = success_json(deployment(&fixture));
    let seed = node_id_by_name(&snapshot, &"界".repeat(33));
    let report = success_json(diagram(&fixture, &seed, &[]));
    assert_diagram_contract(&report);

    let components = report["diagram"]["components"].as_array().unwrap();
    assert_eq!(components.len(), 2);
    let long = components
        .iter()
        .find(|component| component["sublabel"] == "service")
        .unwrap();
    assert_eq!(long["label"], format!("{}…", "界".repeat(32)));
    assert_eq!(long["label"].as_str().unwrap().chars().count(), 33);
    assert!(!long["label"].as_str().unwrap().contains("..."));
    assert_eq!(long["type"], "external");
    assert_eq!(
        components
            .iter()
            .find(|component| component["sublabel"] == "network")
            .unwrap()["type"],
        "external"
    );

    let connections = report["diagram"]["connections"].as_array().unwrap();
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0]["label"], "attached_to");
    assert_eq!(connections[0]["variant"], "dashed");
}

#[test]
fn diagram_survives_deleted_source_root_and_repeats_without_graph_mutation() {
    let fixture = orders_fixture();
    publish(&fixture);
    let snapshot_before = success_json(deployment(&fixture));
    let seed = node_id_by_name(&snapshot_before, "orders");

    let first = diagram(&fixture, &seed, &[]);
    let first_bytes = first.stdout.clone();
    let first_report = success_json(first);
    assert_eq!(success_json(deployment(&fixture)), snapshot_before);

    fs::remove_dir_all(&fixture.root).unwrap();
    let second = diagram(&fixture, &seed, &[]);
    assert!(second.status.success());
    assert!(second.stderr.is_empty());
    assert_eq!(second.stdout, first_bytes);
    assert_eq!(success_json(second), first_report);
    assert_eq!(success_json(deployment(&fixture)), snapshot_before);
}

#[test]
fn exact_output_budget_succeeds_and_one_byte_under_emits_no_stdout() {
    let fixture = orders_fixture();
    publish(&fixture);
    let snapshot_before = success_json(deployment(&fixture));
    let seed = node_id_by_name(&snapshot_before, "orders");

    let baseline = diagram(&fixture, &seed, &[]);
    let expected_bytes = baseline.stdout.clone();
    success_json(baseline);
    let exact = diagram(
        &fixture,
        &seed,
        &[
            "--max-output-bytes".into(),
            expected_bytes.len().to_string(),
        ],
    );
    assert!(exact.status.success());
    assert!(exact.stderr.is_empty());
    assert_eq!(exact.stdout, expected_bytes);

    let under = diagram(
        &fixture,
        &seed,
        &[
            "--max-output-bytes".into(),
            (expected_bytes.len() - 1).to_string(),
        ],
    );
    rejected(under);
    assert_eq!(success_json(deployment(&fixture)), snapshot_before);
}

#[test]
fn direction_depth_and_item_caps_preserve_omission_flags() {
    let fixture = orders_fixture();
    publish(&fixture);
    let snapshot = success_json(deployment(&fixture));
    let seed = node_id_by_name(&snapshot, "orders");

    let default_report = success_json(diagram(&fixture, &seed, &[]));
    assert_eq!(default_report["evidence_manifest"]["direction"], "both");
    assert_eq!(default_report["evidence_manifest"]["depth"], 2);

    let outgoing = success_json(diagram(
        &fixture,
        &seed,
        &["--direction".into(), "outgoing".into()],
    ));
    assert_eq!(outgoing["evidence_manifest"]["direction"], "outgoing");
    assert_eq!(
        outgoing["evidence_manifest"]["edges"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let incoming = success_json(diagram(
        &fixture,
        &seed,
        &["--direction".into(), "incoming".into()],
    ));
    assert_eq!(incoming["evidence_manifest"]["direction"], "incoming");
    assert_eq!(
        incoming["evidence_manifest"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        incoming["evidence_manifest"]["edges"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let node_cap = success_json(diagram(
        &fixture,
        &seed,
        &["--max-nodes".into(), "1".into()],
    ));
    assert_eq!(
        node_cap["evidence_manifest"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        node_cap["evidence_manifest"]["edges"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(node_cap["evidence_manifest"]["omitted_nodes"], 3);
    assert_eq!(node_cap["evidence_manifest"]["omitted_edges"], 2);
    assert_eq!(node_cap["evidence_manifest"]["budget_limited"], true);

    let edge_cap = success_json(diagram(
        &fixture,
        &seed,
        &["--max-edges".into(), "0".into()],
    ));
    assert_eq!(
        edge_cap["evidence_manifest"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        edge_cap["evidence_manifest"]["edges"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(edge_cap["evidence_manifest"]["omitted_nodes"], 3);
    assert_eq!(edge_cap["evidence_manifest"]["omitted_edges"], 2);
    assert_eq!(edge_cap["evidence_manifest"]["budget_limited"], true);

    let depth_cap = success_json(diagram(&fixture, &seed, &["--depth".into(), "0".into()]));
    assert_eq!(depth_cap["evidence_manifest"]["depth"], 0);
    assert_eq!(depth_cap["evidence_manifest"]["depth_limited"], true);
    assert!(
        depth_cap["evidence_manifest"]["edges"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn invalid_limits_and_direction_are_rejected_without_json() {
    let fixture = orders_fixture();
    publish(&fixture);
    let snapshot = success_json(deployment(&fixture));
    let seed = node_id_by_name(&snapshot, "orders");

    for args in [
        vec!["--direction".into(), "diagonal".into()],
        vec!["--depth".into(), "17".into()],
        vec!["--max-nodes".into(), "0".into()],
        vec!["--max-nodes".into(), "13".into()],
        vec!["--max-edges".into(), "25".into()],
        vec!["--max-output-bytes".into(), "16777217".into()],
    ] {
        rejected(diagram(&fixture, &seed, &args));
    }
}

#[test]
fn missing_tombstoned_and_unknown_seeds_fail_closed_without_stdout() {
    let fixture = orders_fixture();
    let missing = diagram(&fixture, "orders", &[]);
    rejected_with_message(missing, "no graph");

    publish(&fixture);
    rejected_with_message(diagram(&fixture, "unknown-seed", &[]), "seed not found");

    invalidate(&fixture);
    rejected_with_message(diagram(&fixture, "orders", &[]), "invalidated");
}

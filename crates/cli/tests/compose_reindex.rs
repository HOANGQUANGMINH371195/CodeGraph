use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

const ORDERS_COMPOSE: &[u8] = include_bytes!("../../../fixtures/orders/compose.yaml");
const ORDERS_HASH: &str = "a2a29afb353a588cc28759f281819f5cfda82fd5044f8fd5e38247959fa37701";
const MALFORMED_COMPOSE: &[u8] = b"services:\n  api: [\n";
const CHANGED_COMPOSE: &[u8] = br#"services:
  orders:
    image: orders
    volumes:
      - orders-data:/data
  billing:
    image: billing
    volumes:
      - billing-data:/data
volumes:
  orders-data: {}
  billing-data: {}
"#;

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

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.db");
    let root = dir.path().join("root");
    fs::create_dir_all(&root).unwrap();
    write_source(&root, ORDERS_COMPOSE);

    let project = project("snapshot");
    let citation = json!({
        "schema_version": 1,
        "id": "e1",
        "project": project,
        "graph_version": "g1",
        "path": "compose.yaml",
        "content_sha256": ORDERS_HASH,
        "start_line": 1,
        "end_line": 20,
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
            ["record-evidence", citation_path.to_str().unwrap()],
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

fn write_source(root: &Path, source: &[u8]) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("compose.yaml"), source).unwrap();
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

fn invalid_report(output: Output) -> Value {
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn reindex(fixture: &Fixture, expected: u64, extra: &[String]) -> Output {
    reindex_id_with_run(fixture, "e1", "run1", expected, extra)
}

fn reindex_id(fixture: &Fixture, id: &str, expected: u64, extra: &[String]) -> Output {
    reindex_id_with_run(fixture, id, "run1", expected, extra)
}

fn reindex_id_with_run(
    fixture: &Fixture,
    id: &str,
    analysis_run: &str,
    expected: u64,
    extra: &[String],
) -> Output {
    let mut args = vec![
        "reindex-compose".to_owned(),
        id.to_owned(),
        fixture.task_path.to_str().unwrap().to_owned(),
        fixture.root.to_str().unwrap().to_owned(),
        "--analysis-run".to_owned(),
        analysis_run.to_owned(),
        "--expected-generation".to_owned(),
        expected.to_string(),
    ];
    args.extend(extra.iter().cloned());
    invoke(&fixture.database, args)
}

fn query(fixture: &Fixture) -> Value {
    success_json(invoke(
        &fixture.database,
        ["deployment", "e1", fixture.task_path.to_str().unwrap()],
    ))
}

fn report(
    status: &str,
    generation: u64,
    source_valid: bool,
    changed: bool,
    reason: Option<&str>,
) -> Value {
    json!({
        "schema_version": 1,
        "kind": "compose_reindex",
        "status": status,
        "generation": generation,
        "source_valid": source_valid,
        "changed": changed,
        "historical": true,
        "snapshot_binding": "caller_supplied",
        "relationship_verified": false,
        "reason": reason
    })
}

fn report_bytes(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).unwrap();
    bytes.push(b'\n');
    bytes
}

fn assert_graph(snapshot: &Value, generation: u64) {
    assert_eq!(snapshot["generation"], generation);
    assert_eq!(snapshot["historical"], true);
    assert_eq!(snapshot["snapshot_binding"], "caller_supplied");
    assert_eq!(snapshot["relationship_verified"], false);
    assert_eq!(snapshot["graph"]["adapter"], "compose");
}

#[test]
fn changed_source_publishes_new_graph_through_original_locator() {
    let fixture = fixture();

    assert_eq!(
        success_json(reindex(&fixture, 0, &[])),
        report("published", 1, true, true, None)
    );
    write_source(&fixture.root, CHANGED_COMPOSE);

    assert_eq!(
        success_json(reindex(&fixture, 1, &[])),
        report("published", 2, true, true, None)
    );
    let snapshot = query(&fixture);
    assert_graph(&snapshot, 2);
    assert_ne!(snapshot["graph"]["evidence"]["content_sha256"], ORDERS_HASH);
    assert_ne!(snapshot["graph"]["evidence"]["id"], "e1");
    assert!(
        snapshot["graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["name"] == "billing")
    );
}

#[test]
fn stale_generation_fails_before_mutation() {
    let fixture = fixture();
    assert_eq!(
        success_json(reindex(&fixture, 0, &[])),
        report("published", 1, true, true, None)
    );
    let before = query(&fixture);
    write_source(&fixture.root, CHANGED_COMPOSE);

    rejected_with_message(reindex(&fixture, 0, &[]), "deployment generation conflict");
    assert_eq!(query(&fixture), before);
}

#[test]
fn malformed_and_deleted_sources_invalidate_then_recover() {
    let fixture = fixture();
    assert_eq!(
        success_json(reindex(&fixture, 0, &[])),
        report("published", 1, true, true, None)
    );

    write_source(&fixture.root, MALFORMED_COMPOSE);
    assert_eq!(
        invalid_report(reindex(&fixture, 1, &[])),
        report(
            "invalidated",
            2,
            false,
            true,
            Some("compose_analysis_failed")
        )
    );
    assert_eq!(query(&fixture)["graph"], Value::Null);

    assert_eq!(
        invalid_report(reindex(&fixture, 2, &[])),
        report(
            "invalidated",
            2,
            false,
            false,
            Some("compose_analysis_failed")
        )
    );

    write_source(&fixture.root, ORDERS_COMPOSE);
    assert_eq!(
        success_json(reindex(&fixture, 2, &[])),
        report("published", 3, true, true, None)
    );

    fs::remove_dir_all(&fixture.root).unwrap();
    assert_eq!(
        invalid_report(reindex(&fixture, 3, &[])),
        report("invalidated", 4, false, true, Some("source_unavailable"))
    );
    assert_eq!(query(&fixture)["graph"], Value::Null);

    write_source(&fixture.root, CHANGED_COMPOSE);
    assert_eq!(
        success_json(reindex(&fixture, 4, &[])),
        report("published", 5, true, true, None)
    );
    assert!(
        query(&fixture)["graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["name"] == "billing")
    );
}

#[test]
fn identical_capture_is_unchanged_with_stable_generation_and_citation() {
    let fixture = fixture();
    assert_eq!(
        success_json(reindex(&fixture, 0, &[])),
        report("published", 1, true, true, None)
    );
    let before = query(&fixture);

    assert_eq!(
        success_json(reindex(&fixture, 1, &[])),
        report("unchanged", 1, true, false, None)
    );
    assert_eq!(query(&fixture), before);
}

#[test]
fn exact_output_budget_succeeds_and_one_byte_under_fails_before_mutation() {
    let fixture = fixture();
    assert_eq!(
        success_json(reindex(&fixture, 0, &[])),
        report("published", 1, true, true, None)
    );
    let before = query(&fixture);
    write_source(&fixture.root, CHANGED_COMPOSE);

    let expected = report("published", 2, true, true, None);
    let exact = report_bytes(&expected).len();
    rejected(reindex(
        &fixture,
        1,
        &["--max-output-bytes".to_owned(), (exact - 1).to_string()],
    ));
    assert_eq!(query(&fixture), before);

    assert_eq!(
        success_json(reindex(
            &fixture,
            1,
            &["--max-output-bytes".to_owned(), exact.to_string()],
        )),
        expected
    );
    assert_graph(&query(&fixture), 2);
}

#[test]
fn invalid_locator_task_and_config_never_invalidate_existing_graph() {
    let fixture = fixture();
    assert_eq!(
        success_json(reindex(&fixture, 0, &[])),
        report("published", 1, true, true, None)
    );
    let before = query(&fixture);

    rejected_with_message(
        reindex_id(&fixture, "missing", 1, &[]),
        "evidence not found",
    );
    assert_eq!(query(&fixture), before);

    rejected_with_message(
        reindex_id_with_run(&fixture, "e1", "", 1, &[]),
        "analysis run",
    );
    assert_eq!(query(&fixture), before);

    let original_task = fs::read(&fixture.task_path).unwrap();
    fs::write(&fixture.task_path, br#"{"secret":"invalid-task"}"#).unwrap();
    rejected_with_message(reindex(&fixture, 1, &[]), "invalid deployment task input");
    fs::write(&fixture.task_path, original_task).unwrap();
    assert_eq!(query(&fixture), before);
}

#[test]
fn zero_output_budget_on_invalid_source_fails_before_invalidation() {
    let fixture = fixture();
    assert_eq!(
        success_json(reindex(&fixture, 0, &[])),
        report("published", 1, true, true, None)
    );
    let before = query(&fixture);
    write_source(&fixture.root, MALFORMED_COMPOSE);

    rejected(reindex(
        &fixture,
        1,
        &["--max-output-bytes".to_owned(), "0".to_owned()],
    ));
    assert_eq!(query(&fixture), before);

    assert_eq!(
        invalid_report(reindex(&fixture, 1, &[])),
        report(
            "invalidated",
            2,
            false,
            true,
            Some("compose_analysis_failed")
        )
    );
    assert_eq!(query(&fixture)["graph"], Value::Null);
}

struct WatchProcess {
    child: Child,
}

impl WatchProcess {
    fn spawn(fixture: &Fixture) -> Self {
        Self {
            child: Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
                .arg("--database")
                .arg(&fixture.database)
                .args([
                    "watch-compose",
                    "e1",
                    fixture.task_path.to_str().unwrap(),
                    fixture.root.to_str().unwrap(),
                    "--analysis-run",
                    "run1",
                    "--expected-generation",
                    "0",
                    "--interval-ms",
                    "500",
                    "--max-cycles",
                    "8",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        }
    }

    fn line_receiver(&mut self) -> Receiver<std::io::Result<String>> {
        line_receiver(&mut self.child)
    }

    fn wait_bounded(&mut self) -> ExitStatus {
        wait_bounded(&mut self.child)
    }
}

impl Drop for WatchProcess {
    fn drop(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn line_receiver(child: &mut Child) -> Receiver<std::io::Result<String>> {
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    receiver
}

fn next_line(receiver: &Receiver<std::io::Result<String>>) -> String {
    receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("watch did not emit a line before the timeout")
        .expect("watch stdout reader failed")
}

fn wait_bounded(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("watch process did not exit before the timeout");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn finite_watch_synchronizes_edits_and_continues_after_source_failure() {
    let fixture = fixture();
    let mut process = WatchProcess::spawn(&fixture);
    let receiver = process.line_receiver();

    assert_eq!(
        serde_json::from_str::<Value>(&next_line(&receiver)).unwrap(),
        report("published", 1, true, true, None)
    );
    write_source(&fixture.root, MALFORMED_COMPOSE);

    let mut invalidated = false;
    for _ in 0..3 {
        let observed = serde_json::from_str::<Value>(&next_line(&receiver)).unwrap();
        if observed["status"] == "invalidated" {
            assert_eq!(
                observed,
                report(
                    "invalidated",
                    2,
                    false,
                    true,
                    Some("compose_analysis_failed")
                )
            );
            invalidated = true;
            break;
        }
    }
    assert!(invalidated, "watch did not observe invalid source");
    write_source(&fixture.root, CHANGED_COMPOSE);

    let mut recovered = false;
    for _ in 0..3 {
        let observed = serde_json::from_str::<Value>(&next_line(&receiver)).unwrap();
        if observed["status"] == "published" {
            assert_eq!(observed, report("published", 3, true, true, None));
            recovered = true;
            break;
        }
        assert_eq!(
            observed,
            report(
                "invalidated",
                2,
                false,
                false,
                Some("compose_analysis_failed")
            )
        );
    }
    assert!(recovered, "watch did not observe source recovery");
    assert!(process.wait_bounded().success());

    let mut stderr = String::new();
    process
        .child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(stderr.is_empty(), "unexpected watch stderr: {stderr}");
    assert_graph(&query(&fixture), 3);
}

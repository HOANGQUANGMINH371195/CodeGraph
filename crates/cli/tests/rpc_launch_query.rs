//! Historical ledger inspection only; the described RPC program is never spawned.
#[path = "rpc_launch_query/output.rs"]
mod output;
use graph_application::{
    AnalysisRepository, RpcLaunchQueryRepository, RpcLaunchRepository,
    RpcSpawnObservationRepository, RpcTerminalReceiptRepository, TaskRepository, TaskService,
};
use graph_protocol::{RpcLaunchSpec, TaskSpec};
use graph_store::Store;
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command, Output},
};

fn task() -> Value {
    json!({"schema_version":1,"id":"task",
        "project":{"repository_id":"repo","worktree_id":"source","git_head":"head",
            "working_tree_fingerprint":"tree","config_hash":"config","ignore_policy_version":"1"},
        "graph_version":"graph","role":"reader","account_lane":"native","scope":["src"],
        "dependencies":[],"context_ref":"context-secret","expected_artifacts":["report"],"token_budget":100})
}

fn fixture(db: &Path) -> (Store, RpcLaunchSpec) {
    let task: TaskSpec = serde_json::from_value(task()).unwrap();
    let mut tasks = TaskService::new(Store::open(db).unwrap());
    tasks
        .enqueue(task.clone().try_into_domain().unwrap(), 0)
        .unwrap();
    let lease = tasks.lease("task", "worker", 10, 100).unwrap();
    let mut snapshot = serde_json::to_value(&task.project).unwrap();
    snapshot["worktree_id"] = json!("execution-secret");
    snapshot["git_head"] = json!("execution-head-secret");
    snapshot["working_tree_fingerprint"] = json!("execution-tree-secret");
    let wire = serde_json::from_value(json!({
        "schema_version":1,"transport":"stdio_jsonl","id":"launch","task":task,
        "origin_lease":graph_protocol::Lease::from(&lease),"host_id":"host",
        "approval_id":"approval-secret","execution_snapshot":snapshot,
        "process":{"program":"/fixture/never-executed","args":["argument-secret"],
            "cwd":"src","executable_sha256":"a".repeat(64),"environment_sha256":"b".repeat(64),
            "timeout_ms":1000,"cleanup_timeout_ms":200,"stdout_max_bytes":4096,"stderr_max_bytes":8192},
        "connection":{"epoch":"epoch","client_name":"client-secret","client_version":"1.2",
            "experimental":true,"max_pending":7,"max_frame":16384}
    })).unwrap();
    (tasks.into_inner(), wire)
}

fn query(db: &Path, task_path: &Path, id: &str, cap: Option<usize>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"));
    command
        .arg("--database")
        .arg(db)
        .arg("rpc-launch")
        .arg(id)
        .arg(task_path);
    if let Some(cap) = cap {
        command.arg("--max-output-bytes").arg(cap.to_string());
    }
    command.output().unwrap()
}

fn expected(launch: Value) -> Value {
    json!({"schema_version":1,"launch":launch,"observation_only":true,
        "execution_authority_granted":false,"process_liveness":"unknown",
        "retry_authorized":false,"snapshot_filter":"caller_supplied"})
}

fn brief(claimed: Option<i64>) -> Value {
    json!({"id":"launch","host_id":"host","connection_epoch":"epoch",
        "claimed_at_ms":claimed,"claim_state":if claimed.is_some() {"recorded"} else {"not_recorded"},"spawn_observation":null,"terminal_receipt":null})
}

fn report(wire: &RpcLaunchSpec, disposition: &str) -> graph_protocol::RpcSpawnObservation {
    serde_json::from_value(json!({"schema_version":1,"launch":wire,"observed_at_ms":13,
        "disposition":disposition,"process_id":if disposition=="spawned" {Some(42_u32)} else {None}})).unwrap()
}

#[test]
fn terminal_report_is_bounded_brief_and_never_grants_retry() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("db");
    let task_path = root.path().join("task.json");
    std::fs::write(&task_path, task().to_string()).unwrap();
    let (mut store, wire) = fixture(&db);
    let spec = wire.clone().try_into_domain().unwrap();
    store.register_rpc_launch(&spec, 11).unwrap();
    store.claim_rpc_launch(&spec, 12).unwrap();
    let spawn = report(&wire, "spawned");
    store
        .record_rpc_spawn_observation(&spawn.clone().try_into_domain().unwrap())
        .unwrap();
    let completion = json!({"schema_version":1,"reason":"host_error",
        "child":{"state":"reaped","exit_code":1},"stdout":"incomplete",
        "stderr":"incomplete","cleanup":"unverifiable"});
    let receipt: graph_protocol::RpcTerminalReceipt = serde_json::from_value(json!({
        "schema_version":1,"spawn":spawn,"output_run":{
            "schema_version":1,"id":"run-secret","project":wire.execution_snapshot,
            "graph_version":spec.task().graph_version(),"analyzer":"analyzer-secret",
            "analyzer_version":"1","configuration_sha256":"c".repeat(64),
            "input_manifest_sha256":"d".repeat(64)},
        "finished_at_ms":15,"supervised_elapsed_ms":2,"stdout":null,"stderr":null,
        "completion":completion,"pending":[{"id":1,"method":"method-secret","uncertain":true}],
        "input_progress":null
    }))
    .unwrap();
    let receipt = receipt.try_into_domain().unwrap();
    store.record_analysis_run(receipt.output_run()).unwrap();
    store.record_rpc_terminal_receipt(&receipt).unwrap();
    let events = store.events(0, 100).unwrap();
    drop(store);
    let mut launch = brief(Some(12));
    launch["spawn_observation"] =
        json!({"observed_at_ms":13,"disposition":"spawned","process_id":42});
    launch["terminal_receipt"] = json!({"finished_at_ms":15,"supervised_elapsed_ms":2,
        "completion":completion,"uncertain_request_count":1});
    let output = query(&db, &task_path, "launch", None);
    success(&output, expected(launch.clone()));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("secret"));
    success(
        &query(&db, &task_path, "launch", Some(output.stdout.len())),
        expected(launch),
    );
    let short = query(&db, &task_path, "launch", Some(output.stdout.len() - 1));
    assert!(!short.status.success());
    assert!(short.stdout.is_empty());
    assert_eq!(Store::open(&db).unwrap().events(0, 100).unwrap(), events);
}

#[test]
fn stored_spawn_reports_are_brief_historical_observations_after_reopen_and_cancel() {
    for disposition in [
        "cancelled_before_spawn",
        "expired_before_spawn",
        "spawn_failed",
        "spawned",
    ] {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("ledger.sqlite");
        let task_path = root.path().join("task.json");
        std::fs::write(&task_path, task().to_string()).unwrap();
        let (mut store, wire) = fixture(&db);
        let spec = wire.clone().try_into_domain().unwrap();
        store.register_rpc_launch(&spec, 11).unwrap();
        store.claim_rpc_launch(&spec, 12).unwrap();
        let observation = report(&wire, disposition).try_into_domain().unwrap();
        store.record_rpc_spawn_observation(&observation).unwrap();
        store
            .cancel(spec.task().id(), spec.origin_lease().owner(), "stop", 14)
            .unwrap();
        let events = store.events(0, 100).unwrap();
        drop(store);
        let mut launch = brief(Some(12));
        launch["spawn_observation"] = json!({"observed_at_ms":13,"disposition":disposition,
            "process_id":if disposition=="spawned" {Some(42_u32)} else {None}});
        let output = query(&db, &task_path, "launch", None);
        success(&output, expected(launch));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("secret"));
        let store = Store::open(&db).unwrap();
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(
            store.rpc_spawn_observation(&spec).unwrap(),
            Some(observation)
        );
    }
}

fn success(output: &Output, value: Value) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    assert_eq!(output.stdout.iter().filter(|b| **b == b'\n').count(), 1);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        value
    );
}

fn rejected(output: &Output) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn missing_registered_claimed_reopened_and_cancelled_are_observations_only() {
    for claimed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("ledger.db");
        let task_path = dir.path().join("task.json");
        std::fs::write(&task_path, task().to_string()).unwrap();
        let (mut store, wire) = fixture(&db);
        let spec = wire.try_into_domain().unwrap();
        let events = store.events(0, 100).unwrap();
        success(
            &query(&db, &task_path, "missing", None),
            expected(Value::Null),
        );
        success(
            &query(&db, &task_path, "launch", None),
            expected(Value::Null),
        );
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert!(store.register_rpc_launch(&spec, 11).unwrap());
        let events = store.events(0, 100).unwrap();
        success(
            &query(&db, &task_path, "launch", None),
            expected(brief(None)),
        );
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(
            store
                .rpc_launch_snapshot("launch", spec.task())
                .unwrap()
                .unwrap()
                .claimed_at_ms(),
            None
        );
        if claimed {
            assert!(store.claim_rpc_launch(&spec, 12).unwrap());
        }
        drop(store);
        let mut store = Store::open(&db).unwrap();
        let at = claimed.then_some(12);
        let before = store.rpc_launch_snapshot("launch", spec.task()).unwrap();
        let events = store.events(0, 100).unwrap();
        success(&query(&db, &task_path, "launch", None), expected(brief(at)));
        assert_eq!(
            store.rpc_launch_snapshot("launch", spec.task()).unwrap(),
            before
        );
        assert_eq!(store.events(0, 100).unwrap(), events);
        store
            .cancel(spec.task().id(), spec.origin_lease().owner(), "stop", 13)
            .unwrap();
        let events = store.events(0, 100).unwrap();
        drop(store);
        success(&query(&db, &task_path, "launch", None), expected(brief(at)));
        let store = Store::open(&db).unwrap();
        assert_eq!(store.events(0, 100).unwrap(), events);
        assert_eq!(
            store.rpc_launch_snapshot("launch", spec.task()).unwrap(),
            before
        );
        assert_eq!(store.rpc_launch("launch", spec.task()).unwrap(), Some(spec));
    }
}

#[test]
fn full_task_contract_mismatch_including_id_returns_null() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("ledger.db");
    let task_path = dir.path().join("task.json");
    let (mut store, wire) = fixture(&db);
    let spec = wire.try_into_domain().unwrap();
    store.register_rpc_launch(&spec, 11).unwrap();
    store.claim_rpc_launch(&spec, 12).unwrap();
    store
        .record_rpc_spawn_observation(
            &report(&graph_protocol::RpcLaunchSpec::from(&spec), "spawned")
                .try_into_domain()
                .unwrap(),
        )
        .unwrap();
    let events = store.events(0, 100).unwrap();
    for pointer in [
        "/id",
        "/project/repository_id",
        "/project/worktree_id",
        "/project/git_head",
        "/project/working_tree_fingerprint",
        "/project/config_hash",
        "/project/ignore_policy_version",
        "/graph_version",
        "/role",
        "/account_lane",
        "/scope",
        "/dependencies",
        "/context_ref",
        "/expected_artifacts",
        "/token_budget",
    ] {
        let mut wrong = task();
        let field = wrong.pointer_mut(pointer).unwrap();
        *field = if field.is_array() {
            json!(["other"])
        } else if field.is_number() {
            json!(101)
        } else {
            json!("other")
        };
        // Ensure this is a valid but different filter, not a malformed request.
        serde_json::from_value::<TaskSpec>(wrong.clone())
            .unwrap()
            .try_into_domain()
            .unwrap();
        std::fs::write(&task_path, wrong.to_string()).unwrap();
        success(
            &query(&db, &task_path, "launch", None),
            expected(Value::Null),
        );
    }
    assert_eq!(store.events(0, 100).unwrap(), events);
    assert_eq!(
        store
            .rpc_launch_snapshot("launch", spec.task())
            .unwrap()
            .unwrap()
            .claimed_at_ms(),
        Some(12)
    );
}

#[test]
fn output_cap_counts_exact_json_bytes_including_lf_and_never_emits_partial_json() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("ledger.db");
    let task_path = dir.path().join("task.json");
    std::fs::write(&task_path, task().to_string()).unwrap();
    let (mut store, mut wire) = fixture(&db);
    wire.host_id = "höst-🚀-\"-\\".into();
    let spec = wire.try_into_domain().unwrap();
    for stage in 0..4 {
        if stage == 1 {
            store.register_rpc_launch(&spec, 11).unwrap();
        }
        if stage == 2 {
            store.claim_rpc_launch(&spec, 12).unwrap();
        }
        if stage == 3 {
            store
                .record_rpc_spawn_observation(
                    &report(&graph_protocol::RpcLaunchSpec::from(&spec), "spawned")
                        .try_into_domain()
                        .unwrap(),
                )
                .unwrap();
        }
        let events = store.events(0, 100).unwrap();
        let baseline = query(&db, &task_path, "launch", None);
        let launch = if stage == 0 {
            Value::Null
        } else {
            let mut value = brief((stage >= 2).then_some(12));
            value["host_id"] = json!(spec.host_id());
            if stage == 3 {
                value["spawn_observation"] =
                    json!({"observed_at_ms":13,"disposition":"spawned","process_id":42});
            }
            value
        };
        success(&baseline, expected(launch));
        let exact = query(&db, &task_path, "launch", Some(baseline.stdout.len()));
        assert!(exact.status.success());
        assert_eq!(exact.stdout, baseline.stdout);
        let maximum = query(&db, &task_path, "launch", Some(16 * 1024 * 1024));
        assert!(maximum.status.success());
        assert_eq!(maximum.stdout, baseline.stdout);
        for cap in [0, baseline.stdout.len() - 1, 16 * 1024 * 1024 + 1] {
            rejected(&query(&db, &task_path, "launch", Some(cap)));
        }
        assert_eq!(store.events(0, 100).unwrap(), events);
    }
}

#[test]
fn malformed_input_diagnostics_are_bounded_and_do_not_echo_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("ledger.db");
    let task_path = dir.path().join("task.json");
    let secret = "sensitive-payload-marker";
    let mut wrong_type = task();
    wrong_type["token_budget"] = json!(secret.repeat(1000));
    // Demonstrate the shared raw serde error path would leak this value.
    assert!(
        serde_json::from_value::<TaskSpec>(wrong_type.clone())
            .unwrap_err()
            .to_string()
            .contains(secret)
    );
    let mut unknown_field = task();
    unknown_field[secret] = json!(true);
    let mut invalid_domain = task();
    invalid_domain["id"] = json!("");
    invalid_domain["context_ref"] = json!(secret);
    for input in [
        format!("{{\"{secret}\":"),
        wrong_type.to_string(),
        unknown_field.to_string(),
        invalid_domain.to_string(),
    ] {
        std::fs::write(&task_path, input).unwrap();
        let output = query(&db, &task_path, "launch", None);
        rejected(&output);
        assert_eq!(output.stderr, b"invalid rpc-launch task specification\n");
        assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
    }
    std::fs::write(&task_path, vec![b'x'; 8 * 1024 * 1024 + 1]).unwrap();
    let output = query(&db, &task_path, "launch", None);
    rejected(&output);
    assert_eq!(output.stderr, b"JSON input exceeds 8 MiB byte limit\n");
}

#[test]
fn help_documents_db_open_and_absence_limits() {
    let output = Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .args(["rpc-launch", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("no ledger mutation by query"));
    assert!(help.contains("standard DB open may initialize/migrate"));
    assert!(help.contains("does not prove an absent process or authorize retry"));
    assert!(help.contains("<ID> <TASK_SPEC>"));
    assert!(help.contains("65536"));
}

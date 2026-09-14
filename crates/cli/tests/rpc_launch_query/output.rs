use super::*;
use graph_application::ArtifactRepository;

fn invoke(db: &Path, spec: &Path, root: &Path, content: u64, output: usize) -> Output {
    Command::new(env!("CARGO_BIN_EXE_project-graph-agent"))
        .arg("--database")
        .arg(db)
        .arg("verify-rpc-outputs")
        .arg(spec)
        .arg(root)
        .arg("--max-total-bytes")
        .arg(content.to_string())
        .arg("--max-output-bytes")
        .arg(output.to_string())
        .output()
        .unwrap()
}
fn failure(output: Output) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("secret"));
}

#[test]
fn rpc_output_cli_checks_bytes_without_promoting_rpc_completion() {
    for present in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("db");
        let input = root.path().join("launch.json");
        let cas = root.path().join("cas");
        std::fs::create_dir(&cas).unwrap();
        let (mut store, wire) = fixture(&db);
        std::fs::write(&input, serde_json::to_vec(&wire).unwrap()).unwrap();
        let launch = wire.clone().try_into_domain().unwrap();
        store.register_rpc_launch(&launch, 11).unwrap();
        store.claim_rpc_launch(&launch, 12).unwrap();
        let spawn = report(&wire, "spawned");
        store
            .record_rpc_spawn_observation(&spawn.clone().try_into_domain().unwrap())
            .unwrap();
        failure(invoke(&db, &input, &cas, 3, 65536));
        let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let artifact = json!({"schema_version":1,"id":"artifact-secret",
            "project":wire.execution_snapshot,"graph_version":"graph","analysis_run":"run-secret",
            "content_sha256":hash,"byte_length":3,"kind":"stdout",
            "retention":"evidence","declared_protection":"unreviewed"});
        let completion = json!({"schema_version":1,"reason":"host_error",
            "child":{"state":"reaped","exit_code":1},"stdout":"incomplete",
            "stderr":"incomplete","cleanup":"unverifiable"});
        let receipt: graph_protocol::RpcTerminalReceipt = serde_json::from_value(json!({
            "schema_version":1,"spawn":spawn,"output_run":{
                "schema_version":1,"id":"run-secret","project":wire.execution_snapshot,
                "graph_version":"graph","analyzer":"analyzer-secret","analyzer_version":"1",
                "configuration_sha256":"c".repeat(64),"input_manifest_sha256":"d".repeat(64)},
            "finished_at_ms":15,"supervised_elapsed_ms":2,
            "stdout":if present {artifact} else {Value::Null},"stderr":null,"completion":completion,
            "pending":[{"id":1,"method":"method-secret","uncertain":true}],"input_progress":null
        }))
        .unwrap();
        let receipt = receipt.try_into_domain().unwrap();
        store.record_analysis_run(receipt.output_run()).unwrap();
        if let Some(a) = receipt.stdout() {
            store.record_artifact(a).unwrap();
        }
        store.record_rpc_terminal_receipt(&receipt).unwrap();
        let events = store.events(0, 100).unwrap();
        drop(store);
        if present {
            failure(invoke(&db, &input, &cas, 3, 65536));
            std::fs::write(cas.join(hash), b"abc").unwrap();
            failure(invoke(&db, &input, &cas, 2, 65536));
        }
        let output = invoke(&db, &input, &cas, if present { 3 } else { 0 }, 65536);
        let expected = json!({"schema_version":1,
            "stdout":if present {json!({"content_verified":true,"byte_length":3,"sha256":hash})} else {Value::Null},
            "stderr":null,"completion":completion,"uncertain_request_count":1,
            "observation_only":true,"execution_authority_granted":false,"retry_authorized":false,
            "process_liveness":"unknown","protection_verified":false,"expectation_source":"caller_supplied"});
        success(&output, expected.clone());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("secret"));
        success(&invoke(&db, &input, &cas, 3, output.stdout.len()), expected);
        failure(invoke(&db, &input, &cas, 3, output.stdout.len() - 1));
        if present {
            std::fs::write(cas.join(hash), b"xyz").unwrap();
            failure(invoke(&db, &input, &cas, 3, 65536));
        }
        let mut wrong = wire;
        wrong.host_id.push_str("secret");
        std::fs::write(&input, serde_json::to_vec(&wrong).unwrap()).unwrap();
        failure(invoke(&db, &input, &cas, 3, 65536));
        std::fs::write(&input, br#"{"secret-field":true}"#).unwrap();
        failure(invoke(&db, &input, &cas, 3, 65536));
        assert_eq!(Store::open(&db).unwrap().events(0, 100).unwrap(), events);
    }
}

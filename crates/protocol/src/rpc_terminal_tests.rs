use super::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    let project = json!({"repository_id":"repo-secret","worktree_id":"main","git_head":"head",
        "working_tree_fingerprint":"tree","config_hash":"cfg","ignore_policy_version":"1"});
    let output = |kind: &str| {
        json!({"schema_version":1,"id":kind,"project":project,"graph_version":"graph",
        "analysis_run":"output-run","content_sha256":"a".repeat(64),"byte_length":0,"kind":kind,
        "retention":"evidence","declared_protection":"unreviewed"})
    };
    json!({"schema_version":1,"finished_at_ms":0,"supervised_elapsed_ms":100,
        "spawn":{"schema_version":1,"observed_at_ms":100,"disposition":"spawned","process_id":7,
            "launch":{"schema_version":1,"transport":"stdio_jsonl","id":"launch-secret",
                "task":{"schema_version":1,"id":"task","project":project,"graph_version":"graph",
                    "role":"worker","account_lane":"native","scope":["src"],"dependencies":[],
                    "context_ref":"context","expected_artifacts":["report"],"token_budget":100},
                "origin_lease":{"task_id":"task","owner":"owner","fencing_token":1,"expires_at_ms":1000},
                "host_id":"host","approval_id":"approval","execution_snapshot":project,
                "process":{"program":"fixture","args":[],"cwd":".","executable_sha256":"a".repeat(64),
                    "environment_sha256":"b".repeat(64),"timeout_ms":1000,"cleanup_timeout_ms":100,"stdout_max_bytes":4096,"stderr_max_bytes":4096},
                "connection":{"epoch":"epoch","client_name":"client","client_version":"1","experimental":false,"max_pending":2,"max_frame":4096}}},
        "output_run":{"schema_version":1,"id":"output-run","project":project,"graph_version":"graph",
            "analyzer":"host","analyzer_version":"1","configuration_sha256":"c".repeat(64),"input_manifest_sha256":"d".repeat(64)},
        "stdout":output("stdout"),"stderr":output("stderr"),
        "completion":{"schema_version":1,"reason":"exited","child":{"state":"reaped","exit_code":0},"stdout":"complete","stderr":"complete","cleanup":"unverifiable"},
        "pending":[{"id":2,"method":"secret-method","uncertain":true}],
        "input_progress":{"total":4096,"written":32,"failed":false}})
}
fn decode(v: Value) -> Result<graph_domain::RpcTerminalReceipt, ProtocolError> {
    serde_json::from_value::<RpcTerminalReceipt>(v)
        .unwrap()
        .try_into_domain()
}

#[test]
fn journal_stream_read_is_bounded_and_preserves_io_failure() {
    use crate::rpc_journal::{self, JournalError};
    let receipt = decode(fixture()).unwrap();
    let launch = receipt.spawn().launch();
    let bytes = rpc_journal::encode(&receipt, launch, 65536, 0).unwrap();
    assert_eq!(
        rpc_journal::decode_reader(bytes.as_slice(), launch, bytes.len(), 0).unwrap(),
        receipt
    );
    let mut input = std::io::Cursor::new(vec![b'x'; 100]);
    assert!(matches!(
        rpc_journal::decode_reader(&mut input, launch, 8, 0),
        Err(JournalError::TooLarge)
    ));
    assert_eq!(input.position(), 9);
    struct Missing;
    impl std::io::Read for Missing {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        }
    }
    match rpc_journal::decode_reader(Missing, launch, 8, 0) {
        Err(JournalError::Io(error)) => assert_eq!(error.kind(), std::io::ErrorKind::NotFound),
        other => panic!("expected retained I/O error, got {other:?}"),
    }
}

#[test]
fn journal_manifest_preserves_uncertainty_and_rejects_foreign_or_oversized_input() {
    use crate::rpc_journal::{self, JournalError};
    let receipt = decode(fixture()).unwrap();
    let launch = receipt.spawn().launch();
    let bytes = rpc_journal::encode(&receipt, launch, 65536, 0).unwrap();
    assert_eq!(
        rpc_journal::decode(&bytes, launch, bytes.len(), 0).unwrap(),
        receipt
    );
    assert!(matches!(
        rpc_journal::decode(&bytes, launch, bytes.len() - 1, 0),
        Err(JournalError::TooLarge)
    ));
    assert!(matches!(
        rpc_journal::encode(&receipt, launch, bytes.len() - 1, 0),
        Err(JournalError::TooLarge)
    ));
    let mut foreign = fixture();
    foreign["spawn"]["launch"]["host_id"] = json!("other-host");
    let foreign = decode(foreign).unwrap();
    assert!(matches!(
        rpc_journal::decode(&bytes, foreign.spawn().launch(), 65536, 0),
        Err(JournalError::LaunchMismatch)
    ));
    for field in ["journal_version", "extra"] {
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value[field] = json!(2);
        assert!(
            rpc_journal::decode(&serde_json::to_vec(&value).unwrap(), launch, 65536, 0).is_err()
        );
    }
    assert!(matches!(
        rpc_journal::decode(b"{", launch, 0, 0),
        Err(JournalError::TooLarge)
    ));
    assert!(matches!(
        rpc_journal::decode(b"{", launch, 1, 0),
        Err(JournalError::Json(_))
    ));
    let mut value = fixture();
    value["stdout"]["byte_length"] = json!(3);
    value["stderr"]["byte_length"] = json!(4);
    let receipt = decode(value).unwrap();
    assert!(matches!(
        rpc_journal::encode(&receipt, launch, 65536, 6),
        Err(JournalError::Outputs)
    ));
    let bytes = rpc_journal::encode(&receipt, launch, 65536, 7).unwrap();
    assert!(matches!(
        rpc_journal::decode(&bytes, launch, 65536, 6),
        Err(JournalError::Outputs)
    ));
}

#[test]
fn roundtrip_retains_full_report_and_required_nulls_without_debug_payload() {
    for nullable in [false, true] {
        let mut value = fixture();
        if nullable {
            value["stdout"] = Value::Null;
            value["stderr"] = Value::Null;
            value["input_progress"] = Value::Null;
            value["completion"]["stdout"] = json!("incomplete");
            value["completion"]["stderr"] = json!("truncated");
        }
        let domain = decode(value.clone()).unwrap();
        let wire = RpcTerminalReceipt::from(&domain);
        assert_eq!(serde_json::to_value(&wire).unwrap(), value);
        let encoded = serde_json::to_string(&wire).unwrap();
        assert_eq!(
            serde_json::from_str::<RpcTerminalReceipt>(&encoded)
                .unwrap()
                .try_into_domain()
                .unwrap(),
            domain
        );
        assert_eq!(format!("{wire:?}"), "RpcTerminalReceipt { .. }");
        assert!(!format!("{:?}", wire.pending).contains("secret-method"));
    }
}

#[test]
fn new_objects_require_every_field_reject_duplicates_unknowns_and_wrong_nulls() {
    let original = fixture();
    for pointer in ["", "/pending/0", "/input_progress"] {
        let object = original.pointer(pointer).unwrap();
        for (key, value) in object.as_object().unwrap() {
            let mut missing = original.clone();
            missing
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(key);
            assert!(
                serde_json::from_value::<RpcTerminalReceipt>(missing).is_err(),
                "{pointer}/{key}"
            );
            let raw = serde_json::to_string(object).unwrap();
            let duplicate = format!(
                "{{{}:{value},{}",
                serde_json::to_string(key).unwrap(),
                &raw[1..]
            );
            let rejected = match pointer {
                "" => serde_json::from_str::<RpcTerminalReceipt>(&duplicate).is_err(),
                "/pending/0" => serde_json::from_str::<UncertainRpc>(&duplicate).is_err(),
                _ => serde_json::from_str::<RpcInputProgress>(&duplicate).is_err(),
            };
            assert!(rejected, "{pointer}/{key}");
            if pointer.is_empty() && ["stdout", "stderr", "input_progress"].contains(&key.as_str())
            {
                continue;
            }
            let mut null = original.clone();
            null.pointer_mut(pointer).unwrap()[key] = Value::Null;
            assert!(
                serde_json::from_value::<RpcTerminalReceipt>(null).is_err(),
                "{pointer}/{key}"
            );
        }
        let mut extra = original.clone();
        extra.pointer_mut(pointer).unwrap()["verified"] = json!(true);
        assert!(serde_json::from_value::<RpcTerminalReceipt>(extra).is_err());
    }
}

#[test]
fn nested_schema_and_domain_constraints_are_not_bypassed_by_wire() {
    for pointer in [
        "",
        "/spawn",
        "/spawn/launch",
        "/spawn/launch/task",
        "/output_run",
        "/stdout",
        "/stderr",
        "/completion",
    ] {
        let mut wrong = fixture();
        wrong.pointer_mut(pointer).unwrap()["schema_version"] = json!(2);
        assert!(
            matches!(decode(wrong), Err(ProtocolError::UnsupportedSchema(2))),
            "{pointer}"
        );
    }
    let mut wrong = fixture();
    wrong["schema_version"] = json!(2);
    wrong["pending"][0]["uncertain"] = json!(false);
    assert!(matches!(
        decode(wrong),
        Err(ProtocolError::UnsupportedSchema(2))
    ));
    for (pointer, value) in [
        ("/pending/0/uncertain", json!(false)),
        ("/pending/0/id", json!(0)),
        ("/pending/0/method", json!(" ")),
        ("/stdout/analysis_run", json!("wrong")),
        ("/stderr/byte_length", json!(4097)),
        ("/output_run/project/git_head", json!("wrong")),
        ("/finished_at_ms", json!(-1)),
        ("/supervised_elapsed_ms", json!(u64::MAX)),
        ("/input_progress/written", json!(4097)),
        ("/completion/child", json!({"state":"unreaped"})),
    ] {
        let mut wrong = fixture();
        *wrong.pointer_mut(pointer).unwrap() = value;
        assert!(decode(wrong).is_err(), "{pointer}");
    }
    for ids in [vec![2, 1], vec![1, 1], vec![1, 2, 3]] {
        let mut wrong = fixture();
        wrong["pending"] = json!(
            ids.into_iter()
                .map(|id| json!({"id":id,"method":"m","uncertain":true}))
                .collect::<Vec<_>>()
        );
        assert!(decode(wrong).is_err());
    }
}

#[test]
fn wire_scalar_types_do_not_coerce_uncertainty_ids_or_counters() {
    for (pointer, values) in [
        (
            "/pending/0/id",
            vec![json!("2"), json!(true), json!(1.5), json!(u64::MAX)],
        ),
        (
            "/pending/0/uncertain",
            vec![json!("true"), json!(1), json!({"true":null})],
        ),
        (
            "/input_progress/total",
            vec![json!(-1), json!("1"), json!(0.5)],
        ),
        ("/input_progress/failed", vec![json!("false"), json!(0)]),
        (
            "/finished_at_ms",
            vec![json!(u64::MAX), json!("0"), json!(0.1)],
        ),
    ] {
        for value in values {
            let mut wrong = fixture();
            *wrong.pointer_mut(pointer).unwrap() = value;
            assert!(
                serde_json::from_value::<RpcTerminalReceipt>(wrong).is_err(),
                "{pointer}"
            );
        }
    }
}

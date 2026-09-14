use graph_protocol::native::{CompletedStatus, LifecycleNotification, ThreadStatus};
use serde_json::{Value, json};

fn parse(value: Value) -> Result<LifecycleNotification, serde_json::Error> {
    serde_json::from_value(value)
}

#[test]
fn fragmented_frame_reaches_native_parser_only_after_delimiter() {
    use graph_protocol::framing::LineDecoder;
    let payload = b"{\"method\":\"thread/closed\",\"params\":{\"threadId\":\"child\"}}\n";
    let mut decoder = LineDecoder::new(payload.len()).unwrap();
    for byte in &payload[..payload.len() - 1] {
        assert_eq!(decoder.push(&[*byte]).unwrap(), (1, None));
    }
    let (consumed, frame) = decoder.push(b"\n").unwrap();
    assert_eq!(consumed, 1);
    let event: LifecycleNotification = serde_json::from_slice(&frame.unwrap()).unwrap();
    assert!(matches!(event, LifecycleNotification::ThreadClosed(_)));
    assert_eq!(event.thread_id().as_str(), "child");
    assert_eq!(decoder.finish(), Ok(()));
}

#[test]
fn native_turn_observations_never_become_thread_close() {
    for (status, expected) in [
        ("completed", CompletedStatus::Completed),
        ("interrupted", CompletedStatus::Interrupted),
        ("failed", CompletedStatus::Failed),
    ] {
        let event = parse(json!({"method":"turn/completed","params":{
            "threadId":"child", "turn":{"id":"turn-1", "status":status,
            "items":[{"text":"finished everything"}], "error":{"message":"detail"}}
        }}))
        .unwrap();
        assert_eq!(event.thread_id().as_str(), "child");
        let LifecycleNotification::TurnCompleted(turn) = event else {
            panic!("not turn completion")
        };
        assert_eq!(turn.turn.id.as_str(), "turn-1");
        assert_eq!(turn.turn.status, expected);
    }
    assert!(matches!(
        parse(json!({"method":"turn/started","params":{
            "threadId":"child","turn":{"id":"turn-2","status":"inProgress","items":[]}
        }}))
        .unwrap(),
        LifecycleNotification::TurnStarted(_)
    ));
    let closed = parse(json!({"method":"thread/closed","params":{"threadId":"child"}})).unwrap();
    assert!(matches!(closed, LifecycleNotification::ThreadClosed(_)));
    assert_eq!(closed.thread_id().as_str(), "child");
}

#[test]
fn native_thread_statuses_preserve_distinct_observations() {
    for (status, expected) in [
        ("notLoaded", ThreadStatus::NotLoaded),
        ("idle", ThreadStatus::Idle),
        ("systemError", ThreadStatus::SystemError),
    ] {
        let event = parse(json!({"method":"thread/status/changed","params":{
            "threadId":"child","status":{"type":status}
        }}))
        .unwrap();
        let LifecycleNotification::ThreadStatusChanged(event) = event else {
            panic!("not status")
        };
        assert_eq!(event.status, expected);
    }
    let active = parse(json!({"method":"thread/status/changed","params":{
        "threadId":"child","status":{"type":"active","activeFlags":["waitingOnApproval","waitingOnUserInput"]}
    }})).unwrap();
    let LifecycleNotification::ThreadStatusChanged(event) = active else {
        panic!("not status")
    };
    let ThreadStatus::Active { active_flags } = event.status else {
        panic!("not active")
    };
    assert_eq!(active_flags.len(), 2);
}

#[test]
fn malformed_unknown_and_request_shaped_messages_are_not_lifecycle() {
    for invalid in [
        json!({"method":"thread/closed"}),
        json!({"method":"thread/closed","params":null}),
        json!({"method":"thread/closed","params":{}}),
        json!({"method":"thread/closed","params":{"threadId":42}}),
        json!({"method":"thread/closed","params":{"threadId":" "}}),
        json!({"method":"thread/closed","params":{"threadId":"x".repeat(257)}}),
        json!({"method":"thread/closed","id":1,"params":{"threadId":"child"}}),
        json!({"method":"thread/closed","result":{},"params":{"threadId":"child"}}),
        json!({"method":"future/closed","params":{"threadId":"child"}}),
        json!({"method":"thread/status/changed","params":{"threadId":"child","status":{"type":"finished"}}}),
        json!({"method":"thread/status/changed","params":{"threadId":"child","status":{"type":"active","activeFlags":["future"]}}}),
        json!({"method":"turn/completed","params":{"threadId":"child","turn":{"id":"t","status":"inProgress"}}}),
        json!({"method":"turn/started","params":{"threadId":"child","turn":{"id":"t","status":"completed"}}}),
        json!({"method":"turn/completed","params":{"threadId":"child","turn":{"id":"","status":"failed"}}}),
        json!({"id":1,"result":{}}),
    ] {
        assert!(parse(invalid.clone()).is_err(), "accepted {invalid}");
    }
    // Envelope duplicate keys must not be normalized away by Value first.
    assert!(
        serde_json::from_str::<LifecycleNotification>(
            r#"{"method":"thread/closed","method":"thread/closed","params":{"threadId":"child"}}"#
        )
        .is_err()
    );
}

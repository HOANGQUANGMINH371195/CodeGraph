use serde_json::json;

#[test]
fn artifact_json_is_strict_and_cannot_supply_reference_counts_or_trust() {
    let fixture = json!({"schema_version":1,"id":"artifact1","project":{
        "repository_id":"repo","worktree_id":"w","git_head":"h",
        "working_tree_fingerprint":"s","config_hash":"c","ignore_policy_version":"1"},
        "graph_version":"g1","analysis_run":"r1","content_sha256":"a".repeat(64),
        "byte_length":0,"kind":"stdout","retention":"evidence","declared_protection":"unreviewed"});
    let wire: graph_protocol::Artifact = serde_json::from_value(fixture.clone()).unwrap();
    let domain = wire.try_into_domain().unwrap();
    assert_eq!(
        serde_json::to_value(graph_protocol::Artifact::from(&domain)).unwrap(),
        fixture
    );
    for (field, value) in [
        ("reference_count", json!(0)),
        ("verified", json!(true)),
        ("path", json!("/etc/passwd")),
        ("retention", json!("delete_now")),
        ("declared_protection", json!("verified")),
        ("byte_length", json!(-1)),
    ] {
        let mut invalid = fixture.clone();
        invalid[field] = value;
        assert!(
            serde_json::from_value::<graph_protocol::Artifact>(invalid).is_err(),
            "{field}"
        );
    }
}

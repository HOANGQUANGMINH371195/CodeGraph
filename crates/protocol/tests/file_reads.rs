use graph_protocol::file_reads::Report;

#[test]
fn accepts_the_bounded_candidate_transport_without_promoting_verification() {
    let report: Report = serde_json::from_str(r#"{
      "schemaVersion":1,"kind":"file_reads","status":"observed",
      "sourceCoordinateEncoding":"utf16-code-unit",
      "source":{"path":"src/store.mjs","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","byteLength":1,"lineCount":1},
      "discovery":{"candidates":[]},"targets":[{
        "status":"captured-candidate","runtimeVerified":false,"atomicSnapshotVerified":false,
        "raceFreeContainmentVerified":false,"containmentChecksPassed":true,"requiresStableFilesystem":true,
        "source":{"path":"src/store.mjs","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","byteLength":1,"lineCount":1},
        "target":{"path":"sql/a.sql","sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","byteLength":2,"lineCount":1},"candidateIndex":0
      }],"runtimeVerified":false,"atomicSnapshotVerified":false,"raceFreeContainmentVerified":false,"requiresStableFilesystem":true,"persisted":false
    }"#).unwrap();
    assert!(report.valid_candidate_transport());
    assert_eq!(report.targets[0].candidate_index, 0);
}

#[test]
fn rejects_unknown_transport_fields_and_positive_runtime_claims() {
    let unknown = r#"{"schemaVersion":1,"kind":"file_reads","status":"observed","sourceCoordinateEncoding":"utf16-code-unit","targets":[],"runtimeVerified":false,"atomicSnapshotVerified":false,"raceFreeContainmentVerified":false,"requiresStableFilesystem":true,"persisted":false,"surprise":true}"#;
    assert!(serde_json::from_str::<Report>(unknown).is_err());
    let positive = unknown
        .replace(",\"surprise\":true", "")
        .replace("\"runtimeVerified\":false", "\"runtimeVerified\":true");
    assert!(
        !serde_json::from_str::<Report>(&positive)
            .unwrap()
            .valid_candidate_transport()
    );
}

#[test]
fn binds_candidate_extent_to_the_same_source_hash() {
    let report: Report = serde_json::from_str(r#"{
      "schemaVersion":1,"kind":"file_reads","status":"observed","sourceCoordinateEncoding":"utf16-code-unit",
      "source":{"path":"src/a.mjs","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","byteLength":10,"lineCount":1},
      "discovery":{"coordinateEncoding":"utf16-code-unit","sourceSha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","candidates":[{"status":"candidate","sourceSha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","read":{"invocation":{"start":2,"end":7}}}]},
      "targets":[],"runtimeVerified":false,"atomicSnapshotVerified":false,"raceFreeContainmentVerified":false,"requiresStableFilesystem":true,"persisted":false
    }"#).unwrap();
    assert_eq!(report.candidate_read_extents().unwrap()[0].start, 2);
    let mut mismatched: serde_json::Value = serde_json::from_str(r#"{
      "schemaVersion":1,"kind":"file_reads","status":"observed","sourceCoordinateEncoding":"utf16-code-unit",
      "source":{"path":"src/a.mjs","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","byteLength":10,"lineCount":1},
      "discovery":{"coordinateEncoding":"utf16-code-unit","sourceSha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","candidates":[]},
      "targets":[],"runtimeVerified":false,"atomicSnapshotVerified":false,"raceFreeContainmentVerified":false,"requiresStableFilesystem":true,"persisted":false
    }"#).unwrap();
    assert!(
        serde_json::from_value::<Report>(mismatched.take())
            .unwrap()
            .candidate_read_extents()
            .is_err()
    );
}

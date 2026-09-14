use crate::{ProtocolError, RpcLaunchSpec, SCHEMA_VERSION, require_current_schema};
use graph_domain::RpcSpawnDisposition;
use serde::{Deserialize, Serialize};

/// Untrusted observation. String disposition is explicitly validated, not a
/// serde externally-tagged enum accepting alternate object representations.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcSpawnObservation {
    pub schema_version: u32,
    pub launch: RpcLaunchSpec,
    pub observed_at_ms: i64,
    pub disposition: String,
    #[serde(deserialize_with = "required_process_id")]
    pub process_id: Option<u32>,
}
impl std::fmt::Debug for RpcSpawnObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RpcSpawnObservation { .. }")
    }
}
fn required_process_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    Option::<u32>::deserialize(d)
}
impl RpcSpawnObservation {
    pub fn try_into_domain(self) -> Result<graph_domain::RpcSpawnObservation, ProtocolError> {
        require_current_schema(self.schema_version)?;
        let disposition = match self.disposition.as_str() {
            "cancelled_before_spawn" => RpcSpawnDisposition::CancelledBeforeSpawn,
            "expired_before_spawn" => RpcSpawnDisposition::ExpiredBeforeSpawn,
            "spawn_failed" => RpcSpawnDisposition::SpawnFailed,
            "spawned" => RpcSpawnDisposition::Spawned,
            _ => {
                return Err(
                    graph_domain::DomainError::Invalid("unknown RPC spawn disposition").into(),
                );
            }
        };
        Ok(graph_domain::RpcSpawnObservation::new(
            self.launch.try_into_domain()?,
            self.observed_at_ms,
            disposition,
            self.process_id,
        )?)
    }
}
impl From<&graph_domain::RpcSpawnObservation> for RpcSpawnObservation {
    fn from(value: &graph_domain::RpcSpawnObservation) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            launch: value.launch().into(),
            observed_at_ms: value.observed_at_ms(),
            process_id: value.process_id(),
            disposition: value.disposition().as_str().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn fixture() -> Value {
        let project = json!({"repository_id":"repo-secret","worktree_id":"worktree-secret",
            "git_head":"head","working_tree_fingerprint":"tree","config_hash":"config","ignore_policy_version":"1"});
        json!({"schema_version":1,"observed_at_ms":0,"disposition":"spawn_failed","process_id":null,
            "launch":{"schema_version":1,"transport":"stdio_jsonl","id":"launch-secret",
                "task":{"schema_version":1,"id":"task","project":project.clone(),"graph_version":"graph",
                    "role":"worker","account_lane":"native","scope":["src"],"dependencies":[],
                    "context_ref":"context-secret","expected_artifacts":["report"],"token_budget":100},
                "origin_lease":{"task_id":"task","owner":"owner","fencing_token":1,"expires_at_ms":100},
                "host_id":"host-secret","approval_id":"approval-secret","execution_snapshot":project,
                "process":{"program":"executable-secret","args":["argument-secret"],"cwd":".",
                    "executable_sha256":"a".repeat(64),"environment_sha256":"b".repeat(64),
                    "timeout_ms":1000,"cleanup_timeout_ms":100,"stdout_max_bytes":4096,"stderr_max_bytes":4096},
                "connection":{"epoch":"epoch-secret","client_name":"client","client_version":"1",
                    "experimental":false,"max_pending":2,"max_frame":4096}}})
    }
    fn decode(value: Value) -> Result<graph_domain::RpcSpawnObservation, ProtocolError> {
        serde_json::from_value::<RpcSpawnObservation>(value)
            .unwrap()
            .try_into_domain()
    }

    #[test]
    fn spawn_dispositions_roundtrip_and_debug_does_not_expose_launch() {
        for (label, expected) in [
            (
                "cancelled_before_spawn",
                RpcSpawnDisposition::CancelledBeforeSpawn,
            ),
            (
                "expired_before_spawn",
                RpcSpawnDisposition::ExpiredBeforeSpawn,
            ),
            ("spawn_failed", RpcSpawnDisposition::SpawnFailed),
            ("spawned", RpcSpawnDisposition::Spawned),
        ] {
            for at in [0, i64::MAX] {
                let mut value = fixture();
                value["disposition"] = json!(label);
                value["observed_at_ms"] = json!(at);
                if expected == RpcSpawnDisposition::Spawned {
                    value["process_id"] = json!(u32::MAX);
                }
                let wire: RpcSpawnObservation = serde_json::from_value(value.clone()).unwrap();
                let domain = wire.clone().try_into_domain().unwrap();
                assert_eq!(domain.disposition(), expected);
                assert_eq!(domain.observed_at_ms(), at);
                assert_eq!(
                    domain.process_id(),
                    if expected == RpcSpawnDisposition::Spawned {
                        Some(u32::MAX)
                    } else {
                        None
                    }
                );
                assert_eq!(RpcSpawnObservation::from(&domain), wire);
                let raw = serde_json::to_vec(&wire).unwrap();
                assert_eq!(
                    serde_json::from_slice::<RpcSpawnObservation>(&raw)
                        .unwrap()
                        .try_into_domain()
                        .unwrap(),
                    domain
                );
                assert_eq!(serde_json::to_value(&wire).unwrap(), value);
                assert!(!format!("{wire:?}").contains("secret"));
                assert!(!format!("{domain:?}").contains("secret"));
            }
        }
    }

    #[test]
    fn required_nullable_pid_and_all_outer_fields_reject_missing_unknown_duplicates() {
        let original = fixture();
        let encoded = serde_json::to_string(&original).unwrap();
        for (key, value) in original.as_object().unwrap() {
            let mut missing = original.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(
                serde_json::from_value::<RpcSpawnObservation>(missing).is_err(),
                "{key}"
            );
            let duplicate = format!(
                "{{{}:{value},{}",
                serde_json::to_string(key).unwrap(),
                &encoded[1..]
            );
            assert!(
                serde_json::from_str::<RpcSpawnObservation>(&duplicate).is_err(),
                "{key}"
            );
            if key != "process_id" {
                let mut null = original.clone();
                null[key] = Value::Null;
                assert!(
                    serde_json::from_value::<RpcSpawnObservation>(null).is_err(),
                    "{key}"
                );
            }
        }
        for pointer in ["", "/launch", "/launch/process"] {
            let mut wrong = original.clone();
            wrong.pointer_mut(pointer).unwrap()["execution_verified"] = json!(true);
            assert!(serde_json::from_value::<RpcSpawnObservation>(wrong).is_err());
        }
    }

    #[test]
    fn impossible_disposition_pid_pairs_and_negative_time_fail_in_domain() {
        let launch = decode(fixture()).unwrap().launch().clone();
        for disposition in [
            RpcSpawnDisposition::CancelledBeforeSpawn,
            RpcSpawnDisposition::ExpiredBeforeSpawn,
            RpcSpawnDisposition::SpawnFailed,
            RpcSpawnDisposition::Spawned,
        ] {
            for pid in [None, Some(0), Some(1), Some(u32::MAX)] {
                let valid = if disposition == RpcSpawnDisposition::Spawned {
                    pid.is_some_and(|pid| pid > 0)
                } else {
                    pid.is_none()
                };
                assert_eq!(
                    graph_domain::RpcSpawnObservation::new(launch.clone(), 0, disposition, pid)
                        .is_ok(),
                    valid
                );
            }
        }
        assert!(
            graph_domain::RpcSpawnObservation::new(
                launch,
                -1,
                RpcSpawnDisposition::SpawnFailed,
                None
            )
            .is_err()
        );
        for (label, pid) in [
            ("spawned", Value::Null),
            ("spawned", json!(0)),
            ("spawn_failed", json!(1)),
            ("expired_before_spawn", json!(1)),
            ("cancelled_before_spawn", json!(1)),
        ] {
            let mut wrong = fixture();
            wrong["disposition"] = json!(label);
            wrong["process_id"] = pid;
            assert!(decode(wrong).is_err());
        }
        let mut wrong = fixture();
        wrong["observed_at_ms"] = json!(-1);
        assert!(decode(wrong).is_err());
    }

    #[test]
    fn wire_numeric_types_and_alternate_disposition_forms_are_rejected() {
        for (key, bad) in [
            ("process_id", json!(-1)),
            ("process_id", json!(u64::from(u32::MAX) + 1)),
            ("process_id", json!(1.5)),
            ("process_id", json!("1")),
            ("process_id", json!(true)),
            ("observed_at_ms", json!(u64::MAX)),
            ("observed_at_ms", json!(0.1)),
            ("observed_at_ms", json!("0")),
            ("disposition", json!({"spawn_failed":null})),
            ("disposition", json!(false)),
        ] {
            let mut wrong = fixture();
            wrong[key] = bad;
            assert!(
                serde_json::from_value::<RpcSpawnObservation>(wrong).is_err(),
                "{key}"
            );
        }
        for bad in ["", "Spawned", "completed", "secret-invalid-disposition"] {
            let mut wrong = fixture();
            wrong["disposition"] = json!(bad);
            let error = decode(wrong).unwrap_err();
            assert!(!error.to_string().contains("secret"));
        }
    }

    #[test]
    fn schema_checks_precede_domain_conversion_and_nested_launch_is_validated() {
        for version in [0, 2, u32::MAX] {
            let mut wrong = fixture();
            wrong["schema_version"] = json!(version);
            wrong["disposition"] = json!("bad");
            assert!(
                matches!(decode(wrong), Err(ProtocolError::UnsupportedSchema(v)) if v == version)
            );
            let mut wrong = fixture();
            wrong["launch"]["schema_version"] = json!(version);
            assert!(
                matches!(decode(wrong), Err(ProtocolError::UnsupportedSchema(v)) if v == version)
            );
        }
        let mut wrong = fixture();
        wrong["launch"]["origin_lease"]["task_id"] = json!("other");
        assert!(decode(wrong).is_err());
    }
}

//! Untrusted descriptions for bounded JSONL RPC; conversion grants no spawn authority.

use crate::{Lease, ProjectRef, ProtocolError, SCHEMA_VERSION, TaskSpec, require_current_schema};
use serde::{Deserialize, Serialize};

/// RPC requires writable stdin; closed-stdin check commands are a separate contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RpcTransport {
    #[serde(rename = "stdio_jsonl")]
    StdioJsonl,
}

impl<'de> Deserialize<'de> for RpcTransport {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Require the string spelling; Serde's derived enum decoder also accepts
        // externally tagged objects such as {"stdio_jsonl": null}.
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "stdio_jsonl" => Ok(Self::StdioJsonl),
            _ => Err(serde::de::Error::custom("unsupported RPC transport")),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcProcessSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub executable_sha256: String,
    pub environment_sha256: String,
    pub timeout_ms: u64,
    pub cleanup_timeout_ms: u64,
    pub stdout_max_bytes: u64,
    pub stderr_max_bytes: u64,
}

impl std::fmt::Debug for RpcProcessSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcProcessSpec")
            .field("argument_count", &self.args.len())
            .finish_non_exhaustive()
    }
}

impl RpcProcessSpec {
    pub fn try_into_domain(self) -> Result<graph_domain::RpcProcessSpec, ProtocolError> {
        Ok(graph_domain::RpcProcessSpec::new(
            self.program,
            self.args,
            self.cwd,
            self.executable_sha256,
            self.environment_sha256,
            self.timeout_ms,
            self.cleanup_timeout_ms,
            self.stdout_max_bytes,
            self.stderr_max_bytes,
        )?)
    }
}

impl From<&graph_domain::RpcProcessSpec> for RpcProcessSpec {
    fn from(value: &graph_domain::RpcProcessSpec) -> Self {
        Self {
            program: value.program().into(),
            args: value.args().to_vec(),
            cwd: value.cwd().into(),
            executable_sha256: value.executable_sha256().into(),
            environment_sha256: value.environment_sha256().into(),
            timeout_ms: value.timeout_ms(),
            cleanup_timeout_ms: value.cleanup_timeout_ms(),
            stdout_max_bytes: value.stdout_max_bytes(),
            stderr_max_bytes: value.stderr_max_bytes(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcConnectionSpec {
    pub epoch: String,
    pub client_name: String,
    pub client_version: String,
    pub experimental: bool,
    pub max_pending: u32,
    pub max_frame: u32,
}

impl std::fmt::Debug for RpcConnectionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcConnectionSpec")
            .field("max_pending", &self.max_pending)
            .field("max_frame", &self.max_frame)
            .finish_non_exhaustive()
    }
}

impl RpcConnectionSpec {
    pub fn try_into_domain(self) -> Result<graph_domain::RpcConnectionSpec, ProtocolError> {
        Ok(graph_domain::RpcConnectionSpec::new(
            self.epoch,
            self.client_name,
            self.client_version,
            self.experimental,
            self.max_pending,
            self.max_frame,
        )?)
    }
}

impl From<&graph_domain::RpcConnectionSpec> for RpcConnectionSpec {
    fn from(value: &graph_domain::RpcConnectionSpec) -> Self {
        Self {
            epoch: value.epoch().into(),
            client_name: value.client_name().into(),
            client_version: value.client_version().into(),
            experimental: value.experimental(),
            max_pending: value.max_pending(),
            max_frame: value.max_frame(),
        }
    }
}

/// Every identity, approval, and snapshot remains a claim for the host to verify.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcLaunchSpec {
    pub schema_version: u32,
    pub transport: RpcTransport,
    pub id: String,
    pub task: TaskSpec,
    pub origin_lease: Lease,
    pub host_id: String,
    pub approval_id: String,
    pub execution_snapshot: ProjectRef,
    pub process: RpcProcessSpec,
    pub connection: RpcConnectionSpec,
}

impl std::fmt::Debug for RpcLaunchSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcLaunchSpec").finish_non_exhaustive()
    }
}

impl RpcLaunchSpec {
    pub fn try_into_domain(self) -> Result<graph_domain::RpcLaunchSpec, ProtocolError> {
        require_current_schema(self.schema_version)?;
        // Exhaustive match prevents a future wire transport silently losing its meaning.
        match self.transport {
            RpcTransport::StdioJsonl => {}
        }
        Ok(graph_domain::RpcLaunchSpec::new(
            self.id,
            self.task.try_into_domain()?,
            self.origin_lease.try_into_domain()?,
            self.host_id,
            self.approval_id,
            self.execution_snapshot.into(),
            self.process.try_into_domain()?,
            self.connection.try_into_domain()?,
        )?)
    }
}

impl From<&graph_domain::RpcLaunchSpec> for RpcLaunchSpec {
    fn from(value: &graph_domain::RpcLaunchSpec) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            transport: RpcTransport::StdioJsonl,
            id: value.id().into(),
            task: value.task().into(),
            origin_lease: value.origin_lease().into(),
            host_id: value.host_id().into(),
            approval_id: value.approval_id().into(),
            execution_snapshot: value.execution_snapshot().into(),
            process: value.process().into(),
            connection: value.connection().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn fixture() -> Value {
        let project = json!({"repository_id":"repo-secret","worktree_id":"source-secret",
            "git_head":"head-secret","working_tree_fingerprint":"tree-secret",
            "config_hash":"config-secret","ignore_policy_version":"policy-secret"});
        let mut snapshot = project.clone();
        snapshot["worktree_id"] = json!("execution-secret");
        snapshot["git_head"] = json!("execution-head-secret");
        snapshot["working_tree_fingerprint"] = json!("execution-tree-secret");
        json!({"schema_version":1,"transport":"stdio_jsonl","id":"launch-secret",
            "task":{"schema_version":1,"id":"task-secret","project":project,
                "graph_version":"graph-secret","role":"worker-secret","account_lane":"native",
                "scope":["src"],"dependencies":["dependency-secret"],"context_ref":"context-secret",
                "expected_artifacts":["patch"],"token_budget":100},
            "origin_lease":{"task_id":"task-secret","owner":"owner-secret",
                "fencing_token":2,"expires_at_ms":100},
            "host_id":"host-secret","approval_id":"approval-secret","execution_snapshot":snapshot,
            "process":{"program":"program-secret","args":["","a b","$(literal-secret);*","tiếng Việt","line\nnext"],
                "cwd":"directory-secret","executable_sha256":"a".repeat(64),"environment_sha256":"b".repeat(64),
                "timeout_ms":1000,"cleanup_timeout_ms":500,"stdout_max_bytes":1024,"stderr_max_bytes":2048},
            "connection":{"epoch":"epoch-secret","client_name":"client-secret","client_version":"version-secret",
                "experimental":false,"max_pending":16,"max_frame":4096}})
    }

    fn decode(value: Value) -> Result<graph_domain::RpcLaunchSpec, ProtocolError> {
        serde_json::from_value::<RpcLaunchSpec>(value)
            .unwrap()
            .try_into_domain()
    }

    #[test]
    fn exact_roundtrip_and_redacted_debug() {
        for experimental in [false, true] {
            let mut original = fixture();
            original["connection"]["experimental"] = json!(experimental);
            let wire: RpcLaunchSpec = serde_json::from_value(original.clone()).unwrap();
            let domain = wire.clone().try_into_domain().unwrap();
            assert_eq!(domain.process().args(), wire.process.args);
            assert_eq!(RpcLaunchSpec::from(&domain), wire);
            assert_eq!(serde_json::to_value(&wire).unwrap(), original);
            let encoded = serde_json::to_vec(&wire).unwrap();
            let decoded: RpcLaunchSpec = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(decoded.try_into_domain().unwrap(), domain);
            assert_eq!(format!("{wire:?}"), "RpcLaunchSpec { .. }");
            assert_eq!(format!("{domain:?}"), "RpcLaunchSpec { .. }");
            for debug in [
                format!("{:?}", wire.process),
                format!("{:?}", wire.connection),
                format!("{:?}", domain.process()),
                format!("{:?}", domain.connection()),
            ] {
                for sensitive in [
                    "secret",
                    "a b",
                    "tiếng Việt",
                    "line",
                    &"a".repeat(64),
                    &"b".repeat(64),
                ] {
                    assert!(!debug.contains(sensitive), "{debug}");
                }
            }
        }
    }

    const BOUNDARIES: [&str; 7] = [
        "",
        "/task",
        "/task/project",
        "/origin_lease",
        "/execution_snapshot",
        "/process",
        "/connection",
    ];

    #[test]
    fn every_field_is_required_and_non_null_at_every_object_boundary() {
        let original = fixture();
        for path in BOUNDARIES {
            for key in original.pointer(path).unwrap().as_object().unwrap().keys() {
                let mut missing = original.clone();
                missing
                    .pointer_mut(path)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(key);
                assert!(
                    serde_json::from_value::<RpcLaunchSpec>(missing).is_err(),
                    "{path}/{key} missing"
                );
                let mut null = original.clone();
                null.pointer_mut(path).unwrap()[key] = Value::Null;
                assert!(
                    serde_json::from_value::<RpcLaunchSpec>(null).is_err(),
                    "{path}/{key} null"
                );
            }
        }
    }

    #[test]
    fn unknown_fields_and_authority_claims_are_rejected_at_every_boundary() {
        for path in BOUNDARIES {
            for key in [
                "unknown",
                "approved",
                "verified",
                "spawn_authority",
                "stdin",
                "env",
            ] {
                let mut extra = fixture();
                extra.pointer_mut(path).unwrap()[key] = json!(true);
                assert!(
                    serde_json::from_value::<RpcLaunchSpec>(extra).is_err(),
                    "{path}/{key}"
                );
            }
        }
        for path in ["/process", "/connection"] {
            let mut extra = fixture();
            extra.pointer_mut(path).unwrap()["schema_version"] = json!(1);
            assert!(serde_json::from_value::<RpcLaunchSpec>(extra).is_err());
        }
    }

    #[test]
    fn duplicate_json_fields_are_rejected_at_every_boundary() {
        let original = fixture();
        let encoded = serde_json::to_string(&original).unwrap();
        for path in BOUNDARIES {
            let object = original.pointer(path).unwrap();
            let object_json = serde_json::to_string(object).unwrap();
            // Use raw JSON: Value would erase duplicate fields before deserialization.
            assert_eq!(encoded.matches(&object_json).count(), 1, "{path}");
            for (key, value) in object.as_object().unwrap() {
                let duplicate = format!(
                    "{{{}:{value},{}",
                    serde_json::to_string(key).unwrap(),
                    &object_json[1..]
                );
                let malformed = encoded.replacen(&object_json, &duplicate, 1);
                let error = serde_json::from_str::<RpcLaunchSpec>(&malformed).unwrap_err();
                assert!(
                    error.to_string().contains("duplicate field"),
                    "{path}/{key}: {error}"
                );
            }
        }
    }

    #[test]
    fn unsupported_schema_and_transport_cannot_become_domain_specs() {
        for version in [0, 2, u32::MAX] {
            let mut wrong = fixture();
            wrong["schema_version"] = json!(version);
            wrong["process"]["program"] = json!("");
            assert!(
                matches!(decode(wrong), Err(ProtocolError::UnsupportedSchema(v)) if v == version)
            );
            let mut wrong = fixture();
            wrong["task"]["schema_version"] = json!(version);
            assert!(
                matches!(decode(wrong), Err(ProtocolError::UnsupportedSchema(v)) if v == version)
            );
        }
        for transport in [
            json!("null"),
            Value::Null,
            json!("stdio"),
            json!("StdioJsonl"),
            json!("content_length"),
            json!("pty"),
            json!(""),
            json!(true),
            json!({"stdio_jsonl":null}),
        ] {
            let mut wrong = fixture();
            wrong["transport"] = transport.clone();
            assert!(
                serde_json::from_value::<RpcLaunchSpec>(wrong).is_err(),
                "{transport}"
            );
        }
        for path in ["", "/process"] {
            let mut wrong = fixture();
            wrong.pointer_mut(path).unwrap()["stdin"] = json!("null");
            assert!(serde_json::from_value::<RpcLaunchSpec>(wrong).is_err());
        }
    }

    #[test]
    fn unsupported_transport_error_does_not_echo_supplied_string() {
        let secret = "secret-like-transport-credential-123";
        let mut wrong = fixture();
        wrong["transport"] = json!(secret);
        let encoded = serde_json::to_string(&wrong).unwrap();
        let error = serde_json::from_str::<RpcLaunchSpec>(&encoded)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsupported RPC transport"));
        assert!(!error.contains(secret));
        // Other Serde diagnostics still require host-side bounds and redaction.
    }

    fn rejects_domain_change(path: &str, value: Value) {
        let mut wrong = fixture();
        *wrong.pointer_mut(path).unwrap() = value;
        assert!(
            matches!(decode(wrong), Err(ProtocolError::Domain(_))),
            "{path}"
        );
    }

    #[test]
    fn malformed_process_fields_are_revalidated_by_domain() {
        for program in ["", " ", "bad\0program"] {
            rejects_domain_change("/process/program", json!(program));
        }
        rejects_domain_change("/process/args", json!(["bad\0arg"]));
        for cwd in [
            "", "/tmp", "../src", "a/../b", "a//b", "a/./b", "a/", "C:/repo", "a\\b", "a\0b",
        ] {
            rejects_domain_change("/process/cwd", json!(cwd));
        }
        for path in ["/process/executable_sha256", "/process/environment_sha256"] {
            for hash in [
                "A".repeat(64),
                "g".repeat(64),
                "a".repeat(63),
                String::new(),
            ] {
                rejects_domain_change(path, json!(hash));
            }
        }
        for path in ["/process/timeout_ms", "/process/cleanup_timeout_ms"] {
            for value in [0, u64::MAX, i64::MAX as u64] {
                rejects_domain_change(path, json!(value));
            }
        }
        for path in ["/process/stdout_max_bytes", "/process/stderr_max_bytes"] {
            rejects_domain_change(path, json!(u64::MAX));
            let mut valid = fixture();
            *valid.pointer_mut(path).unwrap() = json!(0);
            assert!(decode(valid).is_ok());
        }
    }

    #[test]
    fn malformed_connection_fields_are_revalidated_by_domain() {
        for path in [
            "/connection/epoch",
            "/connection/client_name",
            "/connection/client_version",
        ] {
            for value in [
                String::new(),
                " ".into(),
                "bad\0label".into(),
                "bad\nlabel".into(),
                "x".repeat(257),
            ] {
                rejects_domain_change(path, json!(value));
            }
        }
        for (path, maximum) in [
            ("/connection/max_pending", 4096_u32),
            ("/connection/max_frame", 16 * 1024 * 1024),
        ] {
            for value in [0, maximum + 1, u32::MAX] {
                rejects_domain_change(path, json!(value));
            }
            for value in [1, maximum] {
                let mut valid = fixture();
                *valid.pointer_mut(path).unwrap() = json!(value);
                assert!(decode(valid).is_ok(), "{path}={value}");
            }
        }
    }

    #[test]
    fn numeric_widths_and_boolean_types_are_strict() {
        for path in [
            "/schema_version",
            "/connection/max_pending",
            "/connection/max_frame",
        ] {
            for value in [
                json!(-1),
                json!(u64::from(u32::MAX) + 1),
                json!(1.5),
                json!("1"),
            ] {
                let mut wrong = fixture();
                *wrong.pointer_mut(path).unwrap() = value;
                assert!(
                    serde_json::from_value::<RpcLaunchSpec>(wrong).is_err(),
                    "{path}"
                );
            }
        }
        for path in [
            "/process/timeout_ms",
            "/process/cleanup_timeout_ms",
            "/process/stdout_max_bytes",
            "/process/stderr_max_bytes",
        ] {
            for value in [json!(-1), json!(1.5), json!("1")] {
                let mut wrong = fixture();
                *wrong.pointer_mut(path).unwrap() = value;
                assert!(
                    serde_json::from_value::<RpcLaunchSpec>(wrong).is_err(),
                    "{path}"
                );
            }
        }
        for value in [json!(0), json!(1), json!("false")] {
            let mut wrong = fixture();
            wrong["connection"]["experimental"] = value;
            assert!(serde_json::from_value::<RpcLaunchSpec>(wrong).is_err());
        }
    }

    #[test]
    fn well_formed_claims_roundtrip_without_authenticating_them() {
        let mut claims = fixture();
        for path in [
            "/id",
            "/host_id",
            "/approval_id",
            "/connection/epoch",
            "/connection/client_name",
            "/connection/client_version",
        ] {
            *claims.pointer_mut(path).unwrap() = json!("x".repeat(256));
        }
        claims["connection"]["max_frame"] = json!(16 * 1024 * 1024);
        let domain = decode(claims.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(RpcLaunchSpec::from(&domain)).unwrap(),
            claims
        );
        // No host, approval ledger, clock, or executable was consulted by conversion.
    }

    #[test]
    fn malformed_identity_lease_task_and_snapshot_are_revalidated_by_domain() {
        for path in ["/id", "/host_id", "/approval_id"] {
            for value in [
                String::new(),
                " ".into(),
                "bad\0label".into(),
                "bad\nlabel".into(),
                "x".repeat(257),
            ] {
                rejects_domain_change(path, json!(value));
            }
        }
        for path in [
            "/task/id",
            "/task/graph_version",
            "/task/role",
            "/task/account_lane",
            "/task/context_ref",
            "/origin_lease/task_id",
            "/origin_lease/owner",
        ] {
            rejects_domain_change(path, json!(""));
        }
        rejects_domain_change("/task/token_budget", json!(0));
        rejects_domain_change("/task/dependencies", json!(["task-secret"]));
        rejects_domain_change("/origin_lease/task_id", json!("other-task"));
        for path in ["/origin_lease/fencing_token", "/origin_lease/expires_at_ms"] {
            for value in [-1, 0] {
                rejects_domain_change(path, json!(value));
            }
        }
        for boundary in ["/task/project", "/execution_snapshot"] {
            for key in fixture()
                .pointer(boundary)
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
            {
                rejects_domain_change(&format!("{boundary}/{key}"), json!(""));
            }
        }
        for key in ["repository_id", "config_hash", "ignore_policy_version"] {
            rejects_domain_change(
                &format!("/execution_snapshot/{key}"),
                json!("different-policy"),
            );
        }
    }
}

use crate::{
    Artifact, CheckCommand, Lease, ProtocolError, RequiredChecks, SCHEMA_VERSION, TaskSpec,
    require_current_schema,
};
use serde::{Deserialize, Serialize};

/// Untrusted execution request provenance; host must validate against its ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckRunBinding {
    pub schema_version: u32,
    pub run_id: String,
    pub task: TaskSpec,
    pub origin_lease: Lease,
    pub submission_sequence: i64,
    pub candidate: Artifact,
    pub policy: RequiredChecks,
    pub check_name: String,
    pub command: CheckCommand,
}
impl CheckRunBinding {
    pub fn try_into_domain(self) -> Result<graph_domain::CheckRunBinding, ProtocolError> {
        require_current_schema(self.schema_version)?;
        Ok(graph_domain::CheckRunBinding::new(
            self.run_id,
            self.task.try_into_domain()?,
            self.origin_lease.try_into_domain()?,
            self.submission_sequence,
            self.candidate.try_into_domain()?,
            self.policy.try_into_domain()?,
            self.check_name,
            self.command.try_into_domain()?,
        )?)
    }
}
impl From<&graph_domain::CheckRunBinding> for CheckRunBinding {
    fn from(value: &graph_domain::CheckRunBinding) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            run_id: value.run_id().into(),
            task: value.task().into(),
            origin_lease: value.origin_lease().into(),
            submission_sequence: value.submission_sequence(),
            candidate: value.candidate().into(),
            policy: value.policy().into(),
            check_name: value.check_name().into(),
            command: value.command().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> serde_json::Value {
        let project = json!({"repository_id":"repo","worktree_id":"w","git_head":"h",
            "working_tree_fingerprint":"s","config_hash":"c","ignore_policy_version":"1"});
        json!({"schema_version":1,"run_id":"check-run-1",
            "task":{"schema_version":1,"id":"task","project":project,
                "graph_version":"g","role":"worker","account_lane":"native",
                "scope":["src"],"dependencies":[],"context_ref":"context",
                "expected_artifacts":["patch"],"token_budget":100},
            "origin_lease":{"task_id":"task","owner":"worker","fencing_token":2,"expires_at_ms":100},
            "submission_sequence":3,
            "candidate":{"schema_version":1,"id":"candidate","project":project,
                "graph_version":"g","analysis_run":"r1","content_sha256":"a".repeat(64),
                "byte_length":3,"kind":"patch","retention":"evidence","declared_protection":"unreviewed"},
            "policy":{"schema_version":1,"names":["test"]},"check_name":"test",
            "command":{"schema_version":1,"program":"fixture","args":[],"cwd":".",
                "executable_sha256":"b".repeat(64),"environment_sha256":"c".repeat(64),
                "timeout_ms":1000,"cleanup_timeout_ms":500,"stdout_max_bytes":1024,"stderr_max_bytes":1024}})
    }

    fn decode(value: serde_json::Value) -> Result<graph_domain::CheckRunBinding, ProtocolError> {
        serde_json::from_value::<CheckRunBinding>(value)
            .unwrap()
            .try_into_domain()
    }

    fn receipt_fixture() -> serde_json::Value {
        let binding = fixture();
        let mut snapshot = binding["task"]["project"].clone();
        snapshot["worktree_id"] = json!("execution-worktree");
        snapshot["working_tree_fingerprint"] = json!("candidate-applied");
        let output = |kind: &str| {
            let mut artifact = binding["candidate"].clone();
            artifact["id"] = json!(kind);
            artifact["kind"] = json!(kind);
            artifact["analysis_run"] = binding["run_id"].clone();
            artifact["project"] = snapshot.clone();
            artifact
        };
        json!({"schema_version":1,"binding":binding,"host_id":"host-epoch-1",
            "execution_snapshot":snapshot,"started_at_ms":200,"finished_at_ms":210,
            "elapsed_ms":10,"stdout":output("stdout"),"stderr":output("stderr"),
            "completion":{"schema_version":1,"reason":"exited",
                "child":{"state":"reaped","exit_code":0},
                "stdout":"complete","stderr":"complete","cleanup":"complete"}})
    }

    fn receipt(value: serde_json::Value) -> Result<graph_domain::ExecutionReceipt, ProtocolError> {
        serde_json::from_value::<crate::ExecutionReceipt>(value)
            .unwrap()
            .try_into_domain()
    }

    #[test]
    fn execution_receipt_roundtrip_retains_snapshot_outputs_and_clock_observations() {
        let original = receipt_fixture();
        let report = receipt(original.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(crate::ExecutionReceipt::from(&report)).unwrap(),
            original
        );
        let mut reversed = original.clone();
        reversed["finished_at_ms"] = json!(199);
        assert!(
            receipt(reversed).is_ok(),
            "wall clock reversal must not erase observations"
        );
        let mut missing = original;
        missing["stdout"] = json!(null);
        assert!(receipt(missing.clone()).is_err());
        missing["completion"]["stdout"] = json!("read_failed");
        let report = receipt(missing).unwrap();
        assert_eq!(
            report.completion().reported_check_outcome(),
            graph_domain::CheckOutcome::Unknown
        );
    }

    #[test]
    fn execution_receipt_rejects_output_substitution_and_invalid_timing() {
        for stream in ["stdout", "stderr"] {
            for (field, value) in [
                ("id", json!("candidate")),
                ("kind", json!("patch")),
                ("analysis_run", json!("another-run")),
                ("graph_version", json!("other")),
                ("byte_length", json!(1025)),
                ("schema_version", json!(2)),
            ] {
                let mut wrong = receipt_fixture();
                wrong[stream][field] = value;
                assert!(receipt(wrong).is_err(), "{stream}.{field}");
            }
            for field in [
                "repository_id",
                "worktree_id",
                "git_head",
                "working_tree_fingerprint",
                "config_hash",
                "ignore_policy_version",
            ] {
                let mut wrong = receipt_fixture();
                wrong[stream]["project"][field] = json!("other");
                assert!(receipt(wrong).is_err(), "{stream}.project.{field}");
            }
        }
        for (field, value) in [
            ("host_id", json!(" ")),
            ("started_at_ms", json!(-1)),
            ("finished_at_ms", json!(-1)),
            ("elapsed_ms", json!(u64::MAX)),
            ("schema_version", json!(2)),
        ] {
            let mut wrong = receipt_fixture();
            wrong[field] = value;
            assert!(receipt(wrong).is_err(), "{field}");
        }
    }

    #[test]
    fn execution_receipt_requires_explicit_fields_and_rejects_authority_claims() {
        let original = receipt_fixture();
        for field in original.as_object().unwrap().keys() {
            let mut missing = original.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<crate::ExecutionReceipt>(missing).is_err(),
                "{field}"
            );
        }
        let mut extra = original;
        extra["verified"] = json!(true);
        assert!(serde_json::from_value::<crate::ExecutionReceipt>(extra).is_err());
    }

    #[test]
    fn binding_roundtrip_preserves_complete_provenance() {
        let original = fixture();
        let binding = decode(original.clone()).unwrap();
        assert_eq!(binding.run_id(), "check-run-1");
        assert_eq!(binding.origin_lease().fencing_token(), 2);
        assert_eq!(binding.origin_lease().owner().as_str(), "worker");
        assert_eq!(binding.submission_sequence(), 3);
        assert_eq!(
            serde_json::to_value(CheckRunBinding::from(&binding)).unwrap(),
            original
        );
        // Constructor has no wall clock: origin lease is provenance, not current permission.
        let mut other = fixture();
        other["origin_lease"]["expires_at_ms"] = json!(1);
        assert!(decode(other).is_ok());
    }

    #[test]
    fn mismatched_scope_lease_and_check_are_rejected() {
        for field in [
            "repository_id",
            "worktree_id",
            "git_head",
            "working_tree_fingerprint",
            "config_hash",
            "ignore_policy_version",
        ] {
            let mut wrong = fixture();
            wrong["candidate"]["project"][field] = json!("different");
            assert!(decode(wrong).is_err(), "{field}");
        }
        let mut wrong = fixture();
        wrong["candidate"]["graph_version"] = json!("different");
        assert!(decode(wrong).is_err());
        for (field, value) in [
            ("task_id", json!("different")),
            ("fencing_token", json!(0)),
            ("expires_at_ms", json!(0)),
            ("expires_at_ms", json!(-1)),
        ] {
            let mut wrong = fixture();
            wrong["origin_lease"][field] = value;
            assert!(decode(wrong).is_err());
        }
        for (field, value) in [
            ("run_id", json!(" ")),
            ("check_name", json!("other")),
            ("submission_sequence", json!(0)),
            ("submission_sequence", json!(-1)),
        ] {
            let mut wrong = fixture();
            wrong[field] = value;
            assert!(decode(wrong).is_err());
        }
    }

    #[test]
    fn future_nested_schema_or_extra_authority_is_not_accepted() {
        for section in ["task", "candidate", "policy", "command"] {
            let mut future = fixture();
            future[section]["schema_version"] = json!(2);
            assert!(decode(future).is_err());
        }
        let mut future = fixture();
        future["schema_version"] = json!(2);
        assert!(decode(future).is_err());
        let original = fixture();
        for key in original.as_object().unwrap().keys() {
            let mut missing = original.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<CheckRunBinding>(missing).is_err());
        }
        let mut extra = original;
        extra["verified"] = json!(true);
        assert!(serde_json::from_value::<CheckRunBinding>(extra).is_err());
    }
}

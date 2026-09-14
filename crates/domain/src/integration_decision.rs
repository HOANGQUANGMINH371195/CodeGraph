use std::collections::HashSet;

use crate::{Artifact, CheckObservation, DomainError, ExecutionReceipt, RequiredChecks, TaskSpec};

/// A complete, structurally valid set of check claims for one candidate.
///
/// This is deliberately not named `VerifiedIntegration`: receipt completion is
/// still a producer claim. The application verifier must authenticate the
/// target and receipt/output observations before asking the store to commit an
/// integration decision atomically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrationDecision {
    task: TaskSpec,
    candidate: Artifact,
    policy: RequiredChecks,
    receipts: Vec<ExecutionReceipt>,
    accepted_at_ms: i64,
    verifier_version: String,
}

impl IntegrationDecision {
    pub fn new(
        task: TaskSpec,
        candidate: Artifact,
        policy: RequiredChecks,
        receipts: Vec<ExecutionReceipt>,
        accepted_at_ms: i64,
        verifier_version: String,
    ) -> Result<Self, DomainError> {
        task.validate()?;
        if accepted_at_ms < 0 {
            return Err(DomainError::Invalid(
                "integration decision time is negative",
            ));
        }
        crate::validate_text("integration verifier version", &verifier_version)?;
        if candidate.project() != task.project()
            || candidate.graph_version() != task.graph_version()
        {
            return Err(DomainError::Invalid(
                "integration candidate does not match task snapshot",
            ));
        }

        let mut run_ids = HashSet::with_capacity(receipts.len());
        let mut observations = Vec::with_capacity(receipts.len());
        for receipt in &receipts {
            let binding = receipt.binding();
            if binding.task() != &task
                || binding.candidate() != &candidate
                || binding.policy() != &policy
            {
                return Err(DomainError::Invalid(
                    "integration receipt binding differs from decision",
                ));
            }
            if !run_ids.insert(binding.run_id()) {
                return Err(DomainError::Invalid("duplicate integration receipt run id"));
            }
            observations.push(CheckObservation {
                name: binding.check_name().to_owned(),
                candidate: candidate.clone(),
                outcome: receipt.completion().reported_check_outcome(),
                // The run ID is nonempty by CheckRunBinding construction. This
                // establishes exact policy membership only, not execution proof.
                evidence_ref: binding.run_id().to_owned(),
            });
        }
        if !policy.evaluate(&candidate, &observations).is_empty() {
            return Err(DomainError::Invalid(
                "integration receipts do not exactly satisfy required checks",
            ));
        }
        Ok(Self {
            task,
            candidate,
            policy,
            receipts,
            accepted_at_ms,
            verifier_version,
        })
    }

    pub fn task(&self) -> &TaskSpec {
        &self.task
    }
    pub fn candidate(&self) -> &Artifact {
        &self.candidate
    }
    pub fn policy(&self) -> &RequiredChecks {
        &self.policy
    }
    pub fn receipts(&self) -> &[ExecutionReceipt] {
        &self.receipts
    }
    pub fn accepted_at_ms(&self) -> i64 {
        self.accepted_at_ms
    }
    pub fn verifier_version(&self) -> &str {
        &self.verifier_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{
        ChildCompletion, ExecutionCompletion, ScopeCleanup, StopReason, StreamCompletion,
    };
    use crate::{
        ArtifactProtection, ArtifactRetention, CheckCommand, CheckRunBinding, Lease, ProjectRef,
        TaskId, WorkerId,
    };

    fn project() -> ProjectRef {
        ProjectRef {
            repository_id: "repo".into(),
            worktree_id: "worktree".into(),
            git_head: "head".into(),
            working_tree_fingerprint: "clean".into(),
            config_hash: "config".into(),
            ignore_policy_version: "v1".into(),
        }
    }
    fn task() -> TaskSpec {
        TaskSpec::new(
            TaskId::new("task").unwrap(),
            project(),
            "graph".into(),
            "worker".into(),
            "lane".into(),
            vec!["src".into()],
            vec![],
            "context".into(),
            vec!["patch".into()],
            1,
        )
        .unwrap()
    }
    fn artifact(id: &str, project: ProjectRef, graph: &str, run: &str, kind: &str) -> Artifact {
        Artifact::new(
            id.into(),
            project,
            graph.into(),
            run.into(),
            "a".repeat(64),
            1,
            kind.into(),
            ArtifactRetention::Evidence,
            ArtifactProtection::Unreviewed,
        )
        .unwrap()
    }
    fn receipt(
        task: &TaskSpec,
        candidate: &Artifact,
        policy: &RequiredChecks,
        check: &str,
        run: &str,
        exit: i32,
    ) -> ExecutionReceipt {
        let command = CheckCommand::new(
            "fixture".into(),
            vec![],
            ".".into(),
            "b".repeat(64),
            "c".repeat(64),
            1,
            1,
            10,
            10,
        )
        .unwrap();
        let binding = CheckRunBinding::new(
            run.into(),
            task.clone(),
            Lease::issue(task.id().clone(), WorkerId::new("worker").unwrap(), 1, 10).unwrap(),
            1,
            candidate.clone(),
            policy.clone(),
            check.into(),
            command,
        )
        .unwrap();
        let mut snapshot = task.project().clone();
        snapshot.worktree_id = format!("execution-{run}");
        snapshot.working_tree_fingerprint = format!("applied-{run}");
        ExecutionReceipt::new(
            binding,
            "host".into(),
            snapshot.clone(),
            1,
            2,
            1,
            Some(artifact(
                &format!("{run}-stdout"),
                snapshot.clone(),
                task.graph_version(),
                run,
                "stdout",
            )),
            Some(artifact(
                &format!("{run}-stderr"),
                snapshot,
                task.graph_version(),
                run,
                "stderr",
            )),
            ExecutionCompletion {
                reason: StopReason::Exited,
                child: ChildCompletion::Reaped {
                    exit_code: Some(exit),
                },
                stdout: StreamCompletion::Complete,
                stderr: StreamCompletion::Complete,
                cleanup: ScopeCleanup::Complete,
            },
        )
        .unwrap()
    }

    #[test]
    fn exact_unique_passed_receipts_are_required() {
        let task = task();
        let candidate = artifact(
            "candidate",
            task.project().clone(),
            task.graph_version(),
            "candidate-run",
            "patch",
        );
        let policy = RequiredChecks::new(vec!["lint".into(), "test".into()]).unwrap();
        let decision = IntegrationDecision::new(
            task.clone(),
            candidate.clone(),
            policy.clone(),
            vec![
                receipt(&task, &candidate, &policy, "test", "run-test", 0),
                receipt(&task, &candidate, &policy, "lint", "run-lint", 0),
            ],
            3,
            "w1-i/v1".into(),
        )
        .unwrap();
        assert_eq!(decision.task(), &task);
        assert_eq!(decision.candidate(), &candidate);
        assert_eq!(decision.receipts().len(), 2);
    }

    #[test]
    fn missing_duplicate_extra_or_failed_receipt_cannot_form_decision() {
        let task = task();
        let candidate = artifact(
            "candidate",
            task.project().clone(),
            task.graph_version(),
            "candidate-run",
            "patch",
        );
        let policy = RequiredChecks::new(vec!["test".into()]).unwrap();
        assert!(
            IntegrationDecision::new(
                task.clone(),
                candidate.clone(),
                policy.clone(),
                vec![],
                3,
                "v1".into()
            )
            .is_err()
        );
        assert!(
            IntegrationDecision::new(
                task.clone(),
                candidate.clone(),
                policy.clone(),
                vec![
                    receipt(&task, &candidate, &policy, "test", "run-1", 0),
                    receipt(&task, &candidate, &policy, "test", "run-2", 0)
                ],
                3,
                "v1".into()
            )
            .is_err()
        );
        assert!(
            IntegrationDecision::new(
                task.clone(),
                candidate.clone(),
                policy.clone(),
                vec![receipt(&task, &candidate, &policy, "test", "run-fail", 1)],
                3,
                "v1".into()
            )
            .is_err()
        );
        let extra = RequiredChecks::new(vec!["test".into(), "lint".into()]).unwrap();
        assert!(
            IntegrationDecision::new(
                task.clone(),
                candidate.clone(),
                policy.clone(),
                vec![receipt(&task, &candidate, &extra, "test", "run-extra", 0)],
                3,
                "v1".into()
            )
            .is_err()
        );
    }
}

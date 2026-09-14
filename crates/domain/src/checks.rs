use crate::{Artifact, DomainError, validate_text};
use std::collections::{BTreeMap, BTreeSet};

/// Reported result, not proof that an executable ran or was trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    Passed,
    Failed,
    Skipped,
    Cancelled,
    TimedOut,
    Unknown,
}

/// Untrusted observations; the verifier must resolve and authenticate evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckObservation {
    pub name: String,
    pub candidate: Artifact,
    pub outcome: CheckOutcome,
    pub evidence_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckIssue {
    Missing(String),
    Duplicate(String),
    Unexpected(String),
    CandidateMismatch(String),
    MissingEvidence(String),
    NotPassed(String, CheckOutcome),
}

/// Nonempty, exact-name policy. An empty issue list means only that supplied
/// observations satisfy this policy, never permission to integrate a candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredChecks {
    names: BTreeSet<String>,
}

impl RequiredChecks {
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }

    /// Constructs a nonempty policy of uniquely named required checks.
    ///
    /// # Errors
    /// Returns an error when the list is empty, a name is invalid text, or a
    /// name occurs more than once.
    pub fn new(names: Vec<String>) -> Result<Self, DomainError> {
        if names.is_empty() {
            return Err(DomainError::Missing("required checks"));
        }
        let mut unique = BTreeSet::new();
        for name in names {
            validate_text("check name", &name)?;
            if !unique.insert(name) {
                return Err(DomainError::Invalid("duplicate required check"));
            }
        }
        Ok(Self { names: unique })
    }

    #[must_use]
    pub fn evaluate(
        &self,
        candidate: &Artifact,
        observations: &[CheckObservation],
    ) -> Vec<CheckIssue> {
        let mut by_name: BTreeMap<&str, Vec<&CheckObservation>> = BTreeMap::new();
        for observation in observations {
            by_name
                .entry(&observation.name)
                .or_default()
                .push(observation);
        }
        let mut issues = Vec::new();
        for name in &self.names {
            match by_name.get(name.as_str()) {
                None => issues.push(CheckIssue::Missing(name.clone())),
                Some(entries) if entries.len() != 1 => {
                    issues.push(CheckIssue::Duplicate(name.clone()));
                }
                Some(entries) => {
                    let entry = entries[0];
                    if entry.candidate != *candidate {
                        issues.push(CheckIssue::CandidateMismatch(name.clone()));
                    }
                    if entry.evidence_ref.trim().is_empty() {
                        issues.push(CheckIssue::MissingEvidence(name.clone()));
                    }
                    if entry.outcome != CheckOutcome::Passed {
                        issues.push(CheckIssue::NotPassed(name.clone(), entry.outcome));
                    }
                }
            }
        }
        for name in by_name.keys() {
            if !self.names.contains(*name) {
                issues.push(CheckIssue::Unexpected((*name).into()));
            }
        }
        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArtifactProtection, ArtifactRetention, ProjectRef};

    fn artifact(id: &str) -> Artifact {
        Artifact::new(
            id.into(),
            ProjectRef {
                repository_id: "repo".into(),
                worktree_id: "w".into(),
                git_head: "h".into(),
                working_tree_fingerprint: "s".into(),
                config_hash: "c".into(),
                ignore_policy_version: "1".into(),
            },
            "g".into(),
            "run".into(),
            "a".repeat(64),
            3,
            "patch".into(),
            ArtifactRetention::Evidence,
            ArtifactProtection::Unreviewed,
        )
        .unwrap()
    }

    fn observation(name: &str) -> CheckObservation {
        CheckObservation {
            name: name.into(),
            candidate: artifact("a"),
            outcome: CheckOutcome::Passed,
            evidence_ref: "receipt".into(),
        }
    }

    #[test]
    fn configuration_cannot_pass_vacuously_or_hide_duplicate_requirements() {
        assert!(RequiredChecks::new(vec![]).is_err());
        assert!(RequiredChecks::new(vec![" ".into()]).is_err());
        assert!(RequiredChecks::new(vec!["test".into(), "test".into()]).is_err());
        let policy = RequiredChecks::new(vec!["test".into()]).unwrap();
        assert_eq!(
            policy.evaluate(&artifact("a"), &[]),
            vec![CheckIssue::Missing("test".into())]
        );
    }

    #[test]
    fn only_passed_with_matching_candidate_and_evidence_satisfies_policy() {
        let policy = RequiredChecks::new(vec!["test".into()]).unwrap();
        for outcome in [
            CheckOutcome::Failed,
            CheckOutcome::Skipped,
            CheckOutcome::Cancelled,
            CheckOutcome::TimedOut,
            CheckOutcome::Unknown,
        ] {
            let mut report = observation("test");
            report.outcome = outcome;
            assert_eq!(
                policy.evaluate(&artifact("a"), &[report]),
                vec![CheckIssue::NotPassed("test".into(), outcome)]
            );
        }
        let mut report = observation("test");
        report.candidate = artifact("other");
        report.evidence_ref = " \t".into();
        assert_eq!(
            policy.evaluate(&artifact("a"), &[report]),
            vec![
                CheckIssue::CandidateMismatch("test".into()),
                CheckIssue::MissingEvidence("test".into())
            ]
        );
        assert!(
            policy
                .evaluate(&artifact("a"), &[observation("test")])
                .is_empty()
        );
    }

    #[test]
    fn duplicate_and_extra_reports_cannot_be_hidden_by_order() {
        let policy = RequiredChecks::new(vec!["test".into(), "lint".into()]).unwrap();
        let candidate = artifact("a");
        let mut reports = vec![observation("test"), observation("lint")];
        assert!(policy.evaluate(&candidate, &reports).is_empty());
        reports.reverse();
        assert!(policy.evaluate(&candidate, &reports).is_empty());
        reports.push(observation("test"));
        reports.push(observation("extra"));
        let expected = vec![
            CheckIssue::Duplicate("test".into()),
            CheckIssue::Unexpected("extra".into()),
        ];
        assert_eq!(policy.evaluate(&candidate, &reports), expected);
        reports.reverse();
        assert_eq!(policy.evaluate(&candidate, &reports), expected);
    }
}

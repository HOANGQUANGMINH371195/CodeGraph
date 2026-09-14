//! Observed execution status, not authenticated execution evidence.
use crate::CheckOutcome;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Exited,
    Cancelled,
    TimedOut,
    OutputLimit,
    SpawnFailed,
    HostError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildCompletion {
    NotSpawned,
    Unreaped,
    /// No numeric exit status may be available (e.g. signal/platform failure).
    Reaped {
        exit_code: Option<i32>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamCompletion {
    /// EOF observed, reader terminated, and complete raw output retained.
    Complete,
    Truncated,
    ReadFailed,
    Incomplete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeCleanup {
    /// Host reports no owned descendant/process resource left unresolved.
    Complete,
    Incomplete,
    Unverifiable,
}

/// Caller claims, including cleanup. Parsing this does not attest to a host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionCompletion {
    pub reason: StopReason,
    pub child: ChildCompletion,
    pub stdout: StreamCompletion,
    pub stderr: StreamCompletion,
    pub cleanup: ScopeCleanup,
}

impl ExecutionCompletion {
    /// Conservative summary of the report, never a verified-check capability.
    /// Keep the full report: a timeout summary does not imply cleanup succeeded.
    pub fn reported_check_outcome(&self) -> CheckOutcome {
        match self.reason {
            StopReason::Cancelled => return CheckOutcome::Cancelled,
            StopReason::TimedOut => return CheckOutcome::TimedOut,
            StopReason::SpawnFailed if self.child == ChildCompletion::NotSpawned => {
                return CheckOutcome::Failed;
            }
            StopReason::Exited => {}
            _ => return CheckOutcome::Unknown,
        }
        if self.stdout != StreamCompletion::Complete
            || self.stderr != StreamCompletion::Complete
            || self.cleanup != ScopeCleanup::Complete
        {
            return CheckOutcome::Unknown;
        }
        match self.child {
            ChildCompletion::Reaped { exit_code: Some(0) } => CheckOutcome::Passed,
            ChildCompletion::Reaped { exit_code: Some(_) } => CheckOutcome::Failed,
            _ => CheckOutcome::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn complete() -> ExecutionCompletion {
        ExecutionCompletion {
            reason: StopReason::Exited,
            child: ChildCompletion::Reaped { exit_code: Some(0) },
            stdout: StreamCompletion::Complete,
            stderr: StreamCompletion::Complete,
            cleanup: ScopeCleanup::Complete,
        }
    }

    #[test]
    fn zero_exit_needs_reap_both_complete_streams_and_cleanup() {
        assert_eq!(complete().reported_check_outcome(), CheckOutcome::Passed);
        for child in [
            ChildCompletion::NotSpawned,
            ChildCompletion::Unreaped,
            ChildCompletion::Reaped { exit_code: None },
        ] {
            assert_eq!(
                ExecutionCompletion {
                    child,
                    ..complete()
                }
                .reported_check_outcome(),
                CheckOutcome::Unknown
            );
        }
        for stream in [
            StreamCompletion::Truncated,
            StreamCompletion::ReadFailed,
            StreamCompletion::Incomplete,
        ] {
            assert_eq!(
                ExecutionCompletion {
                    stdout: stream,
                    ..complete()
                }
                .reported_check_outcome(),
                CheckOutcome::Unknown
            );
            assert_eq!(
                ExecutionCompletion {
                    stderr: stream,
                    ..complete()
                }
                .reported_check_outcome(),
                CheckOutcome::Unknown
            );
        }
        for cleanup in [ScopeCleanup::Incomplete, ScopeCleanup::Unverifiable] {
            assert_eq!(
                ExecutionCompletion {
                    cleanup,
                    ..complete()
                }
                .reported_check_outcome(),
                CheckOutcome::Unknown
            );
        }
        assert_eq!(
            ExecutionCompletion {
                child: ChildCompletion::Reaped { exit_code: Some(1) },
                ..complete()
            }
            .reported_check_outcome(),
            CheckOutcome::Failed
        );
    }

    #[test]
    fn stop_reason_cannot_be_erased_by_later_zero_exit() {
        for (reason, expected) in [
            (StopReason::Cancelled, CheckOutcome::Cancelled),
            (StopReason::TimedOut, CheckOutcome::TimedOut),
            (StopReason::OutputLimit, CheckOutcome::Unknown),
            (StopReason::SpawnFailed, CheckOutcome::Unknown),
            (StopReason::HostError, CheckOutcome::Unknown),
        ] {
            assert_eq!(
                ExecutionCompletion {
                    reason,
                    ..complete()
                }
                .reported_check_outcome(),
                expected
            );
        }
        assert_eq!(
            ExecutionCompletion {
                reason: StopReason::SpawnFailed,
                child: ChildCompletion::NotSpawned,
                ..complete()
            }
            .reported_check_outcome(),
            CheckOutcome::Failed
        );
    }
}

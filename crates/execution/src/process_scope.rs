//! Owned Linux process-group scope.
//!
//! A process group is a useful teardown boundary for an owned fixture, but it
//! is not a complete process-tree or sandbox authority. In particular, a
//! descendant can call `setsid` or move to another namespace. Callers must
//! preserve that distinction in their receipts.

use std::process::Child;

use rustix::{
    io::Errno,
    process::{Pid, Signal},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessGroupState {
    /// No process remains in the group, or the group no longer exists.
    Gone,
    /// The group exists and the caller can address it.
    Present,
    /// The host could not determine the group state.
    Unverifiable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessGroupAction {
    Signalled,
    AlreadyGone,
    Unverifiable,
}

/// Host-owned identity for the process group created for one child.
///
/// The identity is derived from the exact `Child` returned by `Command`; it is
/// never reconstructed from a persisted PID or a tool result. `Pid` values are
/// still racy OS references, so this type deliberately does not claim a
/// race-free containment guarantee.
#[derive(Debug)]
pub(crate) struct OwnedProcessGroup {
    pgid: Pid,
    term_sent: bool,
    kill_sent: bool,
}

impl OwnedProcessGroup {
    pub(crate) fn from_child(child: &Child) -> Self {
        Self {
            pgid: Pid::from_child(child),
            term_sent: false,
            kill_sent: false,
        }
    }

    pub(crate) fn state(&self) -> ProcessGroupState {
        match rustix::process::test_kill_process_group(self.pgid) {
            Ok(()) => ProcessGroupState::Present,
            Err(Errno::SRCH) => ProcessGroupState::Gone,
            Err(_) => ProcessGroupState::Unverifiable,
        }
    }

    /// Request graceful termination of all members still in the owned group.
    pub(crate) fn terminate(&mut self) -> ProcessGroupAction {
        if self.term_sent || self.kill_sent {
            return match self.state() {
                ProcessGroupState::Gone => ProcessGroupAction::AlreadyGone,
                ProcessGroupState::Present => ProcessGroupAction::Signalled,
                ProcessGroupState::Unverifiable => ProcessGroupAction::Unverifiable,
            };
        }
        match rustix::process::test_kill_process_group(self.pgid) {
            Err(Errno::SRCH) => ProcessGroupAction::AlreadyGone,
            Err(_) => ProcessGroupAction::Unverifiable,
            Ok(()) => match rustix::process::kill_process_group(self.pgid, Signal::TERM) {
                Ok(()) => {
                    self.term_sent = true;
                    ProcessGroupAction::Signalled
                }
                Err(Errno::SRCH) => ProcessGroupAction::AlreadyGone,
                Err(_) => ProcessGroupAction::Unverifiable,
            },
        }
    }

    /// Escalate the same owned group at the cleanup deadline.
    pub(crate) fn force_terminate(&mut self) -> ProcessGroupAction {
        if self.kill_sent {
            return match self.state() {
                ProcessGroupState::Gone => ProcessGroupAction::AlreadyGone,
                ProcessGroupState::Present => ProcessGroupAction::Signalled,
                ProcessGroupState::Unverifiable => ProcessGroupAction::Unverifiable,
            };
        }
        match rustix::process::test_kill_process_group(self.pgid) {
            Err(Errno::SRCH) => ProcessGroupAction::AlreadyGone,
            Err(_) => ProcessGroupAction::Unverifiable,
            Ok(()) => match rustix::process::kill_process_group(self.pgid, Signal::KILL) {
                Ok(()) => {
                    self.kill_sent = true;
                    ProcessGroupAction::Signalled
                }
                Err(Errno::SRCH) => ProcessGroupAction::AlreadyGone,
                Err(_) => ProcessGroupAction::Unverifiable,
            },
        }
    }
}

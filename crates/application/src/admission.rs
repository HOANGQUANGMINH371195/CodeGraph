//! Host-owned, in-memory reservation accounting for one native agent tree.
//! Not an executor or authentication boundary. The host must serialize access,
//! persist/reconcile reservations, and route every descendant through this gate.
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum AdmissionError {
    #[error("invalid swarm limits or identifier")]
    Invalid,
    #[error("swarm admission stopped or deadline reached")]
    Stopped,
    #[error("attempt or deduplication key already reserved")]
    Duplicate,
    #[error("parent reservation is missing, inactive for admission, or unbound for reconciliation")]
    ParentUnavailable,
    #[error("swarm active, total or depth limit reached")]
    Capacity,
    #[error("reservation is unknown")]
    Unknown,
    #[error("native thread binding conflicts or reservation is already terminal")]
    BindingConflict,
}

#[derive(Debug)]
struct Reservation {
    parent_attempt: Option<String>,
    dedup_key: String,
    depth: u32,
    active: bool,
    native_thread: Option<String>,
}

/// Not Clone: copying this gate must not silently duplicate its budget.
#[derive(Debug)]
pub struct SwarmAdmission {
    root_thread: String,
    max_active: usize,
    max_total: usize,
    max_depth: u32,
    deadline_ms: i64,
    stopped: bool,
    reservations: BTreeMap<String, Reservation>,
}

impl SwarmAdmission {
    /// Limits are harness policy, not model/account entitlements. Root is depth 0
    /// and not counted as a child slot. Time must come from the trusted host clock.
    pub fn new(
        root_thread: &str,
        max_active: usize,
        max_total: usize,
        max_depth: u32,
        deadline_ms: i64,
    ) -> Result<Self, AdmissionError> {
        if !valid_id(root_thread)
            || max_active == 0
            || max_active > 256
            || max_total < max_active
            || max_total > 10_000
            || max_depth == 0
            || max_depth > 32
            || deadline_ms < 0
        {
            return Err(AdmissionError::Invalid);
        }
        Ok(Self {
            root_thread: root_thread.into(),
            max_active,
            max_total,
            max_depth,
            deadline_ms,
            stopped: false,
            reservations: BTreeMap::new(),
        })
    }

    /// Reserve BEFORE dispatch. An uncertain dispatch retains its slot. A dedup
    /// key is host-derived canonical work identity, not a worker's arbitrary label.
    /// Failed/finished attempts still consume total budget and retain their keys.
    pub fn reserve(
        &mut self,
        attempt: &str,
        dedup_key: &str,
        parent: Option<&str>,
        now_ms: i64,
    ) -> Result<u32, AdmissionError> {
        if now_ms < 0 || !valid_id(attempt) || !valid_id(dedup_key) {
            return Err(AdmissionError::Invalid);
        }
        if now_ms >= self.deadline_ms {
            self.stopped = true;
        }
        if self.stopped {
            return Err(AdmissionError::Stopped);
        }
        if self.reservations.contains_key(attempt)
            || self.reservations.values().any(|r| r.dedup_key == dedup_key)
        {
            return Err(AdmissionError::Duplicate);
        }
        let depth = match parent {
            None => 1,
            Some(id) => {
                self.reservations
                    .get(id)
                    .filter(|r| r.active)
                    .ok_or(AdmissionError::ParentUnavailable)?
                    .depth
                    + 1
            }
        };
        if depth > self.max_depth
            || self.reservations.len() >= self.max_total
            || self.active_count() >= self.max_active
        {
            return Err(AdmissionError::Capacity);
        }
        self.reservations.insert(
            attempt.into(),
            Reservation {
                parent_attempt: parent.map(str::to_owned),
                dedup_key: dedup_key.into(),
                depth,
                active: true,
                native_thread: None,
            },
        );
        Ok(depth)
    }

    pub fn active_count(&self) -> usize {
        self.reservations.values().filter(|r| r.active).count()
    }

    pub fn total_reserved(&self) -> usize {
        self.reservations.len()
    }

    /// Record a dispatch/reconciliation result from the trusted native host.
    /// Binding is not dispatch and remains allowed after stop for already
    /// reserved attempts. Parent lineage is checked against this gate; the
    /// adapter must authenticate the event and account/root session binding.
    pub fn bind_native_thread(
        &mut self,
        attempt: &str,
        thread: &str,
        parent_thread: &str,
    ) -> Result<bool, AdmissionError> {
        if !valid_id(thread) || !valid_id(parent_thread) {
            return Err(AdmissionError::Invalid);
        }
        let reservation = self
            .reservations
            .get(attempt)
            .ok_or(AdmissionError::Unknown)?;
        let expected_parent = match &reservation.parent_attempt {
            None => self.root_thread.as_str(),
            Some(parent) => self
                .reservations
                .get(parent)
                .and_then(|r| r.native_thread.as_deref())
                .ok_or(AdmissionError::ParentUnavailable)?,
        };
        if thread == self.root_thread || parent_thread != expected_parent {
            return Err(AdmissionError::BindingConflict);
        }
        if let Some(existing) = &reservation.native_thread {
            return if existing == thread {
                Ok(false)
            } else {
                Err(AdmissionError::BindingConflict)
            };
        }
        if !reservation.active
            || self
                .reservations
                .values()
                .any(|r| r.native_thread.as_deref() == Some(thread))
        {
            return Err(AdmissionError::BindingConflict);
        }
        self.reservations
            .get_mut(attempt)
            .ok_or(AdmissionError::Unknown)?
            .native_thread = Some(thread.into());
        Ok(true)
    }

    /// Unknown events are not buffered or treated as completion. The adapter
    /// must reconcile pre-binding events; never release an arbitrary attempt.
    /// This is thread-final evidence, NOT turn-completed/idle or task acceptance.
    pub fn observe_native_terminal(&mut self, thread: &str) -> Result<bool, AdmissionError> {
        let reservation = self
            .reservations
            .values_mut()
            .find(|r| r.native_thread.as_deref() == Some(thread))
            .ok_or(AdmissionError::Unknown)?;
        let changed = reservation.active;
        reservation.active = false;
        Ok(changed)
    }

    pub fn native_thread(&self, attempt: &str) -> Result<Option<&str>, AdmissionError> {
        Ok(self
            .reservations
            .get(attempt)
            .ok_or(AdmissionError::Unknown)?
            .native_thread
            .as_deref())
    }

    /// Stop admission only; does not release reservations or claim process death.
    pub fn stop(&mut self) {
        self.stopped = true;
    }

    /// Trusted host confirmation that dispatch never occurred. Not a worker
    /// tool: missing response, timeout and idle text do not prove no dispatch.
    /// A bound thread must instead be reconciled by native identity.
    pub fn confirm_not_dispatched(&mut self, attempt: &str) -> Result<bool, AdmissionError> {
        let reservation = self
            .reservations
            .get_mut(attempt)
            .ok_or(AdmissionError::Unknown)?;
        if reservation.native_thread.is_some() {
            return Err(AdmissionError::BindingConflict);
        }
        let changed = reservation.active;
        reservation.active = false;
        Ok(changed)
    }
}

fn valid_id(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 256
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_is_one_to_one_idempotent_and_retained_after_terminal() {
        let mut gate = SwarmAdmission::new("root", 2, 3, 2, 100).unwrap();
        gate.reserve("a", "work-a", None, 0).unwrap();
        gate.reserve("b", "work-b", Some("a"), 1).unwrap();
        assert_eq!(gate.native_thread("a"), Ok(None));
        assert_eq!(gate.bind_native_thread("a", "thread-a", "root"), Ok(true));
        assert_eq!(gate.bind_native_thread("a", "thread-a", "root"), Ok(false));
        assert_eq!(
            gate.bind_native_thread("a", "replacement", "root"),
            Err(AdmissionError::BindingConflict)
        );
        assert_eq!(
            gate.bind_native_thread("b", "thread-a", "thread-a"),
            Err(AdmissionError::BindingConflict)
        );
        assert_eq!(
            gate.observe_native_terminal("unknown"),
            Err(AdmissionError::Unknown)
        );
        assert_eq!(gate.active_count(), 2);
        assert_eq!(
            gate.confirm_not_dispatched("a"),
            Err(AdmissionError::BindingConflict)
        );
        assert_eq!(gate.active_count(), 2);
        assert_eq!(gate.observe_native_terminal("thread-a"), Ok(true));
        assert_eq!(gate.observe_native_terminal("thread-a"), Ok(false));
        assert_eq!(gate.active_count(), 1);
        assert_eq!(gate.native_thread("a"), Ok(Some("thread-a")));
        assert_eq!(
            gate.bind_native_thread("b", "thread-a", "thread-a"),
            Err(AdmissionError::BindingConflict)
        );
        assert_eq!(gate.bind_native_thread("a", "thread-a", "root"), Ok(false));
        assert_eq!(gate.total_reserved(), 2);
    }

    #[test]
    fn root_exclusion_and_parent_lineage_are_checked_before_replay() {
        let mut gate = SwarmAdmission::new("root", 2, 3, 2, 100).unwrap();
        gate.reserve("a", "work-a", None, 0).unwrap();
        gate.reserve("b", "work-b", Some("a"), 1).unwrap();
        assert_eq!(
            gate.bind_native_thread("a", "root", "root"),
            Err(AdmissionError::BindingConflict)
        );
        assert_eq!(
            gate.bind_native_thread("b", "thread-b", "thread-a"),
            Err(AdmissionError::ParentUnavailable)
        );
        assert_eq!(gate.native_thread("a"), Ok(None));
        assert_eq!(gate.native_thread("b"), Ok(None));
        assert_eq!(gate.active_count(), 2);
        assert_eq!(gate.bind_native_thread("a", "thread-a", "root"), Ok(true));
        for parent in ["root", "unrelated"] {
            assert_eq!(
                gate.bind_native_thread("b", "thread-b", parent),
                Err(AdmissionError::BindingConflict)
            );
        }
        assert_eq!(
            gate.bind_native_thread("b", "root", "thread-a"),
            Err(AdmissionError::BindingConflict)
        );
        assert_eq!(gate.native_thread("b"), Ok(None));
        assert_eq!(
            gate.bind_native_thread("b", "thread-b", "thread-a"),
            Ok(true)
        );
        assert_eq!(
            gate.bind_native_thread("b", "thread-b", "thread-a"),
            Ok(false)
        );
        assert_eq!(
            gate.bind_native_thread("b", "thread-b", "root"),
            Err(AdmissionError::BindingConflict)
        );
        assert_eq!(gate.active_count(), 2);
        gate.observe_native_terminal("thread-b").unwrap();
        assert_eq!(
            gate.bind_native_thread("b", "thread-b", "thread-a"),
            Ok(false)
        );
        assert_eq!(
            gate.bind_native_thread("b", "thread-b", "root"),
            Err(AdmissionError::BindingConflict)
        );
        assert_eq!(gate.active_count(), 1);
        assert_eq!(gate.total_reserved(), 2);
    }

    #[test]
    fn late_child_binding_uses_retained_parent_identity_after_parent_terminal() {
        let mut gate = SwarmAdmission::new("root", 2, 3, 2, 100).unwrap();
        gate.reserve("a", "work-a", None, 0).unwrap();
        gate.bind_native_thread("a", "thread-a", "root").unwrap();
        gate.reserve("b", "work-b", Some("a"), 1).unwrap();
        gate.observe_native_terminal("thread-a").unwrap();
        assert_eq!(
            gate.reserve("c", "work-c", Some("a"), 2),
            Err(AdmissionError::ParentUnavailable)
        );
        gate.stop();
        assert_eq!(
            gate.bind_native_thread("b", "thread-b", "thread-a"),
            Ok(true)
        );
        assert_eq!(gate.active_count(), 1);
        assert_eq!(gate.observe_native_terminal("thread-b"), Ok(true));
        assert_eq!(gate.active_count(), 0);
        assert_eq!(gate.total_reserved(), 2);
    }

    #[test]
    fn late_bind_after_stop_reconciles_without_spawning_or_resurrecting() {
        let mut gate = SwarmAdmission::new("root", 2, 3, 1, 100).unwrap();
        gate.reserve("a", "work-a", None, 0).unwrap();
        gate.reserve("b", "work-b", None, 0).unwrap();
        assert_eq!(
            gate.observe_native_terminal("thread-a"),
            Err(AdmissionError::Unknown)
        );
        gate.stop();
        assert_eq!(gate.bind_native_thread("a", "thread-a", "root"), Ok(true));
        assert_eq!(gate.active_count(), 2);
        gate.confirm_not_dispatched("b").unwrap(); // Host has confirmed no dispatch.
        assert_eq!(
            gate.bind_native_thread("b", "late-thread", "root"),
            Err(AdmissionError::BindingConflict)
        );
        assert_eq!(
            gate.bind_native_thread("missing", "thread", "root"),
            Err(AdmissionError::Unknown)
        );
        assert_eq!(
            gate.bind_native_thread("a", " ", "root"),
            Err(AdmissionError::Invalid)
        );
        assert_eq!(gate.observe_native_terminal("thread-a"), Ok(true));
        assert_eq!(gate.active_count(), 0);
        assert_eq!(gate.total_reserved(), 2);
        assert_eq!(
            gate.reserve("c", "work-c", None, 1),
            Err(AdmissionError::Stopped)
        );
    }

    #[test]
    fn descendants_share_limits_and_parent_no_dispatch_does_not_release_children() {
        let mut gate = SwarmAdmission::new("root", 2, 3, 2, 100).unwrap();
        assert_eq!(gate.reserve("a", "work-a", None, 0), Ok(1));
        assert_eq!(gate.reserve("b", "work-b", Some("a"), 1), Ok(2));
        assert_eq!(
            gate.reserve("c", "work-c", None, 2),
            Err(AdmissionError::Capacity)
        );
        assert_eq!(
            gate.reserve("c", "work-c", Some("b"), 2),
            Err(AdmissionError::Capacity)
        );
        assert_eq!(gate.confirm_not_dispatched("a"), Ok(true));
        assert_eq!(gate.confirm_not_dispatched("a"), Ok(false));
        assert_eq!(gate.active_count(), 1);
        assert_eq!(
            gate.reserve("c", "work-c", Some("a"), 3),
            Err(AdmissionError::ParentUnavailable)
        );
        assert_eq!(gate.reserve("c", "work-c", None, 3), Ok(1));
        gate.confirm_not_dispatched("b").unwrap();
        assert_eq!(
            gate.reserve("d", "work-d", None, 4),
            Err(AdmissionError::Capacity)
        );
        assert_eq!(gate.total_reserved(), 3);
    }

    #[test]
    fn dedup_survives_completion_and_deadline_or_stop_never_refunds_slots() {
        let mut gate = SwarmAdmission::new("root", 2, 4, 2, 10).unwrap();
        gate.reserve("a", "work", None, 0).unwrap();
        assert_eq!(
            gate.reserve("b", "work", None, 1),
            Err(AdmissionError::Duplicate)
        );
        gate.confirm_not_dispatched("a").unwrap();
        assert_eq!(
            gate.reserve("a", "new-work", None, 1),
            Err(AdmissionError::Duplicate)
        );
        assert_eq!(
            gate.reserve("b", "work", None, 1),
            Err(AdmissionError::Duplicate)
        );
        gate.reserve("b", "other", None, 9).unwrap();
        assert_eq!(
            gate.reserve("c", "third", None, 10),
            Err(AdmissionError::Stopped)
        );
        assert_eq!(
            gate.reserve("c", "third", None, 0),
            Err(AdmissionError::Stopped)
        );
        assert_eq!(gate.active_count(), 1);
        gate.stop();
        assert_eq!(gate.active_count(), 1);
        assert_eq!(
            gate.confirm_not_dispatched("unknown"),
            Err(AdmissionError::Unknown)
        );
    }

    #[test]
    fn malformed_limits_and_requests_do_not_consume_budget() {
        for root in [String::new(), " ".into(), "x".repeat(257)] {
            assert!(matches!(
                SwarmAdmission::new(&root, 1, 1, 1, 100),
                Err(AdmissionError::Invalid)
            ));
        }
        for args in [
            (0, 1, 1, 1),
            (257, 257, 1, 1),
            (2, 1, 1, 1),
            (1, 10001, 1, 1),
            (1, 1, 0, 1),
            (1, 1, 33, 1),
            (1, 1, 1, -1),
        ] {
            assert!(SwarmAdmission::new("root", args.0, args.1, args.2, args.3).is_err());
        }
        let mut gate = SwarmAdmission::new("root", 1, 2, 1, 100).unwrap();
        assert_eq!(gate.reserve("", "x", None, 0), Err(AdmissionError::Invalid));
        assert_eq!(
            gate.reserve("a", &"x".repeat(257), None, 0),
            Err(AdmissionError::Invalid)
        );
        assert_eq!(
            gate.reserve("a", "x", Some("absent"), 0),
            Err(AdmissionError::ParentUnavailable)
        );
        assert_eq!(gate.total_reserved(), 0);
        gate.reserve("valid", "valid-work", None, 0).unwrap();
        for parent in [String::new(), " ".into(), "x".repeat(257)] {
            assert_eq!(
                gate.bind_native_thread("valid", "thread", &parent),
                Err(AdmissionError::Invalid)
            );
        }
        assert_eq!(gate.native_thread("valid"), Ok(None));
        assert_eq!(gate.active_count(), 1);
        gate.stop();
        assert_eq!(
            gate.reserve("a", "x", None, 0),
            Err(AdmissionError::Stopped)
        );
    }
}

# W1 check-policy event/outbox

Source gate: ready before implementation.

Re-read clean Grit `0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`,
`src/db/sqlite_store.rs:48–82,448–515`: transaction-before-check/write and
separate-connection regression with one winner/one persisted row. Adopt atomic
scope; no lock code copied. Reference tests read, not run.

Local `check_policy.rs`, cancellation transaction and injected-event-failure
test, V2 outbox trigger reviewed. Extend first policy registration with explicit
nonnegative host time and CheckPolicyRegistered event containing canonical
versioned policy JSON. Insert event before commit so trigger/insert errors roll
back policy as well. Replay preserves the first event/time; no new event for
conflicts, late registration, invalid time or existing legacy policy.

Planned tests: exact event payload/time, one event across replay/restart, pending
outbox consumption and rollback injection on both event and outbox insertion.
No synthetic backfill of old policy events; no integration or execution claim.
Skills: rust-router, m12-lifecycle. No migration rewrite or dependency addition.

## Results

Registration now takes host `now_ms`, rejects negative time, and atomically
inserts canonical policy JSON as `check_policy_registered` event. Existing V2
trigger enqueues the event. Tests cover event/payload/time, replay with changed
time, restart + ordered consumer acknowledgements, event/outbox abort rollback
and successful retry, plus legacy policy replay without synthetic event.

Foundation passes: 108 Rust + 11 Node tests, 6 crates / 30 direct dependencies.
No migration edited; event kind is new, so consumers must handle it explicitly.
Older consumers that reject unknown kinds are not forward compatible by this
change alone. No backfill, authenticated actor proof, live dispatch or integration
authority claimed. Concurrent registration/lease stress remains open.

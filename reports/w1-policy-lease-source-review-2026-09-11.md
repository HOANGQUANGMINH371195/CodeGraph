# W1 policy-bound lease

Source gate: ready before implementation.

Re-read clean Grit `0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`,
`src/db/sqlite_store.rs:48–112,448–515`: BEGIN IMMEDIATE before ownership checks
and write; separate-connection test asserts one winner. Source assertions read,
not executed. Adopt atomic read/check/write; no copied lock implementation.

Local lease update/fence/event logic and check-policy reader reviewed. Add a
PolicyLeaseRepository port requiring expected complete TaskSpec and RequiredChecks.
Store checks both under the lease transaction; missing policy or mismatch cannot
fall back to legacy lease. Factor existing lease logic into one private transaction
helper, with SQL in files, shared by both paths. Policy immutable registry remains
the binding; the returned lease is not execution, check or integration proof.

Tests planned: missing/wrong policy and task, success/reclaim/stale owner, cancelled
task and event/outbox rollback. Existing dependency and legacy lease tests exercise
the shared SQL. Legacy CLI lease is host-only compatibility, not native dispatch.
Skills: rust-router, m12-lifecycle. No runtime/permissions or dependencies added.

## Results

Implemented policy-bound port and store path; extracted one transaction-level
lease helper and two SQL files, reusing event insertion SQL. Both lease paths
reject negative time, nonpositive duration and overflow. New tests cover missing
policy, mismatched task/policy, unchanged events on refusal, successful lease,
expiry-boundary reclaim/fence, stale submit, cancel and event/outbox rollback.
Existing dependency/concurrent legacy lease tests remain green.

Foundation: 110 Rust + 11 Node tests pass, 6 crates / 30 direct dependencies.
The native dispatcher is not wired. Policy-bound simultaneous callers and
registration-versus-lease interleavings still need explicit race coverage;
existing legacy concurrency tests are not claimed as coverage of that new path.
No execution receipt verification or integration authority enabled.

# W1 pre-launch execution plan registration

Source gate ready before code. Re-read clean Grit
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`, src/db/sqlite_store.rs:48–112,
448–515: acquire write transaction before checking competing state. Adopt atomic
precondition+insert, not lease refresh or mutable replay. Reference tests read,
not run. Local receipt, submission query, policy and artifact contracts reviewed.

Plan: immutable ExecutionPlan binds CheckRunBinding, intended host and execution
snapshot, with strict versioned wire. Validate the same execution target rules
as receipts. V9 stores plans without backfill. First registration requires current
submitted candidate (full task/owner/fence/sequence/artifact), exact registered
policy and candidate descriptor, no existing receipt with the run ID. Perform
checks and insert in one IMMEDIATE transaction. Exact replay remains historical
after cancellation; it does not permit relaunch. Read filters exact TaskSpec.

This reserves immutable metadata, not command approval, host authentication,
an executable launch lease or proof candidate was applied. Later lifecycle must
prevent duplicate launches and reconcile uncertain execution. Legacy receipt-only
records must not be retroactively given plans. Tests: strict roundtrip, replay,
restart, changed host/command, cancellation, late receipt, missing/mismatched
policy/candidate and upgrade preservation. No source copied. Skills rust-router,
m09-domain, m05-type-driven and m12-lifecycle.

Receipt writes will check any existing plan within their write transaction and
reject a different binding/host/snapshot. Legacy unplanned diagnostics stay readable
and writable as untrusted claims; no plan is inferred. Registration reconciles
origin task/owner/fence, not original lease expiry or live execution authority.

Validation: `cargo check --workspace --locked --offline` and foundation exited 0.
Foundation: 143 Rust + 11 Node pass; existing subprocess helper remains ignored
in discovery but is run twice by its parent test. Five new tests cover strict
plan fields/schema/target policy, registration and receipt matching, restart/
historical replay, scope filtering, missing policy/candidate, mismatched sequence/
owner/hash, late receipt/cancellation and V8 upgrade preserving old bytes without
plans. Earlier migrations unchanged; current/future counts now V9/V10. Shared
submission decoder is reused inside registry transaction; existing query tests pass.
Format and PLAN mirror checked. Registry concurrency/corruption and launch
authority/lifecycle are not proved by this step; W1-C not marked complete.

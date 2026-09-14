# W1 task check-policy ledger

Source gate: ready before implementation.

Re-read Grit `src/db/sqlite_store.rs:48–112,448–515`, clean HEAD
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`. It acquires BEGIN IMMEDIATE before
checking ownership and writing locks; separate-connection test asserts one
winner and one persisted row. Test read, not executed. Adopt transaction-first
check/write, not its mutable re-lock semantics or inline SQL. No code copied.

Local analysis registration and migration tests reviewed. Adaptation: V7 table
binds one immutable required-check policy to existing task. First registration
must match complete stored TaskSpec and precede any lease (queued, fence zero).
Same-policy replay remains legal after leasing; changed policy conflicts.
No policy inferred for existing tasks. Scoped query checks the full contract.
Use versioned wire policy, canonical sorted names, separate SQL files. No
credentials, native host, execution or integration permission added.

Planned tests: restart/reordered replay, conflict, wrong task contract, late
registration, missing legacy policy, and upgrade/future-version migration gates.
Policy events/host authorization and requiring policy in future dispatch remain
separate work; existing legacy enqueue/lease are not silently redefined here.
Skills: rust-router, m09-domain, m12-lifecycle; transaction stays inside store.

## Results

Added canonical versioned RequiredChecks wire format, CheckPolicyRepository,
V7 migration and separate select/insert SQL. No existing migration changed.
Tests verify restart/reordered replay, full-contract filtering, immutable policy,
late registration rejection, strict wire validation and V6 task preservation with
no invented policy. Unsupported persisted policy fails rather than being replaced.
Future-history fixture moved to V8 to remain newer than the current binary.

Foundation passed: 106 Rust + 11 Node tests; 6 crates / 30 direct dependencies.
No external reference tests run. No policy event/outbox or dispatcher wiring yet;
multi-connection registration-versus-lease race coverage remains to add. SQL
transaction serializes the write path, but this run did not stress that race.

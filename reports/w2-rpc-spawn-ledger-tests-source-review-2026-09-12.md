# W2 RPC spawn ledger tests — 2026-09-12

## Source study recorded before test implementation

Scope: only the assigned store integration test, owned SQL fixtures, migration
tests, and this report. No production implementation, migrations, references,
services, accounts, subagents, or plan edits. Main owns integration/full gates.
Read project AGENTS.md, PLAN.md §0.4.1, rust-router and m07-concurrency skills.

Product repository has no HEAD commit; all existing files are untracked and
treated as concurrent user work. Initial SHA-256 fingerprints:

- store/src/migration_tests.rs: e00c7d5550b1e0ec1cafbe5852628fe8be7c95c0a039d28763f860916b7532ab
- store/src/rpc_launch.rs: 45c7684011bd1d6d4481b47e6d5f682b14fb058dbdda85ea9db459d6de040dc3
- domain/src/rpc_spawn.rs: 564bfdfc564eb7f7d9b7c68beda7f8be8b397fe5751567314131718c4a647fdb
- protocol/src/rpc_spawn.rs: c8b1a6356212ef97381d68420bbff7caa49f598ec94594747dff8ca2e23dde26
- V12: a77a2de8abd18b76ac9b56d627daab8f61e110401e56b159c9e63cce818762e2

Orca revision 26f9fd8ea152ad6126c005e5ae602c7d201f4a99, clean status.
Read entire src/main/runtime/orchestration/db/mutation-receipts/mutation-receipt-store.ts
and orchestration-mutation-question-db.test.ts:1–94 in that orchestration directory.
beginMutationReceipt serializes lookup/insert with BEGIN IMMEDIATE, checks
method/payload identity on replay, commits matches and rolls back failures.
Tests assert completed replay and mismatched payload rejection plus caller scope.
Adopt exact identity/replay assertions and rollback checks. Avoid its mutable
checkpoint/completion/discard semantics: spawn reports are immutable diagnostics.

ICM revision 2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42, clean status.
Read crates/icm-store/src/store/facts.rs:102–132 and
crates/icm-store/src/store/tests/facts.rs:164–195 (actual tests path).
set_fact wraps multi-statement state changes in BEGIN IMMEDIATE/commit/rollback;
the test checks one active latest fact after supersession. Adopt atomic failure
assertions and independent concurrent connections. Avoid supersession: changed
reports must conflict and original bytes must survive. License files inspected:
Orca MIT, ICM Apache-2.0. Behavioral study only; no upstream code copied.

Local sources read: RPC launch repository decode/read/register/claim flows,
existing RPC integration fixtures/assertions, migration test history comparisons,
V1/V2/V11/V12 schemas, public spawn domain/wire and application trait. V2 creates
outbox entries through the event trigger; tests must fail both event and outbox
insertion and verify rollback across all three ledgers.

Adaptations requiring original tests: all four dispositions, historical write
after cancellation/expiry, exact full descriptor filtering, missing/unclaimed
rejection, corruption sanitization, eight independent writers (one true/seven
false), and V11 upgrade with semantically validated task/launch/claim preserved.
SQL will live only in owned fixture files. Migration expectations advance to 12
while historical slices retain their original version lengths.

## Validation

Actual focused results (all exit 0):

- `scripts/with-local-tools cargo test -p graph-store --test rpc_spawn`:
  9 passed, 0 failed; includes all four dispositions/reopen/replay, historical
  cancellation/expiry writes, absent/unclaimed rejection, 13 changed full-spec
  components, changed time/disposition/PID conflicts, event/outbox rollback and
  retry, eight independent connections (one true/seven false), seven observation
  corruptions, and six registered-launch/task/claim corruption cases.
- `scripts/with-local-tools cargo test -p graph-store --lib migration_tests`:
  8 passed, 0 failed, 34 filtered out. New V11 fixture uses real enqueue/lease/
  register/claim APIs against a connection migrated only through V11. Upgrade
  and second reopen preserve complete task/spec/claim/event/outbox values and
  all 11 applied migration records; V12 table remains empty and claim consumed.
- `scripts/with-local-tools rustfmt --edition 2024 --check
  crates/store/tests/rpc_spawn.rs crates/store/src/migration_tests.rs`: passed.
  Formatting was limited to those two owned Rust files.

Files authored/edited: `crates/store/tests/rpc_spawn.rs`,
`crates/store/src/migration_tests.rs`, this report, the two
`crates/store/src/sql/fixture_rpc_spawn_{v11_snapshot,empty}.sql` fixtures,
and 13 `crates/store/tests/sql/rpc_spawn_*.sql` fixtures: counts, event_outbox,
fail_event, restore_event, fail_outbox, restore_outbox, corrupt_observation,
corrupt_host, corrupt_launch, corrupt_task, corrupt_claim, remove_claim,
remove_launch. V1–V12 migration files were not edited. Old V7/V8/V9/V10 history
comparison slice lengths remain unchanged; only final expected counts became 12.

The main agent's new store implementation and SELECT/INSERT SQL were read before
testing: writes use an immediate transaction; reads use a deferred transaction
and validate persisted observation, registered descriptor, claim and time before
full-spec filtering. No store implementation changes were needed for these tests.

Gaps/handoff: main owns full foundation/workspace validation and runtime wiring.
These tests prove historical storage behavior, not process liveness, approval,
terminal RPC results, crash recovery, or relaunch authority. Corrupt linkage
fixtures verify read errors; corrupt observation fixtures also verify write
errors. This is one contention run, not a prolonged stress test. Timestamp zero/
maximum and domain/wire rejection matrices remain with main's contract tests.
Reference AGENTS files were also inspected; no ICM memory writes were performed
because the explicit task allowlist excludes reference/service writes.

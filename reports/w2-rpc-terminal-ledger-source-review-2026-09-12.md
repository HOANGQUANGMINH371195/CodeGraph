# W2 terminal receipt ledger

status: done (initial ledger); source-gate: ready; scope: application/store migration/ledger tests.

- [x] Read relevant outsource source/tests before patch.
- [x] Record adopt/avoid and gaps.
- [x] Choose transaction/linkage and tests before coding.

Orca clean HEAD `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`: read
`src/main/runtime/orchestration/db/mutation-receipts/mutation-receipt-store.ts:1–142`
and `orchestration-mutation-question-db.test.ts:1–78`. BEGIN IMMEDIATE serializes
initial receipt creation and method/payload mismatch rejects reused identity;
tests prove replay and caller namespace separation. Adopt serialized identity
checking. Avoid mutable completion overwrite/discard for our immutable terminal
report. Upstream tests not executed.

Product read rpc_spawn ledger, execution_receipt, analysis/artifact registration,
V4/V5/V12 migrations, event mapping and migration count expectations. New V13,
no edits to applied migration. One terminal row per spawn launch, indexed output
run and optional artifacts with FKs, immutable triggers. Validate complete stored
spawn and run/artifact descriptors, not just IDs. Reads use one deferred snapshot;
writes one IMMEDIATE transaction including event and existing outbox trigger.
Late reports after cancellation/expiry allowed; no claim reset or task success.
Missing prerequisites reject new writes; broken existing linkage is corruption.
Rust/domain/lifecycle skills inform repository port and scoped transactions.

Planned validation: replay/reopen/conflict, missing prerequisites, atomic rollback,
corrupt linkage and V12 upgrade history preservation. Metadata linkage is not CAS
byte verification or host attestation. Full W2 remains open.

## Validation

Five tests passed: immutable replay/reopen/conflict and event count, four missing
prerequisite stages and three exact-descriptor mismatches, outbox-trigger failure
rollback, malformed persisted receipt with fixed errors, V12 upgrade preserving
history/events/spawn/run across two reopens and allowing first receipt afterward.
No old migration edited; count expectations updated to 13, fixture timestamps
12/13 unchanged. SQL lives in named files. Main reviewed code and assertions;
no separate reviewer.

`scripts/with-local-tools cargo test -p graph-store --locked --offline rpc_terminal`
passed five tests. `sh scripts/validate-foundation.sh` passed 269 Rust + 12 Node;
`scripts/with-local-tools cargo fmt --all -- --check` exit 0. Architecture
7 packages, 41 direct dependencies, no errors. No dependency added.

Final untracked product SHA-256:
- `crates/store/src/rpc_terminal.rs`: `909891180aca0e3e926772c78d40641c6a2332b39300854568a69c3ffd1ec883`
- `crates/store/src/rpc_terminal_tests.rs`: `a327605423118fadf27118a8a624e9168c3b2909d67635a1ac97466d4bd9a299`
- `crates/store/migrations/V13__rpc_terminal_receipts.sql`: `820c1fff7f977ad1e593544736fbc1b30a74298590d2ea7be124b2e2817619b0`

Remaining tests before host integration: simultaneous independent writers,
late cancel report, corruption of indexed/reference columns beyond malformed
terminal JSON, and upgrade full artifact row comparison. Next implementation:
actual launch/epoch-bound receipt publication with output CAS verification;
CLI observation and crash reconciliation. No full W2/W1-I completion.

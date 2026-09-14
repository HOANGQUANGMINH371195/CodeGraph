# W2 RPC launch ledger — source-first receipt

status: ledger implemented and locally verified | owner: main
source-gate: ready
scope: application port, V11 migration, SQL adapter and owned SQLite tests

Sources read before implementation:

- ICM clean `2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42`,
  crates/icm-store/src/store/memory.rs:11–28 and facts.rs:102–130: validation,
  BEGIN IMMEDIATE, commit/rollback around coupled writes. Its facts tests
  :168–192 inspect superseded fact indexing, not an RPC launch/replay proof.
- Product untracked/no HEAD: execution_plan.rs and execution_launch.rs, task
  lease SQL/schema, EventKind, V2 outbox trigger and migration history tests.
  Existing execution claim is tied to submitted checks and cannot grant RPC.

Adopt transaction ownership and atomic read/validate/write. Use rusqlite RAII
transactions, not manual BEGIN/COMMIT strings. New SQL stays in named files;
append V11 without editing V1–V10. Registration stores the entire immutable RPC
description and validates exact current leased task/owner/fence/expiry at supplied
time. Claim rechecks inside IMMEDIATE; one committed marker + event/outbox.
Exact replay is historical false even after cancellation/expiry, never permission.
Changing an ID's content conflicts. Reserve (task,fence) and (host,epoch) to
prevent alias IDs from bypassing one-shot accounting for one launch-origin
attempt. New attempt needs new lease/epoch and fresh host authorization.

Readback validates descriptor against denormalized keys and stored TaskSpec.
Caller timestamps are observations, not an authenticated clock. IDs/approval
references are not resolved here: a true claim is ledger accounting, not spawn
authority or exactly-once OS execution. No reset/relaunch API. Host approval,
process preflight, claim-to-spawn crash reconciliation remain separate work.

Tests planned: registration/readback/reopen/replay, changed spec and alias IDs,
expired/cancelled/released fence, concurrent claim winner, atomic outbox rollback,
V10→V11 history preservation/no fabricated RPC rows. Source-study required for
delegated tests as well; no accounts/services/native dispatch.

## Migration and integration review

V11 is appended, old migrations unchanged. Main ran all seven migration tests:
pass, including the new V10 upgrade assertion (first ten history entries
unchanged, empty RPC tables, preserved task/event/outbox). The first run exposed
an old hard-coded V11 future fixture and old migration counts; future fixture
now derives max(version)+1 and still asserts rejection without history/data
rewrite. Main then ran all 41 store library tests: pass after updating the
fresh/concurrent-open expected migration counts to eleven.

Main independently read the worker's first ten integration tests and all seven
named SQL fixtures, and ran the external RPC target: ten pass. This includes
eight independent preopened connections synchronized at claim time, not eight
calls serialized behind a shared Store mutex. Four fault cases cover both
registration and claim failing at event or outbox insertion, exact unchanged
counts/event list across reopen, and successful retry after fixture-only cleanup.
Final worker additions and full foundation results follow.

## Final verification

Main read the additional negative-time and corrupt-host tests plus the named
SQL corruption fixture. Worker Dewey (`01a093e1-4aaf-7993-90f1-93c1b821e690`)
completed its disjoint test/report scope and was closed after review. See
w2-rpc-ledger-tests-source-review-2026-09-12.md for the source receipt and all
twelve external tests. Corruption rejection preserves claim timestamp and does
not silently repair or reset the ledger.

- `sh scripts/validate-foundation.sh`: 216 Rust + 12 Node pass, including all
  twelve RPC tests and the V10→V11 upgrade. Two standalone ignored helpers are
  still invoked by their parent tests. Architecture: seven packages, 41 declared
  direct dependencies, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: pass.
- All new runtime/test SQL uses named files; no applied migrations changed.
  No storage/process dependency added to domain/application.

Post-format SHA-256 (untracked working tree, not a committed revision):

- store/src/rpc_launch.rs: `d24b2c5e6802919c3de3e7b494ea468ad69b4a92a257a83b4730a54fe4e1ca2d`
- migrations/V11__rpc_launch_ledger.sql: `16ea07a22f1d8b3a0ebaed52c22df77636e053d27ef83cd4b38d235531717d93`
- store/tests/rpc_launch.rs: `3942506c647a72bd8c94304e98657d933e35615db10f08cd586869b705980e69`

Limits: concurrent threads/separate SQLite connections and fault rollback are
not multiprocess crash/power-loss proof. Lease replacement is expiry/reacquire;
no explicit release API was invented. Approval resolution, authenticated host
clock, process identity/preflight, claim-to-spawn reconciliation and native W3
remain unimplemented. A stored descriptor/true claim is not a launch grant.
Full W1/W2 and W0–W13 package gates remain open. Next migration must be V12.

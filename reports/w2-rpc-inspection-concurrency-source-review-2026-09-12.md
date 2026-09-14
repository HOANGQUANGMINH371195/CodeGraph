# W2 inspection concurrency

status: done (bounded test task); source-gate: ready; owner/reviewer: main agent.

- [x] Reread Orca clean revision 26f9fd8ea152ad6126c005e5ae602c7d201f4a99:
  db-task-dispatch-invariant.test.ts:285–345 injects another connection between
  precheck and insert; dispatch-context-store.ts:13–114 implements conditional
  claim and savepoint; task-status-transition.ts:14–95 reserves the WAL writer
  before lifecycle reads. Mutation receipt begin/replay source/tests also reread.
- [x] Product read: rpc_launch snapshot transaction/join, complete terminal
  linkage/read/write flow, fixture/prerequisites, existing eight-writer test and
  Store WAL/busy-timeout setup. Adopt separate connections and explicit ordering;
  avoid pretending our read-only inspection needs a writer reservation.
- [x] Pre-code tests: pin a real read transaction before another connection
  publishes prerequisites and receipt; old snapshot stays old, next snapshot
  sees exact new receipt. Separately eight public-API readers race staged writes,
  require coherent monotonic observations and one event per actual write.

Skills rust-router/m07-concurrency: each thread owns its connection; bounded
channel waits coordinate test phases, no shared connection mutex or async runtime.
No upstream tests run or source copied. Test-only task, no new dependency/schema.
Deterministic transaction test does not inject inside the public method; the
public concurrent test samples schedules, not every possible interleaving.

## Validation

- `scripts/with-local-tools cargo test -p graph-store --lib --locked --offline rpc_terminal::tests`:
  exit 0, all 11 tests passed (two new).
- `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0;
  280 Rust + 12 Node passed, no failures. Architecture: 7 packages,
  41 declared dependencies, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- Pinned transaction retains absence after a second connection commits both
  spawn and terminal; a fresh public snapshot returns exact spawn and receipt.
- Eight independent connections each check initial absence, 100 public API
  snapshots and a final post-commit snapshot. Observed spawn/terminal never
  regress; terminal always matches the spawn. Event/outbox counts grow only
  by the writer's two events; claim remains consumed and reopen preserves receipt.
- No production code/dependency/migration changes; no independent reviewer.
- Untracked product file `crates/store/src/rpc_terminal_tests.rs` SHA-256:
  `fa782c49ef64bf18d368ded86ef5b1470beb0a7e2972da10fe8c8333a4d30b59`.

## Remaining scope

This verifies read consistency, not OS-crash recovery, power-loss durability,
CAS reconstruction or production containment. Thread scheduling in the stress
test is sampled; the deterministic test exercises the transaction/read helpers,
not an injected pause inside the public method. Full W2/W1-I and W0–W13 stay open.

# W2 RPC ledger process-kill boundaries

status: done (ledger kill test only); source-gate: ready; owner/reviewer: main agent.

- [x] Reread full Orca orchestration-legacy-worker-terminal-recovery.ts and its
  tests at clean revision 26f9fd8ea152ad6126c005e5ae602c7d201f4a99. Adopt separation
  of durable accounting from live process identity: missing or settled records
  do not authorize adopting another process. No PTY recovery code is copied.
- [x] Read product receipt_transaction_crash_fixture and its parent test,
  RPC fixture/prerequisites, terminal transaction SQL/event/outbox and inspection.
  Existing process::exit fixture is not SIGKILL evidence. New child runs only
  owned test code and remains alive with its Store until the parent kills/reaps it.
- [x] Pre-code plan: Linux parent/ignored helper, fresh temp DB per stage:
  committed claim, committed spawn, uncommitted terminal+event, committed terminal.
  Signal reached stage only after its writes; parent has timeout and cleanup guard.
  Reopen must retain claim/spawn as appropriate, rollback incomplete transaction,
  retain exact committed receipt/event/outbox, and never reset claim on replay.

Skills rust-router/m12-lifecycle: parent owns Child cleanup even on assertion
failure. No workload RPC process is launched: spawn/PID and output descriptors
are synthetic ledger fixtures, not attestations. This is host-process kill,
not power loss, actual orphaned-worker recovery or CAS byte reconstruction.
Uncommitted case will use the same named SQL under a test transaction; other
cases exercise production repository methods. Upstream tests not run.

## Validation

- `scripts/with-local-tools cargo test -p graph-store --lib --locked --offline killed_rpc_ledger_host_preserves_boundaries`:
  exit 0, one parent test passed, all four stages reached and each owned child
  reaped with signal 9. Ignored helper is invoked explicitly by the parent.
- `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0,
  281 Rust + 12 Node passed, no failures. Architecture: 7 packages/41 declared
  dependencies, no errors. Three subprocess helpers are ignored in default
  enumeration and invoked by their parent tests.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- Two reopens per stage validate exact launch/claim/spawn/terminal and full event
  sequence against an independently built expected ledger; counts include outbox.
  Uncommitted terminal/event disappear together. Committed receipt replay adds
  no row/event; consumed launch claim never resets.
- Main reviewed the test; no independent reviewer. No production code, migration
  or dependency change. New untracked `crates/store/src/rpc_terminal_kill_tests.rs`
  SHA-256: `990e36a0661116a0eec0bc94631923e1b85a51b865d421706d8eca455e49cbf0`.

## Remaining scope

Only ledger writer process death at these explicit boundaries is verified.
No migration/power-loss injection, actual RPC host/worker orphan handling,
live-process identity reconciliation, durable prepared output recovery or CAS
reconstruction. The uncommitted fixture shares named SQL but is not fault
injection inside record_rpc_terminal_receipt. Full W2/W1-I remains open.

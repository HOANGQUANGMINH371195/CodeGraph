# W2 process-scope source study — 2026-09-12

Status: source-gate `ready` for the bounded Linux process-group teardown slice;
this receipt is not a containment or execution-acceptance claim.

## Sources reread

| Source | Observation | Adopt / avoid |
|---|---|---|
| `/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/bash/foreground.rs` | starts a new Unix process group and kills the group on timeout/cancel; stdout/stderr drain is tracked separately | Adopt group-scoped termination and separate stream/child observations; avoid treating the group signal as proof of descendant cleanup. |
| `/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/bash/helpers.rs` | group kill is TERM then delayed KILL; the helper has no durable ownership or post-kill proof | Adopt bounded escalation as a mechanism; avoid shell `kill`, a fixed PID claim, or a sleep as authority. |
| `/home/minh/projects/outsource/opendev/crates/opendev-mcp/src/transport/process.rs` and `process_tests.rs` | descendant discovery uses `pgrep -P` and tests only non-panicking/bounded behavior | Keep descendant discovery out of the authority path; no `pgrep` output is accepted as proof. |
| `/home/minh/projects/outsource/opendev/crates/opendev-mcp/src/transport/stdio.rs` | close captures descendant IDs before parent wait, then terminates the parent and descendants | Preserve the ordering insight, but replace unowned PID lists with the owned process-group scope in the trusted fixture. |
| `crates/execution/fixtures/owned.rs`, `crates/execution/tests/descendant.rs` | owned fixture deliberately keeps a pipe open from a descendant and uses a dedicated subreaper helper | Turn this into a teardown regression: the descendant must be stopped by the owned scope; the helper remains the only reaper. |

## Bounded implementation decision

1. Launch trusted Linux fixtures in a fresh process group using the standard
   Unix `CommandExt::process_group(0)` API.
2. Keep the group identity owned by the supervisor and signal it through
   `rustix`; never invoke `pgrep`, shell `kill`, or a caller-provided PID.
3. Send TERM at stop/child completion and escalate to KILL at the cleanup
   deadline. Keep direct-child reap, stream EOF, and process-group observation
   as separate facts.
4. A group disappearing is not enough to prove that a descendant called
   `setsid`, escaped into another namespace/cgroup, or was adopted elsewhere.
   Therefore this slice keeps `ScopeCleanup::Unverifiable`; it cannot create a
   passed execution receipt or close W1-I.
5. The production containment authority remains an explicit future adapter
   (dedicated supervisor/cgroup/namespace or equivalent), with crash recovery
   and cross-platform tests still required.

## Plan impact

This receipt supports a partial W2 process-scope milestone only. It does not
tick W2, W1-I, host authentication, sandbox, or full process-tree acceptance.

## Implementation result

- `graph-execution` now launches trusted Linux fixtures and RPC fixtures in a
  fresh process group and keeps the group identity tied to the returned
  `Child`.
- Teardown sends TERM to the owned group and escalates to KILL at the bounded
  cleanup deadline through `rustix`; no shell command or persisted PID is used.
- The descendant fixture now proves that a child which inherits both output
  pipes is terminated before its one-second sleep completes. The test remains
  under a dedicated subreaper so the harness, rather than the library, owns
  descendant reaping.
- `cargo test -p graph-execution --locked --offline`: pass (3 unit tests and
  all execution integration tests; one intentionally ignored helper).
- `cargo test --workspace --locked --offline`: pass (2026-09-12; subprocess-only
  helpers remain intentionally ignored, including the descendant subreaper,
  RPC-ledger kill, and receipt-transaction crash helpers).

The implementation intentionally leaves `ScopeCleanup::Unverifiable` in the
completion contract. A successful group signal/absence observation is not a
proof against `setsid`, namespace/cgroup escape, host crash, or PID reuse.

# W2 RPC launch ledger inspection before recovery

status: ledger inspection through CLI verified; full W2 open | owner: main
source-gate: ready

Source study BEFORE patch: Orca clean 26f9fd8ea152ad6126c005e5ae602c7d201f4a99,
`src/cli/orchestration-mutation-recovery.ts:1–98` and corresponding tests:1–130.
The error adapter emits query-before-retry guidance and workerDeathInferred=false;
tests assert ordering and no invented dispatch. These are envelope tests, not
proof of process crash recovery. Read ICM clean
2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42,
`crates/icm-store/src/store/facts.rs:102–132` and tests/facts.rs:164–195:
transaction-bound multi-statement consistency; tests cover normal supersession,
not RPC races. No upstream tests/services run. Product AGENTS, task_query,
rpc_launch, execution_receipt and launch/CLI contracts read; untracked/unborn HEAD.

Adopt a separate query before any recovery mutation; never use claim(false) as
inspection. No inferred worker death or safe retry from a missing claim/row.
Unlike Orca keyed retry guidance, this product has no authenticated recover/join
capability yet, so inspection always grants no retry. Keep descriptor, stored
task and claim time in ONE SQLite statement snapshot; validate linkage and
nonnegative time, exact full TaskSpec filtering, sanitize corrupt record errors.
An application read model is a ledger observation, not a domain execution result.
No schema migration or new dependency needed for existing V11 claim data.

Before code tests planned: absent/registered/claimed, reopen and stale/cancelled
task without mutation; full task mismatch; denormalized linkage/negative claim
corruption rejects; eight readers/writer exercise snapshot consistency without
granting execution; bounded CLI output and no args/env disclosure delegated.
Existing CLI Store::open may initialize/migrate: query itself writes no ledger,
but do not call the whole command filesystem-read-only. Missing record also
does not establish that this is the correct complete database.

Remaining outcome persistence, process reconciliation, crash injection, live
authority and containment stay open. This prerequisite is not crash recovery.

Additional integration source review: read Orca request-show CLI handler and
runtime `src/main/runtime/rpc/methods/orchestration/runs/mutation-request-show.ts`
fully, and `orchestration-mutation-request-show.test.ts:62–195`. Runtime uses
caller-fingerprint-scoped durable lookup, and tests distinguish pending from
death/restart, absent from no effect, and no duplicate mutation. Our caller
TaskSpec is only a filter, NOT equivalent authentication; no claim to copy that
security boundary. The upstream local fingerprint helper can initialize state,
another reason not to imply whole-command filesystem immutability.

Targeted verification: 16 store RPC tests pass (12 existing + four new).
Initial added scope test incorrectly changed a launch's task without its bound
execution policy; domain correctly rejected that fixture before query. Fixed
test to construct a standalone valid mismatched TaskSpec, then all passed.
Coverage: three historical states/reopen/cancel, eight scope fields, blank/missing
ID, consistent one-statement read, negative timestamp/host linkage corruption
with no repair, eight concurrent readers racing the one claim, and read-model
timestamp boundaries. Counts/events unchanged by queries; claim timestamp remains
historical, not refreshed by the current clock. No DB schema migration added.

Final verification: added corrupt-descriptor payload redaction case, bringing
targeted store RPC tests to 17 (five new). Concurrent-reader test passed ten
additional repetitions. CLI worker Bohr 01a093f8-6fb1-7343-a44d-bfbc83aaf72e
implemented only main.rs, rpc_launch_query.rs and its source receipt, then main
read all changes/assertions and reviewed source references. Five CLI tests pass,
including 15 complete-task fields, Unicode/escaping byte budget, exact LF cap,
nonpartial cap rejection, redacted malformed input, history and help caveats.
Worker completed and was closed after review; no active child remains.

`sh scripts/validate-foundation.sh` exit 0: 231 Rust + 12 Node tests pass,
7 packages/41 direct declared dependencies and architecture clean. The two
standalone ignored helpers retain their existing parent-invoked test paths.
`cargo fmt --all -- --check` exit 0. No release/platform or full-W2 claim.
Skill influence: domain/read-model separation prevents calling ledger data an
execution capability; lifecycle/CLI guidance separates inspection from mutation
and preserves explicit unknown process state with bounded output.

Implementation fingerprints (untracked product, no clean HEAD):
- application/src/rpc_query.rs: 3ec4a10600dfd2b761b00cbd33de325ae270e0683073b872e7a6ab6ed7f83ff6
- store/src/rpc_launch.rs: 0776c59b8463e9c7bbd18f3c20a57d416f5868adacae9df54830a647da7d7f0e
- store/src/sql/select_rpc_launch_snapshot.sql: 284ad260c7abff6ab2d5aaa1076ff0590d5defb1b86de2434afbdfd07c4fbbd8

Next: append durable launch outcomes and explicit reconciliation protocol for
claim/spawn/attach crash windows. Query absence must not cause automatic retry,
claim reset, PID-based kill, fabricated completion or current approval. Outcome
recording and process recovery remain unimplemented by this inspection change.

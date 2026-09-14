# W2 durable RPC spawn observation

status: durable ledger and trusted fixture recording verified; full W2 open | owner: main
source-gate: ready

Before code, read Orca clean 26f9fd8ea152ad6126c005e5ae602c7d201f4a99:
`src/main/runtime/orchestration/db/mutation-receipts/mutation-receipt-store.ts`
fully (begin, identity checks, complete/update, pending/discard and lookup);
`src/main/runtime/rpc/orchestration-mutation-executor.ts:1–235` (in-flight join,
completed replay, pending unknown and effect-possible tracking), and
`orchestration-mutation-question-db.test.ts:1–94` (identity conflict/replay and
namespaces). ICM clean 2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42:
`crates/icm-store/src/store/facts.rs:102–132`, tests/facts.rs:164–195, transaction
and normal supersession assertions. No upstream services/tests run. Product
AGENTS, PLAN next step, RPC domain/wire/ledger/query and migration tests read.
Product is untracked/unborn HEAD, not a clean revision.

Adopt bound identity and historical result replay; incomplete claim is unknown,
not safe retry. Use RAII IMMEDIATE transaction for observation + event/outbox,
no transaction spanning OS spawn. Do NOT copy discard-pending or overwrite a
completed receipt: one immutable observation per consumed launch; exact replay
false, differing report conflict. Existing reports remain caller/host claims,
not Orca's authenticated caller capability. No automatic spawn, reset or PID kill.

Specific model: RpcSpawnObservation binds the full RpcLaunchSpec, observed wall
time and one disposition: cancelled_before_spawn, expired_before_spawn,
spawn_failed, spawned. Only spawned requires a positive u32 process_id; others
require explicit null. This is the result of the launch attempt, NOT terminal
RPC completion, handshake, output receipt, cleanup or process-liveness evidence.
A later terminal receipt must be separate; a missing spawn observation never
proves that spawn did not happen. Wall clock can move backwards; do not enforce
observed time >= claim time. Reject negative/out-of-range times and invalid PID.

Strict schema-v1 wire with required fields including nullable process_id; no
unknown/duplicate fields, unrecognized dispositions or implicit enum-object
forms. Domain private/validated, Debug must not expose bound descriptor values.
V12 adds a table FK to existing consumed claims. No old migration modification,
no synthesized outcomes when upgrading V11. Reads validate stored full spec,
claim linkage and denormalized observation time within a consistent snapshot.
Late recording after lease expiry/cancel is diagnostic and never refreshes rights.

Before code tests planned: wire roundtrip and negative cases; actual SQLite
absent/unclaimed rejection, replay/reopen/cancel, changed-spec/report conflicts,
event/outbox rollback, corrupt data and concurrent same-report writers; V11
upgrade preserves rows/history and leaves new table empty. Main implements
contract/store; bounded worker implements migration/ledger tests with source-first
receipt. Host automatic recording/ownership on persistence failure, query CLI
projection and crash-process reconciliation are follow-up, not proven here.

Initial validation: `cargo check --workspace --locked --offline` exit 0;
`cargo test -p graph-protocol rpc_spawn --locked --offline` five tests pass.
Tests cover four dispositions, timestamp boundaries, direct domain PID matrix,
schema precedence/nested launch validation, required explicit nullable PID,
missing/duplicate/unknown fields, numeric overflow/types, alternate enum-object
forms and redacted Debug/unknown-disposition errors. Generic serde diagnostics
still need boundary redaction as elsewhere; not all parse errors are secret-safe.

Final integration: main reviewed all nine worker integration tests, fifteen SQL
fixtures and V11 upgrade test/history changes. Worker Feynman
01a09400-bbac-7fc3-b2bd-7a96ec9c5fa8 completed its disjoint write scope and was
closed after review. Store tests prove four dispositions/reopen/replay, thirteen
changed spec components, changed report conflict, late diagnostics, missing
claim rejection, event/outbox rollback, eight writers with one winner, seven
observation corruptions and six linkage corruptions. Upgrade uses real V11
enqueue/lease/register/claim APIs and compares complete persisted row values
and eleven migration records across upgrade/reopen; no outcome is fabricated.

Main also implemented host recording after a fresh claim; see the separate
`w2-rpc-spawn-recording-host-source-review-2026-09-12.md` receipt. Its seven real
fixture tests include persistence failure before/after commit with retained
Child ownership. Spawn-stage recording is now wired, terminal receipts are not.

Final commands: `sh scripts/validate-foundation.sh` exit 0, 248 Rust + 12 Node
tests pass; 7 packages/41 direct declared dependencies, architecture clean.
`cargo fmt --all -- --check` exit 0. The two standalone ignored helper tests
retain their parent-invoked paths. No full package is complete.

Final fingerprints (untracked product, not clean HEAD):
- domain/src/rpc_spawn.rs: 0f1a60b2752453f2045ce7357f0a93e25c094227ed477ebba0cf282356edc5d4
- protocol/src/rpc_spawn.rs: 59cf171f2add9986eca811129c9dfcb00057f9e92879312aad01d8cdb715a24e
- store/src/rpc_spawn.rs: cfbd8715c2dc071fff7f56c682a33f3f453610abd38bd58d463d8d4a98528f6b
- V12__rpc_spawn_observations.sql: a77a2de8abd18b76ac9b56d627daab8f61e110401e56b159c9e63cce818762e2

Next: expose spawn observations alongside claim inspection in a consistent
query/CLI view, then terminal RPC/output/pending receipts and explicit crash
reconciliation. Missing report stays unknown; a historical PID never authorizes
attach/kill or proves liveness. No reset, automatic restart, native dispatch,
production approval, containment or W1-I acceptance was introduced.

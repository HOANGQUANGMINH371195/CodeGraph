# W2 RPC recovery inspection CLI — 2026-09-12

## Source-study receipt (recorded before code)

Source gate: ready for this bounded CLI slice only; full W2 remains open.
Read AGENTS.md and PLAN.md §0.4.1 directly. Skills read: rust-router,
domain-cli, m07-concurrency (router-required), m06-error-handling. No delegation,
accounts, authentication, native execution, registration CLI, or live RPC work.
Writes limited to this report, crates/cli/src/main.rs, and
crates/cli/tests/rpc_launch_query.rs. PLAN synchronization is outside this grant.

Reference root: `/home/minh/projects/outsource/orca`.
Revision: `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`; `git status --short`
empty at inspection. Read actual source, not prior audits:

| Path relative to reference root | Read | SHA-256 (whole file) |
| --- | --- | --- |
| src/cli/orchestration-mutation-recovery.ts | lines 1–98, orchestrationMutationRecoveryError and helpers | aa0576ce24dd3f59630d01a58587d49eaec2a2d5512001fed5468f6792dadb08 |
| src/cli/orchestration-mutation-recovery.test.ts | lines 1–130 | d2b2659d3796ae63254343230a26707f25aa79e0398ca43c1a7e97f538bd9935 |
| src/cli/handlers/orchestration/mutation-request-show-handler.ts | entire handler | 66a907a2193da3be14664a91e6ff2d20635303f0ec459705232fb03a88454cbd |

License reviewed: orca/LICENSE, MIT, Lovecast Inc. 2026. Adapt behavioral
lessons with original Rust implementation; no copied TypeScript code.
Product root: `/home/minh/projects/project-graph-agent`; no resolvable HEAD
(unborn repository), all existing top-level content untracked at inspection.
Concurrent application/store changes belong to main and will be preserved.
Product sources read: CLI main/input/output, cancellation integration tests,
store rpc_launch tests (fixture, historical registration/claim/reopen/cancel,
full task matching, transaction and replay assertions).

## Adopt / avoid / adaptations and gaps

Adopt: unknown mutation outcome does not imply failed mutation or worker death;
inspection precedes any recovery decision. Tests explicitly assert no invented
dispatch and that absence does not prove retry safe. Handler performs request
lookup and surfaces old-server incompatibility rather than pretending absence.

Avoid: emitting original commands, retry commands, or raw recovery metadata.
This local command grants no retry or execution authority. It has no remote
runtime compatibility fallback, auth, registration, or process-launch path.

Adapt: call the application RpcLaunchQueryRepository through Store obtained
from tasks.into_inner(); match the complete caller-supplied TaskSpec. Expose
only launch id, host_id, connection_epoch, claimed_at_ms and recorded versus
not_recorded claim state in one schema-version-1 JSON line. Always include
observation_only=true, execution_authority_granted=false,
process_liveness=unknown, retry_authorized=false, snapshot_filter=caller_supplied.
Unknown id or mismatched task yields launch=null; this is not proof of an absent
process. Neither a recorded nor an unrecorded claim determines process liveness.

The query performs no ledger mutation; standard DB open may initialize/migrate.
The whole command is therefore not filesystem-read-only. Existing bounded input
reads at most 8 MiB plus one sentinel byte, but special files may block. Existing
serde parse errors can include unexpected payload values/field names. Use a
command-local fixed diagnostic for deserialization and domain-validation errors;
do not change shared input or other commands. Output uses output::json_line with
default 65536 and maximum 16 MiB, checking the entire line including LF before
writing stdout. This prevents partial output on cap rejection, not OS write faults.

Upstream does not provide this SQLite full-task contract or bounded/redacted JSON
projection: those are product adaptations. Main owns the concurrent application
and store snapshot API. Planned verification: missing/registered/claimed,
reopen, cancellation both before and after claim, all task contract fields,
exact serialized byte cap (LF included), cap-minus-one/zero/over-maximum failures,
malformed/type-invalid/invalid-domain secret input and oversized input. Fixtures
prepare historical ledger state via existing APIs; only the CLI under test runs,
never a described RPC child process. Verify events and snapshots remain unchanged.

## Implementation and validation

Implemented `rpc-launch ID TASK_SPEC --max-output-bytes` in main.rs with the
specified projection and application trait. No dependencies added. Skills informed
the stdout/stderr boundary and command-local error handling; no concurrency was
introduced. The new five-test integration file uses graph_protocol DTOs converted
to domain values, so CLI needs no direct graph-domain dependency.

Actual validation (2026-09-12):

- `scripts/with-local-tools cargo test -p project-graph-agent --test rpc_launch_query --locked --offline`: 5 passed.
- `scripts/with-local-tools cargo test -p project-graph-agent --locked --offline`: all 23 CLI tests passed (including those five).
- `scripts/with-local-tools rustfmt --edition 2024 --config skip_children=true --check crates/cli/src/main.rs crates/cli/tests/rpc_launch_query.rs`: passed.
- Only the two allowed Rust files were formatted; no workspace formatting.

Assertions cover missing id and absent registration; registered/unclaimed and
claimed historical rows; reopen; cancellation both before and after claim;
all 15 task contract fields (including six project fields); exact full JSON
object equality with no descriptor fields; Unicode/JSON escaping byte lengths;
LF-inclusive exact cap success and cap-minus-one/zero/over-16-MiB rejection with
empty stdout; 16-MiB accepted; malformed syntax, wrong type, unknown field,
invalid domain and oversized input. Test explicitly demonstrates that raw serde
wrong-type diagnostics contain the secret before asserting the command emits
only its fixed diagnostic. Events, claim timestamps and launch snapshots remain
unchanged across inspection. No described RPC process was spawned.

Final code fingerprints at handoff:

- crates/cli/src/main.rs: `16c82ce3685d70a4d95f82a8b25aa617ffd800bc2f3f9d156f348a9b9e8813cc`
- crates/cli/tests/rpc_launch_query.rs: `3c209368f64b2eeb7a7b7801a35176dfba0cc4d4f88bb99371e50e56a268cf0e`

Remaining integration gates: main owns `sh scripts/validate-foundation.sh`,
workspace format checking and PLAN synchronization, per the bounded handoff.
Main reports 17 Store RPC tests passing (including corrupt-descriptor redaction) and its concurrent-reader test passing
10 repetitions; those are main's evidence, not tests independently run here.
CLI tests do not fault-inject SQLite corruption or race the claim writer; main's
Store tests own that coverage. Shared parser diagnostics in unrelated commands
remain unchanged and may echo payloads. The standard DB open may initialize or
migrate; special-file input can still block, and OS stdout failures are outside
the serialization-cap guarantee. This implements inspection only: no authority,
live process proof, safe retry decision or full-W2 completion is established.

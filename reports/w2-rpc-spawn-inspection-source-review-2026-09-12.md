# W2 coherent spawn observation inspection

status: combined inspection and CLI verified; full W2 open | owner: main
source-gate: ready

Before code reread clean Orca 26f9fd8ea152ad6126c005e5ae602c7d201f4a99:
`src/main/runtime/rpc/methods/orchestration/runs/mutation-request-show.ts` fully,
`orchestration-mutation-request-show.test.ts:62–195`, and CLI
`src/cli/handlers/orchestration/mutation-request-show-handler.ts` fully. The
runtime returns stored receipt without redoing effects; tests distinguish pending
from death/restart and absent from no effect, and verify caller scope. Its parse
helper drops invalid receipt JSON: do NOT copy that silent-absence behavior.
No upstream runtime/tests run. Main also read product AGENTS, current query model,
single SELECT, spawn read/decode/write, CLI projection and cap tests. Unborn,
untracked product; not a clean HEAD. Rust/CLI/domain/concurrency skills read.

Adopt query-with-stored-report, no retry or liveness inference. Extend existing
single SQL statement to join launch, task, claim and spawn observation. Reuse a
shared validated spawn decoder: corrupt payload/linkage/time rejects the whole
query before scope filtering. Application model requires a recorded claim and
exact same launch when an observation is present. Change its constructor to
accept the optional observation explicitly; update every current call site.
No migration/dependency needed. This is a local snapshot filter, not Orca's
authenticated caller identity. Missing report remains unknown, not a no-spawn fact.

CLI adds `launch.spawn_observation`: explicit null or brief disposition/time/PID,
never the full bound descriptor. Keep unknown process_liveness, false retry and
execution authority, bounded serialized JSON including LF, fixed input diagnostics
and standard DB-open caveat. This is an additive change to the unreleased v1 CLI
projection, not a claim of compatibility with strict external consumers.

Before code tests: all four stored dispositions through query/CLI with reopen/
cancel; exact byte cap with observed result; corrupt observation query fails;
model rejects observation without claim/mismatched launch; concurrent readers
and writer see only consistent ledger combinations. No child processes beyond
the CLI under test, no spawn/recovery/kill path and no reference modifications.

Implementation uses one SELECT with optional spawn row (presence keyed by its
launch_id, not by nullable PID), then shared decode and explicit read-model
validation. Original separate observation reads retain their read transaction.
The constructor now takes a third optional observation argument at every call
site; no legacy default hides omitted state. Canonical disposition labels are
shared by wire encoding and CLI projection, without cloning the full wire report
to print its three brief fields. All storage validation precedes task filtering.

An initial nested Option/Result row conversion failed type inference (E0282/3);
replaced it with an explicit match at the SQL boundary. Focused tests then pass:
12 store spawn tests (three new) and six CLI query tests (one new). Existing
tests now inspect stored observations for all four dispositions and validate
corrupt observation failure through the combined query, including wrong scope.
New tests cover observation without claim, mismatched read-model launch, eight
readers racing claim then observation, and CLI reports after cancellation/reopen.
Full-scope tests include an observation; byte-cap tests include observed output
with Unicode/escaping and exact LF budget. No CLI corruption fault injection;
store tests cover corrupt payload/linkage and CLI propagates that error.

Final validation: `sh scripts/validate-foundation.sh` exit 0, 252 Rust + 12 Node
tests pass; 7 packages/41 declared direct dependencies, architecture clean.
`cargo fmt --all -- --check` exit 0. Two standalone ignored helpers keep their
parent-invoked paths. All current constructor sites inspected; no dependencies
or migrations changed. No subagent needed for this tightly coupled slice.

Final fingerprints (untracked product, no clean HEAD):
- application/src/rpc_query.rs: 14f929ab3eb836ea61cefebed0a5024fa7899ad39350d428a4b1ac0972b637ed
- store/src/sql/select_rpc_launch_snapshot.sql: e980809bca52f397d7eb617428f3fc5c40c812ac83828da29b7f4cf07aa48118
- store/src/rpc_spawn.rs: 8521bb953bde4958ef3d91596ae30290d51942b35c96d74fa772ef9909e48496
- cli/src/main.rs: 357ac36bff4fa9161d61369f1436d8f2b5014c3865191d571095ace30208477b

Skill influence: the read model enforces matching claimed launch/observation,
and CLI projection keeps historical data separate from authority while preserving
bounded output. Next: terminal RPC/output/pending receipts, then explicit crash
reconciliation. No live process probe, automatic retry/reset, PID attach/kill,
production approval, containment or native dispatch has been added.

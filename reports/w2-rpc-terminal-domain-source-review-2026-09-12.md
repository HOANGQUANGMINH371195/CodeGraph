# W2 launch-bound terminal receipt domain

status: done (domain only); source-gate: ready; scope: pure domain contract and tests.

- [x] Read relevant outsource source and test assertions before code.
- [x] Record adopt/avoid and gaps.
- [x] Choose invariants and regression tests before patch.

Orca clean HEAD `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`: read complete
`src/main/runtime/rpc/methods/orchestration/runs/mutation-request-show.ts` and
`orchestration-mutation-request-show.test.ts:62–150`. Handler scopes stored
receipt by caller, returns original result without repeating effects. Tests
cover completed original outcome, pending interrupted/concurrently running work.
Adopt separation of historical reports from retry/execution authority. Do not
adopt `parseReceipt` silently returning undefined on corrupt JSON. Upstream
tests not run. This source is an orchestration receipt, not our RPC byte receipt.

Product untracked/unborn source read: domain execution_receipt, analysis,
artifact, execution status, rpc_spawn and rpc_launch constructors; protocol
correlation identity/terminal snapshot. Reuse artifact metadata validation,
separate wall time and elapsed duration, exact immutable launch identity.
Do not coerce RPC into closed-stdin CheckRunBinding or derive task success from
exit code. Rust/domain skill informs private fields and checked constructors.

New contract: spawned observation only; output AnalysisRun matches execution
snapshot and task graph; complete streams require artifacts with exact run/kind,
snapshot/graph and within launch byte caps, distinct IDs. Nonnegative wall time
and signed-ledger elapsed range without wall-clock ordering. Terminal pending
IDs positive, sorted/unique, within pending cap; methods follow current protocol
nonblank/256-byte rule (including accepted control characters) and implicitly
uncertain. Last-frame input counters bounded by frame cap, not acknowledgement.
Reject NotSpawned/SpawnFailed terminal reports, Exited without Reaped, and
Unreaped with cleanup Complete. Retain unresolved RPC even alongside exit zero.

Planned tests: valid fields/redacted Debug, nonspawn/contradictory completion
matrix, timing bounds, output linkage/caps/completeness, pending ordering/caps
and method/input-counter limits. Wire, live bridge validation, CAS verification,
ledger and crash recovery remain subsequent work; no full W2 claim.

## Validation

Added private validated `RpcTerminalReceipt`, `UncertainRpc`, `RpcInputProgress`.
Five new tests pass, including output run mismatch in all six ProjectRef fields,
graph mismatch, artifact run/kind/cap/identity, nonspawn and contradictory
completion, pending ordering and byte-length limits, backwards wall clock,
and last-frame counters. Main reviewed constructor and assertions; no separate
reviewer. No wire/DB/host binding capability is implied by these tests.

Commands: `scripts/with-local-tools cargo test -p graph-domain --locked --offline rpc_terminal`
(5 passed), `sh scripts/validate-foundation.sh` (260 Rust + 12 Node passed),
`scripts/with-local-tools cargo fmt --all -- --check` (exit 0).
Architecture 7 packages, 41 dependency declarations, no errors. Two standalone
ignored helper tests remain invoked by parent tests. No migration/dependency.

Final untracked product SHA-256:
- `crates/domain/src/rpc_terminal.rs`: `8e87246dc8813c1120d439804d0444a28fbd67acb65124bb7c3bb5604a4f2f0d`
- `crates/domain/src/rpc_terminal_tests.rs`: `582bef2db0f7420f10990746ca36c5825742abe0e55bb79e3f861b01d8258c65`

Next: strict versioned wire contract, durable receipt ledger, launch/epoch-bound
conversion of actual supervisor result and CAS output verification, then crash
reconciliation. Existing CheckCompletion summary is not RPC task success and
must not be used as such. Full W2/W1-I remain open.

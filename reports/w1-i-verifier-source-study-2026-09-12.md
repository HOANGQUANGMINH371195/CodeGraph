# W1-I verifier source study — 2026-09-12

Status: source-gate `ready` for the bounded W1-I authority slice; no acceptance
authority is implemented by this receipt. Product flows and the exact upstream
process/repository examples below were reread before the implementation work.

## Sources reread

| Source | Flow observed | Adopt / avoid |
|---|---|---|
| `crates/execution/tests/composition.rs`, `crates/execution/src/lib.rs`, `crates/execution/src/linux.rs`, `crates/execution/src/output.rs` | owned fixture → distinct completion state + output bytes → artifact publication → immutable receipt → reopen/reverify | Reuse real fixture receipts and byte re-verification; never collapse cleanup/stream/reap unknowns into a pass. |
| `crates/application/src/check_binding.rs`, `receipt_output.rs`, `submission.rs` | rebind submitted candidate/policy/artifact before reads, verify registered bytes, then re-read submitted candidate | Any verifier must retain this pre/post reconciliation and make its final state transition atomic with its decision. |
| `crates/domain/src/execution.rs`, `execution_plan.rs`, `execution_receipt.rs`; `crates/store/src/execution_receipt.rs`, `outbox.rs`, `lib.rs` | receipt is a structural claim matching a plan; production string integration is disabled; events/outbox share one SQLite transaction | Use a new typed authority port and a new decision record; do not revive `integrate(task, string)`, accept one receipt as all required checks, or make a receipt insert itself change task state. |
| Existing receipts `w1-c-development-gate-audit-2026-09-11.md` and `w1-w2-dependency-review-2026-09-11.md` | prior OpenDev study identified distinct child/pipe/teardown observations and the process/repository boundary | Historical context only; reopen exact upstream source/tests before implementation, rather than treating this receipt as a new upstream read. |
| `/home/minh/projects/outsource/opendev/crates/opendev-tools-impl/src/bash/foreground.rs`, `opendev-mcp/src/transport/process.rs`, `process_tests.rs` | exit/reap, stream drain, timeout/cancel and process cleanup are distinct observations; the upstream descendant helper is best-effort | Preserve distinct receipt states; do not treat reader join timeout, `pgrep` output or best-effort teardown as verified integration authority. |
| `/home/minh/projects/outsource/temp-rs-ddd/crates/domain/src/repository/healthy_repo.rs`, `crates/infrastructure/src/repository/pg_healthy_repo.rs`, `crates/domain/src/repository.rs` | domain repository trait stays independent of the infrastructure adapter | Keep target-verification and atomic-decision ports typed at the application boundary; do not put SQLite or process/host dependencies in domain. |

## Gap to implement next

The current model has no typed accepted-decision aggregate, no exact collection
of all policy-required receipt identities, no target-head verification port, and
no atomic state/event/outbox transition consuming those facts. Therefore a
single W2 trusted-fixture receipt is evidence for tests, not an integration
grant. The next patch must define that authority explicitly, including replay,
conflict, cancellation-race and rollback tests; it must not silently infer it
from a zero exit, hash, stored receipt, or host ID string.

## Implementation progress after this study

`crates/domain/src/integration_decision.rs` supplies the exact policy aggregate:
it rejects missing, duplicate, extra or non-passing receipt claims and requires
every receipt binding to match one task/candidate/policy. The application now
has explicit `TargetHeadVerifier` and `verify_integration`: it independently
re-reads candidate bytes, plan/receipt/output bytes, and target observation
before returning a `VerifiedIntegration` pair. V16 stores versioned immutable
decision and target records. `VerifiedIntegrationRepository` re-reads all
durable bindings in one immediate SQLite transaction before its state/event/
outbox transition. Targeted tests pass for exact/missing/duplicate/extra/failed
checks, replay, injected outbox rollback and cancellation-wins.
The same test module now also proves exact decision replay after database
reopen and one commit/one replay across two SQLite connections.

The store suite also exercises `verify_integration → VerifiedIntegrationRepository`
with byte-verified candidate/stdout/stderr fixture blobs and a typed fixture
target verifier before the atomic transition. That closes the application-to-
store seam for the owned fixture; it does not authenticate a production target
host or make the constructed receipt a real W2 execution fixture receipt.

`TaskService::integrate_verified` exposes only that typed verified pair and has
no string/ID/receipt-list overload, so the formerly disabled legacy
`integrate(task, verification_string)` cannot accidentally become the public
success path again.

This is still not full W1-I: no test yet drives a real W2 fixture receipt
through the complete application verifier into this store transition; target
host authentication is an explicit adapter obligation without a production
adapter; and crash/cross-process race recovery remains open.

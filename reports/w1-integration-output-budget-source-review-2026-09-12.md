# W1-I integration output budget

## Failed-completion regression source gate

On resume re-read Ripwire RunCapture flags and runtracecheck timeout assertions
at the revision recorded below; re-read local reported_check_outcome and its
zero-exit/timeout tests, plus store receipt setup and application policy gate.
Extend the owned receipt fixture to accept explicit completion observations.
Test nonzero exit, timeout, cancellation and unverifiable cleanup through the
real ledger/output verifier, preserving raw receipt values. A panic target
proves no host access; task remains Submitted. This is synthetic receipt
integration coverage, not proof of real process execution or containment.

Verified: all four completion cases reject as Decision errors before target
access and preserve Submitted, with both required check names and valid output
bytes present. Existing positive exact-set integration remains green.
`scripts/with-local-tools sh scripts/validate-foundation.sh` exit **0**;
`scripts/with-local-tools cargo fmt --all -- --check` exit **0**.
[Full foundation log](validation/foundation-completion-policy-2026-09-12.log).

## Follow-up source gate: reject invalid check sets before target I/O

Re-read the same Ripwire capture/status source and timeout test on resume:
failed/timed-out observations must not be reinterpreted as success. Their
revision and hashes below remain the reference; this source has no product
integration policy evaluator. Local `IntegrationDecision::new` already uses
`RequiredChecks::evaluate` to reject missing/duplicate/failed observations, but
application calls the target host before that constructor. Reuse the existing
evaluator before target access; retain final constructor/store rechecks.
No new authority or alternate policy rules. Add store regressions with a target
port that fails if called: incomplete set and duplicate check names under
distinct run IDs must return policy error, leaving Submitted. Existing exact
set positive remains the success control. No source copied.

Follow-up validation: shared `RequiredChecks::evaluate` now runs after output
verification and before target access. Missing and duplicate-check-name sets
under distinct run IDs return Decision errors; a panic-on-access target remains
untouched, task stays Submitted. Exact-set integration remains green. This
regression does not independently exercise all failed-outcome variants.
Foundation exit **0**, fmt check exit **0**. Log:
[foundation integration policy](validation/foundation-integration-policy-2026-09-12.log).

Source gate before patch: re-read Ripwire `src/verbs_change.h:730–788`
RunCapture/append/text and `test/runtracecheck.sh:132–146` timeout assertions.
Revision `48222d62f41c6e15f60855127c1d9ee06b3aed4c`, Apache-2.0 previously
inspected. Test SHA-256:
`36f25169b8868df8d2cd41d571f77ed1b4d47595b6901bd797351e7a82ffe63e`.
Append consumes retained capacity across chunks instead of resetting it.
Tests require honest bounded-failure reporting; they do not cover integration
receipt sets. Adopt cumulative accounting, avoid head/tail truncation because
verification must reject insufficient budget instead of accepting partial bytes.

Read local application integration_verification/receipt_output, domain
IntegrationDecision and store integration fixture/setup/receipt/positive test.
Each fixture receipt has two one-byte outputs; two checks currently succeed
with output_max_bytes=2 because the full allowance is reused per receipt.
Change this to one remaining allowance, checked subtraction of both descriptors
before output reads; retain reader hash checks and sentinel semantics. Add
exact four-byte positive and three-byte negative with submitted state preserved.
This is per-call output payload accounting, not a global memory limit or
execution/target authentication. Candidate reads retain their separate budget.

## Verification

Application now subtracts stdout and stderr declared lengths with checked
arithmetic from one remaining budget, before opening that receipt's streams.
Each receipt still undergoes binding/registration/hash verification. Duplicated
content hashes do not refund a read. `OutputTooLarge` is a static error.

Updated store integration regression proves a 3-byte budget rejects the two
2-byte receipts while preserving Submitted, and an exact 4-byte budget allows
the existing verified transition. Foundation command
`scripts/with-local-tools sh scripts/validate-foundation.sh` exited **0**;
`scripts/with-local-tools cargo fmt --all -- --check` exited **0**.
Full log: [foundation integration budget](validation/foundation-integration-budget-2026-09-12.log).
W1-I remains partial: fixture host/target observations are not authenticated
production execution and this change does not establish semantic graph facts.

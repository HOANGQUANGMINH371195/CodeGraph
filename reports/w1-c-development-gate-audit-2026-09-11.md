# W1-C development contract gate audit

Scope: PLAN §11.0 W1-C only. This is NOT completion of W1, W2, W0 or the product.
Status: W1-C DEVELOPMENT gate satisfied for owned trusted W2 fixtures only.
The requirement map and exercised assertions establish this limited gate;
absence of reviewer findings is supplementary, not the sole basis for completion.

Source first: parent re-read OpenDev clean
`d32c660e4eed1a8e988d1fd58da88e41ba641d08`, bash/foreground.rs:110–175 and
helpers_tests.rs:1–30 under crates/opendev-tools-impl/src/bash. Separate child
and stream observations are required; upstream ignores reader-join timeout
results and display truncation is not byte provenance. Tests inspected, not run.
Prior dependency correction is navigation/history, not current implementation
evidence. Actual Rust files and assertions below were inspected.

## Requirement-to-evidence map

| Exact W1-C requirement | Current implementation and exercised evidence | What it does not establish |
|---|---|---|
| Task/policy/lease/fencing contracts | domain/lib.rs TaskSpec/Lease; domain/checks.rs RequiredChecks; store/check_policy.rs; `guarded_lease_requires_exact_policy_and_contract_before_fencing`, policy replay/restart and stale-owner tests | Approval of commands or original expiry attestation in an execution report |
| Versioned command | domain/command.rs and protocol/command.rs; required fields/schema/argv/path/hash/limit tests; application/fingerprint.rs golden command/env vectors | Host executable/environment actually match those claims |
| Versioned execution receipt | domain/execution_receipt.rs, protocol/execution_receipt.rs; roundtrip, explicit null/output fields, unknown authority/schema and output substitution tests | Receipt producer authentication or execution proof |
| Snapshot/candidate/check-policy binding | domain/check_run.rs and execution_plan.rs; application/check_binding.rs; V9 plan/current-candidate/policy/analysis validation and mismatch/corruption tests | Candidate actually applied in worktree or target head unchanged |
| Exit/reap/stdout/stderr/cleanup distinct | domain/execution.rs and protocol/execution.rs; zero exit with incomplete/reap/cleanup reports stays Unknown; cancel/timeout cannot become Passed | Real process/pump termination or platform cleanup |
| Validation and ledger tests | V7–V10 immutable policy, receipt, plan, one-shot claim; restart/conflict/scope, eight-connection winner, plan/receipt/cancel races, event/outbox rollback, migration and receipt subprocess-exit tests | Exactly-once spawn, launch-crash reconciliation, full platform or power-loss safety |
| Registry/replay and receipt comparison (follow-up audit requirement) | store/execution_plan.rs, execution_receipt.rs, execution_launch.rs; exact plan matching, no retrospective plan after receipt, claim never recycled; application/receipt_output.rs checks registered raw output hashes and rechecks submission | Byte checks do not verify cleanup/exit claims; legacy unplanned receipts remain diagnostics only |

Validation run this audit: `sh scripts/validate-foundation.sh` exited 0, 151 Rust
tests passed and 11 Node tests passed. One subprocess-only helper is ignored by
normal discovery but its parent runs it twice and checks exit 23 at both commit
boundaries. `scripts/with-local-tools cargo fmt --all -- --check` and PLAN mirror
comparison passed. Architecture guard remains 6 packages/30 direct dependencies.

## Required boundaries when opening W2 fixture development

- Only harness-owned trusted fixture executables in owned temporary directories;
  no native account, untrusted repository jobs, merge or graph writes.
- New execution adapter owns processes and pipes; domain/application/protocol
  must not acquire process dependencies. New dependencies require architecture review.
- Host sets argv/cwd/environment/executable and verifies fingerprints; never infer
  approval from JSON, stored plan, a hash, receipt or bool claim alone.
- Enforce bounded raw output while reading, independent cancellation/control,
  reap and pump deadlines; inherited/open descendant pipes must not hang cleanup.
- Treat crash after claim and unknown launch result as uncertain; no automatic
  relaunch from a historical claim or an expired local wait.
- Produce actual receipts for W1-I verifier: candidate bytes, source/target
  snapshots, authenticated host execution, and atomic acceptance remain unimplemented.
- W2/W1-I/full package gates, sandbox/platform tests and W0 baseline remain open.
  Full package count remains 0/14. Native W3 stays gated by full W1/W2 acceptance.

## Snapshot evidence

Independent read-only reviewer Hubble (native subagent
`01a09397-a175-74b2-9653-2fcf3218271c`, inherited parent model) inspected the
plan/launch/receipt/output trust boundary and tests. No concrete W1-C blocker
found in that bounded scope. Reviewer explicitly did not claim full coverage or
runtime acceptance and ran no tests. Its source receipt re-read Grit clean
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`, transactional acquisition :48 and
separate-connection assertions :452 in src/db/sqlite_store.rs; do not transfer
TTL reclaim/same-owner renewal to uncertain execution. Parent reviewed these
claims against the mapped sources above. Reviewer was closed after completion.

Next: implement W2 adapter against owned fixtures, with source-first study of
the exact process/pipe path before code. Keep all full package gates open.

Workspace is uncommitted/untracked; no clean product revision is claimed.
SHA-256 of reviewed critical files at audit:

```text
cf17669092410946f9f31cf9132802b5e1cf499da5b077d101c051225a331e17  crates/domain/src/command.rs
30b88cd74bc315b5ab3230ddc83e22d99e82f271cd4c5a4f2bda11eeef7afdfa  crates/domain/src/check_run.rs
8b2f7efd004faa018ffc28a1866f3c36e1a4a395217654d68871523b6fec9ae0  crates/domain/src/execution.rs
3ca5946b26352d1393e4f7802850a00fc9f9cb46e326909a7232289d2ac44f5c  crates/domain/src/execution_receipt.rs
ef38a27251be68009520dda598102103cda08c7ee02fc834c653446ec0c5d61f  crates/store/src/execution_plan.rs
9abc32f557467b9df066183bf71f391ab62e0b87abc4a42ec4ae273dba9154e8  crates/store/src/execution_launch.rs
bfdd8a73bff0e09d7de6dba39c9e539cc4607e7c195ab927a802c3dcdda65a6c  crates/store/tests/submission_content.rs
cbd5e8698dc5148b6b04caa4fdeca2b65809c26f689008af887c13c569d97be0  Cargo.lock
```

# W1 submitted candidate read boundary

status: review (partial W1) | owner: main | source-gate: ready

Source before code: Grit clean HEAD
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`,
`src/cli/mod.rs:905–949` retains worktree/branch on failed merge;
`src/git/mod.rs:133–218` prepares candidate then checks main-tree cleanliness before merge;
`:568–622` tests clean merge and dirty-tree refusal with branch preservation.
Tests read, not run. Adopt candidate/target separation and preserve candidate
until verification; do not treat merge command success or Grit locks as a harness
verification authority.

Current Store::submit_candidate changes task state and appends artifact text in
one transaction; the task spec, owner/fence and submission event already exist,
but verifier has no typed query joining them. Implement SubmissionQueryRepository
and SubmittedCandidate read snapshot from one SQL statement, no migration and no
state write. Preserve event sequence as submission identity and check exactly
one submission event for current one-shot lifecycle. Missing/duplicate event,
invalid spec/row identity, owner/fence/time/artifact become corruption errors.
Unsubmitted/cancelled tasks return None. This is an observation, not a lease or
verified candidate capability, and does not validate artifact existence/content.

Query remains host-only unscoped ledger access, analogous to TaskQueryRepository;
not an agent endpoint. The future verifier must bind returned task scope with
registered artifact, current snapshot/check receipts and target before authority.
No fabricated acceptance and no reopening integrate(string).

Acceptance before code: before/after submit, reopen preserves identical metadata,
read emits no event, cancellation hides current candidate, malformed/missing/
duplicate submission records are rejected. No existing migration/SQL semantics
rewritten. Read existing task_query.rs and submit_candidate before this change.

## Verification

- Added application SubmittedCandidate/SubmissionQueryRepository and one SQL
  statement in store/src/sql/select_submitted_candidate.sql; no schema changes.
- Two tests: query before/after submit, identical reopening, no extra events,
  cancellation hides candidate; corrupt fixture covers missing/duplicate submit,
  absent owner, zero fence, invalid spec, blank artifact and negative timestamp.
- First corruption fixture attempted event deletion and correctly hit an outbox
  FK. Changed fixture to relabel event kind, preserving FK enforcement, then
  reran all checks. No production constraint was disabled.
- `sh scripts/validate-foundation.sh`: exit 0, 92 Rust + 11 Node tests pass;
  architecture 6 crates / 30 direct dependencies unchanged.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- submission_query.rs SHA-256:
  `050589474bc6b311608c563cc7a6835483c4232f32c5434046b3ce117b5dc22c`.
  SQL SHA-256:
  `039831d3aaa8f9d83ffc71418d8bc1c7f8e56771f8019da341e136a52c1383ca`.
- Main review: one-shot lifecycle assumption explicit, no candidate acceptance,
  migration rewrite or CLI exposure. Owner/fence are current row observations,
  not independent historical lease authentication. Future retry history needs
  explicit attempt binding, not arbitrary selection of the latest submission.
  rust-router/m09-domain guided separation of observation from authority.
- Next: bind this submission to registered artifact and verified content/scope,
  then validated source/check/target-head evidence and atomic integration decision.
  Verified integration remains disabled; no full W1 acceptance claimed.

# W2 terminal receipt inspection

status: done (inspection task only); source-gate: ready; owner/reviewer: main agent.

- [x] Read current Orca mutation receipt store `beginMutationReceipt` and
  `completeMutationReceipt` (37–114), and orchestration-mutation-question-db.test.ts
  (17–55). Revision 26f9fd8ea152ad6126c005e5ae602c7d201f4a99, clean worktree.
- [x] Read product rpc_query, rpc_launch, rpc_terminal, CLI projection and test
  fixtures. Adopt exact identity/replay and transaction boundaries; avoid Orca's
  mutable receipt overwrite and do not infer process state from persisted data.
- [x] Design/tests before code: extend launch snapshot with checked optional
  terminal receipt, read launch/spawn/terminal and references in one deferred
  transaction, validate before task filter. CLI emits completion/timing/count only,
  no method names, output bytes or authority. Test before/after receipt, mismatch,
  corrupt reference before filter, CLI cap and non-disclosure; no new migration.

Skills: rust-router, domain-cli, m07-concurrency; keep stdout structured and
bounded, use a synchronous snapshot rather than adding async/shared state.
Upstream tests not run. Product untracked worktree; no independent reviewer.

## Implementation and verification

- Added checked optional terminal receipt to application launch snapshot. Existing
  constructor remains receipt-free; attachment requires exact spawn equality.
- Store uses one deferred transaction for launch join and existing terminal
  linkage validator; corrupt terminal/run/artifact data is checked before full
  caller TaskSpec filtering, without claim/event/receipt writes.
- CLI adds nullable terminal brief, retaining unknown liveness and false retry
  authority; no pending method, run descriptor or blob contents are serialized.
  Unreleased v1 additive output change requires strict consumers to adapt.
- New store test covers no terminal, exact linkage, mismatched/missing spawn,
  reopen and unchanged counts. Existing 12 corrupt-reference cases now also
  exercise launch inspection with matching/different task filters.
- New CLI test checks receipt projection after reopen, secret non-disclosure,
  exact output-byte cap including LF and empty stdout on cap failure.
- Initial compile checks caught ambiguous closure error type and incorrect
  ExecutionCompletion module path; both corrected before the final gate.
- `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0,
  278 Rust + 12 Node passed, no failures. Architecture: 7 packages/41 direct
  dependencies, no errors. `scripts/with-local-tools cargo fmt --all -- --check`:
  exit 0. No migration or dependency added.
- SHA-256 source fingerprints:
  - application/src/rpc_query.rs: `4adf6d0377b6330c1cc6a4ae8f8b4449fce99c73f12a0c13a7a64628c182165e`
  - store/src/rpc_launch.rs: `89d9240d5f001214f31df1a1102e41f23f5fc60d47da84df8fc254043a0ca9a4`
  - cli/src/main.rs: `9f1f30dc540c9340a1a9dface672a50b2298612aad485cef6b56692510b19327`
  Paths relative to `crates/`; these are untracked files, not a clean revision.

## Remaining scope

No OS-crash/reconstruction, production authority or containment. Receipt metadata
is not fresh CAS verification. Standard DB open can initialize/migrate. Existing
concurrent claim/spawn tests pass, but no new terminal-writer interleaving test
was added in this task. Full W2/W1-I and W0–W13 remain open.

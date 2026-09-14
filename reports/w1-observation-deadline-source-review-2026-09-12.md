# W1 observation deadline — source-study receipt — 2026-09-12

## Materialization follow-up source gate (before patch)

Re-read Ripwire `verbs_change.h:831–885`: draining and child wait consume
the same elapsed deadline; timeout does not grant a fresh wait budget.
Re-read `test/regression.sh:40–65`: byte equality requires nonempty successful
outputs, avoiding vacuous success. Revision
`48222d62f41c6e15f60855127c1d9ee06b3aed4c`; Apache-2.0 license inspected.
No upstream code copied. Local `materialize_source_snapshot` currently creates
two separate observation budgets and leaves copying/freezing unbudgeted.
Adopt one deadline across both stable pairs, copy chunks, and freeze traversal.
Retain cleanup without deadline so expiry does not skip owner cleanup. Add
deterministic expired-budget tests for copy/freeze and preserve positive
materialization tests. OS calls remain non-interruptible; no atomicity or
analyzer attestation claim follows. This extends the existing deadline mechanism
because upstream has no product-specific materialized source contract.

### Follow-up verification

`materialize_source_snapshot` now creates the deadline once, before root
resolution. Both stable observation pairs receive it; copying checks it before
entries and chunks and after sync; freezing checks it during traversal and
after permission changes. Owner cleanup retains its independent lifecycle.
The expired-budget regression confirms both copy entry points refuse before
creating a destination file and freezing refuses before changing permissions.
Existing positive snapshot, symlink and cleanup tests remain in the foundation
suite. Blocking filesystem calls and cleanup can still exceed elapsed budget;
this is cooperative timeout enforcement, not a hard realtime bound.

Validation completed after resuming the existing process handle (no duplicate
run): `scripts/with-local-tools sh scripts/validate-foundation.sh` exit **0**;
`scripts/with-local-tools cargo fmt --all -- --check` exit **0**. The prior
interrupted workspace run was not used as proof. No W1/W2 package gate closed.

## Scope

- Work item: make the bounded Git snapshot observation use one wall-clock
  budget across both stable observations and all file hashing.
- Product repo: `/home/minh/projects/project-graph-agent`.
- Source gate: ready before the implementation patch.

## Source and tests read

| Reference | SHA-256 | Lesson carried forward |
|---|---|---|
| `ripwire/src/verbs_change.h` | `6165347d4eef0bf4b3cdcfe86869f1ecb3d6ed5e6f55c899ba398354e35a9959` | command capture uses a bounded poll/deadline loop and kills the child on expiry; timeout is part of the execution contract |
| `ripwire/test/regression.sh` | `2d74f34d499ed438d233ac42e854234f1f1210cbc3997d2c9f6cae731ce6933f` | determinism and cache transparency are tested as whole-output properties, not inferred from a successful subprocess |
| `ripwire/src/ingest_cache.h` | `d6f81ac639b407704169d7cdb19c1cc1ac7b48488444da9caa7593ac0ad807be` | content/parser-version identity plus temp-then-rename protects the previous valid cache from a failed write; this is useful for future materialized snapshots, not proof of a live-tree snapshot |
| `codegraph/src/sync/worktree.ts` | `4c4878f2960fecd5934c01810092dcb9457fce84a677f3c8b429a630bedf2d5d` | worktree/common-directory identity belongs at the host boundary and must not silently select another worktree |
| `codegraph/__tests__/sync.test.ts` | `8e1c87992878a15fbba60eaf6d1ce0d28fc2b4e9dfae3e543cb74f8f0ccd2b74` | modified, untracked and deleted entries need separate freshness coverage; clean status alone is insufficient |
| `crates/source/src/snapshot.rs` | `0bd772813753e1e4bfba854a3aa59a2c620e8a10de867f5c42614bc3d43edefb` | current implementation applies a timeout separately to each Git child and performs two complete observations, while file reads have no operation deadline |
| `crates/source/tests/snapshot_authority.rs` | `e6fd45ff29ebf642e5d635477f57a6bb9d5dc7a3fc158a5eb0901bc2e28a7831` | existing authority tests cover mutation, deletion, mode, symlink, linked worktree, budget and fail-closed behavior; preserve these as regression gates |

## Decisions before patching

- Create one private observation deadline at the start of
  `observe_stable`; pass it through both observations, Git probes, path
  inspection and file hashing.
- Derive each child-process wait limit from the remaining operation budget,
  so a second observation cannot silently receive another full command
  timeout. Check the deadline between file operations and hash chunks.
- Add a bounded, static `SnapshotTimeout` error. Do not expose Git stderr,
  paths or partial identity, and do not return a binding after expiry.
- Preserve the public config field and its maximum; its meaning becomes the
  total authority observation budget rather than an independent budget that
  can be spent once per child.
- Keep the two-observation comparison and the source reader's later hash
  verification. This closes a liveness/accounting gap only; it does not make
  a mutable worktree atomic or eliminate the `symlink_metadata` → `open`
  TOCTOU window.

## Planned tests

- unit-test remaining-budget expiry and positive remaining time without
  relying on scheduler timing;
- keep the existing `graph-source` mutation/budget/linked-worktree suite
  green;
- rerun the CLI snapshot-authority and foundation suites so timeout failures
  remain fail-closed and produce no success JSON.

## Deliberate gap

This task does not implement an immutable materialized tree, no-follow/openat
containment, analyzer execution attestation, semantic CodeGraph/Joern facts,
accepted fact publication, or cross-platform parity.

## Implementation and verification

Implemented in `crates/source/src/snapshot.rs`:

- `ObservationDeadline` is created once by `observe_stable` and shared by both
  full observations;
- `run_git` uses the remaining operation budget for child polling and kills the
  child on expiry;
- Git text/path parsing, directory entry inspection and regular-file hashing
  check the same deadline between bounded steps;
- expiry returns the static `SnapshotTimeout` error and never returns a partial
  project or binding.

Verification:

- `cargo test -p graph-source --locked --offline`: **23 passed, 0 failed**
  (4 unit, 6 artifact, 7 snapshot-authority, 6 verification; doc-tests 0/0);
- `cargo test -p project-graph-agent --test evidence_snapshot --locked --offline`:
  **1 passed, 0 failed**;
- `sh scripts/validate-foundation.sh`: **exit 0**, architecture 8 packages /
  49 reviewed direct dependencies, all 16 architecture probes and the complete
  locked workspace test suite passed;
- `rustfmt` could not be invoked because this environment has `cargo` but no
  `rustfmt`/`rustup` binary. Compilation and tests are green; no full-format
  claim is made.

The implementation narrows the liveness/accounting gap and preserves the
existing source and CLI contracts. It still cannot interrupt a blocking OS
filesystem call, freeze a mutable worktree, or make the stable pair atomic.

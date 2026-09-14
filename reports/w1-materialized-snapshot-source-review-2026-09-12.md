# W1 materialized source snapshot — source-study receipt — 2026-09-12

## Scope

- Work item: replace the `verify-evidence --snapshot-config` reader's mutable
  worktree root with a private, materialized source tree.
- Product repo: `/home/minh/projects/project-graph-agent`.
- Source gate: ready before the implementation patch.

## Pre-code checklist

- [x] Source selected by mechanism: cache-safe materialization, worktree
  identity, bounded copy and resource cleanup.
- [x] Ripwire and CodeGraph implementation/tests were read again for this
  task; this receipt is not relying on the previous stable-pair receipt.
- [x] Main flow, refusal paths, mutation boundary and cleanup ownership are
  recorded below.
- [x] Implementation and regression tests are scoped before coding.

## Source and tests read

| Reference | Revision/fingerprint | Lesson carried forward |
|---|---|---|
| `ripwire/src/ingest_cache.h` | `d6f81ac639b407704169d7cdb19c1cc1ac7b48488444da9caa7593ac0ad807be` | cache records carry version/content/stat identity; a stale or corrupt record is dropped and reparsed, never trusted silently |
| `ripwire/test/portablecachecheck.sh` | `43d559677557cfcc7159a4daa05a31ead4205e0a45e3137c4a6d2f68daf12988` | a portable artifact is consumed without mutating the committed bytes; warm/cold output must be byte-identical |
| `ripwire/test/cacheidentitycheck.sh` | `7405a5d97f583b86ee3c3d85a7df7c6a726dfefdc75e96025b3567916b27e846` | every invalidation/refusal reason is disclosed; a single usable bit is insufficient |
| `codegraph/src/sync/worktree.ts` | `4c4878f2960fecd5934c01810092dcb9457fce84a677f3c8b429a630bedf2d5d` | worktree root and Git common directory are resolved at the host boundary; a different worktree must not silently borrow the index |
| `codegraph/__tests__/worktree-detection.test.ts` | `ae39e053eb49153eec1467a75ca353dd8a4b81e72896180f41a068ef1cca63df` | real Git tests cover linked worktrees, changed/untracked/deleted state, and stale detection cache; successful commands alone do not prove the selected tree |
| `crates/source/src/snapshot.rs` | `11dd37b6d3aa7b1b31208ec0e025f0a8d882ad57434bbecbf49b4bd9eaa51601` | current authority creates a stable pair but explicitly leaves a post-observation read race; the materializer must consume the observed path set and re-observe after copy |
| `crates/source/src/lib.rs` | `799595f6c464afd6fd8a2597962ea41d56a50ee29e6d5a4c601b2efb69fa86c1` | `DirectorySource` is a read capability with scope/hash/size checks; it should own a directory handle rather than reopen arbitrary caller paths |
| `crates/application/src/source.rs` | `174e69c2c0dd565f9e5f5046040e6f8b971c4e7b361092f58496912903a31b55` | snapshot binding and analysis-run identity must be checked before source read; a bound read still remains untrusted source, not an accepted graph fact |

## Flow traced

1. Host config selects one canonical worktree and identity.
2. `GitSnapshotAuthority` performs a complete stable observation and validates
   the caller's exact `ProjectRef`.
3. The authority copies the observed non-deleted source entries into a fresh
   owner-scoped standard-library temporary directory through `cap_std::fs::Dir`
   relative paths. Regular files use a
   no-follow final open; safe internal symlink reads are normalized into a
   regular file containing the captured source bytes, so the reader never
   depends on a mutable link target later.
4. The destination is frozen read-only on Unix and held by an RAII owner. The
   public reader owns only the capability handle, while the materialized owner
   keeps the temporary tree alive.
5. The authority performs another stable observation. Any changed/deleted/
   added/mode/content/path identity, copy error, unsafe link or budget excess
   fails closed and drops the temporary tree.
6. The CLI verifies the returned binding and exact `AnalysisRun` against the
   materialized `DirectorySource`; source hash/line checks remain mandatory.

## Decisions: adopt / avoid / gap

- Adopt Ripwire's versioned/content-bound, temp-then-rename and self-healing
  posture conceptually, but do not add a persistent cache: a source snapshot is
  an owned per-run capability, not reusable authority.
- Adopt CodeGraph's explicit worktree/common-dir boundary and real Git mutation
  tests. Do not treat its best-effort warning cache as snapshot proof.
- Use an owner-scoped standard-library temporary directory with RAII cleanup
  and capability-relative I/O instead of exposing the live worktree to the
  analyzer. No raw path is accepted as the source reader after materialization.
- Normalize safe symlinks to captured regular-file bytes; deny external or
  non-regular targets. This avoids preserving a link whose target could later
  escape or mutate outside the materialized tree.
- Deliberate gaps: a stable pair is still not a kernel atomic snapshot; a
  source that changes and returns to exactly the same observed identity cannot
  be distinguished; read-only file modes are not a hostile same-UID sandbox;
  Git non-UTF-8 path support and analyzer execution attestation remain open.

## Planned tests

- materialized source continues returning old bytes after the live worktree is
  changed or deleted;
- materialized tree has private/read-only entries on Unix and is cleaned up by
  owner drop;
- safe internal symlink is self-contained while an external symlink fails
  closed;
- CLI snapshot-config uses the materialized reader and retains empty stdout on
  authority/materialization failure;
- existing source, snapshot-authority and evidence-snapshot suites remain
  green.

## Implementation and verification

### Symlink byte identity follow-up source gate

Re-read CodeGraph worktree root/common-dir resolution and Ripwire saveCache
checked publication, then local inspect_files/copy_observed_file and internal
link tests. Found symlink observation hashes link text only; materialization
previously followed links without comparing target content to an observed hash.
An ignored target can therefore contribute bytes absent from ProjectRef.
Adapt containment resolution to require a Git-visible observed regular target,
copy that target through the capability with no-follow and its length/hash,
and preserve the alias destination. Reject targets outside the observed set.
Tests retain the supported internal-link behavior and reject ignored targets,
including changed ignored bytes with unchanged project identity. This restriction
is explicit until target-content identity is added to the snapshot schema.

Implemented capability canonicalization and observed-target lookup before
copy, using the target's recorded digest/length and no-follow regular-file
open. The ignored-target regression and existing internal-link positive pass.
Foundation exited 0 (`/tmp/graph-symlink-identity.log`), with 27 source tests;
project-local fmt check passed. Atomicity and hostile same-UID isolation remain
outside this proof.

### Analysis admission before materialization

On the next resume, re-read CodeGraph worktree.ts root/common-dir probes,
the local store analysis lookup, application identity verifier and CLI
evidence_snapshot test. CodeGraph has no AnalysisRun admission equivalent;
adapt the existing exact local repository lookup before CLI authority creation.
The current CLI copies/hashes the tree before verify_bound_source_identity
checks the run. Add a preflight run lookup and retain the later check. Test a
missing run with an unavailable source root: the run error must win before
filesystem canonicalization, Git probes or materialization. This closes the
CLI ordering gap without treating registration as execution attestation.

Implemented the CLI exact run preflight and retained post-materialization
verification. The subprocess regression supplies a nonexistent source root
while the run is absent and requires the admission diagnostic with empty
stdout. Foundation gate exited 0 (`/tmp/graph-analysis-admission.log`), including
the expanded CLI test; project-local fmt check exited 0.

### Cleanup follow-up source gate

Re-read Ripwire `saveCache` failure cleanup (ingest_cache.h:2478–2518)
and portablecachecheck.sh cleanup trap on resume. Adopt cleanup ownership
covering failure before publication as well as normal completion. The local
materializer currently thaws only in MaterializedSource::drop: an error after
freeze but before constructing that value leaves read-only directories behind.
Move thaw into OwnedTempDir::drop and test owner cleanup directly for fully
and partially frozen trees, before any MaterializedSource exists. This is a
standard-library adaptation; upstream handles temporary files, not frozen trees.

Implemented thaw in OwnedTempDir::drop and removed the later duplicate drop
hook. The new Unix regression covers fully and partially frozen unpublished
trees. `sh scripts/validate-foundation.sh` exited 0 (log:
`/tmp/graph-foundation-cleanup.log`); `scripts/with-local-tools cargo fmt --all
-- --check` exited 0. The source package now has 26 passing tests. The local
tool wrapper resolves rustfmt, correcting the earlier direct-command limitation.

Implemented in the product repo:

- GitSnapshotAuthority::materialize_source_snapshot reuses the stable
  observed file set, validates the exact project/graph binding, copies into a
  private owner-scoped standard-library temporary directory with
  capability-relative paths and bounded reads, and re-observes the live
  worktree after the copy.
- Regular files use a no-follow final open and verify their observed length and
  SHA-256 while copying. Safe internal symlink reads are captured as regular
  files; an external/non-regular target fails closed.
- The destination tree is frozen read-only on Unix. MaterializedSource owns
  both the directory reader and the temporary-directory owner, restores
  permissions before drop cleanup, and exposes the temporary root only while
  the owner is alive.
- To preserve the Clean Architecture dependency direction, `tempfile` was
  removed from `graph-source` production dependencies and retained only for
  integration tests. Runtime cleanup is provided by `OwnedTempDir`, using
  `std::fs::DirBuilder`, a bounded collision retry, and `Drop`.
- graph-application::verify_bound_source_identity separates the already
  bound/materialized handoff from authority acquisition while preserving the
  analysis-run-before-source-read and source-hash/line checks.
- verify-evidence --snapshot-config now materializes before verification;
  legacy caller-supplied verification and all other source commands remain
  unchanged.

Verification:

- cargo test -p graph-source --locked --offline: **25 passed, 0 failed**
  (4 unit, 6 artifact, 9 snapshot-authority, 6 verification);
- cargo test -p project-graph-agent --test evidence_snapshot --locked --offline:
  **1 passed, 0 failed**;
- sh scripts/validate-foundation.sh: **passed**, including architecture
  validation (8 packages, 49 reviewed direct declarations), Node checks and
  the locked offline workspace test;
- cargo test --workspace --locked --offline: **full workspace passed**,
  including the new materializer tests and all existing execution/store/CLI
  suites.

Remaining limits are explicit: this is not a kernel atomic snapshot; the
live worktree can change and return to the same observed identity; Unix
read-only modes are not a hostile same-UID sandbox; source observation still
has blocking-OS-call and non-UTF-8 Git-path gaps; analyzer execution
attestation, semantic CodeGraph/Joern verification and accepted fact/outbox
publication remain open.

# W1 stable snapshot boundary — source-study receipt — 2026-09-12

## Scope

- Work item: reduce the point-in-time race window in the Git snapshot authority
  before it can be used as evidence scope.
- Product repo: `/home/minh/projects/project-graph-agent`.
- Source gate: ready before implementation; no product code was changed before
  this receipt was started.

## Source and tests read

| Reference | SHA-256 | Lesson carried forward |
|---|---|---|
| `codegraph/src/sync/worktree.ts` | `4c4878f2960fecd5934c01810092dcb9457fce84a677f3c8b429a630bedf2d5d` | resolve the per-worktree root and common Git directory at the host boundary; best-effort mismatch detection must not silently select another worktree |
| `codegraph/__tests__/sync.test.ts` | `8e1c87992878a15fbba60eaf6d1ce0d28fc2b4e9dfae3e543cb74f8f0ccd2b74` | added, modified, untracked and deleted paths are separately exercised; clean status is not sufficient to establish content freshness |
| `codegraph/__tests__/sync-rebuild-convergence.test.ts` | `d78f5e309c39438f062f87babf00c32aa090cc1b8a342a35aaf09366d27559a3` | incremental output must converge to a fresh rebuild; counts alone can hide stale edges, so identity/freshness checks need a whole-result oracle |
| `ripwire/src/ingest_cache.h` | `d6f81ac639b407704169d7cdb19c1cc1ac7b48488444da9caa7593ac0ad807be` | cache identity is versioned, content/hash/config-aware and written through a unique temporary file plus atomic rename; a failed write preserves the previous valid artifact |
| `ripwire/test/headsnapcachecheck.sh` | `6f554e71447ac1cf8ae9d830db5ff75e6daa84bad31730ac1b1b0637c144fba9` | immutable `HEAD` snapshots can be reused only with key separation, byte-identical cold/warm output and stale-on-HEAD-change tests |
| `ripwire/test/cacheidentitycheck.sh` | `7405a5d97f583b86ee3c3d85a7df7c6a726dfdc75e96025b3567916b27e846` | every refusal branch needs a disclosed reason; a single `usable=false` bit is not an adequate machine-facing contract |

## Decisions before patching

- Add a bounded stability check to `GitSnapshotAuthority`: observe the complete
  Git-visible set twice and accept only identical observations. A changed second
  observation fails closed rather than returning a mixed fingerprint.
- Keep the existing versioned file/path/mode/symlink/HEAD/status digest and the
  existing per-command timeout/output/source budgets. A second observation is a
  consistency check, not a doubled byte budget and not permission to remove the
  source reader's later hash verification.
- Expose a static `SnapshotChanged` error without source paths or Git diagnostics.
  Do not retry indefinitely: bounded attempts preserve CLI liveness and make
  mutation visible to the caller.
- Do not call this atomic. A stable pair narrows the race window but cannot freeze
  a mutable worktree between authority binding and later source reads. A future
  immutable Git worktree/export strategy must own that stronger guarantee.
- Preserve the source/application boundary: stability remains in `graph-source`,
  `SourceSnapshotAuthority` remains Git-independent, and no cache hit bypasses
  authority or evidence revalidation.

## Planned tests

- stable clean/dirty snapshots continue to bind deterministically;
- an observation that changes between bounded passes is rejected without a
  success binding (implemented as a deterministic seam/unit test if the adapter
  is refactored to inject observations);
- existing mutation, deletion, symlink, mode, linked-worktree, budget and
  malformed-output tests remain green;
- CLI output remains empty on authority failure and never exposes config/Git
  diagnostics.

## Deliberate gap

This work does not prove an atomic filesystem snapshot, analyzer execution
attestation, semantic CodeGraph/Joern facts, accepted graph publication, or
cross-platform parity. Those remain separate W1-I/W4/W10 gates.

## Implementation and verification

`GitSnapshotAuthority::current_project` and `bind_source_snapshot` now perform
two complete bounded observations and return `SnapshotChanged` when the
second observation differs. The comparison includes the canonical root/common
directory, `HEAD`, porcelain status and the versioned tracked/deleted/
non-ignored-untracked file identity (content, mode and symlink target). The
authority still retains its existing command, output and total-source budgets;
it does not retry indefinitely or emit partial identity.

The deterministic `stable_observation` unit tests pass **2/2**, and the full
`graph-source` package passes **19/19** tests. Existing CLI wiring remains
opt-in and its subprocess test is covered by the separate CLI receipt. The
stable pair reduces the chance of binding a mixed scan, but a worktree can
still mutate after the second observation and before a later source read; an
immutable export/worktree is required before claiming atomicity.

The focused rustfmt check for the touched Rust files passes. A strict scoped
Clippy invocation remains red on pre-existing `graph-source`/CLI diagnostics
(missing error docs, stack-array policy, and existing structural lints); the
new stability helper's needless ownership warning was removed. This change does
not claim the workspace lint gate is complete.

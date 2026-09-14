# W1 source identity verifier — source-study receipt — 2026-09-12

## Scope

- Work item: W1-I source identity precondition.
- Product repo: `/home/minh/projects/project-graph-agent`.
- Reference revision: CodeGraph `3ed73bc127323e63153bf6ec8354afa82ce36aaf`,
  Ripwire `48222d62f41c6e15f60855127c1d9ee06b3aed4c`.
- Source gate: ready before implementation; no source bytes or graph facts are
  accepted by this receipt.

## Source and tests read

| Reference | SHA-256 | Relevant behavior |
|---|---|---|
| `codegraph/src/sync/index.ts` | `afeb472b1b20e1ec78fce425440b23b71b00e27e6b56d9f95667045e86c6f506` | sync is a bounded projection over a selected project and must refresh after file changes |
| `codegraph/src/sync/worktree.ts` | `4c4878f2960fecd5934c01810092dcb9457fce84a677f3c8b429a630bedf2d5d` | worktree identity is resolved at the host boundary, not inferred from an agent path |
| `codegraph/src/sync/watcher.ts` | `7e46b6bded6b380a5c33c4486062933068771479af1ff6454185ae3e942bb225` | watcher changes trigger re-indexing; freshness is a lifecycle concern |
| `codegraph/__tests__/sync.test.ts` | `8e1c87992878a15fbba60eaf6d1ce0d28fc2b4e9dfae3e543cb74f8f0ccd2b74` | added/modified/deleted files and recovered index states are tested explicitly |
| `codegraph/__tests__/sync-rebuild-convergence.test.ts` | `d78f5e309c39438f062f87babf00c32aa090cc1b8a342a35aaf09366d27559a3` | an unchanged caller can become stale when another definition changes; full rebuild is the oracle |
| `ripwire/src/ingest_cache.h` | `d6f81ac639b407704169d7cdb19c1cc1ac7b48488444da9caa7593ac0ad807be` | cache identity includes content/parser/format details and stale/corrupt cache must self-heal, never silently pass |
| `ripwire/src/handoff.h` | `cc216f5272b883f372043d053efb94f8afe48b3d5dda4874a9f209502c6d04f7` | HEAD/dirty state and verified-vs-heuristic output are disclosed separately |
| `ripwire/test/cacheidentitycheck.sh` | `7405a5d97f583b86ee3c3d85a7df7c6a726dfefdc75e96025b3567916b27e846` | changing cache identity inputs must reject the old cache |
| `ripwire/test/baselinedirtycheck.sh` | `aca90c7e03ee5c79bf009a69cafe9b1c673a1c63194c978d397dd6ebfda25354` | clean and dirty baseline states are distinct and explicitly checked |

## Decisions before patching

1. Add an application `SourceSnapshotAuthority` port. A source reader and a
   caller-provided `ProjectRef` are not authority; they only provide access and
   a claim respectively.
2. Require the authority to return a typed binding for the exact root,
   `ProjectRef`, and graph version. The binding is an opaque capability for the
   current verification call, not a durable graph fact.
3. Query `AnalysisRepository::source_analysis_run` only after the authority
   succeeds. The run must exactly match evidence project, graph version and run
   ID; missing or mismatched registration fails closed.
4. Read and hash source bytes only after both identity gates pass. A matching
   file hash cannot prove the whole worktree, analyzer execution, or semantic
   relationship, so `relationship_verified` stays false.
5. Keep the existing `verify_source` API for low-level hash/line checks and
   legacy adapters. Add `verify_source_identity` as the stricter orchestration
   seam rather than silently changing all callers.

## Implementation/test plan

- Add a source snapshot binding value to the application port and a typed error
  for authority/repository/identity failures.
- Add fake-authority tests for valid binding, authority failure, missing run,
  snapshot mismatch and “reader not called on authority failure”.
- Update the CLI only when a concrete host authority is available; until then,
  do not relabel caller-supplied roots as verified.
- Keep semantic relationship verification in a separate CodeGraph/Joern
  adapter; source bytes and source identity are necessary but insufficient.

## Implementation and verification

- Added `SourceSnapshotAuthority`, `SourceSnapshotBinding`,
  `SourceIdentityVerificationError`, `VerifiedSourceIdentity` and
  `verify_source_identity` to `graph-application`.
- Added focused tests for valid authority/run, authority failure, binding
  mismatch, missing run, mismatched run and invalid binding metadata. Tests
  prove the source reader is not called before identity gates pass.

```text
cargo test -p graph-application --locked --offline   # 21 passed
cargo test -p graph-source --locked --offline        # 12 passed
bash scripts/validate-foundation.sh                  # passed; workspace tests green
```

`cargo fmt --all -- --check` could not run because this environment has no
`cargo-fmt`/`rustfmt` component. No CLI output was relabeled and no semantic
relationship or accepted fact was created.

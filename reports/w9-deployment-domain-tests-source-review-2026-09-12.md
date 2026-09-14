# W9 deployment domain invariant tests — source review

Date: 2026-09-12
Scope owner: `crates/domain/tests/deployment.rs` and this report only.

## Pre-patch source receipt

The current CodeGraph reference repository is `/home/minh/projects/outsource/codegraph`.
Its revision at review time was:

```text
3ed73bc127323e63153bf6ec8354afa82ce36aaf
```

The reviewed source files were read from the worktree before this task's test
patch. Their SHA-256 fingerprints were:

| File | Worktree SHA-256 | HEAD SHA-256 / state |
| --- | --- | --- |
| `src/db/queries.ts` | `e49e1f4b76afa3b832611668b15db67af45b5512287b67ea8c74412d503048db` | `a6d73a0a9209ddfba4850db37df6ccf51688a6acd24f86e6164068a77b2d5a8d` |
| `__tests__/parameter-reconciliation.test.ts` | `1e54ec9356db14271cfa9a06ae75615c1939e465f7cc4ddaec895f96b99a003f` | untracked at reviewed revision; no HEAD blob |

The CodeGraph worktree already contained unrelated changes. The query file's
worktree diff includes the reviewed replacement/snapshot implementation and
other changes; the worktree fingerprint above is intentionally recorded so
the study is reproducible without claiming a clean checkout.

## Reviewed symbols and behavior

- `QueryBuilder.replaceSynthesizedEdges` in `src/db/queries.ts:1909-1951`
  validates a non-empty producer and explicit source scope, serializes the
  candidate snapshot before mutation, validates ownership/coordinates, checks
  all endpoints before deletion, retracts only the owned producer edges,
  preserves foreign/static collisions, deduplicates by stable edge identity,
  and returns removal/insertion/conflict/duplicate counts from a transaction.
- `QueryBuilder.replaceSynthesizedSnapshot` in `src/db/queries.ts:1958-1971`
  discovers previous owned sources, includes zero-candidate sources for stale
  edge retraction, delegates replacement, and persists the receipt in the same
  transaction.
- `__tests__/parameter-reconciliation.test.ts:24-218` exercises real indexed
  lifecycle behavior: incomplete-but-usable results, rebinding and retraction,
  preservation of foreign/static edges, conflict accounting, malformed-source
  and extraction-error fail-closed behavior, abort recovery, receipt-write
  rollback, configuration freshness, and symlink retarget detection.

## Adopt

- Make each invariant independently observable with focused mutation cases.
- Use a valid baseline fixture and mutate exactly one scope, identity, edge
  shape, citation, line, or budget property per negative case.
- Assert rejection through the typed domain `Result`, and assert public getters
  on the valid aggregate so the test proves immutable construction rather than
  only error behavior.
- Exercise empty replacement as a valid explicit state, analogous to a
  zero-candidate producer snapshot.
- Keep rollback/recovery and conflict-preservation ideas as conceptual guidance
  only where this pure domain constructor has no persistence or transaction.

## Avoid / adaptations

- Do not copy CodeGraph's filesystem, SQLite, resolver, symlink, runtime,
  authentication, or network behavior into the domain crate.
- Do not test private-constructor circumvention or weaken assertions to match a
  failing implementation.
- Do not treat shape validation as source verification, runtime relationship
  verification, authorization, or secret handling.
- Adapt edge identity to the requested deployment contract:
  `(source, target, kind, mount_target, evidence.id())`, including the root
  citation in citation-identity checks.
- Adapt scope checking to full-source evidence: line one, exact project/path/
  hash/run/graph-version scope, citation ranges within the root, and unknown
  lines within the root citation.
- Use the existing `SourceEvidence::new` constructor for every fixture and
  mutation; do not construct evidence through private fields.

## Test plan recorded before patch

`deployment.rs` will cover valid Service/Volume/Network nodes and Mounts,
DependsOn, and AttachedTo edges; valid non-interpolated absolute mount targets;
unknowns; public getters; empty graphs; all bounded text and collection budgets;
duplicate node IDs; missing/mistyped endpoints; invalid mount-target rules;
duplicate edges; every root/citation scope field and range mutation; conflicting
same-ID citations; out-of-range unknown lines; and static error output that
does not echo a secret. Tests will remain offline and dependency-free.

## Validation after patch

- `scripts/with-local-tools rustfmt --edition 2024 --check crates/domain/tests/deployment.rs` — passed.
- `scripts/with-local-tools cargo test -p graph-domain --locked --offline` — passed: 14 deployment integration tests, 15 existing domain unit tests, and doc tests.
- No production files or dependencies were modified; validation used no network, authentication, or delegation.

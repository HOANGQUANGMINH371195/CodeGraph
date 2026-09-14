# W9 deployment persistence test source review

Date: 2026-09-12
Source gate: ready for test implementation
Owned files: `crates/store/tests/deployment.rs`, this report

## Source identity

- Reference repository: `/home/minh/projects/outsource/codegraph`
- HEAD: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`
- Reference worktree: dirty; the requested files are modified in that worktree.
- `src/db/queries.ts` SHA-256: `e49e1f4b76afa3b832611668b15db67af45b5512287b67ea8c74412d503048db`
- `__tests__/parameter-reconciliation.test.ts` SHA-256: `1e54ec9356db14271cfa9a06ae75615c1939e465f7cc4ddaec895f96b99a003f`

## Read source and test flows

- `src/db/queries.ts:1921-1971`, `replaceSynthesizedEdges` and
  `replaceSynthesizedSnapshot`: validate the complete candidate snapshot before
  opening the transaction; discover prior ownership; delete only owned rows;
  preserve foreign rows on identity collision; validate endpoint existence; and
  write the durable marker in the same transaction. The outer transaction is
  deliberately allowed to roll back all candidate changes when marker writing
  fails.
- `__tests__/parameter-reconciliation.test.ts:63-81`: replacement retracts the
  old producer-owned evidence and permits recovery on the next valid sync.
- `:83-108`: foreign/static evidence survives replacement, and a foreign
  identity collision is reported without deleting the foreign evidence.
- `:110-144`: malformed input, stored extraction errors, and aborts fail closed
  by retracting owned evidence; a later valid/no-change run restores it.
- `:146-155`: a failure in the durable receipt write rolls back candidate
  edges and leaves a failed marker, proving atomicity across the replacement and
  receipt.

## Adopt

- Use independent real-SQLite integration tests with temporary database paths,
  explicit reopen boundaries, and no mocks.
- Treat a replacement as one atomic snapshot: the persisted graph, citations,
  and generation update must either all commit or all disappear.
- Test ownership and collision behavior through observable reads: foreign
  evidence must survive, while the new root plus its owned children must not
  survive a citation conflict.
- Mirror the recovery shape with an explicit empty graph and a separate
  invalidation tombstone, then verify that a later valid replacement recovers
  the same scope.
- Exercise strict compare-and-swap through stale writes and two independent
  `Store` connections sharing one file; assert exactly one concurrent writer
  succeeds.

## Avoid and adapt

- The reference code is TypeScript graph-edge reconciliation, not relational
  deployment persistence. It has no generation CAS, tombstone representation,
  reopen round-trip, or adapter/project/path scope matrix. Those are supplied by
  the promised Rust application API and must be tested directly here.
- Do not copy the reference's producer-specific metadata or edge identity. The
  deployment domain owns all node kinds (`service`, `volume`, `network`), edge
  kinds (`mounts`, `depends_on`, `attached_to`), unknowns, and source citations.
- Do not treat `adapter_version` as a replacement owner: the API contract says
  changing it must succeed under the same scope and expected generation.
- Do not add dependencies or SQL fixtures. `tempfile` is already an allowed
  store dev-dependency; the tests use the existing migrations and public
  repository API only. No migration, production module, plan, or unrelated
  documentation is in scope.

## Planned test coverage

1. Full graph round-trip across close/reopen, including all node/edge kinds and
   unknowns; adapter-version-only replacement; and scope getter assertions.
2. Empty valid graph versus invalidation tombstone, followed by replacement
   recovery; absent lookup and generation-zero behavior.
3. Stale replace/invalidate CAS failures with unchanged snapshots.
4. Isolation across adapter, every `ProjectRef` field, graph version, and path.
5. Citation-ID conflict after a new root is inserted, proving transaction
   rollback leaves no partial header/nodes/edges/unknowns or generation bump.
6. Two independent file-backed `Store` connections racing generation zero,
   with one success and one `StoreError::Conflict`, followed by generation-one
   verification after reopen.

## Validation receipt

- `scripts/with-local-tools rustfmt --edition 2024 crates/store/tests/deployment.rs` — exit 0.
- `scripts/with-local-tools cargo test -p graph-store --test deployment` — exit 0;
  7 passed, 0 failed.
- `scripts/with-local-tools cargo test -p graph-store --tests` — exit 0; 55 store
  unit tests passed, this file's 7 tests passed, the parent-owned 3 deployment
  corruption tests passed, and all existing store integration suites passed
  (17 RPC launch, 12 RPC spawn, 33 submission-content; only their existing
  ignored tests remained ignored).
- Foundation-wide validation was not run because the parent explicitly owns
  the concurrent migration/latest-count edits and requested no workspace-wide
  formatting during that work.

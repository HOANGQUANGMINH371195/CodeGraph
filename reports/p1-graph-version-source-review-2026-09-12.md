# P1.T06 graph version and snapshot cache — 2026-09-12

status: done; owner/reviewer: main; source-gate: ready → complete.

## Source study before code

- `outsource/ripwire` at `48222d62f41c6e15f60855127c1d9ee06b3aed4c` (clean).
  Read `src/ingest_cache.h` and `src/ingest.cpp`, plus
  `test/cachehashcheck.sh`, `test/cacheoffsetcheck.sh` and
  `test/headsnapcachecheck.sh`. Its cache binds a portable root-relative path,
  content hash, stat gates, parser/extraction version and artifact architecture;
  a bad or incompatible frame is rejected and reparsed. Offset-table loading
  and carry-forward make subset runs bounded without treating cache bytes as a
  graph authority.
- `outsource/codegraph` at `3ed73bc127323e63153bf6ec8354afa82ce36aaf` with
  dirty source fingerprint `75d5c57f6d10b630c62dcc020b9ef1aa21585c1ef0826ee102e43ff20bc2d0a8`.
  Read `src/index.ts`, `src/extraction/index.ts`,
  `src/extraction/extraction-version.ts`, `src/db/schema.sql`,
  `src/db/migrations.ts` and the metadata/revision queries. The live index
  records per-file content hash/index time, extraction version and an advisory
  index state; stat/hash reconciliation and explicit sync are required, and
  `lastIndexedAt` is not a complete graph snapshot identity.

## Decision and implementation

- Adopt immutable `GraphVersion` with generation, `ProjectRef`, source-manifest
  SHA-256, extractor/resolver/analyzer versions and a stable opaque ID. Keep
  repository/worktree identity separate from mutable head/fingerprint inputs.
- Add `GraphSnapshotCache` with observation/expiry timestamps and deterministic
  `Fresh | Stale(reason) | Expired` classification. A future clock reading is
  fail-closed as expired; expiry wins over matching inputs.
- Add forward-only `GraphDelta` between generations. It requires one
  repository/worktree, sorted unique IDs, disjoint add/remove/change classes and
  bounded lists. It carries evidence invalidations but does not accept facts or
  authorize GraphWriter.
- Domain invariants live in `crates/domain/src/graph_version.rs`; strict
  versioned DTOs and nested-schema checks live in
  `crates/protocol/src/graph_version.rs`. This leaves filesystem atomicity,
  stable node/edge ID derivation (P1.T01) and persistence integration for later
  gates.

## Validation

- `scripts/with-local-tools rustfmt crates/domain/src/graph_version.rs crates/protocol/src/graph_version.rs` — pass.
- `git diff --check` — pass.
- `scripts/with-local-tools cargo test -p graph-domain -p graph-protocol --locked --offline` — pass: 19 domain tests, 65 protocol tests, existing deployment/file-read tests included.
- Strict `cargo clippy -p graph-domain -p graph-protocol --all-targets -- -D warnings` remains blocked by pre-existing workspace/domain warnings (missing docs/`must_use`, test unwraps and related findings across older files); no new compile error from this task was observed.
- `sh scripts/validate-foundation.sh` — pass: architecture/preflight checks,
  Orders fixture, and the full current workspace test suite. P1.T06 does not
  close P1 or P1.T10.

## Adopt / avoid

Adopt source/version identity and reject-and-reparse semantics from Ripwire,
per-file hash and extraction metadata from CodeGraph, and the product's
existing snapshot/evidence binding. Avoid using mtime or `lastIndexedAt` alone,
forwarding cache blobs, claiming an atomic filesystem snapshot, or allowing a
stale delta to mutate graph state.

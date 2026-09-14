# W2 reopened RPC output verification

status: done (reverification API task); source-gate: ready; owner/reviewer: main agent.

- [x] Read Orca artifact-create-intent-store.ts:79–201 (scope identity, bounded
  read, strict recovered body) and tests:277–337 (persisted format and size limits).
  Clean revision 26f9fd8ea152ad6126c005e5ae602c7d201f4a99. Adopt identity and budget
  checks; do not treat a filename digest as content verification or copy cloud logic.
- [x] Read product receipt_output.rs, artifact.rs streaming verifier, RPC snapshot
  and stored receipt linkage, and real fixture publication tests. Reuse SHA/length
  verifier, not a second hashing implementation. Upstream tests not run.
- [x] Plan before code: host-selected exact launch; query coherent snapshot with
  terminal receipt; total budget before opening either blob; verify optional streams;
  requery exact snapshot after reads. Private result holds receipt and fresh content
  observations only. Fixed error text/Debug, no raw bytes or new authority.
  Tests: actual fixture publish/reopen, valid bytes, bad/missing bytes, total budget,
  wrong launch, and repository change during read. No DB/CLI writes or migration.

Skills rust-router/m12-lifecycle: verification is point-in-time, not continued
filesystem immutability; reader deadlines remain host-owned.

## Implementation and validation

- `verify_rpc_outputs` requires exact host-selected launch, coherent stored
  terminal snapshot, checked total length before blob opens, streaming verification
  of each present stream, then an identical snapshot on requery. Query port now
  explicitly requires stored spawn/run/artifact linkage (already enforced by Store).
- Private observation has immutable accessors, redacted Debug and no blob bytes;
  error Display/Debug do not print underlying adapter errors. Typed sources remain
  accessible. No durable observation write or execution/retry permission is created.
- New actual RPC fixture test publishes to SQLite/CAS, reopens and checks success,
  missing receipt, wrong host expectation, budget rejection before read, corrupt
  and unavailable bytes, fixed error text, changed snapshot, unchanged events and
  one process launch. Existing malformed/truncated test now re-verifies content
  without upgrading completion. No independent reviewer this task.
- Targeted test: exit 0, one passed. Full
  `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0,
  282 Rust + 12 Node passed, no failures; architecture 7 packages/41 dependencies,
  no errors. `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- No migration/dependency added. Untracked product file fingerprints, SHA-256:
  - `crates/application/src/rpc_output.rs`: `117abdc42397759e4150b815dc6408041d80efc6b72c4a94f04f8add7212eca6`
  - `crates/execution/tests/rpc_fixture/publication.rs`: `f5077d317e7d8f156e490ed1e58f39542c4c7e4c96bd29fa946923ef8d53114a`

## Remaining scope

This verifies surviving bytes, not reconstructing missing content or recovering
an unpublished prepared descriptor after host death. CLI integration, dedicated
absent-output/repository-error tests and production containment remain open.
No post-return filesystem guarantee, RPC success assertion or full W2 completion.

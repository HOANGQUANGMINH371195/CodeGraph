# W2 retained output publication

Source gate ready before code: OpenDev clean revision
`d32c660e4eed1a8e988d1fd58da88e41ba641d08`, foreground.rs:175–235,
truncation.rs:158–205, truncation_tests.rs:70–83 read. Adopt separate retained
artifact and presentation responsibilities; avoid merged text, lossy decoding,
overflow head/tail as raw evidence, and path-only success. Upstream test checks
saved text equality, not binary two-stream publication or partial failure.
No source copied or upstream tests run.

Product application/artifact.rs ingestion and verification ports and domain
Artifact/ExecutionReceipt contracts read. Reuse metadata-first ingestion plus
independent readback, not new storage. Before patch: add output-publication bridge
borrowing FixtureRun (never consume a potentially unreaped child), preflight both
descriptors' hashes, lengths, kind, shared scope and unreviewed protection before
any write. Bound total retained bytes with caller budget. Report stdout success
if stderr fails so retry same descriptors without rerunning process. This is
content publication, not process attestation, command binding or receipt authority.
Skills Rust/CLI and m06-error-handling guide explicit partial-success errors.

Tests planned: in-memory port fixtures for exact binary/empty content, preflight
before I/O, stderr partial failure and idempotent retry. Filesystem publication
already has separate adapter tests; this turn does not prove end-to-end ledger
launch/receipt wiring or clean-release provenance (product has no HEAD).

## Validation

Implemented `publish_fixture_outputs` through existing application ports, no
dependency/migration addition. It borrows run and returns two independently
verified ArtifactIngestion observations; stderr failure retains stdout observation.
Both descriptors are checked before the first write. Raw outputs must be marked
Unreviewed; this bridge cannot claim encryption or redaction.

- Three in-memory port integration tests pass: binary/empty output and replay,
  stderr mismatch/total cap rejection without writes, partial stderr failure and
  recovery using the same run and descriptors. These fixtures do not execute a
  process or prove actual SQLite/filesystem adapter composition.
- `scripts/with-local-tools cargo test -p graph-execution --locked --offline`:
  3 lifecycle + 9 process + 3 publication tests pass, exit 0.
- `sh scripts/validate-foundation.sh`: 166 Rust + 11 Node pass, exit 0;
  existing standalone crash helper remains ignored but parent invokes it twice.
  Architecture gate remains 7 packages/36 direct declarations, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.

Next: compose owned fixture, actual artifact adapters and execution receipt
ledger with host-selected binding; preserve incomplete cleanup and do not grant
integration authority. Launch admission/reconciliation and process-tree cleanup
remain separate unfinished requirements. W2 and full packages remain open.

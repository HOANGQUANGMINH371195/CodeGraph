# W1/W2 snapshot execution source study

## Resume source gate (2026-09-12, before corrective patch)

Foundation compilation reproduced E0624: the integration test calls private
`SourceSnapshotBinding::matches`. Re-read application `source.rs` accessors,
execution `snapshot_execution.rs`, owned fixture source-read branch and domain
`ChildCompletion`. OpenDev revision `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
`bash/foreground.rs` SHA-256
`43d82096fa8b3acae52b8daa843559dcb1838246e9dd702d54037dddecad44a7`:
explicit cwd precedes spawn; timeout/cancel kill and wait; spawn errors fail.
Read `bash/helpers_tests.rs` assertions as supporting helper coverage; upstream
does not test this product's snapshot binding. MIT license inspected; no source
copy planned. Adapt local test to public project/graph accessors, require zero
exit status and both EOFs. Avoid widening private API for a test. Verify with
foundation gate and formatting; this still does not attest an analyzer.

Read OpenDev `crates/opendev-tools-impl/src/bash/foreground.rs` command
construction, cwd, pipe ownership and timeout; local execution fixture/context
tests, snapshot authority tests and materializer lifecycle. OpenDev selects cwd
before spawn but does not bind it to this product's ProjectRef. Adapt explicit
cwd selection to the materialized owner root and hold that owner through reap.
Use the existing owned fixture executor with a bounded source-read mode to
prove actual child bytes come from the materialized snapshot after live source
mutation/deletion. Check stdout, exit, EOF, binding and cleanup. No shell or
ambient executable lookup is needed. This integration proof is a prerequisite
for analyzer execution attestation; it does not provide semantic extraction,
authenticated host identity or full containment.

## Verification

The initial foundation run failed compilation with E0624 (private `matches`).
After the test correction, `scripts/with-local-tools sh
scripts/validate-foundation.sh` exited 0, including
`reaped_child_reads_materialized_bytes_after_live_source_deletion`.
`scripts/with-local-tools cargo fmt --all -- --check` exited 0.
The foundation run required sandbox escalation because cargo metadata spawning
was denied with EPERM. Assertions now cover exact public project/graph binding,
original bytes after live mutation/deletion, zero exit status, both stream EOFs,
no unreaped child, and snapshot directory removal after owner drop.

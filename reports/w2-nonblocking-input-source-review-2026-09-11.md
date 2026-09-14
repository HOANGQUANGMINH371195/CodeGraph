# W2 nonblocking input writer

Source gate ready before implementation. Main read Codex clean revision
818f1cca8ccf8899f0f4d59336baebaccf358eed app-server-transport/src/transport/stdio.rs:
1–175 and connection_handling_stdio_tests.rs:52–110: bounded channels/write ack,
blocked input caused by unread stderr. OpenDev stdio.rs:200–287 and stdio_tests.rs
read for write-under-state-lock and close/error flow at the previously recorded
clean revision. Upstream tests not executed. No source copied.

Adopt explicit partial-write progress and bounded admission; avoid blocking write
under shared lifecycle lock, removing pending requests on write failure, and
MCP Content-Length framing for Codex JSONL. This adapter accepts opaque bytes:
encoding/correlation remain with the existing protocol core, not this writer.

Before code: Linux ChildStdin owner with NONBLOCK, one bounded pending frame
(max 1 MiB), one bounded write per pump, sticky I/O failure and explicit close
returning total/written counters. Write completion means accepted by OS pipe only,
not peer receipt/execution. No automatic retry or queue growth. Keep existing
closed-stdin CheckCommand runner unchanged. Tests own a finite child that never
reads stdin, prove partial writes do not block and close returns retained counters;
normal binary input/EOF and invalid/busy admission also covered. Domain/CLI and
concurrency/lifecycle skills guide control independent of data backpressure.

## Validation

Linux `NonblockingInput` implemented with existing rustix fs API and no new
dependency. Three actual-process tests pass: finite child never reads input and
close retains a partial 1 MiB frame within 500ms; exact binary frames reach child
and close supplies EOF; exited peer causes sticky BrokenPipe and rejects retry.
Test-owned Child guard kills/reaps on failure; no background pump or global state.

- `scripts/with-local-tools cargo test -p graph-execution --test input --locked --offline`:
  3 tests pass, exit 0.
- `sh scripts/validate-foundation.sh`: 174 Rust + 12 Node pass, exit 0; 2 helper
  tests ignored standalone and explicitly invoked by their parent tests.
  Architecture: 7 packages / 39 declarations, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.

This is a writer primitive, not integrated JSONL transport or RPC reconciliation.
No W2/W3 full acceptance: correlation must retain pending metadata independently,
supervisor still needs to combine input/output/control lifecycles, and cleanup
remains unverified. CheckCommand's closed-stdin contract was not changed.

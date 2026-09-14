# W2 bounded connection I/O

Source gate ready before code. Main reread OpenDev foreground.rs:68–112
independent output readers and Codex connection_handling_stdio_tests.rs:52–110
blocked-stderr test. Revisions remain pinned d32c660e4eed1a8e988d1fd58da88e41ba641d08
and 818f1cca8ccf8899f0f4d59336baebaccf358eed. No upstream test run or code copied.
Product Pipe and ConnectionInput were reread. Adopt separate stream progress,
avoid line vectors and blocking consumer queues; preserve raw byte limits.

Before patch: reuse internal Pipe with bounded per-poll stdout/stderr reads;
ConnectionIo emits at most one event per poll, retains unconsumed raw bytes under
the output cap, drives stdin independently, and closes all pipe handles without
waiting for a consumer. stdout EOF closes protocol but stderr may continue draining.
Return closed connection/pending metadata and raw snapshots on finish, not process
cleanup proof. Validated limits precede consuming connection ownership.
Test stderr without newlines alongside JSONL handshake/reply, budget overflow,
and early stop. Existing closed-stdin fixture runner unchanged. Skills CLI,
concurrency and lifecycle inform separation of control, protocol and process scope.

## Verified implementation

ConnectionIo reuses the internal Pipe implementation, emits at most one event
per poll and tracks a parse offset into bounded retained stdout. It drives stdin
and both output pipes, preserves pending correlation on stop/error, and permits
stderr drainage after stdout EOF. finish returns connection plus raw snapshot;
it never returns a cleanup or child-reaping assertion. No dependency added.

- Three actual-process tests: JSONL handshake/reply progresses with 512 KiB raw
  non-newline stderr retained at exact cap; a 64-byte stderr cap truncates and
  stops; immediate stop needs no consumer and preserves Incomplete, not EOF.
  Owned process guards kill/reap fixture processes after tests/failures.
- `cargo test -p graph-execution --test connection_io --locked --offline` using
  local wrapper: 3 pass, exit 0.
- `sh scripts/validate-foundation.sh`: 180 Rust + 12 Node pass, exit 0; existing
  2 helper tests ignored standalone but parent-invoked. Architecture unchanged,
  7 packages/41 declared dependencies, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.

Remaining: process supervisor integration, timeout/reap/cleanup ownership,
consumer saturation under a real event sink, malformed/oversized/truncated pipe
frames and persistent request reconciliation. No native/account/untrusted job
run. W2 acceptance remains open; no graph or integration authority introduced.

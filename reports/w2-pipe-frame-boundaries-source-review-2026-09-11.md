# W2 actual pipe frame boundaries

Source gate ready before code. Read rust-router/domain-cli/m07-concurrency skills
fully; reread product ConnectionIo and LineDecoder. Read Codex clean revision
818f1cca8ccf8899f0f4d59336baebaccf358eed connection_handling_stdio_tests.rs:52–110
for actual-pipe malformed input/blocked-stream assertions. This upstream test
does not establish our exact output-frame limits. No source copied or test run.

Before patch: owned peer first consumes bounded initialize input, then emits
exact-limit valid reply, over-limit bytes, malformed JSON line or partial frame
followed by actual EOF. Keep raw output budget larger than frame budget when
testing frame overflow, so a different output-cap error cannot mask the result.
Assert typed errors, preserved raw bytes and uncertain pending initialize on
failure; exact-limit response may queue ACK but not claim ACK written. Tests
own/reap all peers. No native account or process supervisor authority added.

## Verified results

Two new tests exercise four owned peer modes through actual pipes. Exact 4096
bytes queue ACK but written count stays zero until writer runs. 4097-byte frame
fails TooLarge (output cap is 8192); malformed JSON fails Decode; actual EOF in
partial frame fails Truncated while raw stdout state is Complete. Failure closes
I/O, retains initialize ID/method as uncertain and retains captured raw bytes.
No decoder/production change was required by these tests.

- `scripts/with-local-tools cargo test -p graph-execution --test connection_io --locked --offline`:
  all 5 connection I/O tests pass, exit 0.
- `sh scripts/validate-foundation.sh`: 182 Rust + 12 Node pass, exit 0; existing
  2 standalone ignored helpers remain invoked by parent tests. Architecture
  remains 7 packages / 41 direct declarations, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.

This completes the Linux owned-fixture frame-boundary check only. Combined child
supervisor/deadlines/reaping, event-sink backpressure, cleanup containment and
durable RPC reconciliation remain open; W2/W3 are not complete.

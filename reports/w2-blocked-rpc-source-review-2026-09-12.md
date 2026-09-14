# W2 blocked RPC input — source-first receipt

status: implemented and verified for owned Linux fixture | owner: main
scope: owned Linux fixture, ConnectionIo/Supervisor progress observation and test
source-gate: ready

## Sources read before patch

- OpenDev clean `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
  `crates/opendev-mcp/src/transport/stdio.rs:200–242`: reserve pending before
  write, hold state mutex during write, remove pending on write error.
- Codex clean `818f1cca8ccf8899f0f4d59336baebaccf358eed`,
  `codex-rs/app-server/tests/suite/v2/connection_handling_stdio_tests.rs:60–110`:
  complete initialization, induce unread-stderr/input backpressure, assert
  write timeout and bounded SIGTERM exit. Upstream test not executed here.
- Product current untracked worktree: execution `input.rs`, `supervisor.rs`,
  `connection_io.rs`, tests/input.rs, tests/supervisor.rs and RPC fixture arm.
  No product HEAD/clean revision claimed.

## Adopt / avoid / gap

Adopt actual handshake before inducing backpressure and bounded termination
assertions. Use a finite owned child that publishes an input-paused marker only
after consuming initialized; enqueue the large request after observing it.
Expose read-only InputProgress through I/O/supervisor, observe positive partial
delivery followed by unchanged progress across repeated polls, then cancel
without an event consumer. Retain ID/method, uncertainty and exact byte counters.
Do not copy upstream protocol/runtime or erase pending on a write interruption.
No automatic retry, peer-ack inference or process-tree cleanup claim.

Expected verification: targeted supervisor tests, complete foundation and format
checks. Assert direct-child reap, bounded cancellation, EOF for both raw streams,
unchanged partial counters after stop, no I/O/process errors and one unresolved
request. This closes only the owned Linux blocked-input case, not W2 integration,
approved spawn, native dispatch, containment or other-platform acceptance.

## Verification

- `cargo test -p graph-execution --test supervisor --locked --offline` through
  scripts/with-local-tools: four tests pass, including blocked RPC cancellation.
- New test alone repeated ten times: ten passes; no upstream services/accounts.
- `sh scripts/validate-foundation.sh`: 186 Rust + 12 Node pass, two standalone
  helpers ignored by normal discovery but invoked by their parent tests.
  Architecture: seven packages, 41 declared dependencies, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- Test observes a positive partial frame after 256 polls, unchanged over 32
  additional polls, and exactly preserved through cancellation/finish. Fixture
  never reads the request after publishing its handshake marker. Cancellation
  test allows 1500 ms including scheduling for the 1000 ms cleanup budget;
  this is not a hard real-time guarantee. Both raw streams reach EOF and the
  direct child is reaped without I/O/process errors. Cleanup stays Unverifiable.
- Main review: no new runtime dependency, writable counter access, pending
  removal or retry path introduced. Scope remains a trusted Linux fixture.

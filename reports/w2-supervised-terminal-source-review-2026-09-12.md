# W2 supervised terminal accounting bridge

status: done (bridge only); source-gate: ready; scope: execution connection input/supervisor.

- [x] Read outsource source/tests for this task before implementation.
- [x] Record adopt/avoid and missing behavior.
- [x] Choose bridge and tests before patch.

OpenDev clean HEAD `d32c660e4eed1a8e988d1fd58da88e41ba641d08`:
read `crates/opendev-mcp/src/transport/stdio.rs::close` and complete
`stdio_tests.rs`. Upstream closes stdin, waits/kills, drains pending; tests
cover disconnected state and Python echo/close, not terminal evidence retention.
Adopt explicit input shutdown before final accounting. Do not adopt dropping
pending metadata or interpreting descendant PID scans as verified containment.
Upstream tests not executed.

Read product `supervisor.rs`, `connection_input.rs`, `connection_io.rs` and all
four supervisor integration tests. Current finish returns input, raw output,
completion, attach-based elapsed time, optional owned unreaped Child and errors.
New bridge will consume that result, seal input/core and keep every other field
unchanged. Input counters describe the last frame (possibly a notification),
not a proven request association or peer acknowledgement. No new process action,
serialization, spawn binding or durable receipt is inferred.

Rust lifecycle skill informs explicit ownership transfer: never silently drop
an unreaped Child while creating a report, never claim Drop reaps it.

Tests planned: extend actual normal RPC, timeout/cancel, malformed output and
blocked partial-input cases to consume final results and assert epoch, pending
identity/uncertainty, counters, output and completion/errors survive conversion.

## Validation

Implemented `ConnectionInput::into_terminal`, `SupervisedConnection::into_terminal`
and exported `TerminalConnection`. Four existing actual-process tests expanded;
normal exit retains full stderr and stdout, completion, elapsed and counters;
timeout/cancel retain unanswered initialize; malformed bytes/error remain;
blocked request retains ID 2, method, uncertainty and partial-write counters.

`sh scripts/validate-foundation.sh` and
`scripts/with-local-tools cargo fmt --all -- --check` exit 0: 255 Rust + 12
Node passed, architecture 7 packages/41 dependency declarations/no errors.
No new tests counted, migration or dependency added. Main reviewed changes;
no separate reviewer. Unreaped Child transfer is explicit in source but these
four tests reap normally; no fault-injected unreaped/process-error bridge test.

Final untracked file SHA-256 (product has no committed baseline):
- `crates/execution/src/supervisor.rs`: `c79f4e25e819b4872c31e81ed070dc90281ecf664247f4c0d6cf7befd796ddcc`
- `crates/execution/src/connection_input.rs`: `6a7cc0266e7e23894e52e7756fa96665fcc03b1bc0769248d4ff2108748390b9`
- `crates/execution/tests/supervisor.rs`: `7ef46f18e5f6297ee0d5e668eb5d93823b1dc8ec95ba95241c7fcb00703aca92`

Next: validated launch-bound terminal receipt, artifact ingestion and persistence,
crash reconciliation. Public bridge fields are raw host results, not trusted
domain facts; conversion does not establish spawn identity or verified cleanup.
Full W2/W1-I remain incomplete and no native live dispatch was enabled.

# W3 bounded stdio framing

status: review | owner: main | task: P9.T11 partial (full transport still open)
source-gate: ready

Source-study before code: Codex HEAD
`818f1cca8ccf8899f0f4d59336baebaccf358eed`, clean worktree.

- `codex-rs/app-server-test-client/src/lib.rs:2300–2345`: write payload plus
  newline then flush; read_line returns one message and zero bytes means closed.
- `codex-rs/app-server-transport/src/transport/stdio.rs:55–138`: stdin lines
  cross a capacity-one channel into forward_incoming_message; EOF emits
  ConnectionClosed before RPC/runtime cleanup necessarily finishes. This is
  source inspection, not proof of a bounded line reader.
- `codex-rs/app-server/tests/suite/v2/connection_handling_stdio_tests.rs:52–110`:
  initialize response parsed as JSONRPC Response; deliberately unread stderr
  blocks invalid-input writes and SIGTERM test expects shutdown timeout.
  Assertions read, not executed; no account or subprocess experiment.

Adopt newline framing, keep EOF separate from application completion. Add a
pure byte decoder in graph-protocol, usable by a later async transport without
blocking I/O or dependency changes. Limit includes LF and optional CR; return
one owned frame and consumed byte count so caller controls queue/backpressure.
No scanning beyond remaining frame allowance. Reject oversized frame before
copying; use fallible allocation and terminal failed state, not skip-and-resync.
Require LF even at EOF: incomplete final bytes are truncation, not success.
This strict EOF rule and 1..16 MiB constructor limit are harness policy, not
claims about upstream limits. CRLF normalization removes only delimiter bytes.

Gaps: the inspected line-read paths do not supply this bounded incremental
decoder. Implement this small protocol utility locally; no upstream code copy.
Search covered Codex app-server*/transport and T3 provider line readers, plus
OpenDev crate line/frame patterns. Process ownership, stderr draining, bounded
queues, timeouts, routing, authentication and live acceptance remain separate.

Acceptance: byte-by-byte Unicode/CRLF, many frames in one chunk, exact cap,
oversize without newline, truncated EOF, repeat calls after failure/finish,
and completed bytes passed into existing lifecycle parser. Bound is decoder
buffer length only, not input allocation, allocator overhead or downstream queue.

## Verification

- Three decoder unit tests and one framing→lifecycle contract test added.
  UTF-8/CRLF tested across every chunk size of the fixture; exact cap, oversize,
  closed-state calls and truncated EOF checked without spawning a process.
- `sh scripts/validate-foundation.sh`: exit 0, 74 Rust + 10 Node tests pass;
  6 crates / 29 declared direct dependencies unchanged.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- framing.rs SHA-256:
  `56ba0306343b6ce760cada3910a5adbc8c7046bdd2aaad9acbecc9e2c8f0d0ec`.
- Main review: each successful nonempty input consumes bytes; buffer never
  retains a full-limit unterminated frame; failure clears state and cannot
  resynchronize accidentally. No allocation-failure injection or independent
  review claimed. Skills rust-router/m05-type-driven informed private state.
- Next: transport ownership/routing, bounded queues and stdout/stderr drainage,
  initialize negotiation and generation-aware native lifecycle reconciliation.

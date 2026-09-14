# W3 connection-scoped RPC correlation

status: review | owner: main | task: P9.T11 partial (adapter remains open)
source-gate: ready

Source before code:
- Codex clean HEAD `818f1cca8ccf8899f0f4d59336baebaccf358eed`:
  `codex-rs/app-server-test-client/src/lib.rs:2070–2120` waits for matching ID,
  separately handles server requests, buffers notifications; unmatched replies
  are dropped in the test-client implementation. Adopt identity matching but
  expose unmatched replies rather than silently discarding them.
- `app-server/tests/suite/v2/connection_handling_stdio_tests.rs:72–90` sends
  initialize and asserts response family under read timeout; not executed here.
- T3Code clean HEAD `3836890e4484406813997259efc69065b25ce698`:
  `apps/server/src/textGeneration/CodexTextGeneration.ts:245–285` bounds a CLI
  text-generation operation with timeout error. This is a separate CLI path,
  not native request cancellation evidence or a pending-map implementation.

Design before code: connection-owned, non-Clone pending table in protocol.
Monotonic integer IDs allocated before dispatch, never reused within table.
Host supplies unique connection epoch; receive requires matching epoch before
touching table. This is routing consistency, NOT event authentication. Host
must not label stale frames as current or reuse epochs after reconnect.
Bound pending count to 1..4096. Keep method and uncertain flag only, not request
payload. Timeout/lost write acknowledgement marks uncertain, retains slot;
late reply still correlates. close stops new requests, retains uncertain pending
records for reconciliation and accepts late buffered replies from same epoch.
Unknown/duplicate replies returned explicitly, server request/notification
passed through without consuming local pending entries even when IDs collide.

No automatic retries, approval, dispatch, deadlines, task acceptance or durable
recovery. Caller must validate method-specific reply after correlation and
retain errors/artifacts as appropriate. Table removal only completes RPC
correlation, not domain operation success. Tests cover out-of-order replies,
typed-ID collision, late/duplicate/unknown replies, stale epoch, cap and close.

## Verification

- Four unit tests pass for out-of-order success/error replies, duplicate reply,
  server requests and string/integer ID collision, stale epoch, uncertain/close,
  late replies, invalid limits/methods and i64 exhaustion without wrapping.
  Read-only unresolved iterator tested after close for future host checkpointing.
- `sh scripts/validate-foundation.sh`: exit 0, 82 Rust + 11 Node tests pass;
  6 crates / 30 declared direct dependencies unchanged.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- correlation.rs SHA-256:
  `c95bc3dd4d1d1bbb9043697fdc9ab93efd2d59cd74c3fce4f98971431d52154d`.
- Main review: no task state changes, retries or approval replies. Error response
  is matched but remains Message::Error. Method-specific payload validity still
  belongs to caller; request removal means RPC response observed, not successful
  operation. Transport must supply/authenticate unique epochs; table cannot prove
  that uniqueness or persist it. Skills rust-router/m05-type-driven informed
  private non-Clone state and explicit routing variants.
- Next: serialize outbound requests and handshake; owned process transport,
  pending metadata persistence/recovery and authenticated epoch binding. No
  independent review or live native test claimed.

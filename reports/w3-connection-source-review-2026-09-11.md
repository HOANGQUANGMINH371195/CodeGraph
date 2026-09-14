# W3 composed connection core

status: review | owner: main | task: P9.T11 partial (runtime adapter remains open)
source-gate: ready

Source re-read before code, Codex clean HEAD
`818f1cca8ccf8899f0f4d59336baebaccf358eed`:
- app-server-test-client/src/lib.rs:1737–1779: initialize → typed reply →
  initialized write before thread operations; :2070–2102 response-ID matching
  while handling notifications/server requests separately.
- app-server/tests/suite/v2/connection_handling_websocket.rs:67–108: per-client
  handshake, reject pre-init requests, same ID on different connections isolated.
  Tests read only; no upstream/native process execution.
- Read current local framing/correlation/handshake public interfaces and source
  receipts. These components exist but do not yet enforce one shared send path.

Design before code: one non-Clone sans-I/O connection owns decoder, pending map,
handshake and host epoch. begin emits unsent initialize bytes, receive yields
at most one event/consumed count; valid initialize response produces unsent ack.
Only explicit host ack-write confirmation enables prepare_request. Reject repeat
initialize through ordinary request API. Pass server requests and notifications
to host without auto-approval; unmatched responses remain explicit.

Wrong epoch rejected before decoder mutation. Framing/decode/handshake failures
close logical connection and retain pending uncertain metadata; EOF is connection
closure, not task/process terminal. A request encode failure after reservation
also conservatively closes the core, emits no bytes, and retains reservation;
this is not no-dispatch evidence for other pending requests. Host must reconcile
which request failed pre-write. No replay/resume is authorized by this core.

Acceptance: full in-memory handshake to request/reply, pre-ack send denied,
typed IDs/caps, interleaving server request/notification, old epoch mid-frame,
malformed/oversize/EOF closure and encode failure retains unresolved entries.
No process lifecycle, actual I/O, auth, bounded host queue, durable recovery or
effective capability acceptance. Those remain W2/W3 work, not waived.

## Verification

- Three composed-flow unit tests pass: bytewise initialization, encode ack,
  pre-ack request rejection, repeated initialize denied, later request/reply;
  stale epoch during partial frame, server-request collision; malformed/oversize
  input, truncated EOF and outbound encode failure retain unresolved metadata.
- `sh scripts/validate-foundation.sh`: exit 0, 90 Rust + 11 Node tests pass;
  architecture remains 6 crates / 30 direct dependencies.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- connection.rs SHA-256:
  `4fb826bd5501ee64480f35b51ba57badfacd3edc4cab8f6da1e1ee2bebbb1e7a`.
- Main review: components private, no raw send method on Connection; public
  lower-level modules still exist for testing/reuse, so transport must exclusively
  use this core. Epoch provenance, ack-write confirmation, permission enforcement
  and bounded host queues remain caller responsibilities, not authenticated by
  these tests. Skills rust-router/m05-type-driven informed private composition.
- Next: owned process transport, stderr drainage and bounded write/read event
  loop; server-request response policy and runtime capability acceptance.
  No independent review or live Codex execution claimed.

# W2 terminal RPC accounting

## Source study before patch

status: done (accounting only); scope: protocol correlation/connection terminal accounting.
source-gate: ready

- [x] Read current outsource implementation and tests.
- [x] Record flow, adopt/avoid and gap.
- [x] Choose implementation and tests before code.

OpenDev clean revision `d32c660e4eed1a8e988d1fd58da88e41ba641d08`:
`crates/opendev-mcp/src/transport/stdio.rs`, `send_request` reserves a
pending sender before write, removes it on write failure; `close` drops stdin,
waits/kills and drains pending senders. `stdio_tests.rs` tests disconnected
state and a Python echo roundtrip/close, not retained terminal uncertainty.
Read these source bodies and both test assertions; did not run upstream tests.

Product current untracked/unborn worktree: read `protocol/src/correlation.rs`
reserve/close/receive and its four tests, plus `connection.rs` sequencing.
Existing close preserves pending but the table can still accept late replies.

Adopt explicit shutdown accounting; avoid deleting uncertain metadata or
inferring cleanup from upstream PID scanning. Add an immutable, consuming
terminal snapshot, bound to epoch and sorted original integer IDs, with every
unresolved entry uncertain. Consume the connection to prevent later mutation.
Snapshot carries no request/reply payload and Debug redacts method/epoch.
It is accounting only, not proof of OS exit, RPC success, retry authority or
durable storage. Output/completion binding and durable receipt remain next.

Planned tests: sealing active and closed tables, late matched reply before seal,
empty table, full capacity/ID boundary, redacted Debug, and consuming a fresh
connection retains its unanswered initialize request.

## Validation

Added consuming `PendingRequests::into_terminal` and `Connection::into_terminal`.
Private snapshot fields expose only borrowed epoch and sorted pending entries;
no mutation, receive, dispatch or process API. Three new tests cover the planned
cases. Main reviewed source and assertions; no separate reviewer used.

`sh scripts/validate-foundation.sh` and
`scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
255 Rust tests + 12 Node tests passed; two standalone ignored helper tests are
invoked by their parent tests. Architecture: 7 packages, 41 dependency
declarations, no errors. No dependency/migration added. Upstream tests not run.

Final untracked product file fingerprints (SHA-256):
- `crates/protocol/src/correlation.rs`: `f7e91a4e9000676f9790f46239d088b8ff7843acc704f94f66bfe38f8b4ec445`
- `crates/protocol/src/connection.rs`: `ddd152cbb38164af1ea2ef4057231af3c90a0a2b4b1ecc1e2bbcc1f6c11434da`

Next: bind terminal accounting to actual supervisor output and completion;
persist a validated terminal RPC receipt and reconcile ambiguous crashes.
No full W2/W1-I gate passed by this change, no live account execution.

# W3 initialize handshake state

status: review | owner: main | task: P9.T11 partial (full adapter remains open)
source-gate: ready

Source before code, Codex clean HEAD
`818f1cca8ccf8899f0f4d59336baebaccf358eed`:
- `codex-rs/app-server-protocol/src/protocol/v1.rs:29–82`: clientInfo and
  client-declared capabilities, InitializeResponse userAgent/codexHome/platform
  fields. experimentalApi is a request for experimental fields, not effective
  model capability or account authorization.
- `app-server-test-client/src/lib.rs:1737–1779`: initialize request, typed
  matching response, then initialized notification through write/flush path.
- `app-server/src/request_processors/initialize_processor.rs:50–80,125–210`:
  per-connection initialization checks reject repeated initialize.
- `app-server/tests/suite/v2/connection_handling_websocket.rs:67–108`: two
  connections handshake independently, pre-init request fails, identical IDs on
  distinct connections route separately. Assertions read, upstream test not run.

Adopt ordering into a pure non-Clone handshake object: waiting reply → response
validated → initialized prepared → host confirms write → ready. Matching RPC
error or malformed response fails handshake; unrelated event/ID does not advance.
Host must route/authenticate epoch before observe. Timeout/disconnect/write error
calls fail, never retry on same state. Ready is local handshake sequencing only.

Response projection keeps userAgent/platformFamily/platformOs only, validates
bounded nonblank strings; intentionally ignores codexHome and future fields,
never reads or uses server-provided auth paths. This is not full response schema
validation, version negotiation or proof of model/account/subagent capability.
Client name/version bounded at construction. Experimental API opt-in explicit,
no notification suppression, no attestation opt-in or auth settings touched.

Acceptance: request shape and explicit capability flag, exact typed response ID,
no readiness before ack write, duplicate/out-of-order calls, malformed/error
response, fail-after-preparation and unrelated messages. No native process run.

## Verification

- Three unit tests verify request capability shape, typed-ID mismatch, metadata
  projection, ack order, duplicate calls, rejection/malformed response and
  explicit failure before/after readiness. Unrelated messages remain a routing
  responsibility; no automatic replies or live account operations.
- `sh scripts/validate-foundation.sh`: exit 0, 87 Rust + 11 Node tests pass;
  architecture remains 6 crates / 30 declared direct dependencies.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- handshake.rs SHA-256:
  `4d37d96d5f94993f252c37f195fd0b57d01014f8c92b96cf9e3b9acad4b00a54`.
- Main review: response deserialized by reference without cloning unused
  codexHome; metadata only exposed once locally ready. Local request construction
  remains possible outside this object, so host must enforce this sequencing;
  the standalone handshake does not gate every send yet. Skills rust-router and
  m05-type-driven informed private explicit states rather than capability claims.
- Next: compose handshake/correlation/framing into one connection host, then
  owned transport with bounded stdout/stderr and failure reconciliation. No
  independent review, upstream test execution or live acceptance claimed.

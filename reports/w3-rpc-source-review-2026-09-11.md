# W3 RPC message classification

status: review | owner: main | task: P9.T11 partial (full adapter remains open)
source-gate: ready

Source before code: Codex clean HEAD
`818f1cca8ccf8899f0f4d59336baebaccf358eed`.
Read `codex-rs/app-server-protocol/src/rpc.rs` completely: four message families,
string/i64 IDs, optional request trace/params, no jsonrpc version field expected.
Read `codex-rs/app-server-test-client/src/lib.rs:2070–2127`: response waits match
ID, buffer notifications and dispatch server requests, rather than treating all
messages as notifications. Avoid silently dropping unmatched responses as that
test client does: leave correlation decisions to the host's pending-request map.
Previous stdio test source at `app-server/tests/suite/v2/connection_handling_stdio_tests.rs:72–90`
asserts initialize is a JSONRPC Response; source read, not upstream test execution.

Adopt four-way classification, preserve string versus numeric IDs, optional
params and request trace. Add strict envelope fields to prevent untagged serde
fallback from downgrading a malformed request to notification or mixed response
to success. Unknown methods remain explicit request/notification; never auto
approve. Reject ambiguous envelope, invalid UTF-8/JSON, null/float IDs and invalid
method names. Bound bytes before serde; errors do not echo payload content.
Null result is valid and must remain distinct from missing result.

Architecture decision: graph-protocol is the JSON process boundary, so allow its
direct serde_json dependency using the existing pinned workspace version. No new
crate version, domain dependency or network integration; update intentional
architecture policy with a regression keeping JSON out of domain/application.
Payloads remain untrusted Values and need method-specific validation. Frame cap
is not a bound on serde allocation overhead; no automatic logging of messages.

Acceptance before code: four message kinds and unknown methods, typed IDs,
null result, request trace, mixed/duplicate fields, invalid IDs, byte cap,
trailing JSON; framed stream interleaving request/response/notification/error.
No dispatcher, handshake, auth, pending-map, process or live acceptance claimed.

## Verification

- Four Rust tests cover envelope kinds, strict rejection, byte policy and
  frame→RPC interleaving at chunk sizes 1, 7 and whole stream. No automatic
  approval or lifecycle mutation was added.
- `sh scripts/validate-foundation.sh`: exit 0, 78 Rust + 11 Node tests pass.
  Architecture: 6 crates / 30 declared dependencies. New Node regression
  explicitly keeps serde_json forbidden in domain/application.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- rpc.rs SHA-256:
  `0200e0344e7f9a01b649dfbe4801a3c8443564e3d3f4159a7eaf85abecb9cb9d`.
- Main review: null result accepted; missing result and duplicate/mixed envelope
  fields rejected; string and integer IDs kept distinct. Deserialize bypasses
  decode byte/label policy and is documented; host must use bounded decode.
  Opaque nested params/trace still require method-specific validation.
- rust-router/m11-ecosystem guided reuse of the existing locked serde_json at
  the protocol boundary, with no additional package version or domain leakage.
- Next: pending-request correlation and handshake, method-specific request
  validation/approval, owned transport and authenticated generation binding.
  Independent review and live runtime tests remain pending.

# W3 bounded outbound RPC serialization

status: review | owner: main | task: P9.T11 partial (full adapter remains open)
source-gate: ready

Before code, read Codex clean HEAD
`818f1cca8ccf8899f0f4d59336baebaccf358eed`:
`codex-rs/app-server-test-client/src/lib.rs:2280–2318` serializes response/message,
then writes newline-delimited payload and flushes; write errors propagate.
`app-server-protocol/src/protocol/common.rs:4240–4265` asserts notification JSON
method/params shape. Source tests read, not run. Adopt wire shape and newline;
do not copy pretty-printing arbitrary payloads to logs or unbounded to_string.

Read local `crates/cli/src/output.rs` fully: existing LimitedBuffer checks byte
budget before append, uses fallible reserve, adds LF, returns all bytes before
stdout write; Unicode/exact/one-under test already exists. Move implementation
to protocol as generic Serialize utility and keep CLI facade/tests. Reuse rather
than maintain two copies; not a claim that old code was source-first reviewed.

Add Serialize to RPC envelopes, omit absent optional fields per Codex rpc.rs
contract, preserve null result. Message::encode_line validates identifier/method
policy and 1..16 MiB cap including LF, then uses bounded utility. Derive-based
serialization and generic utility alone do not enforce RPC policy; use method.
No I/O/flush/dispatch, approval or logging. Successful encode is not successful
delivery; write failure/timeout still needs uncertain pending reconciliation.

Acceptance: all message families semantic roundtrip, missing optional fields,
null result, escaped Unicode content as one frame, exact/one-under cap, invalid
outbound identifier/method, and existing CLI output-budget tests unchanged.

## Verification

- Two new RPC tests cover all four families plus trace, optional-field omission,
  null result, escaped Unicode as one frame, exact/one-under limits and invalid
  constructed outbound labels. Existing CLI budget tests retained and pass.
- `sh scripts/validate-foundation.sh`: exit 0, 84 Rust + 11 Node tests pass;
  architecture remains 6 crates / 30 direct dependencies.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- output.rs SHA-256:
  `8ee11a4a8e287c4dcf59ea1ac703e9a748c93a5d4a580518e1f8b8f2e017e086`.
  rpc.rs SHA-256:
  `55b601c205ca96aa4c3d54774456f79f1e3e31eae29f323cff11f32c23c02d24`.
- Main review: CLI now reexports protocol utility; one LimitedBuffer remains.
  RPC encode validates labels then serializes directly without intermediate
  Value. Generic output errors use transport-neutral byte-budget wording.
  Skills rust-router/m11-ecosystem/m04-zero-cost guided reuse at protocol layer.
- Next: initialize/initialized handshake and transport write/flush ownership.
  Serialization tests are not live protocol negotiation or delivery evidence;
  independent review and allocation-failure injection remain unperformed.

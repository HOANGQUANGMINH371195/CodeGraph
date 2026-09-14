# W3 native lifecycle notification boundary

status: review | owner: main | task: P9.T11 (partial; native adapter remains open)
scope: graph-protocol native lifecycle projection + CLI-crate contract tests
source-gate: ready

## Source-study trước code

- Codex HEAD `818f1cca8ccf8899f0f4d59336baebaccf358eed`, worktree sạch.
- `codex-rs/app-server-protocol/src/protocol/common.rs:1910`: method names;
  `:4240` serialization test asserts method/params with camelCase threadId.
- `.../protocol/v2/thread.rs:1645`: notLoaded/idle/systemError/active plus
  waitingOnApproval/waitingOnUserInput flags; `:1975` thread/closed payload.
- `.../protocol/v2/turn.rs:32` status enum; `:509` turn/completed includes
  thread_id plus Turn, not merely the turn object shown in abbreviated docs.
- `codex-rs/app-server/src/request_processors/thread_lifecycle.rs:414`: bounded
  shutdown returns Complete/SubmitFailed/TimedOut; `:441` only Complete removes
  matching runtime and emits thread/closed. Replacement runtime prevents stale
  teardown; timeout leaves runtime loaded and does not emit closed.
- `codex-rs/app-server/tests/suite/v2/thread_unsubscribe.rs:90–195`: delayed
  close, resubscribe cancels unload; closed and notLoaded followed by empty
  loaded-list, then resume returns SAME thread ID idle. Read assertions only,
  not running upstream test or a live authenticated session.
- Official [app-server docs](https://learn.chatgpt.com/docs/app-server), sections
  Events / Unsubscribe / Start or resume: turn finish differs from thread unload.

## Adopt / avoid / acceptance

Implement a narrow Deserialize-only observation projection for thread/closed,
thread/status/changed, turn/started and turn/completed. Preserve thread/turn IDs
and typed statuses, ignore unneeded payload fields (items/text/errors) rather
than retain whole transcripts. Strict envelope rejects requests/responses and
unknown methods; caller must route other protocol families separately, not
silently mark them completed. Unknown lifecycle statuses/flags fail parsing.
Validated nonblank IDs capped at 256 bytes are harness policy, not upstream limit.

Do not link Codex internals or copy implementation; use the wire contract.
Do not derive terminal authority, authenticate account, bind child lineage,
release admission slots or accept tasks here. Adapter must bind transport/runtime
generation and handle close→resume before any lifecycle mutation. A raw JSON
notification is NOT an authenticated capability. Host must bound frame bytes
before serde and avoid logging raw unknown payloads. Transport and negotiation
are not implemented by this projection.

Acceptance fixtures: all supported states, identity validation, unknown status,
malformed/missing params, request-shaped spoof, turn-status/method mismatch;
turn completion and thread status cannot be parsed as a thread-closed variant.
Tests live in the CLI crate using its existing serde_json dependency, avoiding
a new protocol production dependency merely for test deserialization.

## Implementation / verification

- `crates/protocol/src/native.rs`: Deserialize-only four-method projection;
  private NativeId validated at construction; separate StartedStatus and
  CompletedStatus prevent method/status mismatch. No accepted-fact or native
  terminal capability is constructed. Extra payload fields are discarded from
  the result, not claimed to be allocation-free during serde buffering.
- `crates/cli/tests/native_contract.rs`: three contract tests cover supported
  states, malformed identities/envelopes, duplicates and request spoofing.
- `sh scripts/validate-foundation.sh`: exit 0, 70 Rust + 10 Node tests pass;
  architecture check remains 6 crates / 29 declared direct dependencies.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- SHA-256 native.rs:
  `d2f05ad432f5f475f26650b89dcfa2d6e16eb6765124cc63a41ec5215c26076f`.
  SHA-256 native_contract.rs:
  `6a24841a90c9bc2cd0e1907e828bd932c0347e41054c3a71e36b1f66eaf1b65c`.
- Main review: no added dependencies or source changes to outsource. Skills
  rust-router/m05-type-driven informed validated IDs and distinct turn status
  types; OpenAI Docs provided the official contract cross-check.
- Next: full message routing/framing, initialize/capability negotiation,
  authenticated session/generation binding, durable attempt mapping and live
  native acceptance. No independent review or upstream/live tests claimed.

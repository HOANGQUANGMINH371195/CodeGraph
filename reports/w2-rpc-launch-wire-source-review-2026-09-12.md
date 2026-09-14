# W2 RPC launch wire source review — 2026-09-12

Source gate: ready. This receipt was written before the wire implementation.
Owned paths: `crates/protocol/src/rpc_launch.rs` and this report only. Main owns
domain implementation and protocol module registration. No subdelegation.

## Authorization and source identity

Read `/home/minh/projects/project-graph-agent/AGENTS.md` completely and PLAN.md
§0.4.1. Applied rust-router, m09-domain, and m06-error-handling skills: untrusted
wire fields convert through validated domain constructors and propagate typed
errors; this descriptor grants no execution authority.

Before patching, `git rev-parse HEAD` and `git status --short` showed:

- Product repository has no HEAD (unborn repository); all visible product files
  are untracked, including crates, reports, scripts, Cargo files and plans.
  Preserve concurrent work; no clean-baseline claim is possible.
- Codex reference: `818f1cca8ccf8899f0f4d59336baebaccf358eed`, clean.
- OpenDev reference: `d32c660e4eed1a8e988d1fd58da88e41ba641d08`, clean.

Preimplementation SHA-256 fingerprints:

| Source | SHA-256 |
| --- | --- |
| codex `codex-rs/utils/pty/src/pipe.rs` | `eaa4b05fab90c9cc772239cfc9683413e501aa00013cae3984d9f821d8be3bc5` |
| codex `codex-rs/utils/pty/src/tests.rs` | `fee2bb89ffb4345243f6756297c4729d814fb6cee65b32ae700daccb1771bb77` |
| opendev `crates/opendev-mcp/src/transport/stdio.rs` | `b77053e4f86e52f350a2e9d443b5b5c9109105c67981503f3930defd32afc40a` |
| product `crates/protocol/src/command.rs` | `5dac61444014312881c1dffee804ce2bf87d57498f6de9472bbec94537b9eed8` |
| product `crates/protocol/src/execution_plan.rs` | `e55f1b92525e7aeab7ca35a231eae1da99c146ed20cff5d305bbb04c40b7ba7b` |
| product `crates/protocol/src/lib.rs` | `fe83d799e08831adb293fb3dfc61cec0beb06664f635cb4f2ac487fbcc7f19ce` |
| product `crates/domain/src/command.rs` | `cf17669092410946f9f31cf9132802b5e1cf499da5b077d101c051225a331e17` |

## Source findings and adaptations

- Codex `pipe.rs:134-193`, `spawn_process_with_stdin_mode`: argv is passed
  individually, cwd is explicit, environment is cleared then rebuilt, stdin
  distinguishes Piped from Null, and output streams remain separate. Also read
  post-spawn handling through line 265: bounded channels, optional stdin writer,
  independent readers, and Windows containment fallback. Adopt exact argv and
  explicit process settings as data; do not copy spawn/unsafe/platform code.
- Codex `tests.rs:400-442`, `pipe_process_round_trips_stdin`: a real child
  receives a line, stdin closes, output must contain `roundtrip`, and exit is 0.
  The non-Windows test can skip when Python is absent. This proves the intended
  writable-stdin distinction, not JSONL framing or launch authority. No upstream
  process test was run for this wire-only assignment.
- OpenDev `stdio.rs`, `StdioTransport::connect`, `send_request`, and `close`:
  piped stdio, pending response table, read/write errors, timeout, stdin EOF,
  termination, and pending cancellation were inspected. `stdio_tests.rs`
  includes a Python echo integration test and a disconnected-state
  unit test. Do not adopt Content-Length framing, inherited environment,
  command/argv logging, or unbounded notification queues. This contract selects
  only `stdio_jsonl`, records finite pending/frame settings, and redacts Debug.
- Product `protocol/command.rs`, `execution_plan.rs`, `lib.rs`, and
  `domain/command.rs` were read completely. Adopt required fields,
  `deny_unknown_fields`, schema checking, existing task/lease conversions,
  constructor validation, and inverse conversion via getters. CheckCommand's
  closed-stdin semantics must not become the RPC process type. ProjectRef is an
  infallible wire conversion; the enclosing domain constructor must validate
  snapshot/provenance relationships. Also inspected domain execution target
  validation and protocol check-run fixture conventions.
- Licenses reviewed: Codex Apache-2.0; OpenDev MIT. No upstream implementation
  copied. New DTO and tests follow the local protocol conventions and the
  explicitly supplied domain API. No account/authentication or process changes.

## Planned implementation and verification

Implement strict versioned RpcLaunchSpec, RpcTransport::StdioJsonl, and required
unversioned process/connection structs. Give all three structs redacted Debug;
constructors perform all semantic validation. Preserve all values bidirectionally.
No spawn, new dependencies, module registration, or whole-workspace formatting.

Tests will cover exact JSON/domain roundtrip (including literal and Unicode argv),
Debug redaction, missing/null/unknown/duplicate fields at every object boundary,
unsupported schema and transport (including string `null` and JSON null), and
malformed values/provenance rejected through domain conversion. Run scoped
protocol tests once main's domain types and registration are available. Full
foundation gates and plan synchronization remain the main integrator's scope.

## Results

Completed the two owned files. Main separately supplied/exported the domain
types and registered the protocol module. Read the supplied domain source before
wire implementation and matched its constructors/getters: labels are nonempty,
at most 256 bytes, and contain no control characters; pending requests range
from 1 through 4096 and frames from 1 through 16 MiB. The adapter's narrower
frame policy is not imposed by this wire/domain conversion.

The transport decoder accepts only a string equal to `stdio_jsonl`, rejecting
Serde's alternate externally tagged object representation. Following main's
review, unknown transport strings produce the fixed `unsupported RPC transport`
message without echoing input. A raw-JSON secret-string regression verifies
that specific diagnostic. This does not claim generic parser diagnostic
redaction; other Serde diagnostics must still be bounded/redacted by the host.

Final verification:

- `scripts/with-local-tools cargo test -p graph-protocol rpc_launch -- --nocapture`:
  **11 passed, 0 failed, 0 ignored, 33 filtered out**.
- `scripts/with-local-tools rustfmt --edition 2024 --check crates/protocol/src/rpc_launch.rs`:
  passed. Formatting touched only the owned Rust source.
- Missing, null, unknown, and raw duplicate fields are exercised for all seven
  object boundaries: launch, task, task project, lease, execution snapshot,
  process, and connection. Duplicate tests require a duplicate-field error,
  rather than accidentally accepting a different parse failure.
- Exact wire/domain roundtrip preserves literal/Unicode/newline argv, both
  experimental values, distinct source/execution worktree and HEAD claims,
  hashes, and bounds. Launch/process/connection Debug redaction is tested for
  both wire and domain forms. Malformed process, connection, task, lease,
  identity, and snapshot fields fail through typed domain errors. Boundary
  values and 16 MiB frames are accepted; numeric widths/types are strict.

Limits: no launch or upstream process integration test was executed; no host
authority, ledger, clock, filesystem identity, executable/environment digest,
or actual resource cap was verified. Conversion is structural validation only.
No new dependencies, registration edits, domain edits, account changes, or
subdelegation. Full foundation tests, whole-workspace formatting checks, and
plan synchronization are left to main as assigned. The product remains an
unborn, concurrently edited repository, so git cannot supply a committed diff
baseline; both owned files appear untracked.

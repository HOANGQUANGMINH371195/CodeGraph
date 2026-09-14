# Compose CLI/protocol integration — source gate

Before implementation, main reread actual OpenDev CLI runners, Codex CLI
structured output branches, product evidence CLI/tests, source capability,
source verifier, store evidence lookup and bounded JSON output/tests.

- OpenDev revision `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
  `crates/opendev-cli/src/runners.rs` SHA-256
  `75fff7c32e64eae41c17c629cf0cf0b8184f756f342c1807fc523a5aee9fde4f`.
- Codex revision `818f1cca8ccf8899f0f4d59336baebaccf358eed`,
  `codex-rs/cli/src/main.rs` SHA-256
  `aaa8e6b68c603f53572da2088bc7ec0469256a94e2c8f3f9858939c87ccc0902`.
- Product CLI output helper SHA-256
  `495e2bc780cb7faad3935d111c722df1b2b67acdbcd5e1114423f986095811f2`;
  evidence integration tests
  `044f93186d0b8f038ad7b14c46a1b7569494f659536a6a6f9390d1a7edac463c`.

Adopt Codex's structured serialization and OpenDev's nonzero failures/stderr;
do not adopt OpenDev's no-TUI auto-approval behavior or execute an agent runtime
to inspect a manifest. Product `output::json_line` already caps encoded bytes
including newline before publication: reuse it, not a second output buffer.
Store `source_evidence` matches the complete supplied ProjectRef/graph version;
DirectorySource constrains paths and regular-file/source-byte checks; verify_source
authenticates hash/slice; analyze_compose then requires a full-file slice.

Implementation decision: `analyze-compose ID TASK_SPEC ROOT` is an explicit
offline candidate query. Use existing recorded citation and caller-selected
snapshot root; no inference of whole-worktree identity, analysis-run validity,
runtime truth or graph-write authority. Read-only refers to candidate/evidence
records: normal CLI DB open may initialize/migrate, as existing commands document.
Output contains typed versioned protocol DTOs, declarations/edges/unknowns and
source citations, not raw YAML. CLI is the composition root between system adapter
and protocol; protocol/domain do not acquire an adapter dependency. Add only
graph-system to CLI workspace dependencies and its reviewed architecture allowlist.
Defaults: source cap 1 MiB (hard maximum), output cap 256 KiB (maximum 16 MiB,
including newline). Output budget failure publishes no partial JSON.

This exposes candidates for agents to consume and test; durable deployment graph
publication/invalidation and architecture-context/compiler integration remain
required subsequent W9/W11 work, not replaced by CLI JSON output.
Skills: rust-router, domain-cli and m07-concurrency; this synchronous bounded
operation does not need another async runtime. Luna independently owns CLI tests.

Before adding the shared-input regression, main also read actual Codex
`cli/tests/features.rs` (SHA-256
`84b78abab0816217fb6c0818d8620990615b0c58f44e9eefc1e8e6d5ad7d6ba6`):
its binary tests independently assert status/stderr/stdout semantics; these are
not graph or JSON-identity tests. Product `cli/tests/input_limits.rs` demonstrates
8 MiB rejection before mutation. Extend that existing oversized-input test to
the new command, retaining the no-partial-stdout assertion. No Codex feature
configuration or live account is modified by these fixture tests.

## Integrated result and verification

Implemented `crates/cli/src/compose.rs` as the composition root plus output-only
`graph_protocol::deployment` DTOs. `main.rs` adds the explicit `analyze-compose`
command and writes only the pre-encoded complete result. Protocol remains
independent of graph-system; the CLI dependency is explicitly allowlisted and
tested against forbidden reverse dependencies. Cargo.lock updated offline for
that internal edge, with no new external library in this increment.

Real Orders CLI fixture: four nodes (two services, two volumes), two mounts;
the envelope retains exact source evidence, candidate/provenance flags and
unknowns. Derived evidence IDs remain absent from the store after the query.
The malformed fixture first passes `verify-evidence`, then fails specifically
at Compose YAML parsing; stale hash, snapshot mismatch and partial citation
assert distinct diagnostics. Exact 457-byte source succeeds; one byte less
fails. Exact serialized-output length including LF succeeds; one byte less
fails with empty stdout. Root symlink escape and environment-value leakage
are covered by independent Luna black-box tests reviewed by the parent.

Parent commands and actual results:

- `cargo check --workspace --offline`: exit 0 (updates internal lock edge).
- `node --test scripts/check-architecture.test.mjs`: 10 passed; real workspace
  has 8 packages and 47 declared direct dependencies.
- `cargo test -p graph-protocol deployment --locked --offline`: 1 passed.
- `sh scripts/validate-foundation.sh`, session `70847`: terminal exit 0;
  **308 Rust passed, 0 failed, 3 ignored; 15 Node tests passed**.
- After Luna's final assertion strengthening, parent reran
  `cargo test -p project-graph-agent --test compose --test evidence --test
  input_limits --locked --offline`: 8 passed (6 new Compose tests plus 2
  existing integration controls). No production changes after foundation.
- Initial parent workspace fmt check found default-edition formatting in the
  worker test. Applied workspace `cargo fmt --all`; final `cargo fmt --all --
  --check` passes. Do not infer workspace formatting from plain rustfmt defaults.
- `cargo run --bin project-graph-agent --offline --locked -q -- analyze-compose
  --help`: exit 0. The first `cargo run` without `--bin` failed because two
  binaries exist; README examples now select the binary explicitly.
- CLI scoped Clippy: exit 0 with two main.rs warnings (documentation markup
  and the existing large dispatch function). Protocol strict Clippy with
  `--lib --no-deps -- -D warnings`: exit 101, 61 diagnostics in existing
  protocol APIs, including missing Errors documentation. No lint suppression
  or claim of whole-workspace Clippy cleanliness is made.

Core implementation SHA-256:

- CLI compose.rs: `fcf4a04707b91a297832abc6d445979e92dfd71c7342830d08063dbbc4be89fb`
- CLI main.rs: `ebe68473c51e925d9f0bdaeed1d89b2d3bb4612762283c99a57f31aee634a820`
- protocol deployment.rs: `1a0dd0e5543b62ebfd21e4c38681332c8270dcb7ba3bf6fb7d6f7d61148834f5`
- Cargo.lock: `87b240157694d0eb8c9b06745040a793307c8ac30138a3688792e744c3476822`

Limits remain explicit: output byte cap is not an allocator cap or OS-write
atomicity guarantee; JSON input byte cap does not impose filesystem deadlines.
Caller root binding is not authenticated, and claims on the wire are not durable
verification capabilities. No graph publication/invalidation, architecture query,
or rendered diagram acceptance is provided by this command. W9/W11 remain open.

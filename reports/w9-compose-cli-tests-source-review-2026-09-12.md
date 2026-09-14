# W9 Compose CLI tests — source review

Date: 2026-09-12

## Source gate

This task was source-studied before the first test patch, per `PLAN.md` 0.4.1.
The project is an unborn worktree with no commit revision to record. The
current project source was read directly at the working-tree state, including:

- `crates/cli/tests/evidence.rs:5-95`: subprocess invocation through
  `CARGO_BIN_EXE_project-graph-agent`, record/query snapshot setup, exact JSON
  assertions, and the invariant that rejected commands have nonzero status and
  empty stdout.
- `crates/cli/src/compose.rs:11-41,44-95`: the implemented command flow is
  stored evidence scoped by the caller task snapshot, bounded full-source
  verification, offline projection, and one bounded JSON line; it does not
  persist candidates.
- `crates/cli/src/main.rs:160-178`: application errors go to stderr and the
  process returns failure; successful command bytes are written directly to
  stdout.
- `crates/protocol/src/output.rs:3-38`: serialized JSON plus the final newline
  is measured before publication, so the CLI test uses the exact baseline byte
  length and one-byte-under boundary.
- `crates/source/src/lib.rs:36-87`: the directory capability checks project /
  graph scope, regular-file status, byte limits, and bounded reads; the Unix
  symlink escape test follows this behavior at the process boundary.
- `crates/system/src/compose.rs:78-114,143-242,331-368`: Compose requires a
  line-one complete citation and matching full-source hash; unsupported values
  become value-free unknowns, and derived declaration evidence is generated in
  memory only.
- `crates/protocol/src/deployment.rs:8-65`: the exact snake-case wire kinds and
  output-only candidate schema.
- `fixtures/orders/compose.yaml`: the real Orders fixture, SHA-256
  `a2a29afb353a588cc28759f281819f5cfda82fd5044f8fd5e38247959fa37701`, 20
  lines, two services, two declared volumes, and two named mounts.

## Outsource comparison

Codex source revision: `818f1cca8ccf8899f0f4d59336baebaccf358eed`.

- `codex/codex-rs/cli/tests/execpolicy.rs` (SHA-256
  `98edf7cc70dfa79ae25ed688b590c323a826bcdd6f2c3cfc37969a0f4018936c`):
  adopts direct subprocess output capture followed by JSON parsing and exact
  object assertions for machine-readable CLI output.
- `codex/codex-rs/cli/tests/features.rs` (SHA-256
  `84b78abab0816217fb6c0818d8620990615b0c58f44e9eefc1e8e6d5ad7d6ba6`):
  adopts temporary isolated config roots and explicit failure/stderr checks.
- `codex/codex-rs/cli/src/exit_status.rs` (SHA-256
  `f2344dd67618220e6bddb780c4173009a4f8b0504ed12fc86d55edb4fc697e3c`):
  confirms subprocess tests should assert status rather than infer failure from
  text.

OpenDev source revision: `d32c660e4eed1a8e988d1fd58da88e41ba641d08`.

- `opendev/crates/opendev-agents/src/subagents/runner/simple.rs` (SHA-256
  `fc8dd345b6a0f3d49bddd1d2121630dea848c37c73b2d920d8ffbc0dead82c4d`) and
  `simple_tests.rs` (SHA-256
  `0589a56ae4aba5d8664a7e421b3b480fc82ca8088da9c7a9470745b21eee2a59`):
  adopt bounded, structured handling of subprocess/runner results and focused
  assertions over parsed JSON fields.
- `opendev/crates/opendev-tools-impl/src/bash/foreground.rs` (SHA-256
  `43d82096fa8b3acae52b8daa843559dcb1838246e9dd702d54037dddecad44a7`) and
  `helpers.rs`: avoid copying runner output truncation or merged stdout/stderr;
  this command's contract requires exact JSON stdout and empty stdout on
  failure, so the test captures both streams unchanged.

## Adopt / avoid / adapt

- Adopt direct `Command` invocation, temporary per-test state, parsed JSON
  assertions, status assertions, and stderr diagnostics from the Codex tests.
- Adapt those patterns to the project’s existing `record-evidence` and task
  JSON snapshot protocol; no `assert_cmd` or dependency change is needed.
- Adopt the project’s newline-inclusive output helper contract and source
  adapter’s Unix root-boundary behavior.
- Avoid source/environment value assertions that would bless leakage; assert
  only declared names, kinds, relationships, value-free unknown reasons, and
  absence of fixture environment values/secrets.
- Avoid production edits, direct library calls, global configuration, network,
  Docker/runtime behavior, and derived-evidence recording. All roots, files,
  databases, malformed inputs, and symlink targets are temporary fixtures owned
  by the test.

## Planned black-box coverage

The owned integration test will cover: real Orders positive CLI output and
schema, two services/two volumes/two mounts, exact output boundary including
newline, source budget and option caps, stale hash, task snapshot mismatch,
malformed YAML, partial citation, Unix symlink escape, environment-value
redaction, and read-only non-persistence of derived evidence IDs.

Coverage remains intentionally outside this task for production implementation,
unit-level Compose parser behavior (already covered by `crates/system/tests`),
whole-worktree fingerprint authenticity, Docker runtime topology, and
authentication/network behavior.

## Validation receipt

- `scripts/with-local-tools cargo check -p project-graph-agent` — exit 0
  (parent implementation was already present and compiled before this test
  patch).
- `scripts/with-local-tools rustfmt crates/cli/tests/compose.rs` — exit 0;
  `scripts/with-local-tools rustfmt --check crates/cli/tests/compose.rs` — exit
  0. Workspace formatting was not run.
- `scripts/with-local-tools cargo test -p project-graph-agent --test compose` —
  exit 0; 6 passed, 0 failed. The final version derives the source boundary
  from `ORDERS_COMPOSE.len()` (asserted as 457), proves exact-limit success and
  one-byte-under failure, checks stale-hash / snapshot / malformed-YAML /
  partial-line-one diagnostics separately, and verifies the malformed fixture’s
  hash before parser rejection.
- `scripts/with-local-tools cargo test -p project-graph-agent --tests` — exit 0;
  35 passed, 0 failed across the CLI unit and integration test targets.

The initial focused run exposed and corrected two test-only defects before the
passing receipt: custom fixture hashes were initially calculated from literal
backslash escapes, and the exact-budget assertion initially reserialized JSON
instead of comparing raw serialized bytes. No production behavior was changed.

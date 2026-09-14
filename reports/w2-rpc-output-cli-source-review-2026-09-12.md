# W2 RPC output verification CLI

status: done (CLI task only); source-gate: ready; owner/reviewer: main agent.

- [x] Read Orca src/cli/handlers/artifacts.ts (bounded read/request and printResult)
  and handlers/artifacts.test.ts:39–117 (valid projection, unsupported/oversize
  rejection). Clean revision 26f9fd8ea152ad6126c005e5ae602c7d201f4a99.
  Adopt distinct input/content/output budgets and no success output on error;
  do not copy cloud publishing/auth/default-allow capability behavior.
- [x] Read product CLI parsing/VerifyArtifact/RpcLaunch, input byte limiter,
  JSON-line cap, scoped DirectoryArtifacts reader, RPC verifier and CLI fixtures.
- [x] Pre-code plan: verify-rpc-outputs LAUNCH_SPEC ROOT with total byte budget
  and bounded JSON output. Exact launch is caller-supplied expectation, not
  authorization; root binds execution snapshot. Emit optional stream hash/length,
  completion and pending count, no full descriptors or methods. Fixed parse/root
  errors. Test absent/present outputs, malformed input, mismatch, missing/corrupt
  blob, budget and no partial stdout/events. No migration/dependency change.

Skills rust-router/domain-cli/m07-concurrency: keep synchronous local reads,
stdout data/stderr errors and nonzero failure. Upstream tests not run. Byte caps
do not impose filesystem I/O deadlines. Standard Store open may initialize/migrate.

## Validation

- Added `verify-rpc-outputs LAUNCH_SPEC ROOT`; launch input cap 8 MiB,
  content default/max 64 MiB, JSON default 65536/max 16 MiB including LF.
  Exact launch expectation + scoped CAS feed existing verifier; output contains
  only optional verified stream SHA/length, completion/count and explicit false
  authority/protection flags. Error paths do not print partial success JSON.
- New CLI integration test covers both absent/present stdout (stderr absent),
  missing terminal/blob, corrupt bytes, content budget, exact JSON cap, mismatch
  and invalid JSON fields. Real SQLite/CAS; synthetic metadata, no RPC spawned.
  Original completion remains HostError; methods/descriptors/secrets not printed.
  Event sequence unchanged after all calls.
- Targeted `cargo test -p project-graph-agent --test rpc_launch_query --locked --offline rpc_output_cli`
  through local-tools: exit 0, one passed. Full
  `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0,
  283 Rust + 12 Node passed; architecture 7 packages/41 dependencies, no errors.
  `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- No migration/dependency change or independent reviewer. Product files untracked;
  SHA-256 main.rs: `575d5ed2f142bde6a3ace7a7783433365c0d0af52cf8bba87434ffd0dde2353d`;
  tests/rpc_launch_query/output.rs: `5c35bf8202907c19969a4fc4a0a08224419b635af8dde26943e882bd8e96b3fb`
  (both relative to crates/cli).

## Remaining scope

No data repair, durable pending-output reconstruction, orphaned-worker recovery
or production authority/containment. Byte limits are not deadlines, and standard
DB open may initialize/migrate before command validation. No global filesystem
read-only claim. Full W2/W1-I and W0–W13 remain open.

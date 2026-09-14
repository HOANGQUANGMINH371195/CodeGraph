# W2 RPC launch description — source-first design

status: descriptor implemented and locally verified | owner: main
source-gate: ready
scope: pure domain descriptor, strict wire conversion, no spawn or authority

## Actual source inspected before code

- OpenDev clean `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
  crates/opendev-mcp/src/transport/stdio.rs:85–135: argv/environment and piped
  transport. Do not adopt inherited environment or implicit executable lookup.
- Codex clean `818f1cca8ccf8899f0f4d59336baebaccf358eed`,
  codex-rs/utils/pty/src/pipe.rs:134–193 explicitly separates stdin mode, cwd,
  argv and environment reset; tests.rs:423–442 checks pipe roundtrip and exit.
  These mechanics do not establish launch approval. No upstream tests run.
- temp-rs-ddd clean `12398c3c6ebbc67998c4ac10a60332f281eeb793`,
  crates/domain/src/config/setting_client.rs ClientSystemSetting/GrpcClientSetting
  and GrpcClientLimitSetting separate connection, limits and other settings;
  setting.rs and domain/Cargo.toml use public serde configuration. Adopt grouped
  configuration concepts, not public mutable or serde domain state or retry
  defaults. This harness keeps domain validation separate from wire parsing.
- Product current untracked worktree: domain CheckCommand, Lease, ExecutionPlan;
  protocol command.rs conversion/tests, ProjectRef and connection validation;
  execution ConnectionSetup. CheckCommand explicitly requires closed stdin.

## Decision and expected checks

Introduce RpcProcessSpec (exact command/environment digest and execution limits),
RpcConnectionSpec (epoch/client initialization and frame/pending bounds), and
RpcLaunchSpec binding those descriptions to a task, origin lease, host/approval
reference and execution snapshot. Fixed piped JSONL transport is explicit in
wire. Host/approval IDs and lease are untrusted references, not a grant; no
approved boolean, claim consumption, spawn or current filesystem verification.

Keep CheckCommand wire/semantics unchanged; share only structural command
validation to avoid divergent cwd/hash checks. Private fields and constructors
keep domain immutable. Wire denies unknown/missing fields and schema/transport
mismatch; exact argv and limits survive roundtrip, debug omits payload.
Task/lease must match, expiry positive, repository/config/ignore policy must
match execution snapshot. A different worktree/head is permitted as a claimed
execution snapshot, not assumed verified. Adapter-specific caps are not global
contract limits; actual initialize frame fit is checked later by ConnectionSetup.

Required tests: boundaries, invalid hash/path/NUL/labels/times/pending/frame,
task/lease/snapshot mismatch, each missing/extra wire field, unsupported schema,
transport confusion, roundtrip including shell-looking literal argv. No domain
filesystem/process/serde dependency. Full foundation and formatting after merge.
Durable one-shot claim, approval resolution and execution are subsequent gates.

The task/lease records the origin of the process launch, not one child process
per RPC request or native subagent. PLAN §1.4 keeps Codex native agent lifecycle
and Harness domain jobs separate; connection reuse still requires per-request
scope/authority. This descriptor alone cannot change account/model policy.
The host must also bound incoming descriptor bytes before serde allocation;
typed validation is not a transport allocation limit.

## Verification / integration receipt

- Main domain implementation: four new tests cover structural process checks,
  connection labels/bounds, task/lease/snapshot relationships, equality across
  changed launch fields and redacted Debug. `cargo test -p graph-domain --locked
  --offline` via scripts/with-local-tools: ten total tests pass.
- Existing `graph-protocol command::tests`: three pass after extracting shared
  command syntax validation; closed-stdin wire schema and exact argv unchanged.
- Wire worker Ptolemy (`01a093d8-bee5-7be2-80b8-3374006d38dc`) wrote only its
  new protocol source and source-study report. Main read types, conversions and
  all test sections; required fixed unknown-transport errors without echoing
  supplied values. Main independently ran all eleven new wire tests: pass.
  See w2-rpc-launch-wire-source-review-2026-09-12.md. Completed worker closed.
- Full `sh scripts/validate-foundation.sh`: 203 Rust + 12 Node pass, seven
  packages / 41 declared dependencies / no architecture errors. Two standalone
  ignored helpers remain exercised by their parent tests. `cargo fmt --all
  -- --check` through scripts/with-local-tools: exit 0.
- Main reviewed no process, serde or storage dependency added to domain; no new
  migration, authority constructor, approval lookup, claim or spawn was added.
  Structural contract acceptance does not imply W2 fixture gate or W1-I.

Post-format source SHA-256 (untracked worktree, not a committed revision):

- domain/command.rs: `76ab552a6c8d3a598bfac03620271a31626a7cbf8c9e98ee9bb02e3bf5353341`
- domain/rpc_launch.rs: `19fe8e8109e2de3851146839c9a235c69c2c746ce24bb26ef743c5528416f283`
- protocol/rpc_launch.rs: `0e891de2edd3c60b57845d722991f52d03b056e734a932b8b4e741310364fdb4`

Next: host-selected approval resolution and durable one-shot RPC launch claim,
then owned-fixture preflight/spawn/attachment composition and crash/reopen tests.
Do not reuse the check-specific claim as an RPC grant or replay an uncertain
launch. Initial-frame fit, adapter capability limits, actual executable/env/cwd
identity and current lease/snapshot/policy still require host verification.

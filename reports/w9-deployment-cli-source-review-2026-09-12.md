# W9 deployment CLI source receipt — before implementation

Parent reread actual outsource sources and failure/test paths:

- CodeGraph HEAD `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty
  `src/db/queries.ts:1909–1971`, SHA
  `e49e1f4b76afa3b832611668b15db67af45b5512287b67ea8c74412d503048db`:
  explicit ownership, validation before mutation and atomic replacement/receipt.
  `__tests__/parameter-reconciliation.test.ts:146–156`, SHA
  `1e54ec9356db14271cfa9a06ae75615c1939e465f7cc4ddaec895f96b99a003f`:
  receipt failure retracts candidate publication. Avoid nested-transaction
  flattening and implicit conversion of declared relationships to verified flows.
- Codex HEAD `818f1cca8ccf8899f0f4d59336baebaccf358eed`,
  `codex-rs/cli/tests/execpolicy.rs:1–58`, SHA
  `98edf7cc70dfa79ae25ed688b590c323a826bcdd6f2c3cfc37969a0f4018936c`:
  actual executable exit status plus exact parsed stdout assertions. Adopt
  process-boundary tests, not its CODEX_HOME fixture or authentication setup.
- OpenDev HEAD `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
  `crates/opendev-cli/src/runners.rs:109–130`, SHA
  `75fff7c32e64eae41c17c629cf0cf0b8184f756f342c1807fc523a5aee9fde4f`:
  stdout results, stderr errors and nonzero exit. Avoid outputting a success-shaped
  receipt on failure; no runtime/MCP connection behavior is copied.

## Contract before coding

Existing analyze-compose remains candidate-only. New explicit commands:
`publish-compose ID TASK_SPEC ROOT --expected-generation N`,
`deployment ID TASK_SPEC`, and
`invalidate-compose ID TASK_SPEC --expected-generation N`.
ID selects a registered citation's complete snapshot/path/compose adapter owner;
it does not pin the content of a later stored generation. Query/invalidation need
no source file, so deleted sources can be explicitly invalidated. The task JSON
is a caller-supplied scope selector, not a worker authorization token.

Publish reuses verify_source and analyze_compose_graph, never accepts a JSON
candidate as authority. All source work and bounded receipt serialization happen
before the existing Store CAS transaction. Mutation JSON describes the committed
generation only; failed parsing/budgets/CAS emit no success JSON. A stdout failure
after commit cannot roll back the DB: inspect generation before deciding retry.
Failed publish retains historical graph; explicit invalidation is separate, and
watch/reindex orchestration must later choose it on failed/deleted sources.

Query returns null for an absent graph owner, a graph-null tombstone for explicit
invalidation, otherwise a historical declared graph. It never reads source or
sets current byte verification true. Standard Store open may initialize/migrate.
Byte budgets are not source I/O deadlines or allocator caps. No deps/migration
needed; output-only protocol types remain free of system/store dependencies.

Dependency correction before manifest patch: initial compile found CLI cannot
name domain SourceEvidence/DeploymentScope because it only indirectly depends on
domain. Inspected current Cargo.toml and check-architecture policy; choose an
explicit inward CLI→domain dependency (existing workspace crate), rather than
re-exporting types through an unrelated adapter to conceal the dependency.
Update the allowlist and test this direction; no new external crate or migration.

## Implementation review and intermediate verification

Three explicit CLI commands and output-only stored-graph/snapshot/mutation DTOs
are implemented. `analyze-compose` shares source verification but remains
candidate-only. Mutation receipts encode before CAS; no post-commit read race is
used to manufacture the returned generation. Store conflicts receive a bounded
diagnostic covering both stale generation and immutable citation conflict.

Parent Linux transport test redirects publish and invalidation stdout to
`/dev/full`: both exit unsuccessfully after a committed generation; separate
query confirms the state and stale retry makes no additional mutation. Six
existing Compose tests still pass.

An initial foundation run failed the existing input_limits assertion because
the shared task helper replaced the 8 MiB read-limit diagnostic with a generic
error. Fixed implementation, not assertion: read errors propagate, while JSON
parse/domain validation remains sanitized. Expanded the unchanged limit's test
coverage to publish/query/invalidate, all passing alongside transport test.

Architecture test now permits explicit CLI→domain and rejects domain→CLI/clap;
8 packages, 48 declared dependencies, no new external library. CLI Clippy review
has no diagnostics in compose.rs; two existing main.rs warnings remain (doc
backticks and large dispatcher). No whole-workspace strict lint pass claimed.

## Final verification

Native worker Herschel was explicitly requested with `gpt-5.6-luna`, no
descendants. Delivered nine independent CLI tests and source receipt; parent
reviewed them and corrected initial test assumptions about per-item citations
and serialized key order. Parent further asserted exact fixture lines (nodes
2/11/19/20, mount edges 10/17) and actual service→volume endpoints. Neither
source validation nor test budgets were weakened.

- `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0;
  **348 Rust passed / 0 failed / 3 existing ignored; 16 Node passed**.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- Targeted nine deployment tests, transport test and expanded input-limit test
  independently passed. Existing six Compose candidate tests remain passing.
- Source reads, parser rejection, CAS, exact output bytes, query after deletion,
  tombstone/recovery and post-commit output failure exercised through real CLI
  processes and SQLite. No live account/network/Docker operation involved.

Final SHA-256:

- CLI compose: `c979ef2838e8189d8805262f8169ba0a8918f83d7d15b6f50b07005b845f2402`
- protocol deployment: `cb229b314b9312ea87eed9ad8f9216cf2e7b9768e64ccdc6ecb93c60cceba16a`
- CLI deployment tests: `f8669443133fb4c8c9024e66d85c2049d968999e6aac6b4803309e8589cde49d`
- transport test: `71a7eeff9fffea136b7100c4646408e23043f2993a54c38665e19ba3d06285ae`
- Cargo.lock: `5c9c66767989c8fa5c3670929a41bbc119b92e4130ad0ee4b92ca087ef318640`

Remaining: automated watcher invalidation/reindex and build/module/API/SQL,
context compiler and real diagram acceptance. Commands currently operate on an
explicit caller-selected owner; they do not implement fleet authorization,
prove current architecture or complete W9/W0–W13.

# Declared deployment context — before-code source receipt

Parent read PLAN query surface and W11 compiler requirements, plus current
CodeGraph HEAD `3ed73bc127323e63153bf6ec8354afa82ce36aaf` sources:

- `src/context/formatter.ts:1–190`, SHA
  `91a8ab80fed68e548d5b2aaf64b601ba4352a88ef5d5c13f4c66c1e2de9d4083`:
  compact entry points/relationships, structured node/edge serialization,
  adjacency and visited-node handling. Adopt structured graph context, avoid
  implicit slice(0,10) omissions and unnecessary code bodies in this adapter.
- `src/context/markers.ts` complete, SHA
  `78914262dea5355a85caa7adf810646dedf1ba0901d09aff934770d097a9077a`:
  explicit low-confidence handoff must not contradict comprehensive framing.
- `__tests__/context.test.ts:370–392`: existing conditional code-block checks
  are not proof of bounded deployment traversal; add strict independent tests.

## Contract chosen before patches

Application-only deterministic BFS of validated DeploymentGraph, exact seed ID,
outgoing/incoming/both traversal, depth 0..16, nodes 1..1000, edges 0..5000.
Admission considers both node and edge caps before adding an edge/new endpoint;
never emit dangling edges. Preserve original edge orientation and kind. Return
indices into the input graph plus explicit depth/budget limitation flags. Cycles
and self-edges terminate; graph order breaks ties. Counts omitted from the whole
input also include unrelated/direction-excluded facts, not just truncation.

CLI `deployment-context ID TASK --node NODE_ID` reads only stored graph,
defaults depth2/both/nodes40/edges80/output32768bytes (newline included). Output
deduplicates full citations by ID and replaces per-node/edge citation objects
with evidence_id references. Include root source citation, complete generation,
historical/untrusted markers and total/omitted counts plus global unknown_count.
Unknown count describes the whole owner, not causal attribution to this seed.
No source bodies, no source verification, no graph writes except standard DB
open/migration. Missing owner/tombstone/seed is an explicit error, not a made-up
empty architecture. Output cap rejects completely rather than slicing JSON.

This is a narrow declared-graph context building block, not the completed
project_graph_context_v1 intent router, multi-provider context compiler, semantic
search or Archify IR. Those requirements remain open. No new dependency/schema.

## Verification and limits

- Luna worker supplied eight independent traversal tests; parent reviewed their
  assertions on direction, cycles, self-edges, exact limits and endpoint closure.
- Parent added two CLI tests (deployment suite now 11): stored context still
  works after deleting the owned fixture source root, citations deduplicate and
  resolve through evidence lookup, exact byte budget succeeds and one byte less
  fails, invalid limits and absent/tombstoned owners fail without graph changes.
  Input-limit coverage includes the new command. The >4000-line padded fixture
  produces context smaller than one fifth of its bytes; this is synthetic
  fixture evidence, not a general token-saving benchmark.
- Final `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0,
  376 Rust passed, 0 failed, 3 ignored; 16 Node passed, 0 failed;
  architecture gate 8 packages / 48 declared dependencies.
- `cargo fmt --all -- --check`: exit 0. CLI all-target Clippy: exit 0,
  zero diagnostics scoped to deployment_context.rs; this does not assert a
  warning-free workspace. No dependency or migration changes.
- Coding-guidelines informed checked private limits and constructor error docs.
  Stored citations remain historical: source bytes, analysis run and runtime
  relationship verification are all false. Store reconstructs the whole owner
  before traversal, so output limits are not bounded SQL fetch/allocation costs.
- Full W9/W11 acceptance, multi-provider context routing and diagrams remain open.

Final SHA-256:

- application/src/deployment_context.rs:
  `1447a15a31419b8321609efe725ee4662db551fc3aa6fb888e343f0080eb130d`
- application/tests/deployment_context.rs:
  `6cf6b454925585bb00a7280e73e52feb82e6252e0da7804121c1e53d5ab50182`
- cli/src/deployment_context.rs:
  `2ddc792b07b3648e3ac0e7c76ded4b25b1abf5ab5ea5e3d204a8af9afe351fd6`
- cli/tests/deployment.rs:
  `9788e83bb5062f0af3b29a67570fa3fbc548555c50177b940f93e80fc4893685`
- protocol/src/deployment.rs:
  `26fe53795d1e06731f8e4449c613c58b8c24898ceacf45a24e13e2430dd732a6`

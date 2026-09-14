# W4 scoped synthesized-edge replacement

status: done (storage seam only); source-gate: ready;
owner/reviewer: main (no independent review).

- [x] Gate reset for new task; read callback-synthesizer.ts complete commit tail
  (3688–3890), queries.ts edge insert/read/delete and endpoint validation paths,
  schema.sql edge identity, sqlite-adapter transaction, db/index transaction,
  batched-ref-cleanup setup/assertions and synthesis-progress tests.
- [x] Pre-code source revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf,
  dirty fingerprint c98b8c0155112521fe45e863f588efe9956b639832d82b638065bed77dc83f43.
- [x] Observed flow: readonly synthesis passes → registry-order pair dedupe →
  chunked additive insert. Wiring-only changes leave untouched endpoint edges
  alive. insertEdges silently skips missing endpoints; insert identity excludes
  provenance, so replacement must preserve collisions owned by another producer.
- [x] Adopt QueryBuilder's indexed per-source row lookup, endpoint set lookup,
  delete-by-primary-key batching, existing insert and transaction. No new SQL
  query strings/schema/assets. Require explicit nonempty source scope and exact
  heuristic producer tag; validate candidates before deleting, and report foreign
  ownership collisions instead of overwriting static/other-pass edges.
- [x] Test real SQLite replacement A→B with unchanged source/target nodes,
  empty candidate clearing, scope/producer preservation, collision accounting,
  missing endpoint rejection, duplicate/callsite identity and trigger rollback.

This component is a storage seam, not activated constructor inference. Caller
must supply ALL affected source IDs including sources with zero current
candidates, and collect against a stable index. It does not establish source
snapshot freshness or make separate reconciliation calls jointly atomic. Existing
adapter flattens nested transactions: propagate failures to the enclosing
transaction (do not catch them there and commit a partial operation).

## Delivered

QueryBuilder.replaceSynthesizedEdges validates exact producer/scope, serializes
candidate metadata before mutation, checks endpoint existence and replaces only
owned heuristic rows in one transaction. Reuses existing SQL paths; no new
production SQL strings or migration. Returns removed/inserted/conflict/duplicate
counts. Static/SCIP/other-producer collisions are preserved and reported, never
claimed as inserted. Explicit source scopes allow zero-candidate retractions;
duplicate candidates retain first-seen evidence under existing callsite identity.

The 16-test suite uses real temporary SQLite databases and failure triggers.
Covers 601-row deletion across parameter chunks and multi-source rollback. The
first run exposed a test API typo (getNode vs getNodeById), corrected before
rerun; no implementation failure was hidden. Focused regression command:

node node_modules/vitest/vitest.mjs run
__tests__/synthesized-edge-replacement.test.ts
__tests__/batched-ref-cleanup.test.ts __tests__/synthesis-progress.test.ts
__tests__/node-sqlite-backend.test.ts __tests__/constructor-join.test.ts

Run through product scripts/with-local-tools from CodeGraph: **54 pass / 5 files**,
exit 0. tsc --noEmit and git diff --check exit 0. Source
fingerprint c728fe03d5d66317310d2a22df55f2cd1470b608c9251e9e80a0a8f7e8389317.

Full suite, same wrapper/cwd: node node_modules/vitest/vitest.mjs run,
**4797 passed, 11 skipped / 4808 tests, 273 files**, exit 0 in 211.89 seconds.
Includes engine and UI projects plus native/WASM parity tests against the
available binaries. This turn did not rebuild kernel/UI or establish their
complete source correspondence; no fresh Orders or Rust foundation run.

Not yet called by production synthesis: activation must discover all impacted
sources (not just sources of new edges), establish stable input evidence, and
surface collisions/incomplete analysis. Existing additive passes unchanged.
This does not establish full W4 incremental correctness or constructor inference.

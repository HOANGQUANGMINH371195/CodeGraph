# W4 injected call mapping

status: verified bounded mapper; full W4 open; owner: main;
source-gate: ready (reset and reread this task).

Source: CodeGraph HEAD 3ed73bc127323e63153bf6ec8354afa82ce36aaf,
dirty fingerprint 4d810429ed5f46f95810bee8d3fe9df68e0006174fd5ee7f51c406f0f478deb6.

- [x] Read constructor-join.ts interfaces and exact class extent mapping;
  receiver-hazards.ts member scan/this ownership/effect spans;
  receiver-decision.ts worklist/status semantics and receiver-decision.test.ts
  real-index positive/negative assertions.
- [x] Read synth-utils.ts enclosingFn (line-only attribution),
  callback-synthesizer.ts SynthPassDef/registry/merge/insert lifecycle,
  resolution/index.ts post-resolution best-effort catch, and
  db/queries.ts replaceSynthesizedEdges ownership/transaction semantics.
- [x] Adopt exact source/index extent matching from constructor join and
  explicit heuristic provenance/registeredAt from synthesis. Do not use
  line-only enclosingFn for same-line nested functions, global name matching,
  or additive insertion for activation.
- [x] Plan before code: build a read-only call mapping from accepted field
  decisions to direct this.field.method effects and indexed methods. Preserve
  missing/ambiguous caller/target mappings and source-hash mismatch as issues.
  Exact method identity must exist on both sides. Nested arrows must map to
  the smallest indexed callable containing the effect (not the outer method).
  Candidate edges carry observed-source-only/runtime-assumption metadata and
  source hashes; never claim runtime proof or write the graph here.
- [x] Regression plan: real index with same-line methods, nested arrow,
  duplicate names in unrelated classes, unknown/multiple candidate classes,
  mutation rejection, stale source and missing/ambiguous indexed extents.

Gap: function-parameter method dispatch needs incoming-call coverage beyond a
visited handoff state; do not promote visited state to a globally proven type.
Activation/reconciliation remains required for W4. Darwin independently reads
the lifecycle/sync tests for integration risks; no write scope or children.

## Implementation and verification

Added CodeGraph `src/resolution/injected-call-mapping.ts` and real-index
`__tests__/injected-call-mapping.test.ts`; the product Orders diagnostic now
records `injected_call_mapping`. The mapper recomputes receiver decisions,
verifies source hashes/inventory and exact indexed class/method identities,
retains all observed targets and coordinate-distinct call sites, and reports
missing targets/callers instead of global-name or line-only fallback. Private
member names are not treated as transferable public method identities.
The source manifest is returned once with a digest per candidate edge.

Initial run: 10 passed / 1 failed. The failure exposed a real extraction gap:
`return ()=>this.store.send()` has arrow evidence at [73,94), but the index has
only file/class/constructor/run nodes. No exact arrow node exists. The mapper
correctly returned caller-unmapped. Replaced the incorrect positive expectation
with a regression asserting that gap and no outer-method misattribution;
left an explicit TODO for mapping after extraction supplies that node. This is
not claimed as anonymous-arrow support. Temporary diagnostic logging removed.

Final focused verification via product scripts/with-local-tools:

- `node node_modules/vitest/vitest.mjs run __tests__/injected-call-mapping.test.ts
  __tests__/receiver-decision.test.ts __tests__/constructor-join.test.ts
  __tests__/synthesized-edge-replacement.test.ts`: exit 0, 100 passed / 1 TODO,
  four files (mapper: 14 passed / 1 TODO).
- `node node_modules/typescript/bin/tsc`: exit 0; emitted build used by probe.
- `git diff --check`: exit 0. Full suite not rerun for this additive inactive
  module; the preceding 4920-pass full-suite receipt predates this mapper.
- [Orders raw receipt](../.harness/baselines/orders-injected-call-mapping-20260912-01.json):
  exit 0; source/dist/fixture unchanged, watcher observed, five probe exits 0.

Orders mapped four candidate pairs: OutboxRelay.dispatch → OrderStore.pending
and acknowledge; OrderService.create → OrderStore.createOrder;
NotificationService.consume → OrderStore.consume. Endpoint remains
receiver-incomplete. Graph still 51 nodes / 127 edges: candidates are not
persisted, query answers and architecture diagram are not yet accepted.
Post-test source fingerprint:
7dc017125196c20820aaed231f908e0bdc8a6804138029a8312ac69aaa315362.

## Lifecycle integration findings (not implemented)

Darwin, native inherited-model thread 01a094c2-e699-7602-9537-deb5b2ef80a1,
read current lifecycle/reconciliation source and tests; no edits or execution.
Main additionally reread index.ts indexAll lines 630–675 and sync 850–1015.
Worker closed after delivery.

- Ordinary changed-file sync resolves with resolveAndPersist; synthesis in
  resolveAndPersistBatched is not guaranteed to run. Removal-only sync also
  requires its own reconciliation trigger.
- Reconcile after deferred resolution in indexAll/sync, outside the old
  source/target-only merge. Scope must include old producer-owned callers,
  including callers with zero new candidates. Removing the wiring file need
  not delete either endpoint of the old edge.
- Do not equate a thrown/skipped pass with successfully computed empty output.
  Failure needs observable stale/incomplete state and retry semantics, while
  successful empty output must retract producer-owned edges. Existing storage
  rollback tests do not prove this lifecycle scheduling/freshness behavior.
- Required integration regressions: wiring-only A→B, wiring removal/unsupported
  source, language absence, pass failure/retry, coordinate-distinct calls,
  foreign-edge preservation, transaction rollback and fresh-rebuild convergence.

Next: implement source/index freshness and producer reconciliation lifecycle;
complete parameter-dispatch and anonymous-function extraction gaps, then
activate and verify full Orders flows. None of those requirements is waived.

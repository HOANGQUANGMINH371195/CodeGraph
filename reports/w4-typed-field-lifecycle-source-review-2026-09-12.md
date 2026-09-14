# W4 typed this-field lifecycle source review — 2026-09-12

Coordinator moved this receipt into the actual product report directory after
integration; the original worker interpreted the abbreviated `product/reports`
write scope literally. Historical source-study text below is preserved.

## Source study recorded before test implementation

Scope: only `codegraph/__tests__/typed-this-field-lifecycle.test.ts` and this
workspace-relative `product/reports/w4-typed-field-lifecycle-source-review-2026-09-12.md`.
The `product/` directory was absent; this uses the literal requested path under
`/home/minh/projects/outsource`. No delegation, network, runtime edits, other
test edits, global configuration, version changes, tests or builds are authorized
for this task. Concurrent worktree changes belong to the main agent/user.

Read `/home/minh/projects/project-graph-agent/AGENTS.md`,
`codegraph/AGENTS.md`, and outsource `PLAN.md` section 0.4.1 before coding.
Observed codegraph HEAD: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, matching
the supplied baseline. Supplied baseline fingerprint:
`d50245947faea360e37f8eac23552099ef103cf9692f5507244a14899bc0f2e7`
(user-provided; its construction was not independently reproduced).
The current dirty source was read directly, not inferred from HEAD.

Current source SHA-256 at study time:

| Source | SHA-256 |
| --- | --- |
| `codegraph/src/resolution/type-binding.ts` | `d994e02e4ee6968834248f1f8e69f68d2bc9162acd48fcf9ba7277f0a991f8f4` |
| `codegraph/__tests__/returned-callable-lifecycle.test.ts` | `9decd495cbc40e5553e782165a75edf940d9584c85e676f19efaf34620637814` |
| `codegraph/src/index.ts` | `2b1f447390615f3d827d1fd002ddcecce54237334b1ec42a274812a7c5c20dc8` |
| `codegraph/src/resolution/returned-callable-this.ts` | `e41e5597ee4cebdcc760cf3bd91ad9916f0bf77337a06c4893cc560b3da2417f` |

## Sources, adoption and exclusions

- `type-binding.ts:34`, `createTypeBindingResolver`: follows local aliases,
  type imports and explicit barrel exports to exact indexed declaration spans;
  rejects ambiguity, unsupported shapes and changed snapshot evidence. Adopt a
  real exported type alias `Store` and retain same-name classes in separate files.
  Avoid equating declared type identity with runtime allocation evidence.
- `returned-callable-lifecycle.test.ts:14`: owns temporary fixtures, uses real
  `indexAll`/`sync`/`recreate` and checks stale closures after position changes.
  Adopt that lifecycle. Avoid its allocation fixture, injected-producer-only
  filtering and name-only `send` assertion: this case requires exact IDs on
  ordinary declared-type heuristic call edges.
- `index.ts:783`, `CodeGraph.indexFiles`: extraction delegation under locks,
  not the complete full-index resolution lifecycle. Use `indexAll` for initial
  and fresh indexing. `index.ts:804`, `sync`: extraction, cache clearing,
  changed-file resolution, failed-ref retry, definition-delta reopening, orphan
  sweep and injected reconciliation. A same-spelling barrel edit may leave an
  already resolved call in an unchanged consumer untouched. Require barrel-only
  sync to rebind anyway; do not add the consumer to sync paths, force indexAll,
  skip the assertion or mark it as an expected failure to hide this gap.
- `returned-callable-this.ts:22`, `returnedCallableThisClass`: joins arrow
  extents to current lexical class extents; ordinary functions reset ownership.
  Adopt exact closure location and class/method containment checks. Avoid
  deriving ownership only by trimming qualified names.
- `returned-callable-this.test.ts`: uses class/member containment and exact
  outgoing target IDs. `type-binding.test.ts`: type-only imports, alias/barrel
  fixtures, same-name decoys, snapshot invalidation. Adapt these patterns using
  only the public CodeGraph query API, without private contexts or direct SQL.
- `extraction/returned-callable.ts`, `returnedCallableName`: location labels are
  one-based line/zero-based column and are not proof of invocation. Assert the
  closure's exact label/qualified name and its call edge; add no allocations.
- `resolution/index.ts`, lexical-this member handling: corroborates exact
  contains-edge identity. Runtime changes remain owned by the main agent.
- `LICENSE`: MIT, copyright 2026 Colby Mchenry; this is an original regression
  test within that repository using existing fixture/API conventions.
- `vitest.config.mts` and `vitest.workspace.mts`: engine project runs the real
  Node/SQLite suites; existing test configuration disables telemetry.

## Planned assertions and validation

Fixture: real `Store::send` definitions in two target modules plus a third
same-name decoy; an alias module exports `type Store = RealStore`; a barrel
reexports that alias; `Service` imports type `Store` and declares
`constructor(readonly field: Store) {}` and
`run(){return ()=>this.field.send();}`. No allocations or invocation driver.

Assert exactly one closure-owned call to the actual declaration ID, heuristic
confidence, no eager method call and no decoy callers. Shift both caller and
target source positions, sync and reject stale node/edge IDs. Change only the
barrel to the other alias module under the same exported spelling and require
the unchanged closure to rebind. Close/recreate/index the same owned root and
compare stable call-edge records. Remove the call while retaining the closure
and require all stale send edges absent.

Pending commands (not executed; run only when the user/main asks, from
`/home/minh/projects/outsource/codegraph`):

```sh
./node_modules/.bin/vitest run --project engine __tests__/typed-this-field-lifecycle.test.ts
./node_modules/.bin/vitest run --project engine __tests__/typed-this-field-lifecycle.test.ts __tests__/returned-callable-lifecycle.test.ts __tests__/returned-callable-this.test.ts __tests__/type-binding.test.ts
```

No build is scheduled by this test-only task. Execution results remain pending;
source review is not passing-test evidence.

## Implementation receipt

Implemented the fixture and lifecycle assertions above in one integration test.
The two real target classes deliberately also share `Store::send` with the
decoy, so file-qualified declaration identity is essential at every stage.
The position edit shifts both the caller and first target; checks cover removed
nodes and both incoming/outgoing stale edges. The barrel edit syncs only
`barrel.ts`, keeps the consumer closure ID and old target node, and requires a
new target ID. Fresh indexing compares complete serialized call-edge records,
including provenance and metadata, on the same recreated owned root.

Changed paths by this task (both newly added):

- `codegraph/__tests__/typed-this-field-lifecycle.test.ts`
- `product/reports/w4-typed-field-lifecycle-source-review-2026-09-12.md`

Reviewed the test text against the public query/edge types and inspected the
diff for whitespace errors. Additional source study before the final assertion
adjustment: `resolution/index.ts:1175`, `createEdges`, persists ordinary resolver
confidence in metadata and does not set synthesized-edge provenance. Accordingly
the test requires confidence strictly between zero and one, without requiring
the separate `provenance: 'heuristic'` marker. This preserves declared-type
heuristic semantics without imposing an unrelated storage contract.
No tests, builds, fixture execution, direct SQL,
network operations or delegation were performed. Barrel rebind remains an
unconditional requirement; whether the concurrent runtime implementation meets
it is unverified. The exact commands above remain pending.

## Coordinator integration and executed checks

The historical worker section above describes its test-only handoff. Main
subsequently activated explicit heuristic provenance for declared-type edges,
strengthened the test to assert that marker, and added invalid-barrel recovery
plus target-only position changes. The report now lives in the actual product
repository, `/home/minh/projects/project-graph-agent/reports`.

`typed-field-lifecycle-20260912-01.json` preserves the real failing barrel-only
rebind. After atomic source-evidence-driven reopening, the focused
`typed-field-lifecycle-20260912-03.json` run passes all 32 tests across lifecycle,
direct typed-field, legacy field-call and lexical-this suites, exit 0. The
lifecycle test passes without reindexing the unchanged consumer, covers failed
lookup recovery, and compares complete call-edge records with a recreated index.
Whole-product acceptance is not implied. See the main typed-field integration
receipt for full-suite/build/smoke evidence and remaining gaps.

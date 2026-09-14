# W4 route-conditioned flow tests — source review — 2026-09-12

Status: source gate complete; independent test patch pending parent API readiness.
This receipt is written before the test code.

## Pre-code revision and fingerprints

- CodeGraph baseline revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`
  (`feat(ui): the Steps tab lays a screen out in clusters and says a far link in words; a <Card/> is the Card its file imports (#1817)`).
- Baseline is dirty with parent work and unrelated shared-workspace changes; no
  existing edits are overwritten. The only planned writes are this report and
  `codegraph/__tests__/route-flow.test.ts`.
- Relevant pre-code SHA-256 fingerprints:
  - `src/index.ts`: `35255f3fe05af8032c53a787924a7b920ec9ae103c1986273ac2a27e3aafe3e4`
  - `src/resolution/frameworks/node-http.ts`: `ba521e1cb44769f8f482abb172567574450efbf28f8addd907859756d0f9acf2`
  - `src/graph/branch-guards.ts`: `68ae2056a199e87e30dc7d8cbdfd721029e9f32bd76dc0e2fc1d65e708f40ef2`
  - `src/extraction/tree-sitter.ts`: `f664f6af4bbde5412c32d80902121b915ebe030758a04620d1175fe41ef7bb82`
  - `__tests__/branch-guards.test.ts`: `847956da9309db045e52c4daf1592585e627bd4ed298ee6e09888a81be0a6dcd`
  - `__tests__/node-http-routes.test.ts`: `d4d1a8adcb9ff17fea0d5c162183d0086250cd886a904f39ef1315e03ba3dd15`

## Source read before coding

- `src/resolution/frameworks/node-http.ts`: extraction only accepts supported
  built-in HTTP imports, exact literal method/url predicates, and inline arrow
  or function callbacks. It emits route nodes plus a source-hash-backed
  route→callback reference, with callback span line/column metadata and an
  explicit negative-guard continuation bit. Resolution rejects missing or
  drifted source and does not prove runtime registration.
- `src/graph/branch-guards.ts:guardsInTree` and its file helpers: guard lookup
  is line/column-sensitive, returns execution-order guards, joins early-return
  continuations by branch identity, and stops at named/assigned function
  boundaries. Unsupported/unreadable/oversized source yields no asserted guard.
- `__tests__/branch-guards.test.ts:204-271`: sibling arms share a fork key,
  negative arms flip `negated`, and early exits preserve the continuation
  branch/exit evidence. Same-line sites therefore require columns.
- `__tests__/node-http-routes.test.ts`: real temporary files and SQLite-backed
  `CodeGraph.initSync(...).indexAll()` establish route/callback identity,
  same-line uniqueness, negative import/guard cases, and incremental stale-row
  retraction. No DB mocks are appropriate.
- `src/index.ts:2099-2111` / `src/graph/traversal.ts:593-648`: the existing
  `findPath` is an unrestricted structural BFS and remains the raw fallback
  comparison surface; it returns `{node, edge:null|Edge}` entries.

## Adopt / avoid / test adaptation (recorded before code)

- Adopt the existing route node IDs and callback spans, real indexed temp
  projects, source-byte hashing, line+column guard attribution, and the raw
  `findPath` comparison.
- Avoid mocks, raw source fallback, runtime/listen claims, containment edges in
  the conditioned path, guessed guards, and broad edits to parent implementation
  files. A route→callback→calls traversal is only a static candidate.
- Adapt the route fixture to exercise both sibling handlers (`/orders` and
  `/dispatch`), a common `create`→`persist` chain, an `/events` early-return
  branch with same-line call columns, an opaque guard, source-body omission,
  source drift, missing files, and depth/visit caps.
- Assert the stable envelope exactly enough to catch contract drift:
  schema/kind/status/reason, route/target IDs, `{node, edge}` path, excluded
  calls, unknown conditions, visited, source verification, index metadata, and
  `runtimeVerified:false`. Invalid caps must throw `RangeError`; unknown IDs
  must not throw.

## Parent readiness note

At receipt time the shared parent diff did not yet expose `CodeGraph.getRouteFlow`;
the test is intentionally not written against a guessed implementation. Parent
owns CLI integration, build, and the full baseline. Once the method is present,
the focused command is:

```text
/home/minh/projects/project-graph-agent/scripts/with-local-tools node node_modules/vitest/vitest.mjs run __tests__/route-flow.test.ts
```

Worker requested model: `gpt-5.6-luna`; worker ID: `01a09652-3100-7230-9b7b-53220697150a`.
The effective model was not independently exposed to this thread, so no
effective-model claim is made beyond the requested model record.

## Implementation handoff and verification

The parent implementation became available after the pre-code receipt. The
actual public API is async `CodeGraph.getRouteFlow`; the implementation files
read before final assertions were:

- `src/graph/route-flow.ts`, final SHA-256
  `c7fcbe58cd3bfc155f5a36f66efbc791a7a77717f33f094cef1d9c2ce3559685`;
- `src/graph/route-conditions.ts`, final SHA-256
  `cd3597d03c9d384a5e15c24747dbe946ebf16bdf811daac4352b30d8b0106811`;
- `src/index.ts`, final SHA-256
  `a5ba64e4bc909bef4a9df61d0ae103803562d84f0c757c1d02c736b662c1e19a`.

The route-flow implementation emits only the indexed route-file source path
and stored hash, with `verified` reflecting that route file alone; it does not
include a source body or imply runtime execution.

Final independent test command and receipt:

```text
/home/minh/projects/project-graph-agent/scripts/with-local-tools node node_modules/vitest/vitest.mjs run __tests__/route-flow.test.ts
exit 0 — 1 test file, 5 tests passed
```

Final test SHA-256:
`fb8cc24140630858470a1eefbacaa058ddb0869efc908678671b8aa77981b7f0`.
`git diff --check -- __tests__/route-flow.test.ts` passed. The suite uses real
temporary files, real indexing, and real SQLite; it covers same-callback
siblings/raw structural fallback, early-return and opaque conditions,
same-line columns, common calls, no-body output, stale/missing/same-size drift,
invalid caps, depth/visit exhaustion, unknown IDs, and rejection of a stored
non-node-http route.

Parent-reported companion verification: TypeScript emitted pass; 3 CLI tests,
6 legacy tests, 8 node-http tests, and 31 branch-guard tests passed. Parent
retains ownership of CLI integration, build, and the full baseline.

The first two focused attempts exposed only test-fixture defects: a missing
`routes/` directory and a one-byte-changing drift replacement. Both were
corrected in the owned test helper/fixture; no behavior assertion was weakened.

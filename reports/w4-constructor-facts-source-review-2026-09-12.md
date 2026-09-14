# W4 constructor assignment facts

status: done (syntax-fact component only); owner/reviewer: main; source-gate: ready.

- [x] Reset source gate and reread current resolver and tests before implementation.
- [x] Read `codegraph/src/resolution/name-matcher.ts:2649` (this-field gate),
  `store-accessor.ts` (AST scope traversal and tree disposal), `alias-binding.ts`
  (signature-based alias limitations), `extraction/grammars.ts:454` (shared parser),
  `extraction/tree-sitter.ts:518` (parse lifecycle), and complete
  `ts-this-field-call.test.ts` / `call-receiver-no-fabrication.test.ts`.
- [x] Pre-code decision: add AST-derived constructor parameter/field assignment
  facts with exact source offsets, parameter positions and class extent. Return
  plain data after deleting the tree; never retain WASM nodes or delete the shared
  parser. Preserve repeated assignments as separate facts, not a chosen type.

Source revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty worktree;
pre-code fingerprint `742e34304011a5836d3bb58e0aec9e11783e817aacd71c8ae43235b0e953845e`.
Existing changes are preserved. No new dependency, extraction schema or kernel
change. Tests will use real loaded tree-sitter grammars, not mocked syntax nodes.

Adopt AST field access and finally-based tree disposal from store-accessor.
Avoid its name-based regex recognizers for constructor assignment evidence.
The new function reports syntax facts only: `this.field = parameter` as a direct
constructor statement, not proof of stability, execution or runtime type. It must
not interpret comments/strings, nested function `this`, conditionals, computed
fields, default/destructured parameters, compound assignments or static methods
as this direct pattern. Distinguish nested same-named classes by source extent.

Tests: JS/TS/JSX/TSX; parameter position with comments; repeated assignments;
same-named classes; negative syntax forms; parse-error failure; repeated parse
after tree disposal. This is a component of the full inference implementation,
not a substitute for constructor call-site/import/alias binding, mutation/escape
analysis, possible-target provenance, incremental invalidation or unchanged orders
corpus remeasurement. No graph acceptance or W4 checkbox is earned by unit tests.

## Delivered and checked

Added `codegraph/src/resolution/constructor-facts.ts` and
`codegraph/__tests__/constructor-facts.test.ts`. No runtime resolver hook yet.
Existing resolver/extraction implementations remain untouched in this task.

- `node node_modules/vitest/vitest.mjs run __tests__/constructor-facts.test.ts
  __tests__/ts-this-field-call.test.ts __tests__/call-receiver-no-fabrication.test.ts`
  through product `scripts/with-local-tools`, CodeGraph cwd: exit 0,
  **28 tests passed** (21 new syntax-fact cases and 7 existing regressions).
- `node node_modules/typescript/bin/tsc --noEmit` through the same wrapper:
  exit 0 (before the final two test-only additions; production source unchanged).
- Post-change source fingerprint:
  `ed6370fb8fe8a2e83175953a67eb07252703e40c7b1ccd9258d7e973cba0a17b`.

No full upstream suite, product foundation rerun, rebuilt dist, native kernel
parity claim, live agent test or orders-flow improvement is claimed. Next:
consume these facts in scope-aware constructor actual-argument analysis; retain
all uncertain writes and construction sites; add import/alias/shadowing/mutation
tests before wiring possible-target edges and incremental invalidation.

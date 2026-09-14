# W4 external allocation-use evidence

status: done (lexical allocation-effects component only); source-gate: ready;
owner/reviewer: main (no independent review).

- [x] Reset source gate on resume; read CodeGraph AGENTS, full
  construction-sites.ts and its tests, constructor-join.ts and previous local
  receiver report (navigation only; current source reread).
- [x] Pre-code source: revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf,
  dirty fingerprint 7ba17846b1be1418cf149500e01813bc763cedddb92e070ea23a3408325641d4.
- [x] Adopt existing lexical scope lookup, immutable alias resolution, exact
  allocation offsets, unknown propagation and temporary tree disposal. Existing
  collector records origins but not subsequent instance uses. Extend it while
  the AST/bindings are alive; do not duplicate the parser/binding engine.
- [x] Design: collect allocation effects with source spans and property paths;
  distinguish writes, method calls, constructor handoffs, calls, and unmodeled
  escapes. Preserve unknown computed keys and spread argument positions. Join
  argument effects by same-file allocation offset. Coverage is lexical uses
  only, never whole-program safety. Keep requiresReceiverValidation true.
- [x] Verify real grammar tests for aliases, shadows, writes/delete/update,
  destructuring/loops, arguments/spreads, escapes/exports, detached values and
  parenthesized eval; run focused regression suite and typecheck.

Not solved: imported instance aliases, constructor/prototype replacement,
cross-file escape analysis, closed-world completeness, graph edge activation
and stale-edge reconciliation. No-local-effect cannot prove purity or stability.

## Delivered and verified

Extended construction-sites.ts using the existing binding table. Every site has
effectCoverage=lexical-uses-only and detached effect records (kind, source span,
line/column, property path). Constructor handoffs retain target allocation and
argument position; computed keys remain null. Only immutable tracked aliases
are transparent; mutable/destructured aliases, exports and other unsupported
uses remain explicit unmodeled effects. Parenthesized direct eval now invalidates
lexical origins. Constructor join attaches argumentAllocationEffects by same-file
allocation offset; no graph writes or persistent caches added.

- Focused new/modified suites: 64 construction-site + 28 real-index join tests,
  92 pass, exit 0.
- Broader regression command through product scripts/with-local-tools, CodeGraph
  cwd: node node_modules/vitest/vitest.mjs run
  __tests__/receiver-hazards.test.ts __tests__/constructor-join.test.ts
  __tests__/construction-sites.test.ts __tests__/constructor-facts.test.ts
  __tests__/ts-this-field-call.test.ts __tests__/call-receiver-no-fabrication.test.ts
  __tests__/store-exported-later.test.ts: **163 pass / 7 files**, exit 0.
- TypeScript --noEmit and emitting tsc: exit 0; git diff --check: exit 0.
  No full engine/UI/kernel build or Rust foundation rerun this turn.
- Orders smoke: node scripts/orders-graph-smoke.mjs
  /home/minh/projects/outsource/codegraph
  .harness/baselines/orders-allocation-effects-20260912-01.json: exit 0.
  [Raw receipt](../.harness/baselines/orders-allocation-effects-20260912-01.json).
  Source/dist/fixture unchanged, watcher observed with no errors, all five probe
  processes exit 0 (not proof of answer correctness). Still 51 nodes/127 edges.
  Source fingerprint c98b8c0155112521fe45e863f588efe9956b639832d82b638065bed77dc83f43.

Orders main source exposes three Store constructor handoffs and the close method
call; each service owner escapes into an HTTP factory call. Next receiver work
must follow those function handoffs and method bodies, not reject all injections
as inherently unsafe or assume unmodeled calls pure. Existing class-local reviews
and these lexical effects are evidence for that analysis, not a replacement for
it. requiresReceiverValidation remains true; no constructor-derived call edges
activated. W4 full flow and W0 acceptance remain open.

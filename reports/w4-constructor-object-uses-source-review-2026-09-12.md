# W4 constructor-object and prototype effects

status: done (lexical constructor-object evidence only); source-gate: ready;
owner/reviewer: main (no independent review).

- [x] Reset gate; reread receiver-hazards.ts and its tests, construction-sites
  origin/effect classifier, constructor-join class mapping/import memo, and
  Orders store implementation. Existing receiver checks do not account for
  Store.prototype.send replacement in another file.
- [x] Pre-code revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf,
  dirty fingerprint 6a176f3a896f2fe97ac80feb8678dcd3880269b242600de3a72b57a552334333.
- [x] Adopt existing lexical origin + property-path effect engine for class and
  runtime-import objects, including files that never instantiate the class.
  Skip binding declarations and direct new-callee uses, not exported/escaped
  values. Group by exact origin, then map to indexed class through strict export
  identity. Keep unresolved imports explicit (not necessarily class objects).
- [x] Verify alias prototype writes, reflection handoffs, unknown computed
  keys, shadowing, imported patch-only files and refresh/retraction evidence.

This is lexical constructor-object evidence, not full prototype/receiver safety.
Dynamic files, constructor aliases that exhaust bounds, static this and instance
method bodies still need explicit treatment before synthesis activation.

## Delivered and verification

construction-sites now returns constructorUses grouped by lexical class/import
origin. The shared effect classifier records prototype/member paths, writes,
reflection call arguments and escaping prototype aliases. Class value nodes
preserve anonymous/named-expression export evidence; declaration name tokens,
import specifiers, transparent immutable aliases and direct new-callee uses are
not false mutations. constructor-export distinguishes an export boundary from
an arbitrary escape; it does not prove safety or completeness.

Constructor join exposes constructorObjectUses with exact class ID when strict
class-export mapping succeeds; unmatched runtime imports retain classId=null
(they may be functions or external APIs, not necessarily classes). It scans
patch-only files even when they contain no allocations. Real-index regression
checks alias prototype mutation mapping to the injected argument class and
retraction after patch-file refresh. Existing graph remains untouched.

Final focused command (product scripts/with-local-tools, CodeGraph cwd):
node node_modules/vitest/vitest.mjs run
__tests__/constructor-object-uses.test.ts __tests__/function-parameter-uses.test.ts
__tests__/receiver-hazards.test.ts __tests__/constructor-join.test.ts
__tests__/construction-sites.test.ts __tests__/constructor-facts.test.ts
__tests__/ts-this-field-call.test.ts __tests__/call-receiver-no-fabrication.test.ts
__tests__/store-exported-later.test.ts __tests__/synthesized-edge-replacement.test.ts

**252 passed / 10 files**, exit 0 (19 constructor-object tests, 39 join tests).
tsc --noEmit, emitting tsc and git diff --check exit 0. No fresh full suite,
kernel/UI build or Rust foundation validation this turn.

Orders smoke command from product root: node scripts/orders-graph-smoke.mjs
/home/minh/projects/outsource/codegraph
.harness/baselines/orders-constructor-object-uses-20260912-01.json: exit 0.
[Raw receipt](../.harness/baselines/orders-constructor-object-uses-20260912-01.json).
Four mapped classes have constructor-export evidence only; 13 runtime-import
records are unmapped as classes and retained explicitly. This does not prove
there are no runtime prototype changes. Graph 51 nodes/127 edges, source/dist/
fixture stable, watcher observed with no errors, all five probes exit 0 (not
answer correctness). Fingerprint:
617f163c75d538c6e5eed876a0f19b9b22c5011626de54b09b3a0182726e7be4.

Next: class instance/static-this method effects, followed by the composed receiver
gate over constructor/instance/handoff evidence and synthesis activation via
scoped reconciliation. W4/full W0–W13 acceptance remains unproved.

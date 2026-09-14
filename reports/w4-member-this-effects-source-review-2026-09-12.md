# W4 member this effects

status: done (class-local member evidence only); source-gate: ready;
owner/reviewer: main (no independent review).

- [x] Reset gate; reread receiver-hazards common/field traversals and written
  predicate, receiver-hazards tests for arrow/static/nested functions, constructor
  join review collection and Orders Store methods.
- [x] Source revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf, dirty fingerprint
  617f163c75d538c6e5eed876a0f19b9b22c5011626de54b09b3a0182726e7be4.
- [x] Adopt existing parse/class-member traversal and write detection; augment
  the common scan instead of reparsing or introducing a second lexical engine.
  Track instance vs constructor this in methods/initializers/static blocks;
  arrows inherit, ordinary nested functions reset. Computed names/decorators
  and nested class evaluation remain explicit unmodeled hazards.
- [x] Plan member evidence: exact member identity, receiver role, property path,
  write/method-call/unmodeled-use/constructor-return, detached source spans.
  Join to exact class node without claiming runtime method stability.
- [x] Test actual grammars, nested boundaries, initializers, static prototype
  writes, return/alias escapes, write-vs-read patterns and real-index exposure.

This is class-local this-use evidence only. It neither proves method purity nor
closes external/global mutation, call completeness or synthesis activation.

## Delivered and verification

The existing common receiver scan now emits members with exact identity,
instance/constructor receiver roles, property paths, source-span effects and
unmodeled evaluation hazards. Arrows inherit this; ordinary nested functions
and object methods do not. Static blocks/field initializers retain their proper
receiver. Constructor return this is separate from an escaping method return.
Computed member names do not fabricate instance-this uses. The shared written
predicate now excludes destructuring-default RHS reads from write classification.

Constructor join exposes memberThisUses with exact class IDs when indexed,
retaining unmapped evidence otherwise. No graph writes or new parser/cache layer.

Final focused command via product scripts/with-local-tools, CodeGraph cwd:
node node_modules/vitest/vitest.mjs run
__tests__/member-this-effects.test.ts __tests__/constructor-object-uses.test.ts
__tests__/function-parameter-uses.test.ts __tests__/receiver-hazards.test.ts
__tests__/constructor-join.test.ts __tests__/construction-sites.test.ts
__tests__/constructor-facts.test.ts __tests__/ts-this-field-call.test.ts
__tests__/call-receiver-no-fabrication.test.ts __tests__/store-exported-later.test.ts
__tests__/synthesized-edge-replacement.test.ts

**272 pass / 11 files**, exit 0, including 19 new member tests and 40 join tests.
tsc --noEmit, emitting tsc and git diff --check exit 0. No fresh full suite,
kernel/UI build or Rust foundation run.

Orders smoke (product cwd): node scripts/orders-graph-smoke.mjs
/home/minh/projects/outsource/codegraph
.harness/baselines/orders-member-this-effects-20260912-01.json: exit 0.
[Raw receipt](../.harness/baselines/orders-member-this-effects-20260912-01.json).
Four classes map to member evidence. Constructor writes store/endpoint/db are
distinguished from method calls through store, query and db; endpoint remains
an unmodeled scalar read, not a fabricated class receiver. Source/dist/fixture
stable, watcher observed without errors, all five probe processes exit 0 (not
answer correctness). Graph remains 51 nodes/127 edges. Source fingerprint:
80d95899ac9e68dda17ad2a988c026ad01f31d235b3b0d5d05ad9e7afc829519.

Next: combine constructor-local reviews, member/prototype effects, allocation
uses and function handoffs into the receiver decision with explicit incomplete
cases. Then activate synthesis with affected-source discovery and scoped
replacement. Neither W4 nor the full W0–W13 objective is complete.

# W4 lexical function-parameter uses

status: done (lexical parameter evidence only); source-gate: ready;
owner/reviewer: main (no independent review).

- [x] Reset gate and reread construction-sites.ts traversal/origins/effect
  classification and its tests, constructor-join.ts collection seam, upstream
  store-accessor.ts selector/parameter binding traversal, and Orders http.mjs.
- [x] Source before patch: revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf,
  dirty fingerprint c728fe03d5d66317310d2a22df55f2cd1470b608c9251e9e80a0a8f7e8389317.
- [x] Adopt the existing lexical table and effect classifier. Orders factory
  parameters are captured by HTTP callbacks; a nested function can use an outer
  parameter without rebinding it. Reuse declaration-scope alias lookup, but keep
  parameter evidence distinct from construction origins (unknown runtime values).
- [x] Design: one parse returns construction sites plus per-function parameter
  use evidence; preserve legacy site-only wrapper. Plain parameter identities
  carry exact function/parameter extents, shape support, reassignment/ambiguous
  binding flags, and lexical effects. Dynamic evaluation and arguments use must
  be explicit hazards. Call handoffs also retain call offset/argument position.
- [x] Verify grammars, closure capture, aliases, shadows, parameter writes,
  unsupported/default/destructured parameters, arguments/eval hazards and join
  exposure. Probe unchanged Orders fixture after compilation.

This records function-body evidence, not interprocedural proof. Import-to-function
identity, call-target matching, alias-budget exhaustion, prototype replacement,
receiver validation and synthesis activation remain open. Do not convert an
empty lexical effect list into a stable/pure function claim.

## Delivered and verification

collectConstructionEvidence now returns sites and functions from one temporary
AST; collectConstructionSites remains a compatibility projection. Parameter
origins use the existing lexical bindings without becoming ConstructionOrigin
instances. Tracks const aliases, ordinary-function/arrow capture, shadows,
binding/property writes, ambiguous redeclarations, and unsupported shapes.
arguments use flags the nearest ordinary function (arrows inherit arguments);
dynamic eval/with flags all observed functions conservatively. Effects record
executionFunctionStart plus targetCallStart/parameterIndex for ordinary call
handoffs. Constructor join exposes per-file functionUses with source hashes;
no graph edges or persistent caches added.

The first focused run passed 124 tests. Strengthened destructuring reassignment
assertions to require an actual binding-write effect, added shorthand pattern
use collection and a real-index refresh test (factory property mutation appears
after edit without stale memo).

Final focused command through product scripts/with-local-tools, CodeGraph cwd:
node node_modules/vitest/vitest.mjs run
__tests__/function-parameter-uses.test.ts __tests__/receiver-hazards.test.ts
__tests__/constructor-join.test.ts __tests__/construction-sites.test.ts
__tests__/constructor-facts.test.ts __tests__/ts-this-field-call.test.ts
__tests__/call-receiver-no-fabrication.test.ts __tests__/store-exported-later.test.ts
__tests__/synthesized-edge-replacement.test.ts

**212 passed / 9 files**, exit 0 (32 new parameter tests, 29 join tests).
tsc --noEmit, emitting tsc and git diff --check: exit 0. No full suite rerun
after this change; the preceding 4797-pass full run predates these edits.

Orders smoke command from product root: node scripts/orders-graph-smoke.mjs
/home/minh/projects/outsource/codegraph
.harness/baselines/orders-function-parameters-20260912-01.json: exit 0.
[Raw receipt](../.harness/baselines/orders-function-parameters-20260912-01.json).
Orders factory effects identify service.create and relay.dispatch inside callback
at offset 393, and notification service.consume inside callback at offset 1032.
These parameters have no observed reassignments/ambiguous bindings or function
hazards; that is not proof of runtime receiver stability. Graph unchanged at
51 nodes/127 edges, watcher observed with no errors, all five probe processes
exit 0 (not answer correctness). Source/dist/fixture stable during probe.
Source fingerprint 630cb3ea591675fd6571d6e32841d681e0996e448153a89c34fefeb71ea4d60f.

Next: resolve actual call targets/imported function identity and connect handoffs
to these parameter records, while carrying incomplete/unknown evidence. Then
complete receiver validation and activate synthesis via scoped edge replacement.
Full W0/W4 acceptance remains unproved.

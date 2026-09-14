# W4 callback ownership before HTTP parameter dispatch

status: doing | owner: main | source-gate: ready

Current source: CodeGraph revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`,
dirty fingerprint `e0b9c4e9b3264d5875948aefbebdea68ae40c984562abbe00bf960a756f2020a`.
Previous goal turn made verified progress (watchdog correction and full suite).

## Source study before implementation

Read the Orders `http.mjs`, `main.mjs` and latest smoke node/callee records.
`ordersServer` and `notificationsServer` have no separate `createServer`
callback nodes; their callback's calls currently belong to the outer factory.
Thus parameter evidence alone cannot map the correct execution owner.

Read CodeGraph `constructor-join.ts`, `construction-sites.ts` function/parameter
evidence, `receiver-decision.ts`, `injected-call-mapping.ts` and
`injected-reconciliation.ts`, plus function-parameter/receiver/lifecycle tests.
These already track allocation → function parameter handoffs and the actual
executionFunctionStart. The mapper deliberately refuses to substitute a larger
outer function when that exact callback is not indexed; preserve this rule.

Read TS `extractFunction`, `visitForCallsAndStructure`, returned-callable naming
and receiver preservation; native counterparts in `tsjs/mod.rs` and
`tsjs/extractors.rs`; complete returned-callables tests. Both walkers deliberately
skip anonymous call arguments and attribute body references to the outer scope.
Read dead-code synthetic-returned exclusion and its UI label, extraction version
and host kernel build script. Skills: rust-router and coding-guidelines; apply
small named helpers, existing AST traversal and no new dependency/unsafe boundary.

## Adopt / avoid / verification plan

- Extend source value-position naming to anonymous direct call/new arguments,
  including transparent TS wrappers and conditional/logical value branches.
  Use `<callback@line:UTF16column>` labels distinct from returned values.
- Preserve explicit named/declarator/CommonJS/React handler bindings; do not
  relabel named functions or misclassify IIFE callees as callback arguments.
- Give callback bodies their own node/contains ownership in both walkers,
  retain direct `this` receiver text, and prevent export inheritance. Do not
  create a calls edge merely because a function value is passed as an argument.
- Exclude synthetic callbacks from identifier-count dead-code claims with a
  separate reason; do not call them returned functions.
- Bump extracted-content version, not ID format, schema, native ABI or package.
- Validate JS/JSX/TS/TSX, Unicode/CRLF, siblings/nested callbacks, lexical this,
  WASM/native parity and index/sync lifecycle. Then inspect broader regressions;
  framework invocation and receiver mapping must use actual evidence, not restore
  falsely eager factory calls to make old tests green.

This is the execution-ownership prerequisite discovered on the HTTP dispatch
path, not a redefinition of the goal. Function-parameter mapping, call/return
propagation, framework registration/invocation and complete Orders diagrams
remain required. Do not claim end-to-end flow acceptance from node extraction.

Follow-up source gate before resolver edits: `resolveOne`,
`resolveThisMemberFnRef`, `resolveDeferredThisMemberRefs` currently select exact
lexical-this logic only for returned labels; `synth-utils.ts:enclosingFn` has
the same returned-only boundary guard. Apply those existing guards to both
synthetic callable kinds so a new ordinary callback cannot inherit a class by
qualified-name truncation, and line-only queries cannot pick a same-line child
arbitrarily. Keep separate dead-code exclusion counts and add real index/sync
tests for injected-field callbacks, ordinary-this reset and callback exclusion.

- [x] Source study and decision before code.
- [ ] Callback ownership implementation and direct/native tests.
- [ ] Lifecycle and broader compatibility verification.
- [ ] HTTP parameter dispatch and end-to-end acceptance (still required).

## Initial executed verification

- Emitted TypeScript compiler and assets copy: exit 0.
- Host native kernel release build/stage: exit 0; existing unused-mut warnings
  in Go/Rust-language modules and upstream Scala C warnings remain. No unrelated
  formatting/dependency/package changes; native ABI and node ID format unchanged.
- `callback-ownership-20260912-01.json`: 59 pass / 0 fail, four files.
- `callback-ownership-20260912-02.json`: 178 pass / 0 fail, seven files,
  `CODEGRAPH_KERNEL_EXPECT=1`. Includes 56 callback extraction/parity cases,
  three callback lifecycle cases and existing event/tier/dead-code/UI suites.
- Bernoulli wrote only the extraction test matrix and its source-study receipt;
  main read both and executed the tests. Agent completed and closed.
- `orders-callback-ownership-20260912-01.json`: smoke exit 0, 60 nodes /
  140 edges / 7 files (previous 51/131), five probes exit 0 and watch observed.
  Inspect semantic callback owners and remaining diagram output separately;
  `product_acceptance` remains false. Source did not change during smoke.

Full-suite follow-up is required before claiming broad compatibility. Do not
promote the previous 5160-pass suite (before callback extraction) to this patch.

Orders semantic inspection confirms `ordersServer`/`notificationsServer` now
have no callback-body callees; their exact callback nodes own create/dispatch/
consume/body calls. Existing name-based receiver edges are not newly validated
parameter-dispatch facts. Injected receiver status remains incomplete (4 edges,
1 issue). Diagram still returns `No relevant code found`; source/dist/original
fixture unchanged during the smoke. The extra nine nodes/contains edges do not
by themselves establish registration/invocation or complete architecture.

Full-suite launch fingerprint:
`17508f124552397c7c543ca1c8db71f6a1cd2e9bee1edc9565ef11f62764b5a0`,
receipt target `codegraph-callback-ownership-20260912-01.json`; running, unclaimed.

## Full-suite findings and default-parameter source gate

That suite completed **5202 pass / 17 fail / 9 skip**, exit 1, fingerprint
unchanged. Failures span context allocation/displacement/factory closure,
router/screen flows and assertions that still expect callback calls on outer
functions. Preserve all receipts and quality gates. Chandrasekhar owns tools.ts
retrieval changes; Raman owns the scoped flow/router compatibility work; main
owns AST/defaults and their tests. No overlapping write sets.

Independent source review found missing parameter-default calls after taking
ownership of anonymous callback bodies, and a callable on the left of `&&`
incorrectly labeled as a passed value. Re-read TS/native extractFunction and
extractMethod, body walkers and the shared value-position helper before edits.
Adopt initializer-only traversal under the executing callable (not all type
syntax), including destructuring defaults/computed keys; preserve nested
callbacks/constructors. Functions supplied as parameter defaults get distinct
`<default@line:UTF16column>` deferred scopes rather than attributing their bodies
to the containing function. Apply exact-this and dead-code safeguards to that
kind too. No eager invocation is inferred from a default value. Reject the
left operand of `&&` on the function value path in both returned/callback/default
naming: if that operand evaluates to this function object, it is truthy and the
operator yields its right operand. Add regressions before changing implementation.

The m06-error-handling skill was also read completely: absent AST fields are
normal optional data, so native traversal remains Option-based, without panic
or unchecked unwrap for unsupported shapes.

Resume source gate (before default implementation): re-read the current
value-name helpers, both extractFunction/extractMethod implementations, native
body walker, default regression assertions and dead-code guards. HEAD unchanged
at 3ed73bc127323e63153bf6ec8354afa82ce36aaf. Pre-edit SHA256: returned-callable.ts
cb3c2e709808a487ca51ca37185fc025967e6fc05a984e96ea9cda0fdd77ca9c;
tree-sitter.ts 21958a57d87f3cb3527e3bc7075bc0312dfb4e6d421088bbd2f0ea9fb832d995;
tsjs/mod.rs 2012ceb4e3bf06c1983fe9fa2a8de2a39d27a7622d259d6bd4404099d38bf8c2;
tsjs/extractors.rs 5ffb32b9c6e82c72a9faec57e5e91d33de6376f6fe3ef21e3b70a4fecaa868de.
The red receipt callback-defaults-red-20260912-01.json has 0 pass / 18 fail.
Adopt the initializer-only/deferred-scope plan above. Do not traverse parameter
type annotations as runtime expressions. Native optional child fields remain
normal absence, not a panic. A Luna worker now owns the separate React Router
captured-local fix; its write scope does not overlap extraction or retrieval.

Default implementation verification: native release build/stage exit 0 (59.20s),
TypeScript noEmit exit 0, scoped diff check exit 0. Required-native focused run
callback-defaults-20260912-01.json: 128 pass / 0 fail / 0 skip, six files, runner
exit 0. Includes all 18 previously-red parameter/default/left-&& cases, 56
callback extraction cases, returned-callable/this, four lifecycle tests and
dead-code regressions. Added a real index test for default arrow vs ordinary
function this ownership and separate syntheticDefault exclusion accounting.
These are focused results, not full-suite or HTTP end-to-end acceptance.

Additional source gate before class-field parameter support: reread
languages/typescript.ts and javascript.ts resolveBody/classifyTsClassMember,
native resolve_field_body/body_of and both extractMethod implementations.
The existing field-as-method path unwraps a direct function value or the first
callable HOF argument to its body; field AST itself has no parameters. Adopt
that already-selected body's callable parent for initializer traversal, retaining
existing method identity and wrapper selection (not a new inference that a HOF
invokes its argument). Add direct field and wrapped field default cases before
the change. Do not recurse through arbitrary field children/type declarations.

Executed follow-up:
- callback-defaults-20260912-02.json: 24 pass / 0 fail (adds Unicode/CRLF,
  wrapped defaults, generators and exclusion of type syntax from runtime calls).
- callback-field-defaults-red-20260912-01.json: 24 pass / 8 fail, before the
  class-field initializer fix; failures are the new JS/JSX/TS/TSX direct/wrapped
  field cases.
- Native rebuild/stage exit 0 (1m04s), TypeScript noEmit exit 0; field-default
  focused receipt callback-field-defaults-20260912-01.json: 51 pass / 0 fail /
  0 skip in three files, runner exit 0. Scoped diff check exit 0.
- callback-parity-router-20260912-01.json: 733 pass / 1 fail in five files;
  only failure is the delegated React Router test's out-of-scope `hrefs` helper.
  Kernel parity, deep nesting, extraction and function-ref assertions passed.
  Parent review additionally rejected widening a line scan across lexical
  siblings/parameter shadows; Luna is correcting with AST-bound lookup and
  additional negative controls. This receipt predates the field-default fix.
- callback-compatibility-20260912-01.json: 166 pass / 5 fail in ten files.
  Three CG21 whole-source funding assertions, one CG27 closure delivery and
  one Steps callback-trigger assertion remain. CG31 displacement and the other
  delegated flow checks pass; Luna follow-ups own retrieval and Steps separately.

Next HTTP dispatch navigation (not an implementation source-gate for later
resumes): reread fixture http.mjs/main.mjs, construction-sites.ts parameter
effects and executionFunctionStart, constructor-join.ts callHandoffs, its
captured-parameter test and receiver-decision.ts worklist. Existing evidence
already maps imported call arguments to exact parameter definitions and records
the nested executing callable separately. Reuse those identities/validation
boundaries for the missing parameter-call producer; do not create a duplicate
name-only receiver resolver or infer invocation from lexical containment.
Reopen actual source/tests before implementing that next mechanism. A producer
still needs allocation/class validation, mutation/escape uncertainty, snapshot
binding and lifecycle retraction; current injected-field mapping does not prove
general HTTP parameter dispatch.

Before adding the handoff/owner contract test: current constructor-join.ts
functionUses/callHandoffs paths, construction-sites.ts effect stamping and
constructor-join.test.ts captured-factory fixture were read together. The
collector records a captured parameter on its declaring function but stamps
executionFunctionStart from the innermost callable. Adopt those two separate
identities and verify they match the new graph node spans (including Unicode
and CRLF); avoid mistaking parameter declaration ownership for execution owner.
This is a prerequisite contract test only, not a new parameter-call producer.

Handoff contract execution: callback-parameter-handoff-20260912-01.json,
42 pass / 0 fail, runner exit 0. Includes the two new exact executing-callback
span assertions over plain and Unicode/CRLF source. Existing imported function
identity, mutation and captured-parameter refresh tests remain intact.

Independent Luna default review received and read in full; worker closed.
Parent reread the current initializer visitor and anonymous fallback in the
body walker. P1 is actionable: `function f(x=(()=>hidden())&&fallback){}` and
object-valued/default arithmetic variants traverse an anonymous body under f.
Add JS/JSX/TS/TSX regression tests before choosing the ownership repair. The
test must retain the nested callable's body reference under its exact source
scope, not merely remove it to make a negative outer-owner assertion pass.
Keep returned/callback/default *value-position* gates conservative: a distinct
execution-scope label would not itself claim the function is the default value
or is invoked. Implementation decision/source gate must be completed before
that new mechanism is patched. HOC parameter initializers and ArkTS coverage
remain separate open gaps; do not mark the whole ownership item complete.

Executed callback-discarded-defaults-red-20260912-01.json: 32 pass / 12 fail,
runner exit 1. All three fixtures across four languages fail because hidden's
fromNodeId equals f's node ID. This confirms the review finding at runtime;
the newly added guards intentionally remain red pending the ownership repair.

Resume implementation source gate: reread current returned-callable.ts,
initializer/body walkers, extractFunction fallback and native equivalents,
plus the 12 red assertions. The four source SHA256 values match the independent
review table (e6928b39… / 55614faca… / 57933382… / 3304dce8…). Adopt existing
location-based node creation and exact-this/dead-code guards, extending with
`<initializer@line:column>` solely for an otherwise anonymous function contained
in a runtime parameter initializer. Unlike `<default@…>`, this label makes no
claim that the function is the initializer's resulting value. Stop ancestry
at another callable/class/type/parameter boundary; confirm the initializer RHS
or computed pattern key before naming. Retain references under their exact
callable rather than dropping uninvoked bodies. Keep all original value-position
gates and do not add invocation edges. Mirror in native and add lifecycle tests.

Also reread TS/native extractReactComponentNode: adopt the same parameter-only
visitor under the existing component owner before its body. Add HOC tests
before that change; do not broaden wrapper selection or ArkTS in this patch.
rust-router and m06-error-handling are active; missing AST fields remain normal
optional absence rather than unchecked native unwrap/panic.

Implemented initializer execution-scope labels and HOC initializer traversal.
HOC red receipt callback-hoc-defaults-red-20260912-01.json: 0 pass / 8 fail /
44 filtered skip before HOC change. Native release build/stage exit0 (58.90s).
Required-native callback-initializer-ownership-20260912-01.json: 136 pass /
0 fail / 0 skip, four files, runner exit0. All 12 previously-red discarded/
object/arithmetic initializer cases now keep hidden's call under its own
callable, not f. New real-index lifecycle verifies arrow/ordinary this ownership,
syntheticInitializer dead-code exclusion, shifted Unicode/CRLF IDs and removal.
TypeScript noEmit exit0 after final Steps worker handoff. Broad/full suite
still required; ArkTS and broader generic anonymous ownership remain unclaimed.

Final focused integration: retrieval CG21/27/31 45 pass / 0 fail; Steps/branch
guards/router/native events 196 pass / 0 fail / 0 skip in seven files. Full npm
build (compiler, SQL/WASM copy, viewer build/check) exit0; final compiler emit
after Steps correction exit0; global codegraph diff whitespace check exit0.
All delegated workers are closed. Current frozen full-suite source fingerprint:
c76ebba2275838943cbcf68a6aaa7ee26f26d707e8f866067561353505d985e1.
Receipt target codegraph-callback-initializer-20260912-01.json; required-native
runner started, completion/result not yet claimed. No source changes during
this verification window; product reports/PLAN updates are outside that tree.

Full-suite terminal receipt (resumed exact session 96386): runner exit 1,
5292 passed / 3 failed / 9 pending, 5304 tests in 296 files. Post-run source
fingerprint equals the launch c76ebba2275838943cbcf68a6aaa7ee26f26d707e8f866067561353505d985e1.
Failures: explore-oversize-member quarterly file not admitted (two assertions),
and explore-reservation-invariant precise pipeline sink skipped budget-clusters.
These are remaining retrieval admission/budget regressions; current full-suite
gate remains open. A new explicit Luna worker owns tools.ts follow-up, with
fresh source-study required before patching and original test limits preserved.

Parent retrieval acceptance source gate: reread current owedPayableBelow,
fundedHeadroom and final cluster/window trimming; reread CG26 reservation gates
and the complete displacement sink fixture. tools.ts SHA256 at review:
d0f1104082c05c5923c01260cd278d65e7a900048394dd865d3bbee784b7590d;
reservation test eaf24c190d1caae4ad9f406344e09ff60f5f78a8a884243410c72800430a890e;
sink fixture 26b7cf8874651db6fbe4c3aabe1d5d985b31fc30f61905b4c1c4db5197070b27.
Independent owned-mkdtemp source-loaded probe ended exit0: precise query gives
sink funded150/emitted54, while its writeBatch signature alone exceeds54.
Thus the old nonempty assertion is insufficient for useful graph context.
Adopt existing fixture/probe and retain all original ceilings/assertions; add
a complete writeBatch signature/body delivery assertion. Avoid accepting a
header/import-only slice or weakening window floors to obtain nonempty output.
Parent owns this test only; Luna retains tools.ts implementation ownership.

New assertion execution: reservation-callable-body-parent-20260912-01.json
exited1 but all assertions skipped; it is NOT an assertion-red receipt.
Rerun with both default and JSON reporters, -02.json: 13 pass / 2 fail,
runner exit1. The complete writeBatch body assertion fails and original tail
enrich delivery also fails on the in-flight retrieval source. Preserve both
receipts; this is not final frozen-source validation.

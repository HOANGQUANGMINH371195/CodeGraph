# W4 returned callable extraction

Status: returned-callable component verified; W4 end-to-end acceptance incomplete.
Baseline CodeGraph HEAD 3ed73bc127323e63153bf6ec8354afa82ce36aaf; dirty
fingerprint 7de3af97f67a34efc05e7e6659458980d370718fde4f0f2eceb01c1d72ea3595.
Skills: rust-router and m11-ecosystem; use existing parser/native integration,
no new dependency, ABI change, account change or package release.

## Source read before coding

- tree-sitter.ts extractFunction (1591 onward): named/declarator/CommonJS
  functions create nodes, push own scope, visit body then pop. Anonymous fallback
  visits the body without a new owner. visitFunctionBody (5702 onward) separately
  handles nested named functions, React hooks and declarator-bound functions.
- native tsjs/extractors.rs extract_function and tsjs/mod.rs
  visit_for_calls_and_structure mirror both paths. create_node stores exact
  extent and UTF-16 column, emits contains from the scope stack.
- languages/typescript.ts isExported walks every ancestor, so a synthetic
  returned closure MUST explicitly be non-exported rather than inheriting
  export from its factory. Synthetic location name is not a source binding.
- nested-declarator-functions.test.ts pins own-node/own-call/contains behavior
  while excluding unrelated inline callbacks and Python lambdas. Adopt this
  scope pattern, not an invented invocation edge.
- injected-call-mapping.test.ts and injected-call-mapping.ts require the exact
  innermost callable extent; the returned-arrow fixture currently remains
  caller-unmapped. Preserve the strict mapper and fix its missing source node.
- extraction-version.ts distinguishes stale extracted content from DB schema.
  This extends content, not identity format; explicit rebuild remains needed
  for unchanged source. Do not silently delete user indexes.

## Decisions / gaps before patch

Add JS-family syntax detection for anonymous arrow/function/generator values
in return position, including expression-bodied arrows and transparent value
wrappers/conditional branches. Stop at calls, object members and unrelated
expression positions; callback arguments are not returned callables merely
because their enclosing call is returned. Existing named bindings take priority.
Use `<returned@line:UTF16-column>` as a synthetic location label, exact callable
extent, own scope and contains edge. No outer-to-closure calls edge and no
claim that constructing/returning a closure invokes it. Force non-exported.
Mirror TS/native behavior; avoid changing unrelated languages or all anonymous
callback ownership in this task. Those remain separate coverage work, not done.

Checks: first fail new return-value extraction regression on the baseline;
test plain/nested/conditional/parenthesized/TS wrapped/async/generator forms,
Unicode+CRLF positions, named-binding preservation, no IIFE/callback false
return label, native parity, injected mapping and real index/sync removal.
Then build, focused regression and full suite/Orders before broad acceptance.
Reviewer Aquinas is examining downstream callback/ref risks read-only.

- [x] Current source/tests read; adoption and limits recorded before code.
- [x] Implement and verify TS/native callable ownership in focused tests.
- [x] Verify mapping and sync lifecycle in owned real SQLite fixtures.
- [x] Full suite/Orders diagnostic, review and synchronized PLAN evidence.
  This does not mark successful Orders diagrams or W4 product acceptance.

## Review-driven follow-up source gate (before follow-up patches)

Aquinas read the callback, function-ref and dead-code consumers. Main reread
resolveThisMemberFnRef / resolveDeferredThisMemberRefs: both strip one qualified
segment, invalid for nested returned arrows. Main also read constructor-facts
parser ownership (parse/delete in finally, exact spans), ResolutionContext,
and synth-utils enclosingFn + eventEmitterEdges: line-only attribution can move
an earlier same-line emit into the new closure. Follow-up must use source AST
for lexical-this class identity, reject ordinary function boundaries, and use
columns for event emit attribution. Other line-only consumers need explicit
unknown on ambiguous returned-closure lines rather than selecting a wrong node.
Dead-code name corroboration cannot establish liveness for synthetic labels;
Aquinas owns its exclusion/test changes. These are integration correctness,
not evidence that factory-return value propagation or invocation is implemented.

Baseline red receipt: returned-callables-red-20260912-01.json, four failures
at the missing closure-count assertion. After TS patch that assertion passes;
the first follow-up run fails only against the not-yet-rebuilt native binary.

Main's lexical-this follow-up reread found same-named static/instance members
would still use first-candidate selection. Before patching that path, carry the
member's AST static flag with exact class evidence and filter own/inherited
targets accordingly. A returned ordinary function remains receiver-unknown.
Add both direct and inherited static/instance collision assertions.

Main then read extractCall / native extract_call and the resolveOne function-ref
gate: direct JS `this.method()` loses its receiver and becomes a bare name.
For a newly owned returned closure that would bypass the lexical-this guard.
Before patching, decision: retain `this.` for direct calls from synthetic
returned callable scopes in both extractors and route those calls exclusively
through the same class-anchored own/inherited resolver. No fallback to unrelated
same-name methods, and an ordinary returned function keeps unknown receiver.
Test both calls and function-as-value refs, including static/instance decoys.

Reviewer additionally identified block-local same-name classes sharing qualified
names. Main reread the own-member query: restrict returned-closure candidates by
the AST-selected class's actual contains edges, not only its qualified name.
Inherited lookup is already rooted at that exact class node. Add two block-local
classes with identical qualified labels as a regression before acceptance.

## Implemented and focused verification

- TS/native location-labelled returned arrow/function/generator nodes; exact
  extent, non-exported, own calls/refs and containment only. Existing named
  bindings, inline callback and IIFE behavior retained. Transparent wrappers,
  conditional branches, nested arrows, Unicode/CRLF and receiver-reference parity.
- AST lexical-this evidence, per-context bounded source cache cleared at resolver
  cache invalidation; ordinary functions reset receiver, exact class contains
  membership and static/instance filter, exact-root inherited traversal.
  Direct this.member calls from these new scopes retain their receiver in both
  extractors and use the same exclusive resolution path as function refs.
- Aquinas patched dead-code exclusions and its UI label/test after source review;
  23 tests and noEmit check passed. Main read the patch and later reran it in the
  88-test integration. Reviewer identified export, exact-class, liveness and
  downstream-position hazards; no reviewer completion implies W4 completion.
- Epicurus patched enclosingFn column support plus all 13 callback-synthesizer
  call sites; real offsets only. Masking tests preserve UTF-16/newlines in eight
  language modes. Reported 28 then 209 focused tests and noEmit pass; main read
  the diff/assertions. Line-only returned-boundary callers decline ambiguity.
- Schrodinger threaded actual offsets through tier handler/sites and five link
  synthesizers; main read changes and real-route/Next-link regression assertions.
  Reported 210 tests/8 files pass. All three agents closed; no child jobs live.

Receipt files under `.harness/baselines`:

- returned-callables-focused-20260912-01.json: 44 pass / 5 files.
- returned-callable-this-20260912-01.json: 21 pass (before static/direct follow-up).
- returned-callable-this-20260912-02.json: static follow-up passed.
- returned-callable-extraction-regression-20260912-01.json: 691 pass / 4 files.
- returned-callable-lifecycle-20260912-01.json: 31 pass / 1 failure; built CLI still
  extraction version 27 after source moved to 28, so status correctly stale.
- returned-callable-lifecycle-20260912-02.json: rebuilt CLI, 32 pass / 3 files.
- returned-callable-integrated-20260912-01.json: final 88 pass / 7 files, including
  direct calls, exact block-local classes, mapping/retraction, event/tier and
  dead-code behavior. No missing-node TODO remains in injected-call-mapping.

Final emitted tsc exit 0; native release/stage exit 0 (1m00s, preexisting scanner
and unused_mut warnings). No new dependency/schema/ABI/ID format/package version.
Content extraction version is 28; old unchanged projects require explicit rebuild.
`git diff --check` passed. Full suite started with mandatory native and default
workspace config; no concurrent build/probe, CodeGraph source frozen:
`2e381dad998b149f929bbd7aadc41108535cd626bc4353b3a6ab10311f8630e9`.
Output: codegraph-returned-callables-20260912-01.json. Result still pending here.

Remaining beyond this component: factory-result value propagation/invocation,
general unnamed callbacks, typed this-field inference across new closure scopes,
HTTP parameter dispatch, and complete Orders flows/architecture rendering. Do
not call returned-callable node coverage whole-program flow understanding.

## Final verification

Full suite `codegraph-returned-callables-20260912-01.json` terminal exit 0:
**5055 passed, 0 failed, 9 skipped, 0 TODO / 5064 tests across 289 files**.
Mandatory native enabled. Post-run source fingerprint exactly matches
`2e381dad998b149f929bbd7aadc41108535cd626bc4353b3a6ab10311f8630e9`.
The existing warm 500-caller API `elapsed < 100` assertion passed unchanged.
Zero test TODOs does not imply zero product requirements remaining.

`orders-returned-callables-20260912-01.json` terminal exit 0: 51 nodes / 131
edges / 7 files, four injected edges and one incomplete receiver issue; watcher
observed with no errors. All five probe processes exit 0, but the diagram probe
still says `No relevant code found`. Source, dist and original fixture unchanged.
The diagnostic explicitly leaves `product_acceptance: false`. No accounts,
real user indexes or deployment state changed; temporary test copies only.

All build/test handles and all three agents terminal/closed. Next work must
reopen the source gate, prioritizing the remaining closure receiver/value-flow
gaps and HTTP parameter dispatch instead of treating these green tests as a
complete architecture graph.

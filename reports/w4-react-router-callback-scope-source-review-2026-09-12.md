# W4 React Router callback-scope source review — 2026-09-12

Pre-code source receipt. Worker scope: `codegraph/src/resolution/frameworks/react-router.ts`,
`codegraph/__tests__/react-router.test.ts`, and this report only. No tests/builds,
recursive agents, configuration/authentication changes, commits, or writes to
parent-owned files. Existing dirty edits are preserved.

## Source-study

Read `/home/minh/projects/project-graph-agent/AGENTS.md`, PLAN §0.4.1,
`codegraph/AGENTS.md`, and `codegraph/docs/design/framework-coverage.md`
completely before implementation. Read the prior callback-flow receipt only as
navigation; current source and tests below were reread.

Current CodeGraph revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`.
The tree is dirty with unrelated and parent-owned work, including an existing
React Router test edit. Pre-edit SHA256 fingerprints:

- `src/resolution/frameworks/react-router.ts`
  `9595b3371634fa92f5ceac13827740b47758312e4d32bda1a4d033726aa7656f`
- `__tests__/react-router.test.ts`
  `0dff45b93b06a4641c718802a9b255cd65785b1018eb981029195c06c3a44aa9`

Relevant current flow:

- `react-router.ts:166-200`, `reactRouterResolver.resolve`, claims `calls`
  references, reads the call argument, falls back to `readHrefViaLocal`, then
  emits a `navigates` edge whose source remains `ref.fromNodeId`.
- `react-router.ts:176-183` currently derives the fallback scan boundary from
  `context.getNodeById(ref.fromNodeId).startLine`. For the captured
  `history.push(redirect)` in the test, the callback begins on line 5 while
  `const redirect = ...` is on line 4, so the declaration is skipped.
- `expo-router.ts:459-491`, `readHrefViaLocal`, reads the nearest preceding
  `const`/`let`/`var` initializer and parses it through `parseHrefExpression`;
  an intervening assignment returns `null`. This is the reusable mechanism to
  retain. Its caller-supplied boundary is the missing enclosing-scope piece.
- `react-router.test.ts:383-398` defines the captured local in the enclosing
  `LoginScreen` component; `:456-466` already contains the dirty follow-up
  assertions that the callback owns `/` and the component does not. Its
  `:478-488` screen assertion now also requires the explicit login→home link.
- `react-router.test.ts:293-297` retains the computed-destination and array
  `push` precision controls; `:259-272`, `:299-330` retain callback/source,
  destination, metadata, Screens, and Steps assertions.

## Initial decision — superseded by parent review below

Adopt the shared `readHrefViaLocal` and `parseHrefExpression` path, and derive
only a lexical enclosing callable start from current same-file node ranges.
Use the nearest callable that strictly contains the callback node, so the
callback remains the edge source while its enclosing component-local binding is
visible. Keep the existing nearest-declaration and reassignment rejection
unchanged.

Avoid changing shared reader code, callback extraction/ownership, route-table
matching, markup synthesis, Screens/Steps APIs, or parent-owned extraction and
native files. Avoid a component-owned duplicate navigation edge and avoid
falling back to a computed value or an unrelated file-wide declaration.

Targeted tests will cover: (1) a declaration one line outside the callback still
resolves to the callback and projects login→home; (2) a nearer lexical shadow
continues to win; and (3) an intervening reassignment remains unresolved. No
test/build command will be run here; parent schedules validation.

source-gate: ready
receipt: revision/fingerprints above; source-read commands completed successfully; tests/builds not run by instruction
review: pending parent review | blocker: none known | next: apply the bounded resolver/test patch, then hand off without validation execution

## Initial patch — rejected by parent review

Implemented the bounded follow-up:

- `react-router.ts:143-164, 203-207` now finds the nearest strictly enclosing
  same-file callable by node range and passes that callable's start to the
  existing local reader. The callback remains `ref.fromNodeId`, so no outer
  navigation owner is synthesized.
- `react-router.test.ts:394-403, 478-489` adds a callback-local shadow control
  and an intervening-reassignment negative control. The existing captured-local
  callback→`/` assertion and explicit Screens login→home assertion remain.

Post-edit SHA256 fingerprints:

- `src/resolution/frameworks/react-router.ts`
  `d009ba3dd28ed68e8fe9681affc6dee89d88fba2fcf2e958cb7939f1cd3ba2c2`
- `__tests__/react-router.test.ts`
  `76131420a090f150676b1be730e1c05c733b03a280713f8fdc7c65939679873b`

`git diff --check` passed. Tests and builds were not run per the parent
schedule/instruction. Additional needed scope: none identified for this
bounded regression; parent-owned extraction/native/default-test work remains
outside this patch and must be validated by the parent.

## Corrective source gate (before corrective code)

Parent reported session 33089 terminal exit 1; worker runs no suites/builds.
The initial precision claim was unsupported: line-range widening admits sibling
locals and parameter shadows, and the added test calls `hrefs` outside its
describe scope. Diff whitespace checks did not establish correctness.
Current revision remains 3ed73bc127323e63153bf6ec8354afa82ce36aaf; corrective
pre-edit fingerprints are the post-edit fingerprints recorded above, reread now.

Fresh source review: `store-accessor.ts:19-35, 115-180, 211-228` provides AST
binding-pattern collection, parent-linked block/function scopes, var hoisting,
parameter blockers, and nearest-binding lookup. Its helpers are private and
store-specific, so adapt the mechanism locally, not its store semantics.
`receiver-hazards.ts:59-73` supplies AST assignment/update target recognition;
`grammars.ts:454-469` supplies the existing synchronous parser. Missing parser
or syntax errors must leave the local unresolved; delete each temporary tree.
Read current object-literal-methods tests for captured, parameter, mutable and
hoisted shadows. `expo-router.ts:347-381` remains the initializer href parser.

Adopt exact call row/column and AST ancestry; inspect only the enclosing outer
callable subtree with a node budget, never file-wide textual binding lookup.
Record parameters and unknown declarations as blockers, select nearest lexical
binding, reject duplicate bindings, use-before-initialization and writes anywhere
in the enclosing subtree (including deferred writes). Siblings have independent
binding maps. Keep original callback reference identity and screen assertions.
Replace the broken helper assertion with exact navigation target/metadata checks.
Add sibling, parameter, block, same-line and reassignment controls plus captured
positive. No shared source writes required. Corrective source-gate: ready.

## Corrective patch handoff

Replaced `enclosingCallableStart` and the React Router line-reader fallback with
`capturedHref`: exact AST call position, parent-linked scopes, binding blockers,
duplicate/initialization checks and binding-specific write rejection. The parser
reads the source, but binding analysis visits only the outer enclosing callable
subtree, capped at 20,000 nodes. Module bindings are deliberately unresolved.
Temporary trees are deleted in finally; syntax errors, missing parser, nested
classes and dynamic scope return unresolved. Shared href parsing and the original
reference source ID remain unchanged. Assignment/update/delete member targets
also invalidate their root binding (following the reviewed receiver-hazard
mechanism), with an object-property-write negative control.

Fixed the out-of-scope `hrefs` call using local `navs` with exact source, target
and metadata assertions and explicit callback existence checks. Added 14
parameterized controls: captured positive, parameter, destructured parameter,
multiline sibling, same-line sibling, sibling decoy with correct outer binding,
computed block shadow, closed block, late lexical shadow, hoisted var shadow,
reassignment, deferred write, compound write and object property write. These
select callback kind and exact line/column, assert the expected callback name,
and reject a component/outer-function-owned navigation. Existing callback→home
and explicit login→home Screens assertions remain present.

Corrective fingerprints:

- react-router.ts: `1ae0a89b57e28442156cb6c2f2d90fda2d3b1b8c07d1b741ec40d36631c68ccd`
- react-router.test.ts: `385b000b786958816ab3f3e406996cdbded960bc75b0f57c5dc361178e808bb1`

Scoped `git diff --check` passed (exit 0). No tests, suites or builds run by
worker. Parent runtime/type-check review remains required; this receipt does
not claim passing tests or full lexical/dataflow coverage. No shared-source
write was needed. Known limits: module bindings, alias propagation and external
mutation are not modeled; parsing occurs per local navigation lookup, without
a persistent AST cache. Wider coverage or cache integration is a separate task.

## Object assignment pattern follow-up — pre-code gate

Parent reports session 72655 terminal exit 0 for the preceding patch; this is
parent-reported evidence, not worker validation of the upcoming change.
Reread current `construction-sites.ts:82-99` patternNames, React Router
bindingNames and its write collector, and the parameterized test/position helper.
HEAD remains 3ed73bc127323e63153bf6ec8354afa82ce36aaf. Pre-edit fingerprints
match the corrective fingerprints immediately above.

Adopt construction-sites' `object_assignment_pattern` → `left` handling in
bindingNames. Both parameter registration and destructuring write collection
use this helper, so the same guard fixes both missing-name paths. Avoid reading
default RHS identifiers as bindings. Add a defaulted-object-parameter negative
and a defaulted-destructuring-assignment negative. Locate fixture callback start
from its explicit useEffect argument boundary so runtime() in parameters cannot
shift the expected position. Preserve existing positive and Screens assertions.
No scope/cache expansion. Source-gate: ready before source/test edits.

Implemented the one-line binding guard, two negative controls, and precise
fixture callback-start correction described above. Existing captured-positive,
callback ownership and Screens assertions are retained. Scoped `git diff
--check` passed (exit 0); no worker suites/builds run. The two new regressions
remain pending parent execution. No additional write scope needed.

Post-edit SHA256:

- react-router.ts: `8823caed2442d990b7e5e76019987121d7ca5bf780a81b6a94237dd46f5d58fe`
- react-router.test.ts: `8aa7c27f5a83d9dc26d6539bae0d236d054c812f0c037f77c4350bc0c832fac7`

## Parent executed validation

Parent read the corrective source and negative fixtures before running them.
react-router-callback-scope-20260912-01.json: 44 pass / 0 fail, runner exit 0.
Parent then identified the missing object_assignment_pattern binding shape;
the minimal follow-up above was implemented and reviewed. Final focused run
react-router-callback-scope-20260912-02.json: 46 pass / 0 fail / 0 skip,
runner exit 0. Both runs require the staged native kernel. They do not prove
full-suite compatibility, full lexical coverage or Orders diagram acceptance.

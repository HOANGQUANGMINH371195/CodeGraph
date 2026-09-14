# W4 Steps callback trigger source review — 2026-09-12

Pre-code source receipt. Scope: the concrete Steps callback-trigger regression,
with parent-authorized shared trigger derivation. Allowed implementation files
are `src/ui-server/api/steps.ts`, `src/ui-server/api/when.ts`,
`src/graph/branch-guards.ts`, the directly relevant trigger test
`__tests__/branch-guards.test.ts`, `__tests__/ui-steps-api.test.ts`, and this
receipt. No tests/builds, recursive agents, configuration/auth changes,
commits, or writes outside that scope.

## Source-study

Read product `AGENTS.md`, product `PLAN.md` §0.4.1, codegraph `AGENTS.md`,
codegraph `docs/AGENTS.md`, and `docs/design/framework-coverage.md` before
WHEN logic. Read the current Steps/WHEN source and the relevant test fixture,
plus the prior callback-flow receipt before patching.

Product/source revision and dirty-worktree fingerprint:

- codegraph HEAD: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`
- current `steps.ts` SHA-256:
  `95ee6719e861b682323ae1d9f7a7c3ce7f92ae503dde1577c3d2a0542450c2bb`
- current `when.ts` SHA-256:
  `393ab9dec67d01e60cb9e340376109ddc659aa20b075c926a6e9977131410bba`
- current `ui-steps-api.test.ts` SHA-256:
  `25559d33034bfb52db1847d38d0d87119d79b8bc966538b309022289a7e250b2`
- baseline evidence read:
  `.harness/baselines/callback-compatibility-20260912-01.json` (166 passed,
  5 failed; the relevant failure is `ui-steps-api.test.ts:239`)

Relevant current source/test flow:

- `src/graph/branch-guards.ts:798-888`, `triggersForFile` /
  `triggerInTree`: a site climbs to `arguments` and labels any argument of a
  later call (`useEffect`, `addListener`, etc.) as a callback. It currently
  does not distinguish `useEffect`'s callback argument from its dependency
  array. Existing direct trigger tests are in
  `__tests__/branch-guards.test.ts:450-500`; the focused shared regression
  cases are now at `__tests__/branch-guards.test.ts:503-538`.
- `src/ui-server/api/when.ts:143-214`, `createSiteReader`: Steps obtains the
  request-time trigger from that reader and adds `in: caller.name`; this is the
  narrow adapter seam for shared graph trigger facts.
- `src/ui-server/api/steps.ts:279`, `WALK_KINDS`: Steps follows
  `references` marked `metadata.fnRef` and `contains` edges.
- `src/ui-server/api/steps.ts:1115-1294`: arrival classification treats every
  function-reference edge as a handler, while the `contains` edge into the
  anonymous `useEffect` callback is folded as plumbing.
- `src/ui-server/api/steps.ts:890-947`, `link`: links retain their `via`
  chain in the ID, so same `from`/`to` endpoints can legitimately have
  separate links.
- `__tests__/ui-steps-api.test.ts:73-88`: the fixture has
  `useEffect(() => { addListener('onZipComplete', handleZipComplete) },
  [handleZipComplete])`; the actual registration is inside
  `<callback@17:12>`, while the dependency reference belongs to
  `ReviewScreen`.
- `__tests__/ui-steps-api.test.ts:225-239`: the local `link` helper uses an
  unconstrained `.find`, so it selects the direct dependency path before the
  registration path when both endpoint pairs exist.
- `__tests__/ui-steps-api.test.ts:240-282`: downstream bridge → native event →
  store → network/device/navigation and WHEN/source assertions must remain.

## Diagnosis and patch decision

This is both a path-selection bug and a semantic trigger bug. The endpoint
identity is not wrong: the new graph correctly keeps the registration under
the anonymous effect callback and can retain a separate direct dependency
reference. The test helper is wrong to select the first endpoint-equal link.
The dependency reference is also not an invocation/handler: `[handleZipComplete]`
declares an effect dependency and does not register or invoke the listener.

Parent-authorized revision: dependency-array recognition belongs in the shared
`graph/branch-guards.ts` trigger derivation, not as a second parser/inference in
`when.ts`. Steps must skip only a shared-positive React dependency reference;
it must not blanket-require a trigger for all JS `fnRef` edges and must not fold
the skipped dependency into executable body work. The graph's reference edge
remains intact.

Parent follow-up source decision before the next code edit:

- The shared tree helper now needs the request language to apply the
  dependency-array fact. `triggerInTree` is an exported test/utility seam, so
  add an optional language-aware parameter while retaining its existing
  four-argument call shape; internal file/source entry points pass the language explicitly.
  This fixes the compile error without breaking existing callers that do not
  have language metadata.
- Recognize only exact React dependency argument positions: index 1 for
  `useEffect` and `useLayoutEffect`, index 2 for `useImperativeHandle`; do not
  recognize `useFocusEffect`, whose API has no dependency-array argument.
  Do not use a broad `> 0` test that can relabel a callback argument.
- While looking for the enclosing dependency array, stop at an inline function
  boundary. Thus a function body nested lexically in an array, such as
  `useEffect(fn, [() => register(cb)])`, keeps `cb` as a real nested
  registration rather than treating it as a dependency read.
- Add shared positive and negative trigger cases for those positions, the
  excluded hook, and the nested function boundary. Do not change semantics for
  unrelated callback/registry `fnRef` sites.

Adopt:

- Keep lexical callback ownership and the actual listener registration as the
  source of the callback trigger (`addListener`, event literal, and
  `in: <callback@17:12>`).
- Make the shared trigger derivation identify a React dependency-array site by
  parsed source position and return no callback trigger for it, while keeping
  actual callback arguments (`useEffect`'s first argument, `addListener`'s
  handler argument) intact.
- Expose that shared fact through the narrow `SiteReader` seam and have Steps
  skip only confirmed dependency references before arrival classification. Keep
  all other `fnRef` semantics, including registry/function-value references,
  and preserve valid `onPress`, `onSubmit`, `addListener`, and other callback
  bindings.
- Make the test helper select the registration link by its `via` callback and
  assert the endpoint has no dependency-only handler path; add the shared
  trigger regression in `__tests__/branch-guards.test.ts`; retain all existing
  downstream assertions unchanged.

Avoid:

- Do not relabel the listener as `ReviewScreen`, fabricate an outer callback,
  collapse distinct `via` links, weaken exact trigger/source assertions, or
  remove the callback node/contains edge.
- Do not blanket-require `trigger` for all function references or fold a
  skipped dependency as executable plumbing.
- Do not alter the prior `steps.ts` anonymous-event component-boundary change;
  it is preserved and is unrelated to this trigger selection.
- Do not edit the extractor/resolver, framework coverage, tests outside the
  allowed trigger/Steps files, or the parent-scheduled validation flow.

## Pre-code checklist

- [x] Source gate reset and required guidance/source/tests read.
- [x] Current revision/fingerprints and relevant symbols/lines recorded.
- [x] Dependency reference distinguished from actual listener registration.
- [x] Parent scope expansion and revised shared-derivation decision recorded
  before code.
- [x] Final patch and scoped static review complete (`git diff --check` passed).

## Receipt

Command: `git rev-parse HEAD && sha256sum src/ui-server/api/steps.ts
src/ui-server/api/when.ts __tests__/ui-steps-api.test.ts`, plus the numbered
source reads recorded above. Tests/builds intentionally not run; parent owns
validation. The first patch exposed a compile gap: `triggerInTree` called the
language-dependent helper without a language parameter. It also used a broad
dependency position test and included `useFocusEffect`; the follow-up retains
the four-argument seam, passes language through internal callers, uses exact
hook positions, and stops at nested function boundaries. Final post-patch
SHA-256: `branch-guards.ts`
`d7a5b1bae3f09188953accf213833450c475b9a033f7dc77a1c44a7f6cd6293d`,
`steps.ts`
`6686b1ac3c5e3691d04aaee37f6521633002ec99ab1d50da6464c861adba6827`,
`when.ts`
`783ef7361808e3d038c7e42b1e647f4d328ba29380152d7799b2bd212b655b01`,
`branch-guards.test.ts`
`11eec1a4d89e3bf45eab8089280328eec34d87d20d5a47b7dc7489389beecf11`, and
`ui-steps-api.test.ts`
`c2145491dae73b7ad77f3bc627f1c837aea872a1e31ae368504756240f5ac9ba`.
Tests/builds intentionally not run; parent owns validation. The remaining gap
is runtime confirmation by the parent-scheduled focused/full suites.

## Parent runtime review and correction gate

callback-steps-flow-20260912-01.json: 193 pass / 2 fail, runner exit1.
Failures show dependency references still treated as triggers and retained as
direct Steps paths. Parent reread the shared AST helper and exact assertions:
`args.namedChildren.indexOf(node)` compares JavaScript wrapper identity, not
tree-sitter node identity. Adopt node.id matching (used by current extraction
helpers) and filter comments before positional matching. Also include generator
function boundaries so a registration in their body is not a dependency read.
Add comments/generator controls; preserve all existing positive flow assertions.
No budgets, hook positions or graph reference edges are changed by this repair.

Next parent source gate: corrected dependency detection passed its assertions,
but callback-steps-flow-20260912-02.json has194 pass/2 fail at later assertions:
Alert.alert loses its single-site label and setZipUri gets the handler's region.
Reread Steps two-pass arrivals/folding, regionOf, effectLink and final one-site
effect labeling with the full fixture assertions. The named listener is folded
via `contains` before the nested effect callback's registration creates its
step, so its effects are observed twice and its region is assigned prematurely.
Adopt actual non-dependency fnRef wiring inside the same lexical owner as a
reason to defer that structural fold; preserve contains as structure, never
convert it into a runtime call. Nested callable regions belong to their lexical
screen root, while separately rendered components retain their own region.
Keep effect/site/downstream assertions unchanged to verify the repair.

Parent executed callback-steps-flow-20260912-03.json: 196 pass / 0 fail /
0 skip, seven files, runner exit0. Exact dependency index/id matching,
comment/generator boundaries, deferred listener folding and lexical screen
regions now pass the original downstream bridge/event/store/request/navigation
assertions. No test expectations for effect labels/regions were weakened.
Full-suite compatibility remains pending; hook-name recognition is a bounded
source heuristic, not proof of arbitrary runtime hook identity.

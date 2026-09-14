# W4 callback flow compatibility source review — 2026-09-12

Pre-code source receipt. Worker scope: eight assigned compatibility suites;
implementation only the assigned synthesizers and Steps/Screens API files.
No tests/builds, recursive agents, configuration changes or commits.

## Source-study

Read project-graph-agent/AGENTS.md, PLAN §0.4.1, codegraph/AGENTS.md and
codegraph/docs/design/framework-coverage.md before router investigation.
`outsource/product` contains only reports; product/AGENTS.md is absent.
Codegraph HEAD: 3ed73bc127323e63153bf6ec8354afa82ce36aaf, dirty shared tree.
Existing source changes are preserved. Pre-edit SHA256:

- callback-synthesizer.ts: de839ec6ca4536c73a988654455fd4c8a7299cb4a2f21f29dee982caf86a735e
- tier-synthesizer.ts: 0243782d0aa810bd4f39a956236c13d858e9574b33066ba843a891bf5dd931d6
- steps.ts: e834061ece27be717b6127e710cf9541f1b24cf41e6ff5bbdd22d85c178cd128
- screens.ts: fd1fdebbc311db3b49ea8c00918d43ce279591aeb1b01a38deab58bc849057fa

Source mechanisms freshly inspected:

- synth-utils.ts enclosingFn: position-sensitive innermost callable, with
  conservative line-only deferred-boundary rejection. Read only; main owns it.
- callback-synthesizer.ts rnEventEdges (~1430–1615): native dispatcher joins
  literal event registration; inline target currently selected at `.addListener`
  instead of argument start. This incorrectly lands on the effect callback.
  Named-handler regex also accepts `async`/`function`/single arrow parameters,
  producing an extra enclosing-owner endpoint for those inline forms.
- tier-synthesizer.ts handlerNode (~595–623): already uses full-source argument
  offset and whitespace trimming to select actual callback. Adopt position
  selection in RN; do not replace that endpoint with a component.
- react-router-synthesizer.ts reactRouterLinkEdges and vue-router-synthesizer.ts
  vueRouterLinkEdges: markup has separate position-aware navigation ownership.
  Failing push assertions instead concern useEffect / promise callback bodies
  (react-router.test.ts ~389,460; vue-router.test.ts ~166,256).
- steps.ts triggerAt (~950) reports caller.name, correctly now the effect
  callback for a registration in its body. Forward fold (~1090) uses contains
  to discover handlers, not graph invocation evidence. Event boundary (~1003)
  currently recognizes only component-named event endpoints.
- screens.ts attribute (~435–525): reverse structural ownership and fnRef
  mapping recover screen attribution; cross-context native events are excluded;
  file fallback is nearest first. This is UI ownership, not runtime invocation.
- All eight assigned test failure sites and corresponding fixture bodies read;
  baseline JSON is historical completed-suite evidence only (5202 pass, 17 fail,
  9 skip per assignment), not verification of upcoming patches.

## Adopt/avoid and patch decisions

1. Intentional ownership: extraction tests must name exact callback positions,
   assert body references there and absence on the outer component, and retain
   lexical containment assertions. React/Vue push tests must check their exact
   callback endpoint and retain destination/metadata checks and screen projection.
2. Actual RN regression: select inline handler at its argument start, require
   exact callable position, and prevent the named-argument pass duplicating it
   onto the enclosing effect/component. Keep native→listener→body assertions.
3. Steps registration label should report the actual effect callback. Socket
   event must retain actual listener identity, including downstream body calls.
   Investigate component boundary mapping via lexical parents before deciding
   whether an API fix is necessary; never relabel the edge target as Chat.
4. Expo native-bridge test must assert incoming edge to the exact listener and
   retain all screen, guard, site and no-self-transition assertions. If UI chain
   representation changes, distinguish containment from runtime via explicitly.

No external code copied or dependencies added; adapting existing project
mechanisms in place. No new framework coverage row claimed. Parent schedules
validation: all eight assigned suites, then full suite. Missing edges must fail
assertions; no optional skips or broad callback-name matching as endpoints.

## Additional source gate: event boundary projection

Read steps.ts queue boundary (~985–1010), route-roots.ts looksLikeComponent,
and returned-callable.ts naming before this API patch. The component check is
PascalCase/function or component kind. An anonymous socket listener therefore
loses the previous component cut despite still being lexically inside Chat.
Adapt by following only unique same-file `contains` parents through anonymous
callbacks to a component for boundary classification. Preserve listener ID as
the event node; named handlers keep their existing enterable behavior. This is
UI parent mapping, not a synthesized call edge. The cross-tier test must assert
exact listener ID, event metadata, component cut and server→listener link;
the source graph must still expose the listener's setMessages body reference.

## Findings and handoff

Concrete patches delivered in all eight assigned test files, plus
callback-synthesizer.ts (RN argument endpoint) and steps.ts (anonymous event's
component boundary). Existing unrelated and parent-owned edits preserved.
No changes to router synthesizers, tier-synthesizer or screens.ts were needed
for the mechanisms inspected. No tests/builds run; scoped `git diff --check`
passed after the initial patches and again after final edits.

**Actual missing navigation, outside owned paths:** further source review of
frameworks/react-router.ts resolve (~176–182) shows the local `redirect`
lookup starts at the callback's startLine (5). The fixture declares redirect
on line 4. frameworks/expo-router.ts readHrefViaLocal (~459–490) searches only
back to enclosingStart, so the callback's history.push on line 7 cannot see
that declaration. Merely changing the expected owner cannot fix this edge.
The updated test intentionally still requires callback→home AND login→home
screen projection; it must not pass until the resolver is fixed.
Required additional ownership: src/resolution/frameworks/react-router.ts
(or parent-owned shared lexical-scope integration). Recover lexical enclosing
scope for literal binding lookup while retaining callback as source; preserve
shadowing/reassignment rejection. Do not synthesize an outer push or duplicate
the resolver in the markup synthesizer to stay superficially within scope.
Vue's named-object push bypasses this local-binding branch and only needs the
endpoint assertion change in this fixture.

**Vue naming caveat for main extraction owner:** vue-extractor.ts
processScriptBlock (~189–195) rebases start/end lines but not callback display
names. The Login listener is at full-file 8:15 but currently named with its
script-relative line 5. The compatibility test selects exact kind/startLine/
startColumn and asserts its edge ID, so it neither ignores a missing callback
nor depends on the stale label. Fixing display-name rebasing is outside scope.

Follow-up validation belongs to the parent. Expected remaining failure until
scope repair: react-router captured-local destination plus its explicit screen
projection assertion. Runtime outcomes of all patches remain unverified.

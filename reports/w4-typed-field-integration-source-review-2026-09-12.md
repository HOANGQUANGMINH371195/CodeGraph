# W4 declared-type field-call integration

Source-gate ready after rereading current source, before implementation.
CodeGraph HEAD `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty fingerprint
`d50245947faea360e37f8eac23552099ef103cf9692f5507244a14899bc0f2e7`.

Sources: complete `type-binding.ts`, `class-field-evidence.ts`,
`returned-callable-this.ts`, `receiver-hazards.ts`; `name-matcher.ts` field-call
branch and `matchTsThisFieldCall`; `resolution/index.ts` cache clearing,
context construction, createEdges and deferred inheritance processing;
`extraction-version.ts`; field-call and returned-callable lifecycle tests.
Bundled grammar probe confirms class `extends_clause.value` and interface
`extends_type_clause` shapes. Existing local receiver effects distinguish
lexical-this writes/escapes, ordinary-function resets and static/instance mode.

Adopt exact lexical-this owner and type binding, class-local receiver effects,
and AST declaration/member spans. Avoid qualified-name truncation, whole-class
regex annotation lookup, nearest type/member selection, and simple-name
supertype unions. Resolve supported heritage from its declaration scope, not
from first-pass inheritance edges (which do not yet exist).

Before code decisions:

- Generalize lexical-this owner lookup to actual indexed methods/arrows and
  abstract classes while ordinary functions reset the receiver.
- Integrate the strict path for annotated fields/constructor properties,
  including returned arrows. Full annotation must be a supported type name;
  unions/generics/unknown bindings cannot fall through to regex guessing.
- Traverse exact AST class/interface declarations for direct/inherited fields
  and methods; reject computed/ambiguous/non-callable shadowing members.
- Edges mean a declared-type call dependency, not verified runtime dispatch.
  Attach explicit declared-type provenance/assumptions and source evidence;
  block observed class-local field writes/escapes/decorators/dynamic evaluation.
  External mutation/subclass/allocation correctness is not proved by annotations.
- Keep existing unannotated initializer and `typeof` value compatibility paths
  explicit and pending replacement; do not claim their old heuristics are fixed.
- Drop new caches with resolver cache lifetime; verify position changes, barrel
  rebinding, removal and rebuild convergence. Bump extraction-content version
  when activating changed edges; no schema/package/native ABI change.

Lagrange owns only lifecycle test and its source receipt; main owns runtime and
direct regressions. Tests use real temporary projects and exact target IDs.

- [x] Source/adoption gate.
- [x] Annotated field-call integration and direct focused regressions.
- [x] Tested incremental lifecycle and rebuild convergence scenarios below.
- [ ] Whole W4 end-to-end acceptance (still open).

## Source gate — dependency-driven sync reopening

Direct focused tests pass 27 cases, but the lifecycle test fails on the
barrel-only change: the unchanged closure retains the old target. Before fixing,
read `CodeGraph.sync`, `ExtractionOrchestrator.resurrectStaleResolutionEdges`,
`resurrectRefFromDroppedEdge`, QueryBuilder's target-name edge lookup, exact
edge deletion/ref insertion and cross-file incoming SQL. Existing name-delta
reopening deliberately excludes heuristic provenance and does not track alias
source dependencies, so it cannot cover this new edge producer.

Add a scoped, atomic reopening operation for stamped declared-type call edges
whose recorded source evidence includes a changed file. Reuse original refName,
call-site coordinates and current source node context; never delete unstamped
or unrelated heuristic edges. New SQL goes in query assets copied by existing
copy-assets. Retry failed TS/JS this-field call refs after source changes so a
barrel becoming valid again is not permanently lost. This conservative failed
retry needs performance measurement at full-repo scale; it is not a claimed
indexed dependency ledger. No schema migration or global source reindex.

## Source gate — full-suite regressions

First full suite: 5141 pass, 8 fail, 9 skip at fingerprint
`66a8cc409548f46e6d473adb51a95aa1d89e69767f7a67ac9aa7933163518468`.
Failures expose overly broad rejection, not tests to remove. Re-read
`ui-server-api.test.ts` Service/Cache constructor fixture and
`ui-steps-api-servers.test.ts` actual Nest imports/decorators, plus the Nest
resolver's provider/decorator handling. Service initializes its annotated
field exactly once with `new Cache(config)`; that is different from an
unmodeled replacement write. Capture direct constructor initialization by AST
and require its exact bound type to match the field annotation; keep all other
writes/escapes blocked.

Nest fixtures expose a semantic distinction: a decorated method still has a
declared-type dependency even though decorators can alter runtime behavior.
Keep exact binding (no legacy name fallback), retain decorator evidence as an
explicit unverified runtime assumption on these heuristic contract edges;
do not describe them as safe/known runtime dispatch. This refines the initial
blanket decorator rejection without weakening the runtime receiver verifier.
Computed member-name evaluation remains a blocker. No router behavior changes.

The corrected focused run restores all eight missing-flow assertions except
one old UI expectation that these edges have no heuristic marker. Re-read
`api/wire.ts` edge projection/grouping and `api/flow.ts:flowEdgeLabel`: the UI
uses heuristic provenance to mark inferred relationships. Preserve that marker,
update the assertion to require it, and expose the declared-type/runtime-
assumption distinction on the wire and in flow labels. This is an intentional
evidence-label change, not suppressing a missing-edge failure.

## Source gate — new-file import precedence

Re-read emitted-specifier tests: a newly added real `.js` file takes precedence
over the `.ts` remap of an import spelled `.js`. That new path is absent from
an old edge's sourceEvidence, so changed-path intersection alone cannot reopen
it. Add an exact lifecycle regression and invalidate all declared-type call
dependencies on file inventory changes (not on ordinary body edits). Keep
source-evidence scoping for modifications. This does not claim tsconfig/runtime
configuration drift is handled; that still needs separate integration evidence.

## Implemented scope and focused results

The active resolver now consumes class-field evidence and lexical type binding.
`callableThisClass` maps methods/arrows and abstract classes to exact class
extents; ordinary functions reset `this`. `typed-this-field-call.ts` joins
direct class/interface methods by exact declaration extent, follows supported
heritage in declaration scope, separates static field selection from instance
methods of the field's value, and rejects observed replacement writes/escapes.
Single direct constructor `new` initialization is accepted only when the
bound type matches the field declaration. Runtime conformance/decorator/method
replacement remains an explicit assumption, visible in edge metadata and UI.

New query assets atomically reopen stamped declared-type call edges and restore
their original references. Ordinary source modifications use evidence-file
intersection; inventory additions/removals invalidate all such type-dependent
edges because import-path precedence can change. Failed this-field call refs
are retried after a source change; this conservative retry is not a new indexed
dependency ledger or a demonstrated large-repo performance guarantee.

Executed receipts under `.harness/baselines/`:

- `typed-field-integration-20260912-01.json`: 27 pass, no failures.
- `typed-field-lifecycle-20260912-01.json`: 1 failure, real barrel rebind bug.
- `typed-field-lifecycle-20260912-03.json`: 32 pass after source-dependent
  reopening, invalid-barrel recovery and target-only position edits.
- `typed-field-ui-regression-20260912-01.json`: 103 pass / 1 fail after the
  Service/Cache and Nest fixes; remaining failure was old provenance expectation.
- `typed-field-ui-regression-20260912-02.json`: 109 pass / 0 fail with explicit
  declared-type wire fields/flow labels and additional constructor/decorator cases.
- `typed-field-flow-ui-20260912-01.json`: runner exit 0.
- `typed-field-inventory-red-20260912-01.json`: 1 pass / 1 fail before new-file
  precedence invalidation; `typed-field-inventory-20260912-01.json`: 2 pass / 0 fail.
- Emitted `tsc` and `npm run copy-assets`: exit 0; content version 29, ID format 1,
  native ABI 2 and package version unchanged. No native source change this turn.

The first full-suite failure remains preserved above. The second full-suite
JSON receipt reports **5154 pass, 1 fail, 9 skip / 5164 tests, 293 files**,
`success: false`. Its process handle was missing on resume and no Vitest
process remained; terminal exit output was not recovered. Source fingerprint
on resume still matches the launch fingerprint
`3af09d998723b39c855fd63559d5e87d49d1316ee4156988ac84d2d86132fecb`.
The single failure is `mcp-ppid-watchdog.test.ts`: child exit not observed within
5 seconds after wrapper SIGKILL, with empty captured stderr. Do not classify
this as an unrelated flake or a zombie without evidence.

On resume, re-read the watchdog fixture's fixed 800ms startup delay and
`src/mcp/index.ts` early-PPID capture / direct-mode watchdog registration.
The delay is not a readiness handshake; startup timing is a hypothesis, not
a verified cause. Ran that exact suite separately without source changes:
`typed-field-watchdog-recheck-20260912-01.json`, **1 pass, 0 fail**, runner exit 0.
This does not turn that historical failed full suite green. Follow-up
[watchdog readiness verification](w4-watchdog-readiness-source-review-2026-09-12.md)
replaced the fixed startup guess with a real handshake, retained the 5s exit
assertion, tested negative/error controls and confirmed owned-group cleanup.
Final full suite: **5160 pass, 0 fail, 9 skip / 293 files**, exit 0 at unchanged
fingerprint `e0b9c4e9b3264d5875948aefbebdea68ae40c984562abbe00bf960a756f2020a`.
Only the watchdog test changed in that follow-up; no resolver/runtime edit.
The historical PID's exact failure cause was not proven retrospectively.

Re-read `scripts/orders-graph-smoke.mjs` before executing it on the current
emitted build, after all test processes had terminated. Receipt
`orders-typed-field-20260912-01.json`, exit 0: **51 nodes, 131 edges, 7 files**;
injected calls remain **incomplete: 4 edges / 1 issue**. All five probes exit 0,
watch observed with no watch errors; source, dist and original fixture unchanged.
The diagram probe still returns **No relevant code found**. Probe process
success is not diagram acceptance, MCP transport verification or a live-agent
benchmark; `product_acceptance` remains false.

Still open: general `typeof`/unannotated initializer binding, namespace/merge
and new grammar coverage, full runtime points-to and mutation/escape analysis,
configuration-driven import resolution changes, broad inheritance/generic
coverage, factory-return propagation, HTTP parameter dispatch and actual W4
flow/diagram acceptance. The tested lifecycle cases do not close these gaps.

# W4 call identity and parameter handoffs

status: done (call identity/handoff evidence only); source-gate: ready;
owner/reviewer: main (no independent review).

- [x] Reset gate; reread class-exports.ts fully (bounded ES binding resolution),
  construction-sites lexical declarations/origin/effect flow, constructor-join
  collection and import lookup, and real-index join setup/assertions.
- [x] Pre-code revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf,
  dirty fingerprint 630cb3ea591675fd6571d6e32841d681e0996e448153a89c34fefeb71ea4d60f.
- [x] Adopt shared strict export walk, not generic first-exported default
  fallback. Parameterize indexed symbol kind for functions while retaining the
  class wrapper. Function origins use the existing lexical scope/alias/write
  table, separate from constructor origins. Record only calls referenced by
  observed allocation/parameter handoff effects; do not expose every call.
- [x] Design before code: local function extent or runtime import identity →
  exact function evidence → actual argument index/receiver parameter. Keep
  missing, dynamic, reassigned, member-call and ambiguous import targets unknown.
  Preserve unknown/spread positions and import proof; do not publish edges.
- [x] Test local aliases/hoisting/shadows/reassignments, ES imports/re-exports,
  wildcard ambiguity, wrong symbol kinds and indexed refresh; probe Orders.

Remaining: receiver mutation/prototype validation, completeness/alias limits,
function-expression extraction parity and activation with scoped replacement.

## Delivered and verification

The existing export resolver now shares its ES binding walk between class and
function wrappers, preserving explicit/star/default conflict rules and source
hash proof. Indexed definitions are matched by exact extent and kind; unsupported
function-expression extraction is not relabeled into a function. Parenthesized
eval also invalidates export resolution. Generic import resolver unchanged.

Construction evidence now resolves handoff callee origins through lexical
bindings (local declaration/expression, immutable alias, runtime import) and
retains only call sites referenced by tracked allocation/parameter effects.
Constructor join adds callHandoffs: target file/function extent, status,
argument-position to parameter-span mapping and import evidence. These are
observed source handoffs, not receiver-safety decisions or emitted graph edges.

Additional review found a real scope bug: function declarations were given a
private self-binding like function expressions, so assignment to f inside f
failed to invalidate outer f. Added a regression (failed first), then limited
private self-bindings to named function expressions and class scopes. Re-ran
the complete focused set and a new Orders probe after the fix.

Final command (product scripts/with-local-tools, CodeGraph cwd):
node node_modules/vitest/vitest.mjs run
__tests__/function-parameter-uses.test.ts __tests__/receiver-hazards.test.ts
__tests__/constructor-join.test.ts __tests__/construction-sites.test.ts
__tests__/constructor-facts.test.ts __tests__/ts-this-field-call.test.ts
__tests__/call-receiver-no-fabrication.test.ts __tests__/store-exported-later.test.ts
__tests__/synthesized-edge-replacement.test.ts

**232 pass / 9 files**, exit 0. Emitting tsc and git diff --check exit 0.
No fresh full suite/kernel/UI build or Rust foundation validation this turn.

Orders probe (product cwd): node scripts/orders-graph-smoke.mjs
/home/minh/projects/outsource/codegraph
.harness/baselines/orders-call-handoffs-20260912-02.json: exit 0.
[Final receipt](../.harness/baselines/orders-call-handoffs-20260912-02.json);
the -01 receipt predates the scope fix and is retained as historical evidence.
Final source fingerprint 6a176f3a896f2fe97ac80feb8678dcd3880269b242600de3a72b57a552334333.

Main Orders call at offset 452 resolves to http.mjs ordersServer (331–962),
mapping arguments 465/490 to parameters 353/362. Call at offset 569 resolves
to notificationsServer (970–1393), mapping argument 589 to parameter 999.
These connect to the existing factory-parameter effects for create/dispatch/
consume. Source/dist/fixture stable, watcher observed with no errors. Five probe
processes exit 0, not proof of correct answers. Graph remains 51 nodes/127 edges;
requiresReceiverValidation is true and product acceptance is false.

Next receiver gate must combine owner/dependency reviews, allocation effects
and these handoffs, checking method/prototype changes and retaining unsupported
or incomplete cases. Synthesis activation and impacted-source reconciliation
remain required. W4 and the full W0–W13 objective are not complete.

## Resume: HTTP parameter dispatch source study (not implemented)

Main reread current construction-sites scope/effect/call filtering, join contracts
and function identity, receiver-decision in full, injected-call-mapping and
injected-reconciliation in full, plus function-parameter-uses, callback handoff
join tests and receiver-decision tests. An explicit Luna read-only review agrees
on the root limitation; worker closed after handoff. Revision remains
3ed73bc127323e63153bf6ec8354afa82ce36aaf; current file SHA256:

- construction-sites.ts: 1d24f422b6460695ce342191be9ae68ea2d3cd1b5607840d6741cad0d47be6b2
- constructor-join.ts: f42e089b1ca0c5671e9d2f05b8f576a2427b05ff47095cd56bfb4fbfb1345f04
- receiver-decision.ts: 05291189b31727f69470861566129ee2f132dbffa49019f4181e8748cc7662fa
- injected-call-mapping.ts: eff3381822a6948e86062d6d0fbd03e95afb1b213d5e0f27892823591905a878
- injected-reconciliation.ts: ebdc16c3db36f1b078a78a69592eeade9b3dee7bfaa1202d726db3e2cbb4415b

Adopt the existing bounded class/allocation/parameter worklist via explicit root
adapters, exact source spans and indexed-snapshot replacement. Avoid fake fields
to enter the field-anchored analyzer, duplicate mutation engines, or selecting
an outer factory as the caller. executionFunctionStart identifies the actual
callback; contains is not invocation. A distinct parameter-dispatch producer
must preserve constructor-field edges through refresh/failure.

Newly confirmed limitation: construction-sites.ts filters calls to tracked
handoffs (handoffCalls at lines 415/449). A second server(unknown) invocation
with no tracked argument can disappear; this is not an exhaustive call-site
inventory. Mixed calls with a tracked argument are retained, but an unknown
argument at another position is not thereby known. Before a factory-wide
parameter union is accepted, expose missing/unknown/spread invocations and
coverage explicitly. Preserve context-specific call-site provenance and
runtime assumptions; do not silently promote one known allocation to all calls.

Required new tests: no-known-argument invocation retained as unresolved, mixed
known/unknown parameters, spread/missing positions, multiple imports/call sites,
mutation/escape/dynamic hazards, exact callback spans with Unicode/CRLF, and
producer-only replacement under stale source/config/failure/removal. Existing
tracked-handoff API behavior has a specific unrelated-call exclusion test;
an inventory extension must not silently redefine that contract. This source
study is a design prerequisite, not evidence that HTTP dispatch is delivered.

Executed existing prerequisite baseline, required native kernel: 147 pass /
0 fail, five files, runner exit 0; receipt
parameter-dispatch-prerequisites-20260912-01.json. This verifies current contracts
before the planned extension, not the missing parameter-call producer.

## Resume: explicit invocation inventory source gate (2026-09-12)

source-gate: ready; this is a separate reread for the prerequisite inventory,
not a substitute for the earlier handoff review.

- Current outsource revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`
  (working tree is dirty; unrelated changes are preserved).
- Current owned-file hashes before this patch:
  - `codegraph/src/resolution/construction-sites.ts`:
    `1d24f422b6460695ce342191be9ae68ea2d3cd1b5607840d6741cad0d47be6b2`
  - `codegraph/src/resolution/constructor-join.ts`:
    `f42e089b1ca0c5671e9d2f05b8f576a2427b05ff47095cd56bfb4fbfb1345f04`
  - `codegraph/__tests__/function-parameter-uses.test.ts`:
    `b6aa280fec164b6a200b2ebb325395f324805066fb5fe2667dbcc9683d32417b`
  - `codegraph/__tests__/constructor-join.test.ts`:
    `ec5065f016f11660cf84da03599830553203845a96ddfa007e04f4c43d980beb`
  - this report already contained the prior review and is being extended here.

### source-study

Read fresh: `construction-sites.ts` collection pass and final construction/call
materialization; `constructor-join.ts` file facts, import memo, local/import
function target resolution, exact indexed function extent matching, and argument
parameter mapping; the two owned test files, including the unrelated-call
exclusion, re-export identity, refresh, spread/missing construction, and graph
no-write assertions; and PLAN §0.4.1. The current collector records every
`call_expression` temporarily but exports only calls in `handoffCalls`, which are
created by tracked parameter/constructor effects. Therefore `server(unknown)`
without a tracked argument can disappear even though its source call is observed.

### adopt / avoid / gap

- Adopt the existing lexical `functionOrigin` and `origin` tables, the existing
  call argument span/index/origin representation, and the join’s single bounded
  function-export walk plus exact indexed function extent lookup. Use a small
  shared call materializer and join target mapper so the inventory does not
  create a second export or alias resolver.
- Preserve `calls` and `callHandoffs` exactly, including the `log(1); unknown()`
  exclusion. Add a separately named lexical observed-source inventory with
  unknown callee status, actual argument spans, null index after spread, and
  explicit missing target parameters when a target is known.
- Avoid graph edges, lifecycle/reconciliation changes, whole-program claims,
  inferred runtime dispatch, and promoting one known allocation to all calls.
  Dynamic/unavailable files remain unavailable and coverage remains lexical
  observed-source only. Memoize bounded imported function checks per origin to
  keep the all-call inventory proportional to source calls.

Planned tests are added first in the two owned suites: known and no-known-arg
call sites remain separate; unknown/mixed/missing/spread arguments retain exact
spans; imported re-export identity resolves; refresh replaces the inventory;
and the filtered `calls`/`callHandoffs` API remains unchanged. Required checks
are the affected suites plus receiver mapping/reconciliation tests using the
native product runner.

### Inventory implementation receipt

Implemented only in the owned codegraph files. `collectConstructionEvidence`
now materializes every observed `call_expression` once as `callInventory`,
while deriving the existing tracked `calls` view by start offset from the same
records. `joinConstructorEvidence` exposes the separate `callInventory`,
reuses the existing lexical origins and bounded function export resolver,
memoizes imported target checks, maps exact argument/parameter spans, and
reports known-target omitted parameters. `callHandoffs` remains the
tracked-only projection with its prior shape and behavior. No graph edges,
lifecycle writes, or whole-program completeness claim was added.

Final owned-file SHA256:

- `construction-sites.ts`:
  `ee4c50d708c027dd5325fd2a0d6c2adfae9fe8ee751fe120d5d635cd67d622e3`
- `constructor-join.ts`:
  `cf27aa964a89d885fc3b4c01ddeeaa5fc7ab6774e73a625b4fc7810c9ebedd75`
- `function-parameter-uses.test.ts`:
  `385c34547bec18a4332d7418983e47ed47874825cfb527555910eb00ac72a2d3`
- `constructor-join.test.ts`:
  `7eec9d6fc3fce39183d973f7685dead5c5f803bddb934bac8b038c8ab3e37e1d`

Verification with product `scripts/with-local-tools`: affected suites
**90 pass / 0 fail**; `tsc --noEmit` **pass**; `git diff --check` **pass**.
Required receiver/mapping/reconciliation invocation: **62 pass / 2 fail**;
the 14 mapping and 18 reconciliation tests pass, while two concurrently edited
`receiver-decision.test.ts` cases fail before review because their setup omits
`http.mjs` and then dereferences an absent `functionUses` entry. No file outside
the ownership set was changed to repair that fixture. The inventory remains
lexical observed-source evidence only; dynamic/unavailable syntax and runtime
dispatch remain unresolved/retained as limitations.

Parent qualification: these two receiver test failures are not established
pre-existing failures. Before this concurrent batch the five-file prerequisite
run passed147/147. The separate receiver-engine worker owns the new fixture
setup and is repairing it. Parent inventory run -01 recorded89pass1fail in
the new Unicode/CRLF test: expected AST call.end incorrectly included the
semicolon. Worker corrected the test to the call-expression extent (server()),
without changing production spans. Fresh combined verification remains needed
after the receiver refactor is finalized.

Parent post-handoff verification: invocation-inventory-parent-20260912-02.json,
90 pass / 0 fail, two files, required native, runner exit0 (9.06s).
Current implementation includes the requested one-time call materialization
and Set membership, avoiding per-call linear scans of tracked calls.
Post-optimization implementation hashes (supersede the worker's earlier table):
construction-sites.ts 26c8e5448e57737f87bac7ef428efcce31a6e039909fad20df27a0717fa63e0b;
constructor-join.ts d983606bf0b34efcb28457a5672426f6a23107fa4339104799e5e5bb0cccafe5.
Inventory prerequisite verified; shared receiver engine, mapper/producer,
registration/invocation and full Orders diagram acceptance remain open.

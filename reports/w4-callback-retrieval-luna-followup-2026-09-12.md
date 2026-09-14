# W4 callback retrieval — Luna follow-up receipt — 2026-09-12

Owner: `gpt-5.6-luna`. Scope: `/home/minh/projects/outsource/codegraph/src/mcp/tools.ts` and this receipt only. Preserve all other dirty changes; main owns suite/build validation and unrelated extraction/default/UI work.

## Pre-code source gate

status: doing | owner: gpt-5.6-luna
scope: outsource/codegraph `src/mcp/tools.ts` + receipt | depends_on: prior W4 callback retrieval handoff
source-study: product `AGENTS.md`, product `PLAN.md` §0.4.1, codegraph root `AGENTS.md`; current `tools.ts` SHA-256 `86ed427d6c75c8c5f7f5896d13c84b943752f79d5649c272bd604df4608c00b8`; current dirty diff `142 insertions / 62 deletions` in `tools.ts`; `__tests__/explore-allocation-e2e.test.ts` CG21 (including `src/http/etag.ts` and carry-forward cases), `__tests__/explore-factory-closure.test.ts` CG27, `__tests__/explore-displacement-guard.test.ts` CG31, plus current allocator/output-limit tests.
source-gate: ready
adopt/avoid: Adopt the existing source-unit containment grouping, named-call support scoring, signature rescue, proportional allocator, whole-file buy, reservation carry-forward, displacement guard, cluster shrink/windowing, and focus-line machinery already present in `tools.ts`. Adapt only the retrieval decisions needed by the actual remaining failures: preserve a bounded whole-file response for the CG21 line-bounded target when its source is the answer, and make CG27’s source-unit cluster selection retain all relevant inner callable definitions as the indexed fixture grows. Avoid changing graph nodes/edges/ownership, attribution, labels, budgets/constants, max-files behavior, or quality assertions; avoid any label-specific penalty or test edits.
pre-code-checklist:
  - [x] Reset source-gate to pending at task start, then read current source/tests.
  - [x] Selected source by mechanism: explore allocation/rendering and callback-closure retrieval.
  - [x] Read product/codegraph guidance and current relevant source + tests.
  - [x] Recorded current fingerprint, flow, adoption, avoidance and remaining gap before coding.
  - [x] Chosen bounded implementation and targeted probe validation; main remains suite/build owner.
receipt: pre-code fingerprint recorded above; diagnostic probes to be recorded after patch.
review: pending main review | blocker: none | next: run bounded mkdtemp probes, patch only `tools.ts`, run `git diff --check` and report validation gaps.

## Resume source gate — pre-patch 2026-09-12

source-gate: pending → ready after this source study
revision: codegraph `3ed73bc127323e63153bf6ec8354afa82ce36aaf` (product checkout has no usable `HEAD`)
source-fingerprint: `codegraph/src/mcp/tools.ts` SHA-256 `53b9c238a65227fb1470b68665241b12611bd4ec363f00e217158a60bd36b19b`
current-dirty-scope: `tools.ts` already modified, `195 insertions / 64 deletions`; no other file in the owned codegraph/report scope will be changed.
source-study: reread `PLAN.md` §0.4.1, current `tools.ts` `sourceUnit`, `allocateExploreBudget`, reservation/carry-forward, `owedPayableBelow`/`owedSourceBelow`, whole-file arms, cluster range admission/shrink/window/render logic; reread `explore-oversize-member.test.ts`, `explore-reservation-invariant.test.ts`, `explore-factory-closure.test.ts`, `explore-allocation-e2e.test.ts`, and `explore-displacement-guard.test.ts`. Nearby outsource search found no retrieval allocator implementation to reuse; the applicable existing mechanisms are local to this `tools.ts`.
adopt/avoid: Adopt the existing proportional reservations, source-capacity-aware carry-forward, render-space displacement guard, whole-file buy/grace, source-unit containment grouping, deferred-callable alternatives, and bounded cluster shrink/windowing. The live failure points to a budgeting/ranking interaction, so adjust only the source budget/ranking decision that misprices or withholds relevant ranges. Avoid changing tests, limits/constants, max-files semantics, graph ownership/edges, labels, assertions, or unrelated files; do not weaken a gate.
decision/test: First reproduce the three named regressions and inspect diagnostic allowances/emitted/funded values. If the source budget is correct but relevant callable ranges are under-ranked, preserve the source-unit grouping and repair candidate importance/range admission; if a budget is over-held, repair only the accounting of actual source capacity. Verify with all `explore*.test.ts` where feasible using the prescribed `with-local-tools` runner and a unique product baseline.
pre-code-checklist:
  - [x] Reread PLAN §0.4.1 and reset this resume gate to pending.
  - [x] Reread current allocation/sourceUnit/render logic and required/prior retrieval tests.
  - [x] Recorded revision, source fingerprint, source-study, adopt/avoid, and intended checks before patch.
  - [x] Source gate ready: no implementation decision remains uncovered by the reread.

## Scope and known failures

Main verified `callback-compatibility-20260912-01.json`: 166 passed / 5 failed across 10 files. The retrieval failures assigned here are:

- CG21 `explore-allocation-e2e`: `src/http/etag.ts` remains `clusters`, emits about 521 source characters against a full-file source size of about 8,751 and an allowance of about 6,535; the file is within the fixture’s whole-file line bound. The carry-forward/displacement CG31 path is already passing and must remain so.
- CG27 `explore-factory-closure`: the factory file currently delivers 15 indexed inner definitions while the unchanged assertion requires at least 16.
- CG31 displacement: all assertions pass in the stated main run; preserve the current behavior.

## Parent takeover — source gate before correction

Worker interrupted for bounded handoff and closed; no owned probe process remains
per handoff. Main reread current tools.ts source-unit/range admission, both
funding paths, shrink/window logic, complete CG27 test, CG21 carry fixture and
upstream probe-factory-closure.mjs. Current tools SHA256:
97e5407649c1d6fc329d9f9096c233be5e6b2403022c0043784f866e63b7831f.
Main executed callback-retrieval-luna-20260912-01.json: 43 pass / 2 fail.
CG21 whole-file buy now passes, but carry-forward loses its >1.1x reservation
margin (8085 vs required8201); CG27 remains15/32 vs required16. No thresholds
will be weakened. Worker sourceCapacity-aware owedSourceBelow is retained.

Main's source-loaded CodeGraph/ToolHandler probe (temporary fixture, TypeScript
transpileModule hook, no emitted-build claim) reproduced15/32: dashboard store
allowance5593, emitted7126; loadWidgets begins39 but only lines39–45 are shown,
callback67 absent. Definition text inside a gap does not count. Initial tsx
probe failed before execution because tsx is unavailable; no install attempted.

Adopt source containment for candidate admission, not blanket central/entry
file admission: that admits callbacks of unrelated top-level filler functions
and can consume carry-forward. A candidate must descend through callable
contains parents from a gathered source unit, with same-file spans and cycles
checked. Preserve complete deferred callable subranges as alternatives to a
too-large containing range; do not insert isolated definition-line crumbs.
Remove the ineffective single-line focus fallback and indiscriminate focus
inflation. Keep original ceilings/weights/assertions and exact graph ownership.
Verify CG21/27/31 together, then broader retrieval and full suite.

Main correction executed: callback-retrieval-20260912-02.json, 45 pass / 0 fail,
three files, runner exit0. Both CG21 whole-buy/carry-forward, CG27 complete
callable-range delivery and CG31 displacement assertions are unchanged and pass.
This is a focused result; broad relevance/session-dedup/full suite remain open.

## Final handoff — 2026-09-12

source-gate: final source reviewed; no trace instrumentation remains
final-source-fingerprint: `codegraph/src/mcp/tools.ts` SHA-256 `3ed80fcaff3234957f5b86f32f4bd250fa8dc7ba409a5f1988598bc4c8238bd7`

mechanism:

- Source-unit containment resolves deferred callback nodes to their containing callable, so ranking and range admission operate on the useful callable unit rather than an anonymous callback fragment.
- Named-call support is promoted only when it is backed by a shape-precise named seed (camelCase/PascalCase/qualified/path token) and there is at most one named seed file. This replaces the earlier query-word-count proxy: `explain ingestRecords flow` retains precise `ingestRecords` support, while bare English collisions do not gain promotion merely because a query is short.
- Reservation accounting charges actual section overhead and source capacity. Small pending files are protected to their complete available source plus overhead; ordinary files retain partial carry-forward only when the remaining budget can fund a meaningful window. No minimum-window, ceiling, max-files, or assertion was weakened; no import-only/tiny final slice is accepted as the callable fix.

validation:

- `w4-callback-retrieval-semantic-call-support-20260912-01.json`: runner exit 0, 27/27 tests, including the parent-owned complete `writeBatch` body predicate, reservation, oversize, and NL-collision coverage.
- `w4-callback-retrieval-all-explore-semantic-final-20260912-01.json`: runner exit 0, 294/294 tests across 107 suites. The count includes the parent-added regression; the earlier all-explore receipt was 293/293 across 24 files before that addition.
- `git diff --check`: exit 0.
- Trace cleanup check: `TRACE_CLEAN`; no `RETRIEVAL_TRACE`, allocation probe, or sink diagnostic markers remain in `tools.ts`.
- Full suite was not run; main requested coordinated source-freeze review first.

review: ready for main review; no further source edits planned.

Parent post-handoff source gate: reread preciseNamedSeedIds support projection,
named/change-surface comparator, capacity holdback and final cluster windowing;
confirmed tools.ts SHA256 equals worker's 3ed80fcaff3234957f5b86f32f4bd250fa8dc7ba409a5f1988598bc4c8238bd7.
Reread CG26 query setup and complete-body predicate (test hash
98b9e8a64786080a798816266986f7a4b4dcb5d08d3dd2d161ea63c7766385dd).
The exact natural-language example is not yet in that fixture. Adopt its real
index/diagnostic assertions and add `explain ingestRecords flow` as another
query shape, requiring complete writeBatch body and all existing reservation
invariants. This directly verifies the semantic-precision claim rather than
assuming a word-count guard removal proves equivalent source delivery.
Worker closed; parent owns the added test. No production patch planned here.

Executed retrieval-natural-callable-parent-20260912-01.json: 28 pass / 0 fail,
three files, required native, runner exit0 (5.90s). Both precise symbol and
natural-language query deliver the complete writeBatch definition/body; all
CG26 invariants, oversize and natural-language collision controls pass.
Independent all-explore rerun now requested; full-suite gate remains separate.

Parent all-explore runner exited0, receipt retrieval-all-explore-parent-20260912-01.json.
Compiler check then found TS2339/TS7006 at tools.ts4180: the configured library
does not type SetIterator.some. This was introduced in the semantic-support
patch, not a pre-existing baseline error. Source gate: reread that support Set
loop and nearby spread-array predicates; replace the iterator helper with the
existing array predicate idiom without changing membership semantics. Worker
closed; parent owns this one-line compatibility correction.

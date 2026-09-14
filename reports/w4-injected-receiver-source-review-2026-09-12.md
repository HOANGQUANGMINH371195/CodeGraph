# W4 injected receiver investigation

status: done (diagnosis only); source-gate: ready; owner/reviewer: main agent.

- [x] Read name-matcher.ts:2247–2259 and 2649–2752: exclusive this-field
  resolution accepts annotation, typeof value or new initializer, otherwise null.
  Read ResolutionContext/ResolvedRef contracts and ts-this-field-call.test.ts
  plus call-receiver-no-fabrication.test.ts. Preserve the exclusive no-guess gate.
- [x] Current orders corpus injects an untyped constructor parameter. Its call
  sites pass a concrete OrderStore, but the resolver never connects parameter
  position, instance field assignment and constructor actual arguments.
- [x] Before code: diagnostic using isolated owned JS files, same concrete Store
  and consuming service method; compare direct new field initialization, parameter
  injection and unknown receiver. Record exact edges, source fingerprints and
  distinct cases; do not change the benchmark corpus to a more easily indexed form.

Source CodeGraph revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf, dirty worktree
from earlier fixes. No upstream edits or account runs; built dist correspondence
is unverified. This probe is mechanism diagnosis, not the missing feature's fix.

## Verified contrast

`node scripts/injected-receiver-probe.mjs /home/minh/projects/outsource/codegraph .harness/baselines/injected-receiver-20260912-01.json`
from product root: exit 0; `node --check scripts/injected-receiver-probe.mjs`: exit 0.
[Raw receipt](../.harness/baselines/injected-receiver-20260912-01.json) preserves all
three source inputs, index results, callers and exact edge metadata. Upstream
source unchanged; no foundation implementation changed or suite rerun this task.

| Fixture | Actual callees of Service::create |
|---|---|
| this.store = new Store() | Store::createOrder |
| this.store = store; new Service(new Store()) | none |
| unknown externally supplied store | none |

Each input includes an unrelated createOrder method. No edge to it appears.
The missing edge reproduces even in one file with an explicit new Store argument,
so cross-file imports, SQL parsing and HTTP synthesis are not prerequisites for
reproducing this receiver inference gap. This is narrower than saying all forms
of dependency injection are unsupported (typed constructor properties already work).

## Implementation requirements, not yet delivered

Add constructor parameter → instance field → actual argument evidence with lexical
owner/import binding. Follow aliases only when proven; handle argument position,
multiple construction sites, unknown arguments, reassignment, nested/shadowed names
and explicit dynamic boundaries. Do not scan all same-named methods as a fallback
or infer the class solely from a parameter called store. Preserve call-site evidence
and distinguish possible targets from proven dispatch. Reuse receiver-resolution
and no-fabrication suites; then rerun unchanged orders corpus and compare emitted
source/edges, not merely total node count. Main diagnosis only, no independent
review or implementation acceptance. Full W4 and W0 remain open.

## Shared receiver engine follow-up

The Luna worker wrote its receipt at the wrong root:
/home/minh/projects/outsource/product/reports/w4-injected-receiver-source-review-2026-09-12.md.
Parent read that full artifact; it is retained, not treated as missing source
study or copied over this historical report. Worker timing/hash claims are
worker-reported, not independently reconstructed chronology.

Parent inspected the shared-root extraction and required restoration of
transferred field-local review, unavailable/analysis hazards, empty-root
rejection and malformed-span validation before visited dedup. Independent
receiver-worklist-parent-20260912-01.json: 156 pass / 0 fail in five files,
required native, runner exit0. This is not full-suite or HTTP dispatch evidence.

Pre-patch reuse gate: parent reread receiver-decision.ts in full and current
receiver-decision tests, plus constructor join/index lifecycle consumers.
Current hashes: receiver-decision.ts
7ded8b02dc03432ec01975d02dd276013084a50371947be19c8bc28ff1df473f;
test 0df02bc41602b8aff4fbfda2c562e5a175a2fc7fdf732cb8a495177ddfbc455d.
Revision remains 3ed73bc127323e63153bf6ec8354afa82ce36aaf. Worker closed;
parent now owns engine/test. m10-performance requires measurement before an
optimization claim: add a two-field test counting reads of allocation evidence
to demonstrate current index reconstruction per field. Adopt the original
once-per-input maps through a request-local reviewer factory, with fresh issues,
visited state and effect budget for each review. Keep the one-shot API wrapper.
Avoid persistent caches, stale snapshot reuse or changing mutation semantics.
Add independent-invocation state-reset test; rerun the five-file baseline.

Measured red: receiver-index-reuse-red-20260912-02.json, one failed/34 filtered
skip, exit1: two fields read the allocation collection twice instead of once.
The earlier -01 failed a fixture assumption (same parameter assigned twice is
an ambiguous transfer), not the reconstruction counter; preserved historically.
Implemented createReceiverReviewer(input, limits), used once by the field
adapter and available to the forthcoming parameter mapper. No persistent cache;
callers must keep this request-local joined input immutable. Issues/visited/
effect budgets reset on every invocation; one-shot API remains supported.

receiver-index-reuse-parent-20260912-01.json: 158 pass / 0 fail in five files,
required native, runner exit0 (12.54s). Includes the one-index-build counter and
independent repeated review/budget tests. Parent tsc --noEmit exit0 after patch.
This proves reduced repeated indexing work, not a measured latency speedup.

Post-verification owned hashes: receiver-decision.ts
fa0735a2db9d4aae1c6e53fef2657913ff6f7e3b456e1253b5134448983f53b5;
receiver-decision.test.ts
e3ca0ae038012ac1b6f53c1462021e148e7c527c42d672f927d263b70223fb76.
Global codegraph git diff --check exit0. Parameter mapper worker notified of
the factory API and request-local immutable-input constraint.

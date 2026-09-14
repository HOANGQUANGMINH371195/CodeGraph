# W4 composed receiver decision

status: verified (bounded read-only component, not W4 acceptance); source-gate: ready; owner: main;
independent adversarial review: Newton, native inherited-model subagent.

- [x] Reset gate and reread constructor-join interfaces/collection/handoff
  mapping, receiver-hazards member scan, construction-sites effect classifier,
  and real-index join tests. Source revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf,
  dirty fingerprint 80d95899ac9e68dda17ad2a988c026ad01f31d235b3b0d5d05ad9e7afc829519.
- [x] Adopt exact class/file/allocation/function identities and existing hazard
  evidence; traverse handoff states with a bounded worklist and deduplicate
  visited states. No name-based global guesses or graph writes.
- [x] Design: decisions are blocked / incomplete / observed-source-candidate,
  never safe or whole-program proof. Check constructor/member/prototype changes,
  instance escapes, constructor field transfers, function parameter shape/write
  flags and method targets. Retain missing evidence and budget exhaustion.
  Record accessor distinction and discarded direct values before consuming
  those ambiguous shapes; expose allocation class IDs and class reviews once
  per join rather than reconstructing import identity in the decision layer.
- [x] Real-index positive HTTP factory/dependency flow and negative mutation,
  escapes, unsupported handoffs, missing coverage, cycles/limits; Orders probe.

Native inherited-model read-only sidecar reviews collector gaps while main
implements. No inference edges activated; runtime/external-world assumptions
remain explicit even for an observed candidate.

## Implemented

receiver-decision.ts composes class reviews, member-this/prototype effects,
allocation uses and resolved function/constructor handoffs using a bounded
worklist. Decisions carry candidate IDs, issues, visited states and explicit
runtime-assumption requirement. Missing sites, unsupported shapes/handoffs,
unavailable files and state/effect exhaustion remain incomplete. Observed writes
block. Empty/discarded construction values are distinct from unknown escapes.
Class callable/nameKind data prevents accessors or field shadows from becoming
method targets. Explicit TypeScript this parameters remain unsupported shape.

Class reviews/allocation class IDs are exposed by the existing join so the
decision does not duplicate import resolution. orders-graph-smoke now records
the read-only receiver_decisions beside raw constructor evidence. No synthesis
pass calls the decision yet and no graph call edges are activated.

## Adversarial review and fixes

Newton (thread 01a094b2-d19e-75f2-997e-91dad68fe3e0) read current source/tests
under source-first rules, used in-memory diagnostic probes and made no edits.
Two bounded reviews identified missing distinctions; main added real-index
regressions and integrated the fixes. Review is not a general safety certificate.

- Alias depth and dynamic patch-only files could silently lose effects. New
  fileHazards → join.analysisHazards forces incomplete even with empty facts.
- Quoted/escaped member keys and method/field shadows are rejected unless
  supported unambiguously; escaped this paths also remain incomplete.
- Computed object-method key hazards must be consumed for constructors too.
- Unknown prototype keys block; prototype aliases remain incomplete.
- Shorthand {arguments} now flags its owning ordinary function.
- this.constructor / __proto__ / static this.prototype are explicit prototype
  boundaries, including reads that can escape to aliases.
- Main additionally marked runtime namespace-import uses incomplete until
  namespace member identities are supported. This limitation is not completion
  of namespace resolution; unsupported uses must not silently disappear.

Worker closed after findings were received. Main retained ownership of source
changes, regression verification and integration. Initial focused run: 19 pass;
after first fixes: 295 pass / 12 suites; final targeted decision/collector run:
137 pass / 3 suites (30 decision tests).

Resumed verification: polled the original full-suite session 55964; it completed
with exit 0. The machine-readable report records success=true, 4920 passed,
0 failed and 11 pending/skipped tests (4931 total), across 277 test files.
[Full-suite receipt](../.harness/baselines/codegraph-receiver-decision-20260912-01.json).
Recomputed source fingerprint matches 4d810429ed5f46f95810bee8d3fe9df68e0006174fd5ee7f51c406f0f478deb6;
git diff --check also exits 0. No implementation changed during this resumed
verification. This closes verification of the component, not synthesis activation
or end-to-end graph answer correctness. The prior goal turn made progress by
updating the mandatory source-first policy; this turn reconciles the existing
test handle and records its terminal result without restarting the suite.

## Orders observation after fixes

Emitting tsc passed. Final smoke command (product cwd):
node scripts/orders-graph-smoke.mjs /home/minh/projects/outsource/codegraph
.harness/baselines/orders-receiver-decision-20260912-02.json: exit 0.
[Final raw receipt](../.harness/baselines/orders-receiver-decision-20260912-02.json).
The -01 receipt predates the second adversarial fix round.

Three store fields are observed-source-candidate, traversing 18/18/13 states;
endpoint remains incomplete (scalar unknown origins/field-use-unmodeled).
Graph still 51 nodes/127 edges. Source/dist/fixture stable; watcher observed
without errors; all five probe processes exit 0, not proof of answer correctness.
Fingerprint 4d810429ed5f46f95810bee8d3fe9df68e0006174fd5ee7f51c406f0f478deb6.

Open before full W4 acceptance: synthesis call-site/target mapping and activation,
affected-source discovery + reconciliation, full flow verification and remaining
unsupported namespace/alias/extraction forms. Candidate is bounded evidence, not
runtime safety; W0–W13 completion remains unproved.

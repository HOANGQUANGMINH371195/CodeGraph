# W4 class-local receiver hazards

status: done (class-local hazard component only); source-gate: ready;
owner/reviewer: main, no independent review.

- [x] Reset gate and reread constructor-facts, constructor-join, their relevant
  tests, store-accessor lexical traversal/tree disposal, and actual orders
  service/relay constructors and methods.
- [x] Source before patch: CodeGraph revision
  `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty fingerprint
  `508a7a7f4cbad900a39d8b7aa07a23beb85b5ec2ff22a47a33fda3dc051072c8`.
- [x] Design before code: parse each file into bounded class-local reviews;
  retain hazard code + exact span/line for constructor returns, inheritance,
  decorators, dynamic evaluation, field writes/computed access, this escape,
  unmodeled receiver/field uses, and constructor parameter modifications/uses.

Adopt AST field matching, class extent identity and detached evidence from existing
collectors. Distinguish ordinary nested functions/static methods from lexical
arrow `this`. Check direct constructor assignment against AST, not only caller-
supplied offsets. Mark unmodeled constructor statements conservatively. Parameter
name checks may over-report nested shadowing; do not claim complete lexical flow.

Integrate owner-field and argument-constructor reviews into the candidate join.
Statuses are blocked/no-local-hazard/unavailable, never safe/verified runtime type.
External instance writes/escapes, method overrides, closed-world assumptions and
scoped stale-edge replacement remain required; requiresReceiverValidation stays
true. Tests cover mutations, aliases, computed/destructured writes, return scopes,
arrow/static/nested class ownership, malformed/stale facts, and real-index join.

## Delivered

Added `codegraph/src/resolution/receiver-hazards.ts` and its 39-case suite.
Constructor join now includes each owner's field `localReview` and each candidate
argument's `argumentConstructorReview`. Common constructor review covers explicit
return values, inheritance, decorators and dynamic evaluation; owner-field review
adds parameter/field uses, writes and escapes. Neither is a whole-program proof.
Hazards carry code, start/end offsets, line and column; trees are disposed before
return. Unknown/unavailable reviews are not equivalent to a clean review.

Three added regression cases initially failed: constructor parameter initializers,
nested-class heritage expressions and computed member names. These can evaluate
in an enclosing environment even though ordinary nested class methods have their
own this. Added explicit unmodeled-hazard checks and reran the full focused set.

## Verification

- `node node_modules/vitest/vitest.mjs run __tests__/receiver-hazards.test.ts
  __tests__/constructor-join.test.ts __tests__/construction-sites.test.ts
  __tests__/constructor-facts.test.ts __tests__/ts-this-field-call.test.ts
  __tests__/call-receiver-no-fabrication.test.ts __tests__/store-exported-later.test.ts`
  through product `scripts/with-local-tools`, CodeGraph cwd: exit 0,
  **135 passed** in seven files (39/27/37/21/4/3/4 respectively).
- `node node_modules/typescript/bin/tsc`: exit 0, updated emitted engine.
  `git diff --check`: exit 0. No full npm/UI/kernel or Rust foundation run.
- `node scripts/orders-graph-smoke.mjs /home/minh/projects/outsource/codegraph .harness/baselines/orders-local-receiver-20260912-01.json`
  from product root: exit 0.
  [Raw receipt](../.harness/baselines/orders-local-receiver-20260912-01.json).

Orders: OrderService.store, NotificationService.store and OutboxRelay.store all
report no-local-hazard. All ten observed OrderStore argument-constructor reviews
also report no-local-hazard. OutboxRelay.endpoint reports field-use-unmodeled;
this blocks class-receiver inference for a string URL, not execution of the app.
Source/dist/fixture unchanged during probe; watcher observed sync with no errors.
Source fingerprint `7ba17846b1be1418cf149500e01813bc763cedddb92e070ea23a3408325641d4`.

Graph remains **51 nodes/127 edges**, no new constructor-derived call edges.
`requiresReceiverValidation` stays true. External allocation writes/escapes,
prototype/method replacement and the conditions under which known construction
sites represent all possible receivers are not established. Class-expression
extraction/parity and scoped stale-edge replacement are also still open. Do not
turn no-local-hazard into a safe/executable/authoritative capability. W0/W4 full
acceptance remains unproved; next work is external allocation effects, then graph
activation with stale-edge reconciliation and unchanged-corpus flow verification.

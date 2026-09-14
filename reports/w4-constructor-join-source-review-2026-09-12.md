# W4 constructor evidence join

status: done (read-only candidate join only); source-gate: ready;
owner/reviewer: main, no independent review.

- [x] Reset gate; reread constructor-facts/construction-sites source and tests,
  import-resolver.ts:1590–1717 and 2320–2445, ResolutionContext/Node location
  contracts, ReferenceResolver.clearCaches/createEdges, synthesis registry/merge
  and QueryBuilder.deleteEdgesByIds. CodeGraph and product guidance applies.
- [x] Pre-code source: revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty
  fingerprint `18a527bd7705473e86ee807f13df9b9d59a06d5bc93e4aaa488b7e7b76834fc2`.
- [x] Decision: join constructor assignments to observed construction sites by
  exact indexed class extent, and imports by verified runtime mapping followed
  through existing import resolver (including re-exports). No global name fallback.

Source review changed the integration sequence: callback synthesis currently
deduplicates then only inserts edges. New constructor edges must be reconciled
when a wiring-only file changes, even if neither method endpoint changes. Enabling
additive edges now would retain stale relationships. Implement the read-only
candidate join first, preserving multiple assignments/sites, unknown actual args,
unmapped constructors and unavailable files. Receiver mutation/escape and atomic
scoped edge replacement remain required before runtime graph activation.

Reuse the real CodeGraph index and its ResolutionContext in tests; do not mock
import/class lookup. Tests planned: cross-file imports/default/rename/re-export,
same-name unrelated classes, alias chains, positional arguments, multiple known
and unknown sites, changing wiring and parse errors. Candidate source hashes and
offsets are evidence pointers, not a verified immutable project snapshot or type.
Measure candidates on the unchanged orders fixture; do not claim flow completion.

## Delivered and observed limitations

Added `codegraph/src/resolution/constructor-join.ts` and six real-index integration
tests. The join retains source hashes, unmapped assignments, unavailable files,
unknown/unassigned construction sites, repeated assignments and multiple observed
arguments. It does not write graph edges or claim an immutable project snapshot.
Import memoization lives only within one collection, with cooperative yields.

Further source review found `import-resolver.ts:107–139,2384` can choose the first
exported function/class for a default import when the exact default binding is
unavailable. Consequently “exact” above describes the intended contract, NOT a
proven property of every imported candidate. The result explicitly carries
`requiresImportValidation: true` as well as `requiresReceiverValidation: true`.
Strict AST export identity (including conflicting wildcard re-exports) is required
before activation; a successful upstream import lookup is not sufficient proof.

## Checks

- Five suites (`constructor-join`, `construction-sites`, `constructor-facts`,
  `ts-this-field-call`, `call-receiver-no-fabrication`): **71 passed**, exit 0,
  using `scripts/with-local-tools node node_modules/vitest/vitest.mjs run ...`
  from CodeGraph. Latest run includes the grammar lifecycle correction.
- `npm run build` through the same wrapper: exit 0, copied 29 grammars and built
  viewer. After the final join-only lifecycle change, `node
  node_modules/typescript/bin/tsc`: exit 0, emitted updated engine module.
- `node --check scripts/orders-graph-smoke.mjs` and upstream `git diff --check`:
  exit 0. No full upstream or Rust foundation suite rerun.

Extended existing product smoke wrapper with read-only candidate observation;
it still uses upstream explore probes, not a second benchmark scoring engine.

Initial orders run
[`orders-constructor-join-20260912-01.json`](../.harness/baselines/orders-constructor-join-20260912-01.json)
reported all six JS-family files unavailable. Kernel-first indexing had not loaded
the WASM grammars (unit tests had preloaded them). Read grammars.ts:404–438 and
fixed the async join to load only indexed JS-family grammars. Retain run 01 as
failure evidence; its successful smoke exit did NOT prove candidate collection.

Final command:
`node scripts/orders-graph-smoke.mjs /home/minh/projects/outsource/codegraph .harness/baselines/orders-constructor-join-20260912-02.json`
from product root: exit 0.
[Raw receipt](../.harness/baselines/orders-constructor-join-20260912-02.json).

| Field | Observed candidate class | Sites |
|---|---|---|
| OrderService.store | OrderStore | 1 main + 4 test |
| NotificationService.store | OrderStore | 1 main + 1 test |
| OutboxRelay.store | OrderStore | 1 main + 2 test |
| OutboxRelay.endpoint | unknown-origin | 1 main + 2 test |

Unavailable files/unmapped assignments: zero. Test and main wiring retain separate
file/line evidence; test sites do not prove production execution. Source, dist and
fixture stable during final probe; native watcher observed sync with no errors.
Source fingerprint `cba6432f5af7a38e2ddb1703c0a1307abfbbeaea7ab76caa8ff0ff4873e2ee64`.
Built native-kernel correspondence remains unverified (receipt says false).

The graph still contains **51 nodes/127 edges**: no service→store call edges were
added. Explore probe exit codes are not sufficiency scores. Required next work:
strict import/export candidate validation; constructor returns and receiver writes/
escapes; scoped stale-edge replacement on wiring-only sync; then activate and
remeasure full orders flow. W0/W4 and all full-package acceptance remain open.

# Route-conditioned call flow — before-code source receipt

Current CodeGraph HEAD `3ed73bc127323e63153bf6ec8354afa82ce36aaf` was inspected.
An owned Orders fixture freshly indexed through the emitted build proved that
plain findPath(calls) reaches createOrder from BOTH /orders and /dispatch: both
route nodes point to one callback. This is structural reachability, not request
feasibility. Do not turn the latter path into a request sequence diagram.

Source reread before implementation:

- framework/node-http.ts complete (actual path src/resolution/frameworks/):
  SHA `ba521e1cb44769f8f482abb172567574450efbf28f8addd907859756d0f9acf2`.
  Exact callback spans, request mutation/shadowing guards, source hash and strict
  literal method/url predicates. Re-extract the selected route from a bounded
  hash-matching source buffer to validate its current scope, do not trust stored
  metadata as a runtime token. Do not change extraction or global call edges.
- graph/branch-guards.ts public guards, guardsInTree and innermostAt:
  SHA `68ae2056a199e87e30dc7d8cbdfd721029e9f32bd76dc0e2fc1d65e708f40ef2`.
  Reuse branch identities/negation and early-exit handling. Display text is
  truncated at 80 chars: NEVER evaluate the display string as a predicate.
  Resolve branch coordinates to the actual AST condition instead.
- __tests__/branch-guards.test.ts:1–105,204–271, SHA
  `847956da9309db045e52c4daf1592585e627bd4ed298ee6e09888a81be0a6dcd`:
  if/else share a fork identity; negative early return guards continuation;
  named function boundaries stop propagation. __tests__/node-http-routes.test.ts
  positive/negative/sync fixtures, SHA
  `d4d1a8adcb9ff17fea0d5c162183d0086250cd886a904f39ef1315e03ba3dd15`.
- graph/traversal.ts findPath, SHA
  `d6c7d5d38ace73a6105e419c68cfce9581bc5f5ed1c3166c1fda876b15d5ad71`;
  graph/named-symbol-flow.ts token binding and directed/named walks, SHA
  `55cf4c23e4bb1be70b2435593b51988085444a538616108c192046f042373f2b`.
  Existing findPath has no visit cap or route condition; named flow accepts
  symbol tokens, not HTTP route IDs. A dedicated bounded query derivation is
  needed; raw graph APIs remain structural and unchanged.
- ui-server/api/source.ts readFileShape/hash drift, utils.ts root path check;
  adapt bounded regular-file descriptor capture (fstat + cap+1 read), not a
  stat-only freshness verdict or an unbounded read after a size race.
- bin/codegraph.ts context command, SHA
  `c5240503ff4da632f649efa2fa9594146a2417c826ee381e8e78bea6fa4f73af`:
  reuse existing context CLI surface with explicit --route/--to ID mode, JSON
  only and no source body; preserve legacy task context behavior. domain-cli
  informs data-only stdout and explicit invalid-option errors.

## Contract and gaps before patches

CodeGraph.getRouteFlow(routeId, targetId, {maxDepth?, maxVisited?}) returns a
bounded JSON-ready report: kind route_flow, status candidate|not_found|incomplete,
exact node/edge path, excluded handler-call count, unknown-condition count,
route source hash/check marker, index build info and static/runtime caveats.
Defaults depth12 (max32), visited500 (max5000). Calls only; no containment,
imports or factory registration-as-execution. Missing/unsupported route, stale
source or unavailable coordinates must not fall back to raw reachability.

Within the selected callback, shared branch guard identities locate untruncated
AST conditions. Evaluate only literal request.method/request.url comparisons,
boolean literals, !, && and || using three-valued logic. Known-false guards
exclude that call; unknown guards remain explicit candidate assumptions. Hash
matching checks the route file only, not other files/the whole index. Backend
may read/parse bounded source; agent receives no full source bodies.

The new query is not a whole-program path feasibility solver, SQL mapper,
generic natural-language diagram router, or renderer. It is a prerequisite to
honest route-to-storage flow compilation. Tests must distinguish same-callback
sibling routes, negative guards, unknowns, drift and limits. No extraction
version/schema bump because the stored graph is unchanged.

Before CLI regression implementation parent also read the complete current
`__tests__/cli-context-command.test.ts`: real emitted binary against a real
temporary index, no daemon/relaunch, JSON stdout. Adopt this boundary test and
preserve the existing six legacy context assertions. New route tests will check
exact byte budgets, invalid option combinations, hash drift, source-hash
metadata and sibling-route exclusion through the CLI, not only the TS API.

## Resumption review before correction

Reread current route-flow.ts, route-conditions.ts, complete Luna route-flow
tests and branch-guards.ts public contract. An out-of-handler coordinate was
returned as incompatible with one unknown, then counted as an excluded call.
Adopt the existing missing-coordinate incomplete behavior for this case too:
unknown location is not proof of a contradictory condition. Represent this
separately with an unavailable marker; add a direct AST regression before
considering route-flow verification complete. Raw findPath stays unchanged.

Before extending the Orders diagnostic, reread the complete existing
scripts/orders-graph-smoke.mjs and fixture http.mjs, relay.mjs, store.mjs.
Reuse its fresh owned index, existing emitted public API and unchanged five
benchmark questions; add route-ID observations with expected statuses, not a
replacement scoring engine. Check /orders→createOrder, /dispatch→dispatch,
/events→consume and the negative /dispatch→createOrder. This does not claim
SQL file/table mapping or cross-service delivery has been solved.

## Verification and remaining acceptance

Parent reviewed Luna's complete five-test file and reran it with the CLI,
legacy context, route extraction, branch guards and new coordinate regression:
54 tests passed across six files. TypeScript emission passed after the
coordinate correction; git diff --check passed. The first attempted test
launcher failed because npx is absent; the direct installed Vitest entrypoint
was then used successfully. No dependency install was needed.

Fresh emitted Orders receipt:
`.harness/baselines/orders-route-flow-20260912-01.json` (exit 0).
All four route expectations matched: orders→OrderService.create→createOrder,
dispatch→OutboxRelay.dispatch, events→NotificationService.consume→store.consume
are candidates; dispatch→createOrder is not_found. Watch refresh passed;
source/dist/fixture remained unchanged. Existing five probes exited 0, but
the generic diagram probe still says No relevant code found. Exit 0 is NOT
semantic benchmark success; parameter reconciliation remains incomplete.

Final production SHA-256:
- route-flow.ts: `c9f96e322f8a8fd60e01c7c7f28da21719f28c4d25cbb2258c484f3a62b7ea69`
- route-conditions.ts: `da6c3b66828783f016f00896a0854d6c73d06a847599aae6f82bb25a1acf95d7`

Luna worker `01a09652-3100-7230-9b7b-53220697150a` requested
gpt-5.6-luna, effective model unverified; completed worker closed. Parent
owns integration and coordinate regression. The domain-cli skill informed
JSON-only stdout, explicit option validation and nonzero transport errors.

Full current suite receipt `.harness/baselines/codegraph-route-flow-20260912-01.json`:
exit 0, 5353 passed / 0 failed / 11 skipped, 5364 tests across 305 files,
284.9 seconds. Before/after fingerprint both
`d98078361cdf963b2d5fe8f4589a92fd31d513b6f0226404c5be25d71e0f639c`.
This invocation did not set CODEGRAPH_KERNEL_EXPECT: two presence checks were
skipped, along with platform gates and the self-index-dependent busiest-symbol
case. The fixture busy-symbol latency test ran and passed; no threshold changed.
Separate CODEGRAPH_KERNEL_EXPECT=1 kernel-scaffold run: 10/10 passed, exit 0.
Separate CODEGRAPH_KERNEL_EXPECT=1 kernel-deep-nesting run: 8/8 passed,
exit 0 (53.87 seconds), including the presence gate and emitted worker/CLI.
Do not mislabel the full run as having every conditional gate enabled.

W4/W11 remain incomplete regardless
of focused results. No request→SQL graph compilation, general intent routing,
whole-system architecture rendering or runtime verification is claimed here.

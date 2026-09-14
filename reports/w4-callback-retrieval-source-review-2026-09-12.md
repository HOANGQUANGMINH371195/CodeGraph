# W4 callback retrieval source review — 2026-09-12

Owner: delegated retrieval agent. Scope: `codegraph/src/mcp/tools.ts` and this
receipt only. Main owns AST fixes and all test execution. No tests, application
workers, recursive agents, or test-file edits are authorized here.

## Source gate before implementation

Read product and CodeGraph root AGENTS.md in the preceding review; revisited
current source for this disjoint task. CodeGraph HEAD:
`3ed73bc127323e63153bf6ec8354afa82ce36aaf`. Pre-edit tools.ts SHA-256:
`581a5169b3cdcaca40fbd9807a40161e63a15f858720f6a2a983af8184523c54`.
The file already contains dirty output-limit, summary and session-emission edits;
preserve them. This task edits TypeScript only; no Rust skill applies.

Source reads: tools.ts `handleExplore` gather/glue/named seeds (3389–3680),
per-node file scoring (3683–3815), score floor/RWR/signature-type rescue
(3919–4110), relationships/allocation and reservation holdback (4180–4612),
whole-file buy and fit (4920–5064), range construction, cluster shrinking and
focus-line ceiling windows (5079–5533). Also read current context/index.ts hybrid
search and src/index.ts findRelevantContext entry, plus the CG-21 tests in
explore-allocation-e2e.test.ts, CG-31 explore-displacement-guard.test.ts and
explore-factory-closure.test.ts and relevant fixture source.

Main's terminal receipt is `.harness/baselines/codegraph-callback-ownership-20260912-01.json`
under the product repository (5202 passed / 17 failed / 9 skipped). Its seven
failures in these three files establish:

- CG-21 etag.ts renders clusters, delivers 521 versus 8751 source characters,
  and underspends its 6610 reservation. All fixture-shape assertions passed.
- CG-31 types.ts is not admitted; the giant's spendable allocation grew beyond
  the fixture's concentration bound; required peers deliver zero source.
- Factory closure delivers 14 indexed inner definitions; the unchanged gate
  requires 16 after additional callback definitions became indexed.

Source-level causes / adaptation:

- Each new callback currently contributes another callable relevance vote even
  when nested in a declaration already scored. Use one maximum vote per actual
  containing declaration, preserving each original graph node and call edge.
- Source shrinking orders individual ranges by importance then size, making tiny
  anonymous fragments compete against their enclosing definitions. Group these
  by actual contains ancestry and corroborating source spans; carry the strongest
  relevance/spine evidence to the containing source selection. Explicit location
  queries must still be able to select the anonymous node itself.
- A pending tiny file is held back at its entire reservation even when its full
  numbered source cannot spend that much. Bound this holdback by available source,
  retaining header overhead and existing ceilings; do not tune constants.
- Inspect dropped signature-type files against actual named-callable signature
  edges. Preserve relevant signature context when node proliferation changes the
  initial gather/floor, without inventing invocation or binding facts.
- The CG-31 fixture's four named stages each call sink.ts `writeBatch` directly
  (ingest:539, normalize/enrich/publish:113). Count distinct named source units
  supporting a callee, bounded at the existing search-entry weight, rather than
  giving a shared pipeline dependency the same vote as a one-off neighbor. These
  are existing calls edges; callback siblings from one declaration must not add
  independent support.

Adopt the existing isDeferredCallableName helper (main will extend it for default
callables), contains edges, source spans, scoring tiers, cluster/focus machinery,
and per-file funding guards. Avoid label-specific ranking penalties, graph-node
removal, calls reattribution, higher budgets, or weakened quality assertions.

## Validation ownership

Pending main: TypeScript build and the three unchanged regression files; then
broader explore ranking, output-budget, session-dedup and callback ownership
checks. No passing post-change results are claimed by this agent.

## Implemented handoff

Final tools.ts SHA-256:
`86ed427d6c75c8c5f7f5896d13c84b943752f79d5649c272bd604df4608c00b8`.

- `sourceUnit` (3430): cached, unambiguous contains ancestry with same-file,
  full line/column containment. Stops at a real declaration; standalone anonymous
  scopes and explicit location queries keep their own source identity. No graph
  node or edge is modified, and RWR still reads the original graph.
- `namedCallSupport` / `unitScores` (3796/3821): maximum existing-tier vote per
  source unit. Actual calls from distinct named stages corroborate shared
  dependencies, capped at the existing search-entry tier (10). Source siblings
  and duplicate call sites do not multiply that corroboration.
- Signature rescue (4105): an actual named-callable signature dependency whose
  file was removed by the score floor qualifies for the existing rescue path.
- Section headers (4539/5091) use the same containing source units as selection.
- Source funding (4573): pending source holdback is capped at the available
  numbered source length, retaining section overhead. Read/validation failures
  keep the full reservation. Existing output, share, buy and displacement
  thresholds are unchanged.
- `unitRanges` (5190) groups contained anonymous spans before cluster shrinking;
  strongest importance and real spine call locations are retained. Focus-line
  selection (5567) also retains gathered callback entry locations under the
  existing six-focus limit. No invocation edge or named callback binding is
  inferred by this source presentation.

Executed validation: `git diff --check -- src/mcp/tools.ts` passed. Reviewed the
scoped diff against the pre-existing dirty output-limit/session changes; those
changes are preserved. No compiler, tests, app workers, or agents were run.

Main must validate the three unchanged suites first. The seven failing baseline
assertions are not yet shown passing. Broader checks should especially cover
allocation proportions, named-signature rescue, exact anonymous-location queries,
same-line nested scopes, factory-closure depth/coverage and session source ranges.
The funding adjustment reads pending candidate source once per file to establish
its capacity; it is cached for this response. Main's helper extension determines
recognition of forthcoming `<default@...>` nodes.

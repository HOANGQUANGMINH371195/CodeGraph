# W4 UI node latency — source-first investigation

Status: component fix verified by focused tests and one final full-suite run;
not W4 or product acceptance.

Source gate reopened on resume. Main reread CodeGraph root AGENTS, the
m10-performance skill, api/node.ts buildNode and all summary helpers,
api/when.ts annotateWhen, branch-guards.ts guardsForFile/treeFor,
graph/traversal.ts getCallersRecursive/getImpactRecursive, the real loopback
500-caller fixture and its unchanged 100 ms assertion, and Vitest configs.
Source revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf, dirty fingerprint
e58efdbc331cd64611bae8a9149a5fa60423650a6c07095658b770e90f49d2fe.

Rechecked full-suite -03 JSON: 4976 passed / 1 failed / 4990 total. Same
ui-server-api warm endpoint assertion failed at 137.566141 ms; -02 failed at
187.192297 ms. Three isolated passes do not close this repeated failure.

Flow: HTTP handler -> buildNode -> batched endpoints/rails -> annotateWhen
(per-file cached trees, per-site guard walk) -> fan-in -> test caller search
and recursive blast traversal -> JSON. Test search still calls getCallers per
frontier node, and blast re-fetches nodes and incoming edges recursively.
These are hypotheses, not measured bottlenecks yet.

Adopt: existing real indexed 500-caller fixture, CodeGraph API and existing
buildNode implementation; product source-fingerprint and owned-temp patterns.
Avoid: widening timing threshold, dropping semantics, repeated runs for green,
or a persistent response cache that hides stale graph/source evidence.
Gap: existing test reports only total latency. Add a diagnostic script that
times existing endpoint components with aggregate method counts and samples,
without modifying production code. It measures current dist, explicitly not
proof of source/build correspondence or the real HTTP SLA. Use results to
select any implementation change and its semantic regression tests.

- [x] Read current source and assertions before diagnostic implementation.
- [x] Record flow, adoption and measurement limits before patch.
- [x] Measure and identify a specific bottleneck.
- [x] Record chosen fix and regression coverage before production patch.
- [x] Verify unchanged functional and latency gates; preserve failed receipts.

## Pre-code decision after component measurement

Diagnostic receipt `.harness/baselines/ui-node-component-20260912-01.json`:
10 warm samples, buildNode median 11.521 ms (10.158–13.281), impact median
4.196 ms, guards 1.273 ms, 64 caller queries 1.163 ms. Timings are inclusive;
this isolated dist fixture does not establish the full-suite scheduling cause.
Impact is the largest measured component; source confirms one incoming lookup
per expanded node despite batching rail endpoints. Address this demonstrated
N+1 path rather than changing guards based on speculation.

Additional source read: graph.test.ts #536/#1089 and in-memory topology helper;
queries.ts getIncomingEdgesTo/getOutgoingEdgesFrom/getNodesByIds (existing
chunked, filtered batches); synthesized-edge-replacement.test.ts real SQLite
fixture lifecycle. Adopt those batch APIs and real-storage tests, no new SQL.
Replace recursive impact walk with depth layers, zero-cost containment closure
before dependency expansion; each target expanded once at its shortest depth.
Preserve all distinct dependency call-site edges and exclude upward contains.
No cross-request cache, cap reduction or swallowed storage errors. Depth-boundary
nodes are included but not expanded, preserving existing maxDepth semantics.
This also must fix DFS discovery at a longer path suppressing shorter-path
expansion; regression must prove it before changing production traversal.
Test real SQLite with 600 callers for batch query counts/chunking, reversed
edge insertion and unequal paths, container cycles/diamonds, parallel sites,
missing roots and depth boundaries. Existing mock helper needs the same batch
query interface; it is not substituted for real SQLite coverage.

Read-only reviewer James independently confirmed shortest-path and containment
closure pitfalls and missing batch methods in the old in-memory helper. Main
read the referenced source and added containment-before-dependency and cyclic
parallel-callsite cases. Explicit output improvement: retain each actual
containment edge of expanded containers, including shared children/cycles,
instead of the old first-discovery spanning tree. Such edges are real graph
facts, not extra reachability; each container expanded once prevents duplicates.
The initial red fixture also used a nonexistent deleteEdgesFrom name; corrected
to the existing deleteEdgesBySource before accepting the red baseline.

## Implementation and focused verification

Corrected red run `impact-batching-red-20260912-02.json`: 4 passed / 4 failed.
Failures proved 601 scalar incoming lookups for 600 callers, missed shorter-path
dependent, dropped shared/cyclic containment edges, and missed zero-hop closure
dependent. No missing-method error remains in this accepted red baseline.

Changed only GraphTraverser impact traversal, its old test helper's batch API,
and added real SQLite regression file. No SQL/schema or timing-limit changes.
Focused run `impact-batching-focused-20260912-01.json`: 101 passed / 1 skipped,
three files (impact, graph, UI API); the skipped case requires the engine's own
index and is pre-existing. The real loopback 500-caller <100 ms assertion
passed in this focused run. Emitted tsc and git diff --check passed.

Same diagnostic after compilation, `ui-node-component-20260912-02.json`:
source c2b59598cca3ebee9b548d010f4731eeb7ed22c3affe51b44b9e9ce83c7dc4a8,
unchanged throughout probe. Ten warm samples: buildNode median 10.380 ms,
impact median 2.822 ms (versus 11.521 / 4.196 before). This modest isolated
improvement and query-count bound do NOT prove the full-suite latency failure
fixed. Still requires full suite at the final source with default concurrency.

Reviewer James reread the patch and found no blocking correctness issue. Main
added the recommended wide-containment/cold-cache regression (600 classes +
600 methods, real SQLite chunks and bounded batch counts) and exact edge-site
multiset assertion. Final focused -02: 102 passed / 1 existing skip, tsc
--noEmit and diff-check exit 0. Reviewer closed; no workers running.
Remaining robustness coverage: very deep containment and dangling endpoints;
full layer results still materialize, so depth alone is not a memory bound.

Full suite launched with original default config, no concurrent build/probe:
`.harness/baselines/codegraph-impact-batching-20260912-01.json`.
Frozen source fingerprint:
f7796c1fc72cd55def44c4abf784fb50fe1e32adda38aecf89813f17ec80fae3.
Handle 64285 completed exit 0: **4986 passed / 11 skipped / 2 TODO**, 4999
total tests in 281 files; report success true. The original real-loopback
500-caller <100 ms assertion passed. Source fingerprint rechecked after the
run equals the launch fingerprint. This is one verified full-suite run, not
a guarantee of latency on arbitrary machines or under every scheduler load.
Previous failing receipts remain intact. No running test handle remains.

Orders smoke `orders-impact-batching-20260912-01.json` completed exit 0 after
the full suite: 51 nodes / 131 edges, watcher observed, no watcher errors,
all five probe subprocesses exit 0. Source/dist/fixture unchanged throughout.
Index injectedCalls remains incomplete (4 edges, 1 issue); diagram stdout is
still `No relevant code found`. This verifies smoke mechanics, not diagram
quality, HTTP dispatch coverage or product acceptance. Only invocation-owned
temporary fixture copies were removed; original source fixtures preserved.

## Next-task navigation only (source gate remains pending)

While the frozen full suite ran, main used rust-router and reread the actual
TS generateNodeId, Rust ids::node_id, both TS/JS createNode paths, the anonymous
function skips, kernel-scaffold ID assertion, TS/JS parity harness, extraction
version stamp and upgrade tests. No Rust or extractor code changed here.
Both ID helpers hash file/kind/name/line only; column is absent. Rust's col_of
converts positions to UTF-16 like wasm, which a position-based ID must preserve.
The helper is shared across many native language walkers and separate TS
extractors: changing only TS/JS or only the shared hash risks cross-path drift.
Existing parity uses nodes/edges/references, not merely node counts.

Important upgrade caveat for that next task: storeResult skips equal content
hashes, while indexAll can stamp a newer extraction version after filesIndexed
is positive. A version bump alone is not proof that old IDs were rebuilt.
Need to inspect/retest forced reindex and old-index transitions before any ID
change. Returned anonymous arrows intentionally currently walk their bodies
without their own callable node in both implementations; new naming alone
must not invent an invocation edge from the enclosing function. These are
navigation findings, not a completed extraction source-study or acceptance.

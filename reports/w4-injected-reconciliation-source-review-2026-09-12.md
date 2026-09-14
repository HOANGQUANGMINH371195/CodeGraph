# W4 injected call reconciliation lifecycle

status: verifying; owner: main; source-gate: ready (reset for this task).
Source CodeGraph HEAD 3ed73bc127323e63153bf6ec8354afa82ce36aaf,
dirty fingerprint 7dc017125196c20820aaed231f908e0bdc8a6804138029a8312ac69aaa315362.

- [x] Reread indexAll/sync lock, extraction, deferred resolution, maintenance
  and error paths in src/index.ts; resolver clearCaches/readFileCached;
  constructor join/mapper interfaces and source/hash checks.
- [x] Read QueryBuilder.replaceSynthesizedEdges, get/setMetadata, FileRecord
  and storeExtractionResult/buildFileRecord hash/error storage;
  synthesized-edge-replacement.test.ts and sync-rebuild-convergence.test.ts
  fresh-recreate assertion strategy. Read SQLite nested transaction flattening.
- [x] Read SQL asset loading in db/index.ts and package.json copy-assets before
  adding a query file (no new embedded SQL string or schema migration needed).
- [x] Adopt existing writer/file locks, source SHA-256, exact mapper identities
  and producer-scoped replacement. Avoid legacy pair-only additive merge and
  batched-resolution-only trigger; both miss changed-file/removal-only sync.

## Pre-code decision

Clear old producer edges and record a running marker atomically before changing
the index, then collect fresh evidence after deferred resolution on indexAll
and sync. This is intentionally fail-closed: a failed/crashed refresh cannot
leave old heuristic call edges looking current in ordinary graph queries.
Prior sidecar suggested preserving old edges with a stale flag; current query
surfaces do not enforce such a flag, so that alternative is not sufficient.
Other producers/static edges remain untouched. Record failed/running states and
retry even no-change sync. A completed observation may still have unsupported
coverage issues and must not be called full-graph or runtime validation.

Source freshness: bind source read by join to indexed FileRecord.contentHash,
reject extraction errors/missing source, and re-read uncached disk before the
final synchronous edge+receipt commit. Indexed-source coverage only; disk edits
after validation and files outside the indexed inventory are not disproven.
Persist the source manifest once, not per edge. Use existing metadata table;
store owned-source SELECT in a SQL asset and copy it with build assets.

Regressions: wiring-only A→B with unchanged caller/target, removal, unsupported
wiring, no-change retry, stale file hash, extraction errors, transaction failure,
coordinate-distinct calls, foreign ownership, fresh rebuild equivalence and
no-JS empty cleanup. New lifecycle tests must exercise CodeGraph methods, not
only a mocked orchestration function. Independent native subagent reviews the
snapshot/failure boundary; main owns implementation and verification.

Pre-config-extension study: main read path-aliases.ts and workspace-packages.ts
completely plus tsconfig-extends-aliases.test.ts. The independent reviewer found
resolver-lifetime alias/workspace caches survive clearCaches, and config loaders
swallow missing parents/parse failures. Adopt the existing precedence/glob logic
through an observable filesystem seam; reload configs per reconciliation and
record/revalidate content, directory lookups and relevant absence. Propagate
loader uncertainty instead of silently accepting partial config. Include
config-only rewire, config creation/deletion and parent-cycle tests. This is a
new required part of the current freshness task, recorded before editing those
loaders; no second alias implementation will be written.

## Current implementation

- New producer snapshot helper derives all old source IDs from stored ownership
  using src/db/synthesized-source-ids.sql, calls scoped replacement and commits
  the marker in the same transaction. Empty producer snapshots still have a
  durable marker. Build copy-assets now includes every root db/*.sql asset.
- indexAll/sync clear this producer before extraction, then invoke independent
  reconciliation after deferred resolution, including no-change/removal-only
  syncs. Results expose injectedCalls and persist the observation in existing
  project_metadata. Begin-transaction failure propagates before extraction;
  later analysis/commit failures leave no owned edges and failed/running state.
- Freshness checks all indexed JS-family FileRecords, uncached source reads,
  extraction errors, final inventory and content hashes. Source symlinks outside
  the project are rejected. The pass checks cancellation during join yields and
  before final commit. Existing sync extraction itself still has its upstream
  cancellation limitations; this does not claim general terminal/job cancel.
- ConfigIO observation reuses existing alias/workspace loaders with per-run
  state, tracks relevant absence/contents/directory and file lookups, and
  revalidates before commit. Alias partial reads/cycles and malformed workspace
  JSON are surfaced as uncertainty. Config observations have their own digest
  carried by candidate edges; source and config manifests are stored once.
- Direct field calls are now activated. Parameter dispatch, returned arrows,
  whole-flow query evaluation and all broader W4/W0 acceptance remain open.

## Verification so far

Initial lifecycle run: 8 failures, traced to same-line cross-class method IDs:
indexing `class A {send(){}} class B {send(){}}` stores only B::send. A's target
is correctly unmapped; no edge to the surviving wrong method is invented.
The lifecycle fixture now places the classes on separate lines, while a
dedicated negative test reproduces the ID collision and an explicit TODO keeps
the extraction requirement open. This is not a claim that collision is fixed.

- Existing replacement + convergence and lifecycle: 38 passed / 1 TODO,
  three files. Fresh-rebuild test uses a recreated database, not indexAll over
  unchanged rows.
- Final focused run before full suite: 99 passed / 2 TODO, six files
  (injected-reconciliation 17 passed/1 TODO; config-observation 11;
  aliases 11; receiver-decision 30; mapper 14/1 TODO; replacement 16).
- tsc initially identified union narrowing errors in the new observation
  formatter; corrected before final compilation. Subsequent noEmit passed;
  latest emitted build/full suite still being reconciled with live handles.
- copy-assets succeeded and cmp verified the SQL source/dist asset; diff-check
  passed. No version bump or publish operation performed.

Full suite original handle 27971 writes
.harness/baselines/codegraph-injected-reconciliation-20260912-01.json.
Source fingerprint at launch:
b11fea710836ec126564872f4965cd22fd3cccc8c38d3dd159cb573ed6bc972d.
The first full-suite handle completed with exit 1: 4961 passed, 1 failed,
11 skipped and 2 TODO (4975 total, 280 files). Only failure was the old join test
asserting an empty callee list after indexAll; activation now legitimately adds
that edge. Updated the assertion to compare graph contents before/after the
read-only join, retaining the actual no-write contract. Do not treat this first
run as final-source verification: review fixes were made during that run.

## Independent follow-up findings and final-source checks

Boyle completed two read-only review rounds and was closed. The second round
identified malformed pnpm YAML/OHPM recovery, partial glob expansion, duplicate
workspace names, invalid alias field types and silent OHPM depth truncation.
Main added uncertainty reporting and regression cases for each; unsupported
glob/YAML forms remain explicit gaps, not completed resolver capabilities.
JSONC diagnostics are consumed on observed alias/OHPM paths. Also corrected
legacy trailing-comma stripping so quoted literal targets are not rewritten.
Null config, invalid extends entries and zero-field analysis hazards had already
been addressed while the reviewer was inspecting the changing source; their
regressions are retained. Review findings are not independent certification of
the final changed source.

Final focused check: constructor-join/config-observation/lifecycle, **83 passed /
1 TODO**, three files, exit 0. Latest emitted tsc completed exit 0. Source frozen
for final full-suite handle **64397**, report
.harness/baselines/codegraph-injected-reconciliation-20260912-02.json,
fingerprint **e58efdbc331cd64611bae8a9149a5fa60423650a6c07095658b770e90f49d2fe**.
Handle 64397 completed exit 1: 4976 passed, 1 failed, 11 skipped and 2 TODO,
4990 total tests / 280 files. The sole failure was
ui-server-api.test.ts, busiest-symbol latency: 187.192297 ms vs the unchanged
100 ms warm-request threshold. No functional assertion failed in that run.

Resumed performance triage (source unchanged): read the test request/timing
scope, buildNode batched endpoint resolution/annotation flow and Vitest workspace
configuration. Following m10-performance, measured before changing code. Three
isolated invocations of the exact test each passed its original 100 ms assertion:
ui-node-latency-20260912-01.json, -02.json and -03.json in .harness/baselines.
Reporter test durations include search/warm/request/assertions and are NOT the
single endpoint latency; do not substitute them for that measurement. The UI
handler and test have no worktree diff. Contention is a plausible inference,
not a demonstrated cause or a fixed performance regression.

Full suite re-run at the SAME source fingerprint and SAME default configuration,
with no concurrent build/probe/test workload from this agent: handle **12691**,
.harness/baselines/codegraph-injected-reconciliation-20260912-03.json. This
handle completed exit 1: 4976 passed / 1 failed / 11 skipped / 2 TODO,
4990 total tests. The same unchanged 100 ms UI assertion failed at
137.566141 ms. Rechecked terminal JSON on resume; no source changes or relaxed
assertions were made during that run. Repeated failure led to component
profiling, not more identical full-suite retries. See
[UI performance source review](w4-ui-node-performance-source-review-2026-09-12.md)
for measured impact traversal batching and its independent validation.

[Latest Orders receipt](../.harness/baselines/orders-injected-reconciliation-20260912-02.json):
exit 0, 51 nodes / 131 edges (four new producer-owned calls); index observation
incomplete with one endpoint issue. The four service/relay → OrderStore pairs
are now actual getCallees results, not just mapper candidates. Watcher observed;
all five probe subprocesses exited 0. The diagram probe still returns
"No relevant code found"; complete HTTP parameter-dispatch flow and product
acceptance remain unproved. The -01 smoke predates final config review fixes.

Read-only next-task investigation: both tree-sitter.ts createNode and
codegraph-kernel/src/tsjs/mod.rs create_node derive IDs from file/kind/name/line,
before adding qualifiedName; column is stored but not part of that identity.
The kernel decoder preserves the provided ID rather than creating the collision.
This supports the cause of the observed same-line collision; no ID scheme or
Rust implementation was changed in this verification turn.

Follow-up full suite after the measured impact batching fix completed exit 0:
`codegraph-impact-batching-20260912-01.json`, 4986 passed / 11 skipped / 2 TODO
in 281 files, original latency assertion passed, source fingerprint unchanged
at f7796c1fc72cd55def44c4abf784fb50fe1e32adda38aecf89813f17ec80fae3.
This closes this snapshot's full-suite verification, not whole-program coverage.

Remaining: returned-arrow and same-line
method-ID extraction gaps, incoming parameter-call coverage/dispatch, full Orders
flow and diagram evaluation, larger-repository cost of whole-index reanalysis,
unsupported workspace/config forms and portable verification. This does not
complete W4 or any W0–W13 package.

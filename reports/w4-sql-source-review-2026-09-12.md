# SQL linkage source gate and acceptance baseline

Read before modifying the Orders probe or authored ground truth.

## Current outsource evidence

- Joern revision `7c1163d96705d354d7c1957531487a63af34dda6`:
  Java SQLInjection.sqlInjection (complete source), SHA
  `c045de11062ffe3bb302e303580c620027a478e32c8a6a1db5b53bbba7e136fc`,
  traces framework method parameters to query parameters with reachableBy.
  PHP SQLInjection likewise traces request values to query arguments. These
  are taint queries, not a SQL file/table dependency extractor.
  XConfigFileCreationPass (complete source), SHA
  `71286c4e8b5d4a9f9e4e78cc9571591f4018b68cd91434f8d9b70df3a7b60370`,
  stores config name/content; its listed Java filters include MyBatis XML,
  not generic .sql linkage. Adopt source identity and explicit adapter scope;
  do not confuse config inclusion with a proven execution edge.
- Zvec revision `67ea1fa65ff99ee4c3a5bf4f2c0a6799c0390ead`:
  zvec_sql_parser.cc parse/sql_info/sql_type/select_info inspected, SHA
  `547709901cf43b466bf4625647ed1d8170233b6cebad1b123f461ea431590cc8`.
  ANTLR syntax errors are rejected; the semantic switch accepts SELECT only.
  Adopt AST/error handling, not this restricted query language as a SQLite
  INSERT/UPDATE/transaction analyzer. No C++ code copied or dependency added.
- CodeGraph effects.ts withShape/modelOf inspected, SHA
  `a44ec2bd5b3c0dd657ea054723c1c0512e82076ce90f40916da34f1663a7cccc`:
  classification uses receiver/method names and literal argument labels.
  query('enqueue') can name a query key, not a table. This heuristic is not
  evidence that Orders writes a table named enqueue.
- CodeGraph construction-sites.ts public inventory: local/import function
  binding and argument spans are available, but argument origins are objects,
  not resolved string values. Extend/reuse binding evidence for a future
  bounded string/path derivation; do not substitute template regex guessing.
- Existing product orders-graph-smoke.mjs read completely: reuse real fresh
  owned indexing and unchanged fixed questions; no new independent scorer.
  Read Orders store/http/relay, all 11 SQL files and flow.test.mjs assertions.
  The test checks transaction rollback after enqueue conflict, HTTP delivery
  and deduplication; it does not prove graph extraction of these relationships.

## Decision before patch

Add separate authored SQL ground truth with byte hashes and code owner names.
Probe only actual public getFile/getNodesInFile observations, report missing
SQL file inventory without injecting authored table facts into the engine.
Ground-truth hash drift is a diagnostic failure, not silent re-authoring.
Keep SQL linkage/table extraction acceptance false until the provider supplies
evidence. Unchanged generic diagram benchmark remains mandatory.

## Implementation contract still open

1. Bind imported readFileSync, new URL(..., import.meta.url), lexical wrapper
   parameter and literal caller argument; preserve each source span/hash.
2. Join code symbol → query-file candidate using existing stable symbol IDs.
   Missing file, source drift, traversal/symlink escape, computed argument,
   shadowing/reassignment, unsupported wrappers and recursion must stay unknown.
3. Parse SQL with a dialect-aware parser, never execute repository SQL to learn
   dependencies. Preserve statement ordinal/ranges, read/write/DDL/transaction
   distinction, unknown statements and named-table identity.
4. Keep query-file/table identity separate from runtime DB instance. The same
   OrderStore class/schema is used by two different databases in this fixture.
5. Invalidation must include SQL-only changes and deletion; route-to-storage
   context must carry call guards and file/SQL provenance. Renderer must not
   claim transaction control alone proves atomic execution or runtime delivery.

This is a prerequisite acceptance baseline, not implementation of SQL mapping.

## Executed verification

Fresh emitted Orders probe exited 0; receipt
`.harness/baselines/orders-sql-inventory-20260912-01.json` records **11 expected
SQL files, 0 indexed file records, 0 indexed nodes**. Source, emitted build and
fixture fingerprints remained unchanged. Four route-flow checks and watcher
passed; five old probes exited 0 but generic diagram still returned no relevant
code. Linkage/table acceptance explicitly remain false. This proves inventory
is a prerequisite gap, not merely a missing UI label.

`scripts/with-local-tools node --test fixtures/orders/test/flow.test.mjs`:
1 passed / 0 failed; owned temporary DBs and localhost servers were cleaned by
the fixture. This runtime fixture test validates the authored behavior, not
the provider's SQL extraction. No full CodeGraph rerun: provider unchanged.

Next implementation order: SQL file admission with content identity and
sync/removal lifecycle → dialect parser/statement observations → source-bound
wrapper/path join → scoped code/SQL/DB-instance projection. Keep transaction
control and runtime atomicity separate. Do not add raw SQL into Rust queries
or execute arbitrary repository SQL in the analyzer.

## Resumption: admission source gate before implementation

Read grammars.ts extension selection, grammar availability and canonical
isFileLevelOnlyLanguage; tree-sitter.ts no-symbol extraction branch;
index.ts successful zero-node storage and healZeroNodeRows; extraction.test.ts
YAML/Twig/properties indexAll/indexFiles assertions; complete framework coverage
and nested docs instructions. Current hashes: grammars.ts
`51a3a468f6d6c93c7016712119aebb66dda72da6117144ec5c2aab285b6d45ca`,
types.ts `e408f5675ba41256d80670f94372a65404b5b7870dc70e33b00a05b6c5ecedce`,
index.ts `2badf738db8415c304015cabae426650875cc27be21664abf8e9c68d0cf382c5`.
Adopt the existing file-level-only path for SQL, not an empty custom parser or
fake symbol node. SQL has no grammar or semantic support in this step. Normal
sync discovers newly supported extensions; preserve old symbol extraction
version because no existing symbol representation changes. Test indexAll,
indexFiles, unchanged sync, same-size content change, addition/deletion/reopen,
grammar availability and no symbol/edge claims. Broader language-flow acceptance
remains pending; no live-account A/B authorized here.

## Admission implementation and focused verification

Added `sql` language identity and `.sql` to existing selection; SQL goes through
the canonical file-only branch, has no WASM grammar, emits no nodes/edges, and
is not repeatedly healed as a wiped symbol file. No migration or parser added.
Initial tsc caught a missing display-name entry; added `SQL (file tracking)`;
subsequent TypeScript emission passed. Three focused tests passed for inventory,
explicit indexFiles, reopen/unchanged sync, same-size modification, addition and
deletion; invalid SQL remains opaque source rather than a validation claim.

Fresh emitted Orders receipt
`.harness/baselines/orders-sql-tracking-20260912-01.json`: 11/11 SQL file records
now present, all authored hashes match, zero SQL nodes; graph remains 63 nodes /
143 edges and file count increases 7→18. Four route assertions and JS watcher
pass; source/dist/fixture unchanged. Generic diagram still returns no relevant
code. This completes file inventory only, not statement or wrapper linkage.

Final hashes:
- grammars.ts `327442474458ebe4c08fec35e459968363879c0d22acdd218c7909c41b7fd6c2`
- types.ts `bc220fb0b82a614277c5d9b792d045705144da0b0f3ed759e5f0ee3b4325f11b`
- sql-file-tracking.test.ts `691dd3cc6f995c0b9a9f8709be9e8cc81597593fb9dd69e948e8fb398728908e`

Luna read-only worker `01a09664-e81a-79f0-900e-045f3cef4d24` completed and was
closed; requested gpt-5.6-luna, effective model unverified. Its pre-admission
findings identify PHP static includes and MyBatis XML statement linkage as
additional source-study entrypoints, not directly reusable JS string/dataflow
solvers. Parent owns this admission patch and verification.

Linux native watcher probe `.harness/baselines/sql-watch-20260912-02.json`
completed all three add/same-size-update/delete operations and three
onSyncComplete callbacks, exit 0, no sync error. Earlier `-01` observed each
file row before sync completion and closed the DB too soon: stderr reported
`statement has been finalized` after writing the receipt. Preserve that attempt
but do not treat its error:null/exit 0 as clean shutdown evidence. The second
probe waits for actual callback completion before teardown. No watcher
implementation or timing threshold was changed.

Full regression receipt `.harness/baselines/codegraph-sql-files-20260912-01.json`:
CODEGRAPH_KERNEL_EXPECT=1, exit 0, **5358 passed / 0 failed / 9 skipped**,
5367 tests across 306 files, 270.8 seconds. Source fingerprint before/after
matches `c1a83d181a7bfcd8dd330cd1179faf5a50f8c9902b6174866835be8bd0985678`.
This verifies the current admission snapshot, not SQL semantic extraction,
cross-platform conditional tests, language-flow A/B or overall W4 acceptance.

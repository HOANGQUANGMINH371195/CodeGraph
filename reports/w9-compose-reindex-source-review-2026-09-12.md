# Compose reindex/watch source receipt — before patches

Parent reread current CodeGraph HEAD `3ed73bc127323e63153bf6ec8354afa82ce36aaf`:

- `src/resolution/parameter-reconciliation.ts` complete, SHA
  `b8a408a83d59b9c88c6c68ddf27ffc580eda32fa82c2c9114dc9698984980259`:
  snapshot inputs, analyze, recheck freshness, replace owned edges and receipt;
  failure retracts owned evidence. Adopt fail-closed replacement/recovery, not
  TypeScript metadata or a claim that source observations prove runtime flows.
- `__tests__/parameter-reconciliation.test.ts:110–156`, SHA
  `1e54ec9356db14271cfa9a06ae75615c1939e465f7cc4ddaec895f96b99a003f`:
  malformed/deleted evidence, abort/recovery and rollback expectations.
- `src/sync/watch-policy.ts` complete, SHA
  `63cd0d725ae00929bbe66dadcfead623e2daaa1d970ae4c617e59ea842e56cb5`:
  explicit watcher policy and expensive recursive filesystem watcher pitfalls.
  Use opt-in single-owner polling, no recursive registration/global env changes.

Local SourceReader/DirectorySource and verifier were read: reader returns a
bounded byte buffer under a directory capability; verifier checks an existing
citation. Reindex needs a separate capture operation creating evidence from the
same returned bytes, without rereading a changing file or pretending the old hash
still matches. Current application fingerprint code supplies length-delimited
SHA encoding conventions. No upstream implementation copied.

## Contract before implementation

`capture_source(reader, locator, analysis_run, limits)` returns SourceSlice with
new content hash/full-file range and deterministic, fully scoped citation ID.
Locator supplies only caller project/graph/path; old hash/ID/range do not constrain
new bytes. Analysis run is caller-declared, not attested. No whole-tree snapshot
verification. Read only once; do not normalize newline bytes. Invalid/empty/NUL
or oversized input rejects. Identical bytes/scope/run reproduce identity.

`reindex-compose ID TASK ROOT --analysis-run RUN --expected-generation N` reads
current owner first, rejects stale expected generation, captures and parses,
then CAS-replaces or invalidates on source/parser failure. New immutable citations
are stored only with a valid graph. No-op graph equality/tombstone keeps generation;
it is a historical observation, not a fresh authority token. Invalid task/run
configuration, missing locator, output budget or DB errors do not masquerade as
source invalidation. All output bytes encode before mutations; stdout failure
after commit remains an ambiguous transport outcome resolved by query.

`watch-compose` polls the same owner sequentially, with configured interval and
optional finite cycle count, carries successful generation forward, no backlog,
and stops on CAS/DB/output errors. Source failures emit an explicit invalidated
status and permit later recovery. Reindex one-shot returns a report plus nonzero
exit for invalid source; watcher continues such reports. Polling is not a global
watcher/daemon nor proof that no intervening edit was missed. Query remains
historical even between detection and commit. Source/root binding caller supplied.

Pre-patch refinement after implementing capture/cycle: CodeGraph explicitly
rechecks input freshness before publication. Adopt that second observation in
the CLI cycle via existing verify_source against the captured evidence after
parsing. Capture itself still reads once and derives all evidence from that one
buffer. A changed/unavailable source during final verification invalidates,
never combines bytes. This is not an atomic filesystem+SQLite snapshot: an edit
after the final check is still possible and historical query flags remain.

Before extending transport tests, parent reread OpenDev runners.rs:110–130
(HEAD `d32c660e4eed1a8e988d1fd58da88e41ba641d08`, SHA
`75fff7c32e64eae41c17c629cf0cf0b8184f756f342c1807fc523a5aee9fde4f`):
exit/stdio separation, not an atomic commit/transport guarantee. Extend the
owned /dev/full fixture to reindex and multi-cycle watch; assert committed
generation survives and watcher stops instead of consuming remaining cycles.

## Implementation and verification

Implemented application capture_source and explicit reindex/watch CLI, with
output-only ComposeReindex DTO. Capture source is read once; CLI analysis then
verifies the same evidence against a second read through the directory capability.
No filesystem/SQLite atomicity or entire worktree freshness claim is made.

Two native Luna workers (`gpt-5.6-luna`, Hegel/Euler), no descendants:
eight independent capture tests and eight CLI tests. Parent read their source
receipts and assertions, added a ninth capture test with an independently
computed Node-crypto golden plus field-boundary collision test, and a deterministic
ChangingReader test rejecting edits between capture and final verification.
Existing input-limit checks expanded to both commands; Linux stdout failure
fixture expanded to reindex/watch, preserving committed generation on failure.

Real CLI tests cover changed-byte capture/new graph queried through the original
locator; stale CAS; malformed/deleted source invalidation and recovery; repeated
valid/tombstone no-op generation; receipt budgets before writes; invalid task/run
or missing locator without invalidation; finite watch observing malformed→valid
changes via flushed lines. Watch fixture owns child cleanup and timed receives.

Final verification:

- `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0;
  **366 Rust passed, 0 failed, 3 existing ignored; 16 Node passed**.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- CLI Clippy inspection: no new reindex module diagnostics; two existing
  main.rs warnings (documentation/backticks and dispatcher length), not a strict
  whole-workspace lint pass.
- Architecture remains 8 packages / 48 declared dependencies. No new external
  dependency, migration, account/auth configuration or reference-repo edit.

Final SHA-256:

- application source: `c811687fa92f5a2853e71f72ff923a82ff58f59e1302aed025aa3de1b82cd37f`
- capture tests: `627a8f2af8a5fbbd1b2f58c113c513184116ff1e4293f44912016744569b8222`
- CLI reindex: `98e502a01ed909b9436d524965e727db1abdd7049d2f697ac70d9708304c4e54`
- CLI tests: `d31c125507aeb877b1da3a0944f7eba8507e078c12bb38a028f99308322e3489`
- transport test: `e53456fa2d451aed8baa7b549fa54624a1f43fc3bb57d7f902de66ff396e32e7`

Open: connect this cycle to graph authority's multi-owner discovery/watcher,
snapshot/config reconciliation, build/module/API/SQL and context/diagram paths.
Single-owner polling is explicit and optional, not a replacement graph scheduler.
Byte caps are not filesystem I/O deadlines; edits between polls/final-check and
commit can still occur. W0–W13 full scope remains open.

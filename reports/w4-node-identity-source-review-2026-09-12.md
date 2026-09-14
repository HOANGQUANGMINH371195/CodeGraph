# W4 position-sensitive node identity

Status: identity component verified; W4 end-to-end coverage remains incomplete.
Main reopened source gate and read current sources before patching. Baseline
CodeGraph HEAD 3ed73bc127323e63153bf6ec8354afa82ce36aaf, dirty fingerprint
f7796c1fc72cd55def44c4abf784fb50fe1e32adda38aecf89813f17ec80fae3.
Skills: rust-router, m11-ecosystem; unsafe-checker for safe native contract
inspection. No new unsafe code, dependency, ABI buffer layout or package release.

## Source/flow and adoption

- tree-sitter-helpers.ts generateNodeId -> tree-sitter.ts createNode hashes
  file/kind/name/line; startColumn stored separately. Same-line same-name
  methods overwrite one another. injected-reconciliation.test.ts pins the gap.
- ids.rs and all 15 native walker create_node/col_of sites use the same hash;
  every col_of uses textutil::col16, matching wasm UTF-16 columns. Adopt that
  conversion, not raw Rust byte columns. Preserve file-node special IDs.
- Standalone extractors use generateNodeId too: root components/files at 1:0,
  CFML tags have AST columns, Liquid matches already compute columns, DFM is
  line-based, MyBatis already uses statement offset as its identity location.
  Drupal reconstructs IDs only for /^function/ declarations at column zero.
- kernel loader verifyContract currently checks ABI/kind tables only; an old
  binary would silently keep line-only IDs. Add an explicit nodeIdVersion
  contract field rather than falsely changing the buffer-layout version.
- kernel-scaffold and TS/JS parity tests compare identity/edges/refs; native
  binary must be rebuilt and mandatory-native checks must not silently skip.
- indexAll/storeExtractionResult skips equal hashes but increments parsed
  file counts and may stamp extraction version. sync touches only changed files.
  indexFiles is another mutation entry. A version bump alone is insufficient.
- Reviewer Meitner independently read lifecycle/rebuild/upgrade tests and
  confirmed CLI index uses CodeGraph.recreate (not --force). Main reread this
  source, recreate/removeDatabaseFiles, indexAll/sync/indexFiles and advisory.
  Adopt existing explicit recreate workflow. Do not delete a live user's DB
  implicitly during incremental sync. Test only invocation-owned fixtures.

## Decisions before coding

Hash file/kind/name/one-based-line/zero-based-UTF16-column in both paths.
Pass existing columns at AST/regex call sites; explicit zero for file/line-only
records. MyBatis keeps its existing unique offset rather than removing that
disambiguation. Dedicated format stamp means writer format, NOT completed
coverage: set it only when an empty graph first accepts current-format writes;
reject missing/foreign formats on populated indexes before mutation. Guard
indexAll, indexFiles and sync under their writer locks; reads remain possible.
Extraction version advances to reflect changed output, but is not the gate.
Upgrade guidance must distinguish full rebuild from ordinary incremental sync.

Another source hazard: reattachCrossFileEdges keys targets only by kind/name,
so adding unique IDs must also avoid arbitrarily rebinding duplicate methods
after a source edit. Read query projection and reattach flow; extend evidence
to qualified name and preserve exact unchanged IDs, otherwise require a unique
qualified candidate, falling back to original unresolved-ref replay on ambiguity.
No first/last duplicate target. New SQL belongs in a query asset, copied by
existing copy-assets script.

Verification: red same-line/non-ASCII/CRLF extraction tests on wasm and native;
exact parity plus same-line injected edges; real SQLite legacy IDs with matching
hashes refuse indexAll/full+scoped sync/indexFiles unchanged; fresh/recreated
indexes accept and stamp; aborted/incomplete work never claims completed coverage;
CLI rebuilt DB has no old node/edge/reference owners; repeated sync preserves
same-name methods' correct targets and fresh-rebuild convergence. Preserve
unrelated dirty edits; no publish, version bump of package, or global config.

- [x] Source and tests reread; decisions recorded before code.
- [x] Implement and verify identity + native contract.
- [x] Verify legacy/rebuild and incremental target preservation.
- [x] Full suite and Orders diagnostic smoke completed; independent lifecycle
  review recorded above. These checks do not establish product acceptance.

## Follow-up review before lifecycle patch

Meitner reread the format gate and flagged public resolveReferences/Batched
as write bypasses, plus open({sync:true}) leaking the newly created instance
when the new expected mismatch rejects. Main read those functions, their
internal already-locked call sites and utils.ts FileLock/Mutex before fixing.
Public synchronous resolution must reject while an async writer owns the
in-process mutex, then acquire/release the file lock. Public batched resolution
uses the mutex and delegates to an internal already-locked method; index/sync
call that internal method to avoid recursive-lock deadlock. Read-only opens
remain possible; failed open-with-sync closes before rethrowing. Status includes
format mismatch even if advisory extraction stamp is current. Direct low-level
QueryBuilder/ExtractionOrchestrator use is not a format-enforcing facade.
Administrative clear/optimize and vocabulary maintenance do not emit symbol IDs.

Build/initial checks: native release build/staging succeeded (existing scanner
and unused_mut warnings); first focused native/wasm/field lifecycle run 34/34
passed. Upgrade+sync+TSJS parity+convergence+advisory run 96/96 passed. An initial
tsc run caught unchecked candidates[0]; fixed with optional access and rebuild
verification remains required after the follow-up changes. No red baseline is
claimed for the new tests; the pre-change collision was pinned by the prior
negative lifecycle test and report, now replaced by a positive owner assertion.

Follow-up native parity/WAL/Drupal run: 194 passed / 1 failed. The only failure
was Lua's old explicit assertion that two same-line declarations MUST share
an ID; native↔wasm equality in that test already passed. Main reread its source
and changed the assertion to require distinct IDs without weakening parity.
Final follow-up suite (upgrade/sync/Lua): 22/22 passed, including empty-future
and no-file-record status cases and pending indexFiles lock retention followed
by successful public sync/batched resolution. Earlier lifecycle follow-up was
31/31 passed. Reviewer found no blocking normal-flow issue before the final
status/test additions and was closed; no agent left running.

Rust `cargo test --lib ids::tests`: 2 passed, 20 filtered; this is not all Rust
unit tests. rustfmt check for ids.rs passed; entire upstream formatting was not
rewritten. Final emitted tsc and native build launched after all source edits;
full suite and Orders evidence still pending. Existing package versions and
kernel ABI buffer layout unchanged; extraction version 27 and ID-format 1.

## Resume verification

Main reopened the source gate and reread the current identity guard, public
resolution writers, stale-index check, native ID implementation and identity,
legacy/rebuild and sync assertions. No implementation patch in this resume.
Both previously running final builds were polled to terminal exit 0: emitted
TypeScript and native release/staging (59.34 s, existing warnings). The new
SQL source and dist asset compare byte-identically.

Full suite started with `CODEGRAPH_KERNEL_EXPECT=1`, default workspace config,
no concurrent build/probe, JSON output `codegraph-node-identity-20260912-01.json`
under `.harness/baselines`. Source before the run:
`7de3af97f67a34efc05e7e6659458980d370718fde4f0f2eceb01c1d72ea3595`.
This source identity does not by itself prove build correspondence or product
acceptance. Full result and post-run fingerprint must still be checked.

## Final verification result

- Full suite terminal exit 0: **5009 passed, 0 failed, 9 skipped, 1 TODO /
  5019 tests in 284 files**. Mandatory native enabled; JSON receipt above.
  Post-run source fingerprint equals the pre-run fingerprint. Existing
  500-caller warm API assertion `elapsed < 100` passed unchanged; the reported
  whole-test duration is not request latency. Optional self-index test skipped.
- `orders-node-identity-20260912-01.json`: exit 0; 51 nodes, 131 edges, 7 files;
  four injected-field edges, one `receiver-incomplete` issue in relay.mjs.
  Watch observed with no errors; source/dist/fixture unchanged. Five probe
  processes exit 0, but diagram output is still `No relevant code found`.
  This is diagnostic smoke, **not** successful architecture rendering or an
  end-to-end agent benchmark; `product_acceptance` remains false.
- `git diff --check` passes. No package version, global account config or
  user project index changed. Legacy rebuild tests use owned temporary copies.

Remaining: returned anonymous callable extraction (the one TODO), parameter
dispatch and complete evidence-backed Orders flows/diagrams. Preliminary
reread located the anonymous-body fallback in tree-sitter.ts extractFunction /
visitFunctionBody and native tsjs/extractors.rs + mod.rs; both currently walk
unnamed bodies under the outer scope. The nested-declarator tests provide an
existing own-node/own-call/contains pattern, but are not evidence that returned
closures are handled. Reopen source gate for that task before implementation;
do not manufacture an outer-to-closure invocation merely from containment.

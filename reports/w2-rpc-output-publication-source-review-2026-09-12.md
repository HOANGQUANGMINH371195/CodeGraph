# W2 bound RPC output publication

status: done (initial trusted fixture publication); source-gate: ready; scope: execution receipt preparation/publication.

- [x] Read source/tests for this task before implementation.
- [x] Record adopt/avoid and missing behavior.
- [x] Choose API/failure stages and tests before patch.

OpenDev clean `d32c660e4eed1a8e988d1fd58da88e41ba641d08`: reread stdio close
254–302 and complete stdio_tests.rs. Adopt lifecycle-owned result collection;
avoid draining away uncertain pending and interpreting wait/kill as containment.
Source has no our CAS-bound receipt equivalent; not attributed to it. Upstream
tests not run. Product read output.rs, application artifact ingestion/verification
and lying-writer test, bound fixture/normal-cancel test, DirectoryArtifacts::open.
Reuse metadata-first ingestion with independent readback; do not equate writer
success with correct bytes, or adapt RPC into CheckRunBinding.

Plan: preparation borrows opaque bound terminal; derives SHA-256/length/kind,
Unreviewed/Evidence descriptors and all pending/completion/counters from result.
Validate run/epoch/budgets before writes. Hold immutable prepared report including
timestamp for exact retries. Publication checks stored spawn and existing receipt,
registers run, ingests+verifies each output then records terminal receipt. Explicit
failure stages carry prior output progress; original terminal/Child stays owned
by caller through borrow. No cross-resource transaction claim. Concurrent state
changes can still make final ledger reject after blobs are written.

Tests: extend actual normal/cancel bound fixture with preflight rejection, publish,
reopen and exact replay; inject a lying writer to prove no receipt before verified
outputs and retain original terminal for retry. Full host auth/containment and
crash reconciliation remain incomplete. Rust lifecycle skill informs borrowing
rather than consuming resources on a fallible storage operation.

## Validation

Implemented PreparedRpcReceipt borrowing BoundRpcTerminal, exact derived output
descriptors and checked domain report; staged publish with original report kept
for retry. Spawn/previous receipt checks precede run/output writes. Outputs are
independently read back by existing ingest_artifact before terminal ledger write.
Errors expose typed stages and already verified output progress without printing
payload in Debug/Display. Underlying sources remain available to caller.

Extended existing actual normal/cancel fixture test (no new test count): invalid
IDs/time/run reject, lying writer fails stdout verification and leaves no terminal
receipt, retry to real DirectoryArtifacts succeeds, V13 read after reopen matches
exact report, subsequent publish re-verifies bytes with no new blobs/events.
Original terminal remains borrowed until caller explicitly extracts cleanup parts.
Main reviewed implementation/assertions; no separate reviewer.

`scripts/with-local-tools cargo test -p graph-execution --locked --offline --test rpc_fixture`:
8 passed. `sh scripts/validate-foundation.sh`: 273 Rust + 12 Node passed;
`scripts/with-local-tools cargo fmt --all -- --check`: exit 0. Architecture
7 packages/41 declarations/no errors; no migration or dependency added.

Final untracked SHA-256:
- `crates/execution/src/rpc_output.rs`: `c6e4c0202a9d70fb8c6c1aa20a528268320a50022e84cb90235370c3f915cf46`
- `crates/execution/tests/rpc_fixture.rs`: `ae93cfc484ae3f8dee2af98dfbf62d55d56085cb3e81ab56091cf2ca04d11e54`

Remaining: inject stderr and ambiguous final-ledger failures, exercise malformed/
truncated actual output publication, terminal CLI inspection and crash recovery.
No production host authentication/containment or full W2/W1-I gate. Prepared
timestamp is host-supplied, blob verification is point-in-time, adapter deadlines
and any unreaped Child cleanup remain caller responsibilities.

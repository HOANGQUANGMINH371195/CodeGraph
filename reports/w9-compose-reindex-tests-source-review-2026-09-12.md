# W9 Compose reindex/watch CLI tests — source receipt

status: done | owner: Codex
scope: `/home/minh/projects/project-graph-agent`; own only
`crates/cli/tests/compose_reindex.rs` and this report | depends_on: parent CLI
reindex/watch implementation
source-gate: ready

## Pre-code checklist

- [x] Reset this task's source gate to pending before rereading the current
  source and tests.
- [x] Selected sources by mechanism: fail-closed source reconciliation,
  sequential watch recovery, CAS/output-budget ordering, and real process
  CLI assertions.
- [x] Read the requested outsource source and test ranges, the current
  product implementation flow, and `crates/cli/tests/deployment.rs` without
  editing it.
- [x] Recorded hashes before writing the test code.
- [x] Fixed the subprocess, fixture, synchronization, and bounded-cleanup
  test plan; no production, dependency, fixture, global, auth, or sibling-test
  changes are in scope.

## Revision and hash receipt before test code

The product worktree has no commit revision yet (`git rev-parse HEAD` fails),
so the current dirty-tree identity is recorded by SHA-256. These hashes were
collected before creating `crates/cli/tests/compose_reindex.rs`:

### Requested outsource references

- `codegraph/src/resolution/parameter-reconciliation.ts`:
  `b8a408a83d59b9c88c6c68ddf27ffc580eda32fa82c2c9114dc9698984980259`
- `codegraph/__tests__/parameter-reconciliation.test.ts`:
  `1e54ec9356db14271cfa9a06ae75615c1939e465f7cc4ddaec895f96b99a003f`
- `codegraph/src/sync/watch-policy.ts`:
  `63cd0d725ae00929bbe66dadcfead623e2daaa1d970ae4c617e59ea842e56cb5`
- `codegraph/__tests__/watch-policy.test.ts`:
  `33e8e43137f38dd3ed7610c87fa8924813b981661c22dd81a4acdd18bbbd32de`

The outsource tree is not a Git worktree; these file hashes are its available
revision/fingerprint receipt.

### Product sources read before this test patch

- `crates/cli/tests/deployment.rs`:
  `f8669443133fb4c8c9024e66d85c2049d968999e6aac6b4803309e8589cde49d`
- `crates/cli/src/compose.rs`:
  recorded from the current worktree before patching this file
- `crates/cli/src/compose_reindex.rs`:
  `074ffff38ef1556f3a343b81c8dca15f96fc2b4e8eb9e922a171201ab8480a06`
- `crates/application/src/source.rs`:
  `d71411227a4932486e90bcaeac8a5319af8e1901fade7d5241fd053c6dd02cca`
- `crates/domain/src/deployment.rs` and `crates/protocol/src/deployment.rs`:
  read for immutable graph/citation and report fields; parent-owned and not
  changed by this task.
- `fixtures/orders/compose.yaml`:
  `a2a29afb353a588cc28759f281819f5cfda82fd5044f8fd5e38247959fa37701`

## Source study and decisions

The requested CodeGraph reconciliation flow snapshots inputs, performs the
analysis, rechecks freshness, and replaces only the producer-owned evidence;
malformed source or stored extraction errors retract old evidence, while a
later valid sync recovers it. Its tests at
`__tests__/parameter-reconciliation.test.ts:110-156` pin malformed-source
failure, recovery, abort fail-closed behavior, and rollback when the receipt
write fails. The CLI adaptation is a process-boundary assertion: source
failures must publish an invalidated tombstone, but task/config/CAS/DB/output
failures must remain fatal and must not be reclassified as source failures.

The requested watch policy centralizes the decision and avoids expensive
recursive watcher setup on WSL `/mnt` paths. The CLI contract is narrower: a
single owner polls sequentially with bounded finite cycles. The test therefore
does not use filesystem event timing or sleeps to synchronize edits. It reads
each flushed child stdout line, edits the fixture immediately, and receives
the next line through a bounded channel; timeout cleanup kills and waits for
the owned child.

The current product CLI fixture in `deployment.rs` establishes the real
`CARGO_BIN_EXE_project-graph-agent` subprocess boundary, temporary database
and root, registered citation/task JSON, Orders bytes/hash, exact JSON-line
stdout, empty stdout on fatal errors, and historical deployment queries.
`compose_reindex.rs` currently captures fresh bytes from the registered
project/version/path locator, ignores the old citation hash/range, validates
the graph before one CAS replacement, encodes the report before mutation, and
polls sequentially while carrying generation forward. The new tests assert
these observable behaviors rather than calling private Store/application APIs.

## Adopt / avoid / adapt

- Adopt the outsource fail-closed/recovery lifecycle and producer-scoped
  replacement idea; adapt it to immutable source citations, Compose graph
  tombstones, and generation CAS.
- Adopt direct subprocess capture, temporary isolated fixtures, exact parsed
  JSON and stdout/stderr assertions from the existing product tests.
- Adapt watch synchronization to stdout-line handshakes and a bounded
  `recv_timeout`; do not rely on wall-clock sleeps, filesystem watcher events,
  or recursive/global watch policy.
- Avoid editing `deployment.rs`, importing it as a module, using direct Store
  calls, recording replacement citations from the test, adding dependencies,
  authentication/network/runtime claims, or changing production behavior.

## Focused acceptance plan fixed before implementation

Eight tests will cover:

1. A changed source is captured through the registered locator, publishes a
   new graph, and query exposes the changed node plus a new content hash.
2. A stale expected generation fails before mutation and preserves the prior
   historical snapshot.
3. A valid graph followed by malformed and deleted source emits invalidated
   tombstones with generation `N+1`, then valid source recovers with a new
   graph; one-shot invalid source is nonzero while still returning its report.
4. Identical source/scope/run is an unchanged no-op with stable generation,
   citation, and graph bytes.
5. Exact report-byte budget succeeds and one byte under it fails before
   mutation; the previous snapshot remains unchanged.
6. Zero output budget on an invalid source fails before invalidation; the
   subsequent normally-budgeted source failure emits its report and tombstone.
7. Invalid task/locator/config inputs fail without invalidating an existing
   graph.
8. A finite watch emits flushed lines for published → invalidated
   → recovered, continues after the source failure, and exits cleanly without
   timing-flaky sleeps or orphaned children.

No test assumes a new citation must be registered by the caller; the query
itself proves that a changed immutable citation was created by reindex.

## Reopened source gate after parent implementation update

The parent changed the CLI implementation after the initial receipt. Before
the test patch continued, the current sources were reread and rehashed:

- `crates/cli/src/main.rs`:
  `2a38f23bbe11ba9d98794adb8f72f5e58b4c17b389976a2f0098b556d136cd3e`
  — `reindex-compose` writes its report before returning a nonzero result for
  invalid source; `watch-compose` returns success after source-failure lines.
- `crates/cli/src/compose_reindex.rs`:
  `0405a562430a854fe9eaa31a77f0b328abf5f052505242f0a49009d5450e43e8`
  — static reasons are `source_unavailable`, `source_capture_failed`, and
  `compose_analysis_failed`; output encoding precedes CAS; unchanged cycles
  retain generation and recheck CAS; valid capture is verified again before
  graph publication.
- `crates/application/src/source.rs`:
  `fcdfca00d73dca0633606bf5ff127d91ecee5ccb6ad37fd566d8bbbcb58f6303`
  — fresh capture derives a deterministic citation from project/version/path,
  current content hash, full line range, and caller run; the old hash/range is
  only a locator.
- `crates/cli/src/compose.rs`:
  `c4ef7032729b925af5eb98340601a03c05bbe9bf7e7c08ee6fb2415257469990`
  — query selects the deployment owner from the original registered evidence
  scope, so a new capture hash remains queryable through the original ID.

The updated tests therefore assert invalid one-shot stdout plus nonzero exit,
watch continuation across invalid-source cycles, and that a zero output budget
fails before an invalid source can invalidate the current graph.

## Implementation and validation

Implemented only `crates/cli/tests/compose_reindex.rs`. The test fixture
registers the existing `e1` citation, mutates only the temporary Compose root,
and invokes the real binary for every operation. The watch harness reads
flushed stdout lines through a channel from a finite eight-cycle producer,
uses receive timeouts, allows extra polling cycles
under load, and owns a `Drop` cleanup guard that kills/reaps the child on any
failed assertion.

- `scripts/with-local-tools rustfmt --edition 2024
  crates/cli/tests/compose_reindex.rs` — exit 0.
- `scripts/with-local-tools rustfmt --edition 2024 --check
  crates/cli/tests/compose_reindex.rs` — exit 0.
- `scripts/with-local-tools cargo test -p project-graph-agent --test
  compose_reindex --locked --offline` — exit 0; **8 passed, 0 failed** in
  3.57s.

The first focused run exposed one test-only helper mistake (a duplicate
`--analysis-run` flag in the empty-run case); it was corrected, and the final
run above is the authoritative result. Final test-file SHA-256:
`d31c125507aeb877b1da3a0944f7eba8507e078c12bb38a028f99308322e3489`.

No production, dependency, migration, fixture, sibling-test, global, auth,
or network file was changed. Parent-owned full gates remain outside this task.

Final parent-source reread before handoff:

- `crates/cli/src/compose_reindex.rs`:
  `98e502a01ed909b9436d524965e727db1abdd7049d2f697ac70d9708304c4e54`
  — freshness verification is factored through `analyze_observed`; the
  capture is read once, graph analysis is followed by a second verification
  read, and `source_changed_during_analysis` remains a source failure.
- `crates/application/src/source.rs`:
  `c811687fa92f5a2853e71f72ff923a82ff58f59e1302aed025aa3de1b82cd37f`.

The focused CLI test command was rerun against the parent build after this
reread; its final result is recorded above.

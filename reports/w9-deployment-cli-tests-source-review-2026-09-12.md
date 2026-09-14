# W9 deployment CLI tests — source receipt

status: done | owner: Codex
scope: `/home/minh/projects/project-graph-agent`; own only
`crates/cli/tests/deployment.rs` and this report | depends_on: parent CLI command
implementation
source-gate: ready

## Pre-code checklist

- [x] Reset this task's source gate to pending before the reread.
- [x] Selected sources by mechanism: process-boundary CLI assertions and
  validation-before-mutation/rollback, not by repository name alone.
- [x] Reread the requested outsource source and test ranges, plus the existing
  product Compose fixture tests.
- [x] Recorded the source flow, adoption decisions, gaps, and planned tests.
- [x] Chosen implementation and test boundary; no production/dependency/root/
  global/auth changes are in scope.

## Source study

Reference repositories were reread from the current dirty worktrees before this
test patch:

- Codex `818f1cca8ccf8899f0f4d59336baebaccf358eed`,
  `codex-rs/cli/tests/execpolicy.rs:1-58`, SHA-256
  `98edf7cc70dfa79ae25ed688b590c323a826bcdd6f2c3cfc37969a0f4018936c`.
  The test creates an isolated fixture, invokes the real executable through
  `assert_cmd`, checks exit status, parses stdout as JSON, and compares the
  complete JSON value. This is the process-boundary shape to use here; the
  Codex-specific `CODEX_HOME` and policy fixture are not relevant.

- CodeGraph `3ed73bc127323e63153bf6ec8354afa82ce36aaf`,
  `src/db/queries.ts:1909-1971`, SHA-256
  `e49e1f4b76afa3b832611668b15db67af45b5512287b67ea8c74412d503048db`.
  `replaceSynthesizedEdges` freezes and validates the full candidate snapshot,
  ownership, endpoints, and identity before deleting owned rows; the transaction
  preserves foreign rows and retracts only the declared producer scope.
  `replaceSynthesizedSnapshot` includes previously owned sources with no new
  candidates and writes its durable marker in the same transaction.

- CodeGraph `__tests__/parameter-reconciliation.test.ts:63-155`, SHA-256
  `1e54ec9356db14271cfa9a06ae75615c1939e465f7cc4ddaec895f96b99a003f`.
  The reread covered rebinding/retraction, foreign/static preservation,
  malformed-source and abort fail-closed behavior, and especially the receipt
  failure test at `:146-155`, where candidate edges and the receipt roll back
  together. These are conceptual guides for CLI black-box assertions, not code
  or filesystem/database behavior to copy.

- Product `crates/cli/tests/compose.rs` was reread in full. Its fixture records
  an exact source citation before invoking the binary, uses a temporary
  database/root, passes task JSON by path, asserts stdout is exactly one JSON
  line with stderr empty on success, and asserts failures have empty stdout and
  non-empty stderr. The existing Orders Compose bytes/hash and fixture helpers
  are intentionally reused by reading/copying their setup into this independent
  test file; `compose.rs` will not be modified or imported as a test module.
  Its existing tests establish candidate-only analysis, exact output-byte
  budgeting including the newline, source-byte caps, stale hash/snapshot,
  malformed YAML, partial citation, symlink escape, and secret redaction.

## Adopt / avoid / adapt

Adopt the real-binary temporary-fixture boundary and exact JSON/stdio checks from
Codex and the existing Compose tests. Adopt CodeGraph's ordering principle:
source lookup, byte/hash/shape validation, output-size validation, and all graph
construction must complete before the single persistence mutation. Assert the
observable consequence that failed publish, stale generation, malformed source,
mixed snapshot, and candidate analysis do not change the stored deployment.

Adapt the replacement concept to the product contract: the owner is the
registered citation's exact project plus graph version, citation path, and
Compose adapter; generations are strict caller-supplied CAS values; explicit
invalidation is a tombstone distinct from an empty graph; and query is allowed
to return a historical graph after the source root is deleted. Query must not
read source or claim current byte/relationship/analysis verification.

Avoid mocks, direct Store/application calls, production internals, source-file
mutation outside the temporary fixture, authentication/global configuration,
new dependencies, and any assumption that a candidate projection is durable.
Do not import or modify `compose.rs`; this file must independently prove the
CLI process behavior.

## Contract and test plan fixed before implementation

The fixture registers citation `e1` for the Orders Compose source and uses the
matching `task.json` project/graph snapshot. Tests will cover:

1. Actual Orders lifecycle in separate CLI processes:
   analyze → publish generation 1 → separate query → invalidate generation 2 →
   query tombstone → republish generation 3. Assert exact mutation objects and
   query metadata/graph shape.
2. Delete the source root after publication and query successfully, proving the
   stored result is historical and source-independent.
3. Query an absent owner and assert the literal JSON value `null` (the citation
   exists but no deployment owner exists); separately query an existing
   invalidation tombstone and assert the full snapshot with `graph: null`. Both
   query paths work without a source root.
4. Stale publish and stale invalidation reject with empty stdout and leave the
   previous snapshot/generation unchanged.
5. Source-byte and output-byte budgets reject before mutation. The output test
   captures successful bytes and checks the exact accepted limit includes the
   final newline, while one byte less leaves the database unchanged.
6. Stale hash, malformed Compose, partial/mixed snapshot, and source-root
   failures reject without implicit invalidation; the previous historical graph
   remains queryable. A mixed task snapshot uses the same registered citation ID
   with changed project/graph scope and must not write.
7. Candidate `analyze-compose` output remains non-persistent: after analysis,
   deployment query is absent and the output retains candidate-only semantics.

The tests will keep success JSON exact where the contract specifies the whole
object, and use focused structural checks for the persisted graph's evidence,
nodes, edges, and unknowns so source line/hash data remains independently
observable without depending on map order. Every helper will preserve the
existing CLI convention: data on stdout, diagnostics on stderr, one newline on
successful JSON output, nonzero exit on failure.

## Receipt

Pre-code source-study command:

```text
nl -ba /home/minh/projects/outsource/codex/codex-rs/cli/tests/execpolicy.rs | sed -n '1,58p'
nl -ba /home/minh/projects/outsource/codegraph/src/db/queries.ts | sed -n '1909,1971p'
nl -ba /home/minh/projects/outsource/codegraph/__tests__/parameter-reconciliation.test.ts | sed -n '1,180p'
nl -ba crates/cli/tests/compose.rs | sed -n '1,478p'
```

Result: source ranges and current test flows read successfully before the test
patch. Reviewer: parent agent after production CLI commands were ready.

## Implementation and real validation

Implemented independent process-boundary tests in
`crates/cli/tests/deployment.rs` only. The suite has 9 tests and covers the
Orders analyze → publish → query → invalidate → query → republish flow,
literal-null absent owner versus tombstone `graph: null`, historical reads
after root deletion, derived per-item evidence scope and single-line ranges,
stale CAS no-mutation, exact captured receipt/query byte budgets including the
newline, source-budget rejection, malformed source after successful
`verify-evidence`, stale hash, mixed snapshot, static task diagnostics, source
failure retention, and candidate-only nonpersistence. Parent-owned `/dev/full`
post-commit transport coverage was not duplicated.

Real commands and results:

- `scripts/with-local-tools rustfmt --edition 2024 --check crates/cli/tests/deployment.rs`
  — exit 0.
- `scripts/with-local-tools cargo test -p project-graph-agent --test deployment --locked --offline`
  — exit 0; **9 passed, 0 failed**.

Changed files owned by this task:

- `crates/cli/tests/deployment.rs`
- `reports/w9-deployment-cli-tests-source-review-2026-09-12.md`

No external dependency, migration, production/root/global/auth file, or
`crates/cli/tests/compose.rs` change was made. Parent selected
`gpt-5.6-luna` for this native worker (Herschel); the worker spawned no descendants.

# W2 RPC publication staged failures

status: done (bounded test task only); source-gate: ready; scope: owned execution integration tests.

- [x] Read outsource/current implementation before edits.
- [x] Record adopt/avoid and gaps.
- [x] Choose fault injection and assertions before patch.

Orca clean `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`: reread mutation receipt
begin/complete (37–114) and replay/different-input DB test (17–55). Adopt stable
operation identity for replay, not re-execution; avoid mutable completion rewrite.
The source does not test our ambiguous final-write return; inject that explicitly.
Upstream tests not run.

Product read full rpc_output, fixture helpers/guards and owned process modes
rpc-context/rpc-malformed/rpc-stderr. New tests retain terminal guard through
publication assertions, mirroring Rust lifecycle ownership requirements. Use
real SQLite/CAS with forwarding adapters failing stderr write or returning an
error before/after real terminal commit; all other operations forwarded intact.
Check retained prior output, no premature receipt, exact retry after reopen and
no second spawn. Publish malformed/truncated retained bytes without upgrading
completion or losing pending uncertainty. No production behavior change expected.

## Validation

- Resumed by rereading the implementation, complete publication tests and the
  Orca source/test ranges above; upstream revision still matches, worktree clean.
- `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0;
  276 Rust tests passed, 0 failed; 12 Node tests passed, 0 failed.
  Architecture gate: 7 packages, 41 declared dependencies, no errors.
  Two standalone helper tests remain ignored by default and are invoked by
  their parent tests; this is not evidence of full product acceptance.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- Three publication tests exercise real owned processes, SQLite reopen and CAS:
  stderr write failure preserves verified stdout; final record errors before
  and after commit reconcile the same receipt without spawning again;
  malformed/truncated output retains completion and pending count.
- Reviewer: main agent, no independent reviewer this task. No production code,
  dependencies or migrations changed by this test task.
- Product test file is untracked, not a clean committed revision. SHA-256:
  `crates/execution/tests/rpc_fixture/publication.rs`:
  `5d37b2699e8d4361e9232d567c0c4981c81240291b881c295ae008ca16dfb3e9`.
  Production `crates/execution/src/rpc_output.rs`:
  `c6e4c0202a9d70fb8c6c1aa20a528268320a50022e84cb90235370c3f915cf46`.

## Remaining scope

Injected adapter return errors and reopening SQLite are not OS-crash tests.
The prepared descriptor and original bytes remain in memory during retry;
reconstruction after host death is not implemented here. No production execution
authority, containment or full W2/W1-I gate is established. Next: source study
for coherent terminal receipt inspection/CLI, followed by crash reconciliation.

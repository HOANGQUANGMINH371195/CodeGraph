# W2 trusted fixture launch-to-terminal ownership binding

status: done (trusted fixture binding); source-gate: ready; scope: execution host binding and actual fixture tests.

- [x] Read outsource source/tests for this task before patch.
- [x] Record adopt/avoid and gap.
- [x] Choose ownership API and tests before patch.

OpenDev clean `d32c660e4eed1a8e988d1fd58da88e41ba641d08`: read stdio transport
close (254–302) and both stdio_tests.rs tests. Source owns child inside transport,
closes stdin, waits/kills and drains pending; echo test checks response then close.
Adopt transport-owned lifecycle. Avoid deleting pending diagnostics and inferring
containment from numeric descendant PIDs. Upstream tests not run.

Read product complete rpc_fixture and supervisor, connection setup use, fixture
test helpers and actual launch loop. Gap: success returns raw supervisor, losing
the exact spawn observation before terminal report construction. Add launch_bound
returning an opaque LaunchedRpc holding BOTH supervisor and exact observation
created from this child. No public constructor/mutable supervisor escape; explicit
consuming downgrade into_supervisor preserves legacy API. Finish transfers same
observation into opaque BoundRpcTerminal; premature finish returns whole wrapper.
Borrowed access supports inspection; consuming into_parts transfers unreaped
Child ownership to caller. No serialization can create the bound wrapper.

Rust lifecycle skill informs no implicit Drop/reap and no ownership loss on
premature finish. Existing error branches remain ownership-preserving; successful
record helper now returns its exact observation instead of unit, without refetch
or regenerating timestamp. This is trusted-fixture provenance, NOT production
approval, authentication, sandbox containment or verified project state.

Tests planned: actual bound normal RPC and cancellation, premature finish retains
binding/handles, terminal spawn matches stored observation and epoch, pending
uncertainty preserved; legacy host error suite unchanged. Receipt construction,
output CAS publication and persistence follow after binding is established.

## Validation

Added launch_bound/LaunchedRpc/BoundRpcTerminal. Exact observation flows from
successful recording through attachment, polling and terminal sealing. No
public constructor or mutable supervisor access; explicit consuming downgrade
retains legacy launch behavior. One new test runs normal and immediate-cancel
actual fixtures and checks premature finish, stored observation equality,
terminal epoch/pending state and replay rejection. Existing seven host tests,
including ambiguous recording errors retaining Child, also pass.

`scripts/with-local-tools cargo test -p graph-execution --locked --offline --test rpc_fixture`:
8 passed. `sh scripts/validate-foundation.sh`: 273 Rust + 12 Node passed;
`scripts/with-local-tools cargo fmt --all -- --check`: exit 0. Architecture
7 packages, 41 direct dependency declarations, no errors. No migration/dependency.
Main reviewed transfer paths/tests, no separate reviewer.

Final untracked SHA-256:
- `crates/execution/src/rpc_fixture.rs`: `6c5246b31e0d128650d60f1386533bcce059d67eb12ce23691696ac2e76e73fe`
- `crates/execution/tests/rpc_fixture.rs`: `95ed1e24e4c951c05ddb6d9ad3f721f7d8b4e8f595978da2e5a49a97b7d4fe35`

Still trusted owned fixtures only. Unreaped cleanup remains explicit; no new
fault injection for unreaped terminal branch. Output receipt preparation and CAS
publication/persistence are next; production identity/approval/containment,
CLI terminal inspection, crash recovery and full W2/W1-I remain incomplete.

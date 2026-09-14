# W2 host-owned spawn recording and failed persistence ownership

status: host implemented; seven focused fixture tests pass | owner: main
source-gate: ready

Before this separate host task, reread Codex clean
818f1cca8ccf8899f0f4d59336baebaccf358eed,
`codex-rs/utils/pty/src/pipe.rs:134–195` (explicit spawn configuration and returned
Child), tests.rs:423–442 (stdin echo/exit assertions); OpenDev clean
d32c660e4eed1a8e988d1fd58da88e41ba641d08,
`crates/opendev-mcp/src/transport/stdio.rs:88–130` (spawn, fallible extraction,
owned process state). Read current product rpc_fixture, attachment and its actual
RPC fixture tests. Skills Rust/domain/lifecycle plus error-handling, CLI and
concurrency read by main. No upstream runtime invoked or unsafe spawn hook copied.

Adopt explicit ownership throughout fallible post-spawn work. After this host
consumes a claim, record cancellation/expiry before spawn, OS spawn failure,
or Spawned(actual Child.id) before attachment. Do not record preclaim rejection
or infer outcomes on replay. A DB/clock/domain recording failure returns a typed
failure containing the attempted observation if built, its cause, and the exact
Child when spawned; also retain original OS spawn error for spawn-failed reports.
No implicit retry, claim reset, silent fallback or lost handle through map_err.
Lifecycle skill requires explicit transfer/reap, not assuming Drop kills Child.

Test actual SQLite + RPC: successful PID observation with one start/reopen;
no-spawn permission/cancel reports. Inject recording failure before and after DB
commit using a forwarding repository: returned Child retains all pipes, caller
kills/reaps it, durable claim remains and retry never spawns. After-commit error
must be reconciled by query, not treated as no recorded observation. This is not
a process-crash test. No native authority/containment or PID-based recovery.

Gap: synchronous persistence happens between spawn and supervisor attach;
its latency is not covered by the attach-based runtime deadline. Trusted owned
fixtures only. Final control-check→spawn and filesystem TOCTOU races remain;
terminal execution/output receipts and crash reconciliation still open.

Focused verification: `cargo test -p graph-execution --test rpc_fixture --locked
--offline` exit 0, seven tests pass (two new, existing success/failure/cancel
assertions extended). Recorded successful PID equals the fixture's actual start
marker; report survives reopen. Permission failure and postclaim cancellation
store their no-spawn dispositions. Injected recording failures both before and
after an actual SQLite commit return a live Child with all three pipes and no
initialize context written; explicit kill/wait reaps before requery/retry.
After-commit query returns the attempted exact report; before-commit query None.
Both preserve claim and reject a new launch. Combined spawn/recording failure
preserves original PermissionDenied and returns no Child, with no start marker.
No expiry-afterclaim/clock-error injection or attachment-failure integration
test was added here; attachment ownership has its separate existing tests.

Integrated foundation: 248 Rust + 12 Node pass, architecture and workspace fmt
check pass. Main-reviewed ledger/migration tests cover durable write behavior;
no source reference was modified. Final host SHA-256:
80b64494cbd311764c76a2e519f1b7143c2f6dfdb9fd77424e3d208bc1d456da;
fixture integration tests:
b5f824fdf49dbbb4c2ffac96a8ea8e803ef61f0f4407be616203b0af5e341c92.
The error/lifecycle skills influenced explicit ownership-bearing error variants
and retention of both persistence and original OS errors. Native/live dispatch
and automatic recovery remain gated.

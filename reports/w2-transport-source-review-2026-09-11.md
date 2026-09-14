# W2 transport: source review and implementation gates

status: source-reviewed, implementation pending W1-C | owner: main
tasks: P10.T05/P10.T06; P9.T11 integration depends on W2
source-gate: ready for this design review; re-read selected flows before implementation

## Evidence inspected this turn

OpenDev clean HEAD `d32c660e4eed1a8e988d1fd58da88e41ba641d08`:

- `crates/opendev-mcp/src/transport/stdio.rs` read completely. Connect owns
  Child in shared state, retains stdout reader handle, starts detached stderr
  reader. Notification queue is unbounded. send_request holds state mutex while
  writing and removes pending on write error; close needs that same mutex.
  Inference: a blocked pipe write can delay close before its 2-second wait even
  begins. This is source reasoning, not a reproduced runtime failure.
- The MCP writer uses Content-Length framing, not Codex newline framing.
  `stdio_tests.rs` read completely: not-connected test and Python echo test,
  only response.jsonrpc assertion after send; no blocked-writer or stderr-task
  teardown assertion in this selected test file. Not claiming absent repository-wide.
- `crates/opendev-tools-impl/src/bash/foreground.rs:50–172`: separate readers
  append all lines to Vec before post-processing; timeout/cancel kills process
  group, awaits child, then times out joins. Timing out a join is not itself
  proof that the task was cancelled. Output truncation after collection is not
  a cap on collection memory.
- `.../bash/mod.rs:629–647`: process-group cleanup test starts background shell
  and calls kill_process_group; it does not assert descendant disappearance or
  reap result afterward. Do not treat this test as full teardown evidence.

Codex HEAD `818f1cca8ccf8899f0f4d59336baebaccf358eed` (source read; revision
already pinned in reference inventory):

- `codex-rs/app-server-transport/src/transport/stdio.rs:55–145`: capacity-one
  channels, dedicated blocking stdio threads, per-write acknowledgement.
  EOF emits ConnectionClosed before runtime cleanup necessarily completes.
  This server-side stdin design is not a drop-in client ChildStdout transport.
- `app-server/tests/suite/v2/connection_handling_stdio_tests.rs:52–110`:
  unread stderr deliberately blocks invalid-input writes; SIGTERM test checks
  timeout exit code and elapsed duration. Reading these assertions does not
  reproduce them; no upstream test/process was run this turn.

## Adopt / avoid

Use the existing Connection core only for protocol sequencing. A separate
execution adapter owns child, stdin writer, stdout decoder pump and stderr pump.
Do not add process/async dependencies to graph-domain/application/protocol.
Audit any new adapter crate/dependency explicitly; current product has no Tokio
dependency or process transport, so upstream runtime availability is not assumed.

Use independent, bounded byte/chunk budgets, not only message counts. Control
signals and shutdown must not wait behind a full data queue or writer mutex.
Maintain stdout/stderr artifact provenance and explicit truncation/loss reports;
do not silently drop bytes and claim complete receipts, or print raw stderr into
agent context. How to spill/stop at the artifact budget must be specified first.

Own every pump handle. Shutdown is a sequence with independent deadlines:
stop admission → close/send EOF when possible → owned-process termination policy
→ wait/reap → cancel and join remaining I/O tasks → record residual uncertainty.
Process-group signalling is platform-specific, not proof against escaped/reparented
descendants. Sandbox/cgroup/Windows Job Object boundaries require platform tests.
Do not scan/kill by process-name patterns or convert an arbitrary PID to ownership.

## Acceptance fixtures required before W2/W3 integration

- [ ] Child floods stderr without newlines while stdout completes handshake;
  both streams make progress and retained/spilled bytes obey explicit budgets.
- [x] Linux owned fixture stops reading stdin after handshake; supervisor cancel
  responds with bounded cleanup and partial write remains uncertain dispatch.
  Actual stagnant progress, retained ID/method/counters and direct-child reap
  checked by tests/supervisor.rs; see w2-blocked-rpc-source-review-2026-09-12.md.
  Caller must poll; test includes scheduling tolerance, not real-time/platform
  or process-tree containment proof.
- [ ] Consumer stops reading stdout events; queue/backpressure does not block
  shutdown. No unbounded queue and no false complete artifact receipt.
- [x] Linux owned fixture: direct child exits but descendant holds pipe open; join deadline reports
  incomplete drainage and cleanup state accurately, without indefinite wait.
  Verified by tests/descendant.rs; see w2-descendant-pipe-source-review-2026-09-11.md.
  Dedicated test subreaper reaps the finite holder; production scope cleanup and
  other platforms remain unverified.
- [ ] Actual process termination/reap and pump termination checked separately;
  kill requested, transport EOF and task acceptance are distinct observations.
- [x] Linux owned fixture: exact/oversized/truncated protocol frames exercised through child pipes,
  not only the sans-I/O fixtures already passing in protocol.
  Also malformed JSON; see w2-pipe-frame-boundaries-source-review-2026-09-11.md.
  This does not prove full supervisor/platform acceptance.
- [ ] Spawn/config failures, write failure, decoder failure and host cancellation
  retain/reconcile pending request metadata; no automatic duplicate dispatch.
- [ ] Environment/argv/cwd and executable identity follow host approval; no
  credential copying, global Codex configuration changes or live account fixture.
- [ ] Linux/macOS/Windows tests prove supported cleanup mechanisms; unsupported
  platforms fail explicitly rather than claiming equivalent isolation.

## Dependency correction and next action

**Superseded sequencing note:** the historical paragraph below required all W1
verification before W2 development. PLAN §11.0 and
`w1-w2-dependency-review-2026-09-11.md` now separate W1-C contracts from W1-I
joint execution verification. Transport acceptance cases above remain required;
this does not authorize native dispatch before full W1/W2 gates.

PLAN §11 dependency table and §11.1 say W1 gate precedes W2 execution, and W3 native integration
follows W2. W1 remains open. Current Store::integrate rejects caller text with
"verified integration authority is not implemented"; inspected application
TaskRepository and TaskService still expose that disabled compatibility path.
No verified integration port is present there. This contradicts starting a live
execution track now, not the usefulness of the completed pure protocol pieces.

Next implementation returns to W1: source-review verified integration decision
and candidate/evidence/snapshot binding, implement its authority and atomic
decision/outbox transitions. Do not relax the gate just to run native processes.
This is sequencing work, not a request for new user permission or a blocked goal.

Skills rust-router, domain-cli, m07-concurrency and m12-lifecycle informed queue,
ownership and cleanup requirements. No Rust changes/tests this turn; all test
counts elsewhere are prior receipts, not newly reproduced results.

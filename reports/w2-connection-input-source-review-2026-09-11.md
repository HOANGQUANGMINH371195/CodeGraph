# W2 connection / input bridge

Source gate ready before code. Reread Codex app-server-transport stdio.rs:72–153
and connection_handling_stdio_tests.rs:52–92 at clean recorded revision
818f1cca8ccf8899f0f4d59336baebaccf358eed; OpenDev transport/stdio.rs:200–234 at
d32c660e4eed1a8e988d1fd58da88e41ba641d08. Adopt OS-write acknowledgement before
handshake advancement, but not as peer execution acknowledgement. Avoid upstream
pending removal on failed write and shared lock around blocking output.
No source copied or upstream process/tests run. Product Connection, framing,
correlation and NonblockingInput APIs read before implementation.

Before patch: Linux ConnectionInput owns protocol Connection and bounded writer;
initialization and acknowledgement bytes come only from the core, acknowledge
only after full write. Busy admission must not reserve a new request. Decode,
write failure and close retain unresolved correlation, with partial byte counters.
Add graph-protocol and serde_json as reviewed normal adapter dependencies; keep
domain/application/protocol free of process dependencies. No account or child
spawn authorization in bridge. Build owned JSONL peer fixture for pipe handshake
and response tests; closed/broken pipe tests preserve initialize metadata.
Skills concurrency/CLI/lifecycle/error-handling inform bounded progression.

## Validation

Implemented `ConnectionInput` and owned `rpc-peer` fixture. Three actual-pipe
tests pass: bounded initialize→response→initialized→request→correlated response,
close retains unsent request as uncertain and is idempotent, broken initialize
write preserves pending ID/method. Incoming bytes are fed one at a time to the
core, proving fragmented pipe reads do not advance handshake prematurely.
Busy rejection happens before request reservation. Child guards reap test peers.

- `scripts/with-local-tools cargo test -p graph-execution --test connection_input --offline`:
  3 pass, exit 0; lockfile updated offline for existing workspace/registry deps.
- `sh scripts/validate-foundation.sh`: 177 Rust + 12 Node pass, exit 0;
  2 ignored standalone helpers are invoked by their parent tests.
  Architecture: 7 packages / 41 direct declarations, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.

No real app-server, account/config mutation, external network, sandbox or native
dispatch. Caller still owns stdout/stderr byte budgets, child lifecycle, deadlines,
epoch authority, permissions, and durable pending-request checkpoints. This bridge
does not persist a request ledger, prove peer execution, or complete W2/W3.
Next: combined bounded read/write/control supervisor with flood/backpressure and
malformed/oversized/truncated actual-pipe frames, then cleanup scope integration.

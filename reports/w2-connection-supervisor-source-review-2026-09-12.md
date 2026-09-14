# W2 direct-child connection supervisor

Source gate ready before code. Read rust-router, domain-cli, m07-concurrency and
m12-lifecycle completely, product AGENTS, current ConnectionIo. Reread OpenDev
foreground.rs:114–172 at d32c660e4eed1a8e988d1fd58da88e41ba641d08 and OpenSandbox
initmode_linux.go:176–226 / initmode_linux_test.go:221–240 at
eed301cca02b261256b2c5e5a23bb1a570c90160. Adopt one consuming waiter and separate
stop/drain/reap observations; avoid ignored reader deadlines and numeric-group
signalling after reap. No reference code copied or reference tests run.

Before patch: attach only a host-owned trusted Child and its corresponding
ConnectionIo; no spawn approval or account routing. Validated 5s runtime/1s drain
limits, poll cancellation before data, pause event delivery without blocking byte
pumps/control, close stdin on stop while draining output. Kill only unreaped
direct child, retain syscall/I/O errors. finish only after terminal observation
or drain deadline, returning any unreaped handle and pending connection metadata.
Spawned scope cleanup remains Unverifiable. No Drop-based blocking reaper or
implicit relaunch. Tests: full RPC flow, timeout with stalled peer/consumer,
explicit cancel, and malformed-frame cleanup. Full containment remains pending.

## Verification

ConnectionSupervisor now owns the direct child and ConnectionIo, checks stop
before data, centralizes try_wait, closes stdin on shutdown but keeps raw drainage,
and refuses new requests after stopping/reaping. Event delivery can be paused
without pausing bounded byte pumps/control. finish returns self if not done;
after deadline it transfers any unreaped child plus connection/raw/error evidence.

- Three integration tests pass: RPC + 512 KiB stderr / EOF / exit-zero reap;
  timeout and cancellation with event consumer disabled; malformed protocol
  cleanup preserving initialize metadata. Test guard cancels/drains/reaps on failure.
- `scripts/with-local-tools cargo test -p graph-execution --test supervisor --locked --offline`:
  3 pass, exit 0.
- `sh scripts/validate-foundation.sh`: 185 Rust + 12 Node pass, exit 0;
  two standalone ignored helpers remain parent-invoked. Architecture stays
  7 packages / 41 direct declarations, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- OpenDev/OpenSandbox revisions rechecked clean. Post-test process lookup found
  no graph-execution-fixture executable still running. Product has no HEAD.

Limits: duration starts at attach, not OS spawn; host must attach matching child
and pipes promptly and poll regularly. No autonomous timer/thread, approved spawn
factory, persistent lifecycle registry, automatic Drop reaper, process-tree
containment or sandbox. All spawned scope cleanup remains Unverifiable. Unreaped
deadline transfer and syscall failures need adversarial/injected coverage; current
tests all reap the owned child. Full W2/W1-I/native dispatch remain gated.

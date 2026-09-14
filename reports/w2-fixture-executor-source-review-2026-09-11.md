# W2 trusted fixture executor: initial Linux adapter

Source gate ready before code. OpenDev clean
`d32c660e4eed1a8e988d1fd58da88e41ba641d08`, foreground.rs:20–175 and
helpers_tests.rs:1–38 under crates/opendev-tools-impl/src/bash inspected.
Adopt independent stdout/stderr progress and timeout/cancel observation; avoid
unbounded line vectors, shell rewriting, inherited environment and ignored
reader completion. Upstream tests read, not executed. Local rustix 1.1.4
src/fs/fcntl.rs API inspected for safe NONBLOCK on owned pipe handles.

Add graph-execution adapter crate outside domain/application/protocol, explicitly
reviewed in architecture allowlist (domain, application fingerprints, sha2,
thiserror and Linux rustix fs; tempfile dev). No Tokio/network dependency needed
for initial synchronous nonblocking Linux fixture supervisor. This does not
remove full asynchronous execution-fabric, PTY or platform requirements.

Host supplies an absolute executable, owned root and explicit environment. Check
command/executable/environment digests and canonical cwd containment before spawn;
use argv directly, env_clear and closed stdin. Scope limited to trusted immutable
fixtures; path/hash prechecks are not protection against adversarial replacement.
Bound fixture execution to 5 seconds, cleanup to 1 second and each stream to
1 MiB. Read raw bytes nonblocking with fair per-iteration chunks; stop on limit,
cancel or timeout, kill only the owned direct child, poll/reap separately from
EOF. Transfer any unreaped child handle to caller on deadline, never claim cleanup.
Report process-scope cleanup Unverifiable until stronger ownership evidence exists.

Tests planned: actual owned Rust fixture executable with exact argv/env/cwd,
binary stdout/stderr/nonzero exit, output flood, timeout and pre-cancel; hash/env
mismatch must fail before spawn. Keep raw outputs separate and no receipt trust
promotion. Unsupported platforms explicitly fail. No account/untrusted repo jobs,
merge or graph writes. W1-C passed; full W2 and W1-I remain open. Skills domain-cli,
m07-concurrency inform bounded control/data handling. No reference code copied.

Parent continuation source gate: reread OpenDev foreground.rs:1–200 and
helpers_tests.rs:1–85 at the same clean revision, then reviewed both worker
fixture files. Upstream display truncation does not test raw-byte EOF at the
exact cap. Before further code, add actual fixture regressions for exact-cap
versus one-byte overflow, zero-cap empty/nonempty streams, pre-cancellation,
and canonical cwd escaping the owned root. No new dependency or permission.
Filesystem preflight and OS spawn are not bounded by the process-loop timeout;
this adapter must not be exposed to untrusted callers or mutable executables.

## Validation after parent review

- Worker Mendel supplied only fixtures/owned.rs and tests/fixture.rs; parent
  inspected both, added two boundary tests, reran and closed the completed agent.
- `scripts/with-local-tools cargo test -p graph-execution --locked --offline`:
  eight Linux integration tests pass. Exact cap retains EOF; one-byte overflow
  and nonempty zero cap truncate; empty zero cap completes. Pre-cancel and cwd
  symlink escape do not execute the fixture. All spawned tests assert reaping.
- `sh scripts/validate-foundation.sh`: 159 Rust and 11 Node tests pass; one
  ignored standalone crash helper remains invoked twice by its parent test.
  Architecture gate: seven packages, 36 declared direct dependencies, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: pass.
- Product has no committed HEAD; source/tests and local command results are
  development evidence, not clean-release provenance. No reference repo modified.

Remaining: owned descendant/pipe-holder adversarial fixtures and reliable scope
cleanup, platform support, asynchronous transport/control backpressure, launch
claim admission/reconciliation, raw artifact publication and full receipt wiring.
This partial adapter does not satisfy full W2, W1-I or any W0–W13 package gate.

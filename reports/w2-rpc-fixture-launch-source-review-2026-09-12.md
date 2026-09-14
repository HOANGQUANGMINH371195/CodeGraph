# W2 trusted RPC fixture host composition

status: trusted Linux fixture composition verified; full W2 open | owner: main
source-gate: ready

Before code, read Codex clean 818f1cca8ccf8899f0f4d59336baebaccf358eed,
codex-rs/utils/pty/src/pipe.rs:134–193 (explicit cwd/env reset/argv/stdin mode),
tests.rs:423–442 (roundtrip and exit assertions); OpenDev clean
d32c660e4eed1a8e988d1fd58da88e41ba641d08, transport/stdio.rs:88–125 (spawn then
fallible pipe extraction). No upstream services/tests executed. Also read product
execution attachment/linux preflight, application environment fingerprint and
current RpcLaunchRepository (untracked product, no HEAD baseline).

Adopt explicit host-selected executable/root/environment and piped stdin, not
ambient inheritance or PATH lookup. Compose ConnectionSetup BEFORE ledger/spawn,
then actual bounded hash/env/cwd checks, register + one-shot claim, direct argv
spawn and ownership-preserving attachment. Reuse existing digest implementation.
No transaction spans spawn. No retry/reset after consumed/uncertain claim.

TrustedRpcFixtureHost is embedding-owned fixture configuration containing one
exact allowed RpcLaunchSpec plus executable/root/environment. A requested spec
must equal that whole host-selected description, not merely its approval_id.
It is NOT a production approval resolver or authenticated host identity. Never
construct this configuration from untrusted wire input. Project/snapshot fields
remain claims; immutable owned fixture root mapping is host responsibility.
This is the trusted-fixture development path permitted by W1-C, not native W3.

Use host SystemTime for ledger timestamps and repeat cancellation/expiry checks
after blocking work. Preserve stage and underlying errors; return attachment
failure with its Child. Spawn/setup failures after claim never reset it. A
cancel/lease race can still occur between final check and OS spawn; immutable
fixture preconditions, filesystem path/hash race, preflight duration and crash
recovery remain explicit limitations, not generalized authorization proof.

Planned actual tests: RPC initialize/context/response and reap with SQLite;
repeat/reopen no duplicate launch; mismatched allowlist and changed env/hash/cwd
reject before ledger/process; invalid setup/cancellation and durable spawn failure.
Add finite rpc-context fixture recording only its explicitly cleared environment,
argv/cwd and initialize input in owned tempdir. No real credentials/accounts.

Resume 2026-09-12: main reread the above source and assertions; both reference
HEADs remain unchanged and clean. Read actual product attachment, supervisor
tests and RPC ledger tests. Reuse the test cleanup guard and real SQLite lease,
but issue timestamps from the host clock rather than historical ledger fixtures.
Before this test patch: verify literal argv/cleared env/cwd/initialize and replay
after reopen; reject altered request/env/setup before registration; copied owned
executable without execute permission must consume exactly one claim on failure.
These are composition gaps not covered by the upstream roundtrip test.

Verification: `cargo check -p graph-execution --locked --offline` exit 0;
initial three targeted tests passed. After adding hash/symlink and post-claim
cancellation cases, `sh scripts/validate-foundation.sh` exit 0: 221 Rust tests,
12 Node tests, 7 packages/41 direct dependency declarations, architecture clean.
Two standalone helper tests remain ignored in direct enumeration and are invoked
by parent tests. `cargo fmt --all -- --check` exit 0. Product remains untracked
with unborn HEAD; no clean revision or upstream runtime execution claimed.

Five new integration tests in `crates/execution/tests/rpc_fixture.rs` prove:
- exact literal argv, canonical cwd, exclusively selected env, client/version and
  experimental initialize parameters; ping match, exit 0, both EOF and reap;
- SQLite reopen/replay leaves event sequence unchanged and only one start marker;
- altered requested argv, wrong env, invalid initialization frame and precancel
  reject before registration; wrong digest and cwd symlink escape also reject;
- an owned executable copy with mode 0600 produces PermissionDenied after claim,
  and reopen cannot consume that claim again;
- a forwarding real-SQLite repository flips cancellation immediately after
  successful claim: host returns claimed cancellation without a process marker,
  and reopen/retry cannot spawn.

Review: explicit args/no shell, env reset, setup before side effects, and retained
attachment-child ownership match source lessons and lifecycle skill. Cancellation
injection is deterministic, not evidence that the last check→spawn race is closed.
The zero-spawn markers cover this owned fixture only. No full current-authority
resolver, multiprocess crash recovery, attachment fault injection, expiry-after-
blocking-I/O test, filesystem TOCTOU protection or process-tree containment yet.
Next: define durable launch outcome/reconciliation without automatic claim reset,
then production host authority and W1-I/W2 acceptance; native dispatch stays gated.

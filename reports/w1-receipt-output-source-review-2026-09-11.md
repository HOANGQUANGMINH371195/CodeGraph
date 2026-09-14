# W1 receipt output verification

Source gate ready before code. Re-read clean Grit
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe` src/git/mod.rs:185–219,589–620
(current preconditions and preserve candidate on refusal), and clean OpenDev
`d32c660e4eed1a8e988d1fd58da88e41ba641d08` foreground.rs:150–172 plus
helpers_tests.rs:1–30 under crates/opendev-tools-impl/src/bash (reader timeout
results ignored; truncation tests concern display, not raw byte provenance).
Reference tests inspected, not run. Adopt state recheck and separate raw artifacts
from display/exit claims. No copied implementation.

Gap and design: neither reference supplies our scoped receipt verifier. Compose
existing local reconcile_check_binding and verify_artifact; host supplies expected
binding, execution snapshot, host identity and independent total retained-byte
budget. Check all expected fields and budget before I/O; reconcile submission,
require exact registered output descriptors before reading either blob, verify
each present output and recheck submission afterward. Return output observations,
not execution authority. Absent output stays None, failed/truncated reports stay
failed/truncated; content success must not change completion.

Tests planned with real SQLite + controlled readers: two matching outputs/no
events, expectation mismatch/budget rejection before open, hash corruption,
missing output registration and cancellation while reading. Skills rust-router,
m04-zero-cost, m06-error-handling. No migration/dependency needed for this step.
Limits: no candidate-byte verification, real host authentication, snapshot
attestation, process execution, atomic cross-resource snapshot, registry/replay
protection or integration grant. Reader deadlines remain supervisor responsibility.

Validation: `sh scripts/validate-foundation.sh` exited 0: 132 Rust + 11 Node
tests passed. Three new tests use actual SQLite registration/query paths and
controlled byte readers. Success opens stdout then stderr and preserves all
receipt claims without events; unverifiable cleanup remains Unknown. Hash mismatch,
wrong expected host/command/snapshot, insufficient aggregate budget and absent
registration fail. A separate SQLite connection cancels during output open and
the post-read query rejects the observation. No live processes/accounts launched.
`cargo fmt --all -- --check` and equality of the two PLAN files also checked.
Receipt registration/replay and real runtime verification remain pending.

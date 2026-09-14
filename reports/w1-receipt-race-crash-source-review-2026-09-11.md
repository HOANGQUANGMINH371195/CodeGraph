# W1 receipt race and process-exit evidence

Source gate ready before code. Re-read Grit clean
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`, src/db/sqlite_store.rs:48–82
and :448–515. Independent connections and one-winner assertion test actual
SQLite transaction exclusion, unlike sharing one mutex-protected connection.
Adopt that testing approach without copying code. Reference tests not run.
Local V8 receipt transaction, SQL insert and replay tests re-read.

Planned tests: eight independently opened Store connections released together;
identical contenders give one insertion and seven replays, conflicting host
claims give one insertion and seven conflicts, with durable exact winner on
reopen and unchanged events. A subprocess-only ignored fixture executes the
same receipt INSERT in a real SQLite transaction and exits without Rust Drop
before or after commit. Parent bounds child lifetime, reaps it and checks
absence/retry before commit versus persistence/idempotence after commit.

Skills rust-router, m07-concurrency. Owned temporary databases only; no account,
external service or untrusted code. Crash fixture tests SQLite transaction
recovery at these boundaries, not power-loss/storage hardware guarantees or
every instruction of production registration. Concurrent threads use separate
connections but are not separate writer processes. Pre-launch admission and
authenticated first-write ownership remain open. No migration change required.

Validation: `sh scripts/validate-foundation.sh` exited 0; 138 Rust tests passed,
11 Node tests passed. One subprocess-only helper is ignored by normal discovery;
the passing parent invoked it twice and required exit 23, then checked both
recovery outcomes. Concurrent test covered identical and conflicting eight-writer
cases, exact persisted winner and unchanged events. Format and PLAN mirror checked.
No production behavior changed; this adds evidence for existing V8 transactions.

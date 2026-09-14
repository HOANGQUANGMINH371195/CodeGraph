# W1 policy/lease contention

Source gate: ready before implementation.

Re-read Grit `src/db/sqlite_store.rs:48–100,448–515`, clean HEAD
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`. BEGIN IMMEDIATE protects read/write;
16 separate connections contend and test asserts one grant/one database row.
Reference test inspected, not executed. Adopt separate connections, joined
workers and durable-state assertion. Add synchronized starts and event ordering.

Local checked lease helper and policy transaction reviewed. Planned tests:
eight policy-bound contenders yield exactly one lease, other results explicitly
Unavailable; policy registration versus both guarded and legacy leasing has only
legal serialized outcomes. Verify event count/order and outbox agree after reopen.
Use owned pre-opened stores, scoped threads and barrier; no shared Store mutex.
No production mutation planned unless tests expose a defect. This is bounded
in-process/separate-connection testing, not multiprocess/crash/formal proof.
Skills: rust-router, m07-concurrency.

## Results

Two tests added in store: eight synchronized checked-lease contenders yield one
lease/fence=1 and seven explicit Unavailable results; persisted lease payload
matches winner. Sixteen fresh fixtures race registration with guarded/legacy
leasing, accepting only legal serialized outcomes and checking policy-before-
lease event order when registration succeeds. Missing-policy guarded attempt
can retry after registration without skipped fence. Reopen/outbox agree.
All workers joined; no live processes/agents left by these tests.

Foundation passed: 112 Rust + 11 Node tests, 6 crates / 30 direct dependencies.
No production changes required. Test scheduling does not guarantee every possible
interleaving occurred; these runs are not model checking or crash durability tests.

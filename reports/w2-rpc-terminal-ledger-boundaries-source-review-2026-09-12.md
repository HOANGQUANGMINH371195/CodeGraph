# W2 terminal ledger contention and damaged-link boundaries

status: done (specified ledger tests); source-gate: ready; scope: store tests and owned SQL fixtures.

- [x] Read outsource source/tests for this task before edits.
- [x] Record adopt/avoid and missing coverage.
- [x] Choose test cases before patch.

Orca clean `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`: reread mutation-receipt
store begin/complete bodies (lines 37–116) and mutation-question DB tests 17–74.
Adopt database-serialized receipt identity, not an in-process lock. Avoid mutable
completion overwrite/discard. Upstream assertions cover replay/mismatched input,
not our independent connection contention; upstream tests not run.

Product source reread: full rpc_terminal ledger, fixture helpers and existing
five tests, cancellation transaction, V13 migration, eight-connection rpc_spawn
test. Rust concurrency skill: each scoped thread owns its own SQLite connection;
only immutable report/barrier shared, no mutex serializing calls in the test.
Open connections before starting workers to keep migration contention separate.

Add identical/differing receipt races, late report after durable cancellation
and expiry, referenced/indexed row damage in owned in-memory fixtures. Corruption
must fail before caller filter and preserve counts; disable FK/immutable guards
only inside disposable fixture connections to simulate storage damage, never in
production. Extend V12 upgrade with exact artifact row snapshot comparison.
Tests still do not prove process execution, CAS bytes or host authentication.

## Validation

Three new tests passed: eight independent connections yield exactly one insert
and seven replay/no-ops or conflicts (two input modes), late report survives
cancel/reopen without new lease/claim, and twelve damaged-link cases fail read,
wrong-scope read and replay before any new rows/events. V12 migration test now
compares complete artifact rows, including descriptor bytes, across two reopens.
No production behavior or migration changed. Main reviewed source and assertions;
no separate reviewer. These are thread/connection races, not OS-crash fault tests.

`scripts/with-local-tools cargo test -p graph-store --locked --offline rpc_terminal`:
8 passed. `sh scripts/validate-foundation.sh`: 272 Rust + 12 Node passed;
`scripts/with-local-tools cargo fmt --all -- --check`: exit 0. Architecture
7 packages, 41 direct dependency declarations, no errors. Test SQL modifies
only owned disposable fixtures; no user data removed.

Final untracked test SHA-256 `crates/store/src/rpc_terminal_tests.rs`:
`050491a5e8bc59d8d441e9d16ab7cc6cd62053e3a4ac7d66ee4b534c4e70d127`.

Next: bind actual terminal result to approved host launch/epoch, publish retained
output artifacts and receipt, expose historical terminal inspection, then crash
reconciliation. Full W2/W1-I and all full product package gates remain open.

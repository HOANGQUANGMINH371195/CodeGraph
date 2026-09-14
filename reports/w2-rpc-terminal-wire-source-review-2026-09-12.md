# W2 terminal receipt wire

status: done (wire only); source-gate: ready; scope: protocol wire/converters/tests.

- [x] Read outsource source/tests again for this task.
- [x] Record adopt/avoid and gaps.
- [x] Choose format/conversion tests before patch.

Orca clean HEAD `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`: read complete
`src/main/runtime/rpc/methods/orchestration/runs/mutation-request-show.ts` and
its request-show test assertions at lines 62–100. Stored original outcomes are
returned without repeating effects. Adopt original-report preservation; avoid
silently treating malformed receipt JSON as absent (`parseReceipt`). Upstream
tests not run. Read product domain rpc_terminal and protocol rpc_spawn,
execution_receipt, execution, analysis and artifact converters/tests.

Use schema v1 at outer receipt; required nullable stdout/stderr/input progress,
required pending array, deny unknown fields in new structs, required boolean
uncertain=true for each terminal pending entry. Reuse existing nested versioned
contracts, then domain constructor for all linkage/limits. No domain serde.
Inherited completion format permits omitted exit_code as unknown; inherited
serde unit-enum forms are not changed by this task. This is not canonical-byte
JSON signing. Wire Debug redacts methods and receipt fields; raw serde errors
must still be sanitized by external callers. Callers must cap input bytes.

Tests planned: exact roundtrip including nulls, every new field missing/duplicate/
unknown, wrong scalar types, nested schema mismatch, false uncertainty, order,
capacity and domain output mismatch. No ledger/host/CAS authority in parsing.

## Validation

Added public wire RpcTerminalReceipt/UncertainRpc/RpcInputProgress and checked
converters. Four new tests cover complete/null-output roundtrip, every field of
the three new objects missing/duplicate/unknown/null, scalar types, nested
schema versions, uncertainty=false, pending order/cap and domain mismatch.
Main reviewed converter and assertions, no separate reviewer.

`scripts/with-local-tools cargo test -p graph-protocol --locked --offline rpc_terminal`
passed four tests. `sh scripts/validate-foundation.sh` passed 264 Rust + 12 Node;
`scripts/with-local-tools cargo fmt --all -- --check` exit 0. Architecture:
7 packages, 41 dependency declarations, no errors. No migration/dependency.

Final untracked product SHA-256:
- `crates/protocol/src/rpc_terminal.rs`: `8f09149aa0e21fa4c0e8bdfa900151f66ba9478bb1896ea9d23aca7ad16ce8f4`
- `crates/protocol/src/rpc_terminal_tests.rs`: `dd75d8a45b2271f4c3e9cf5f4db7a5c48249506e5e2bb3d0c2b0544df9085e35`

Next: durable terminal ledger with exact stored spawn/run/artifact linkage,
atomic event/outbox, replay/conflict/corruption and migration tests; then actual
host/epoch-bound receipt publication and crash reconciliation. Full W2 remains
incomplete. JSON parsing alone neither authenticates origin nor verifies bytes.

# W1 durable execution receipt claims

Source gate ready before code. Re-read clean Grit
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`, src/db/sqlite_store.rs:48–118
and :448–515. BEGIN IMMEDIATE precedes check-then-write; separate-connection
test asserts one winner. Reference tests read, not executed. Adopt atomic
read/compare/insert; do not adopt refreshing immutable records on replay.
Local artifact/check-policy registration and migration tests also inspected.

Design: new V8 table for untrusted ExecutionReceipt, keyed globally by check
run ID and linked to existing task. Require exact stored TaskSpec, compare full
decoded receipt on replay, reject changes without replacement. Reads validate
stored identity/version/task linkage and filter against host-selected full task.
No default/backfill, no trust boolean, no event or lifecycle transition. Allow
late diagnostics for cancelled tasks; storage is not current execution permission.
This is durable receipt-claim storage, NOT pre-launch run admission/host registry
or replay-proof execution. First-write ownership still requires trusted host API.

Tests planned: restart/idempotence/conflict/full task scope, late cancellation,
missing task, corrupted persisted receipt, V7 upgrade preserves old data without
fabricated receipts, current/future migration history. SQL stays in named files;
do not edit V1–V7. Skills rust-router and m12-lifecycle keep transaction local.
No reference code copied, dependencies or account/runtime behavior changed.

Validation: foundation suite passed (136 Rust + 11 Node), including restart,
identical replay after cancel, conflict in host/elapsed/command/cleanup, seven
task-scope substitutions, missing task and corrupted version/identity/task claims.
V7 upgrade preserves prior migration checksums, task/event/outbox fixture payload
and adds an empty receipt table. Future-binary fixture advanced to V9; existing
grouped migration rollback tests remain passing. Format and PLAN mirror checked.
No concurrent receipt writers or crash-injection tested in this step; that gap
remains explicit. First-write trust/host admission is not provided by the table.

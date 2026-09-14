# W1-I integration query separation

Source gate before patch: read ICM `crates/icm-store/src/store/memory.rs`
MemoryStore::store/get, revision `2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42`,
Apache-2.0 inspected. Store wraps writes in BEGIN IMMEDIATE and rolls back
on error; get uses bound parameters and OptionalExtension. Adopt transaction
and parameter preservation; avoid its inline SQL because this repository
requires query files. No code copied. Local source/test review covers
integrate_verified and atomic/idempotent/outbox-failure tests. These provide
the integration-specific assertions that the upstream memory store lacks.

Move exactly three production SQL literals to src/sql files. Keep SQL text,
parameters, transaction boundaries, replay and affected-row checks unchanged.
No migration change. Run foundation and fmt; test-only fixture SQL is outside
this production extraction and remains a separate cleanup gap.

Validation: foundation gate exited 0, including atomic/idempotent/outbox
rollback and reopen tests. Initial fmt check found query-call formatting;
applied cargo fmt and the subsequent fmt check exited 0. No SQL semantics or
migration changed. Log: [foundation queries](validation/foundation-integration-queries-2026-09-12.log).

Independent read-only target audit requested worker model gpt-5.6-luna; worker
completed but could not independently report effective model identity. No
cost/token savings claim. Root confirmed locally that TargetHeadVerifier has
no production implementer and FixtureTarget clones task.project(). Worker
located CodeGraph sync/worktree.ts and worktree-detection.test.ts as next
source-study inputs. These findings are navigation evidence only; root must
read source before implementation. Existing GitSnapshotAuthority should be
reviewed for reuse rather than adding a second fingerprint algorithm.

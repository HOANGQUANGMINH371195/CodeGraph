# RPC ledger integration tests source review — 2026-09-12

Source gate: ready before test implementation. Scope: `crates/store/tests/rpc_launch.rs`, `crates/store/tests/sql/rpc_*.sql`, and this report only. No delegation, production edits, migration edits, or V10 upgrade test.

Read AGENTS.md and PLAN.md §0.4.1 directly; read rust-router, m09-domain (DDD), and m07-concurrency skills directly. Apply domain identity and lease invariants through repository APIs; exercise concurrency with independent database connections and a barrier, without a shared Store mutex.

Reference: `/home/minh/projects/outsource/icm`, revision `2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42`; `git status --short` empty at inspection. SHA-256:

- `crates/icm-store/src/store/memory.rs`: `e1c695eaa7e51e3d86f2ee571a3c4e0a12f0ea20a980369536337148fce2f8b5`
- `crates/icm-store/src/store/facts.rs`: `c498806e036712187761f48fc1f63e172b30f2c4feb49e8203c8de8429d0157a`
- `crates/icm-store/src/store/tests/facts.rs`: `2cf876b8b2012cd3398acbe1f32d77fc8b6e039ea837ba4ec53987766d7bfea6`

Inspected memory `store`, `update`, `delete`, `prune`, and `consolidate_topic` transaction paths, and facts `set_fact_inner` / `set_fact` (47–129). Adopt atomic related writes and failure propagation: successful operations commit, failures roll back, caches change after commit. Avoid manual transaction cleanup in test code, silent rollback-error handling as proof of success, and the early-return hazards of hand-written transaction control. Tests must observe durable state and retry successfully after a targeted failure.

Requested `tests/facts.rs:168–205` resolves to `crates/icm-store/src/store/tests/facts.rs`; file ends at 191. Its `list_all_facts_returns_only_active` asserts exactly one latest active fact after supersession. It is a success-path cardinality test, **not** a fault-injection or race test. Adopt exact cardinality assertions; do not attribute rollback/concurrency coverage to it. The fault-injection adaptation is specific to this ledger contract. Reference LICENSE is Apache-2.0; no source copied.

Target repository has no HEAD commit and its files are untracked at inspection. Read existing `submission_content.rs` fixtures, execution launch replay/race/fault tests, domain/protocol RPC constructors, TaskRepository methods, and event/outbox migration trigger. Existing reports were not substituted for source reads.

Planned coverage: complete descriptor readback across reopen; exact historical registration/claim replay without extra events/outbox; ID conflicts and both alias uniqueness rules; current task/lease identity and expiry boundary; cancellation and release/reacquire fencing; eight simultaneous claim calls with one durable winner; named SQL event/outbox failure fixtures with rollback and retry. Optional corruption coverage if supported by schema. No process spawning or real host approval is implied. Validation results and remaining gaps will be appended after implementation.

## Delivered evidence

Follow-up source gate (before the two requested regressions): re-read ICM `set_fact_inner`/`set_fact` validation, replay and transaction paths; reference revision and clean status remain unchanged. Re-read local `stored_execution_receipt_corruption_is_not_overwritten_by_replay` and its named SQL fixture, plus RPC adapter `read`/`check_time`. Adopt state-preserving rejection assertions and targeted fixture mutation; avoid treating corrupted historical metadata as valid replay or repairing it during a read. Planned additions: negative times at absent/registered/claimed stages, and host column versus descriptor mismatch returning Corrupt from read/register/claim with unchanged events and durable claim accounting. This extends the already-read Rust/DDD skills' identity-invariant checks; no new research scope.

Changed paths: `crates/store/tests/rpc_launch.rs`; eight fixtures under `crates/store/tests/sql/`: `rpc_ledger_counts.sql`, `rpc_event_outbox_count.sql`, `rpc_claim_time.sql`, `rpc_fail_event.sql`, `rpc_drop_fail_event.sql`, `rpc_fail_outbox.sql`, `rpc_drop_fail_outbox.sql`, `rpc_corrupt_host.sql`; this report. No other files edited by this worker.

Twelve external integration tests pass:

- `negative_time_rejects_without_changing_absent_registered_or_claimed_launch`: −1 and i64::MIN return Invalid from both registration and claim before registration, after registration, and after claim. Descriptor readback, counts and complete event list stay unchanged; valid first registration/claim still succeed and the original claim timestamp remains 12.
- `mismatched_host_column_is_corrupt_without_events_or_claim_reset`: named SQL changes only the indexed host column for the fixture launch. After reopening, read/register/claim all return Corrupt for both unclaimed and claimed rows; subsequent read remains Corrupt, events/outbox/ledger counts stay unchanged, and an existing claim retains its timestamp. No repair/reset fixture is applied.

- `full_readback_reopen_and_historical_replay_do_not_append_events`: complete domain equality including process arguments, digests, limits, execution snapshot and connection; missing claim is Unavailable; reopen; historical registration and claim replay after expiry/cancellation; unchanged first claim timestamp; exact event/outbox increments and snake names.
- `changed_id_content_conflicts_even_after_claim_and_cancellation`: twelve descriptor mutations in registered, claimed and cancelled stages; registration and claim return Conflict without event/state changes.
- `alias_ids_cannot_reuse_task_fence_or_host_epoch`: each independent uniqueness rule returns Conflict while lease is valid; cancelled historical host/epoch remains reserved; another host can reuse the epoch. The live-lease checks ensure cancellation cannot mask a broken uniqueness rule.
- `registration_requires_existing_leased_exact_task_owner_fence_and_expiry`: absent/queued/cancelled task and wrong owner/fence/expiry return Unavailable; changed TaskSpec returns Conflict; no rows or events appear on rejection.
- `readback_requires_complete_task_contract`: all six project scope fields and task context filter mismatched readback.
- `expiry_boundary_blocks_first_registration_and_first_claim`: expiry−1 succeeds; expiry and expiry+1 are Unavailable; registered descriptor remains historically replayable.
- `cancelled_registration_replays_but_unclaimed_launch_cannot_claim`: persistent cancellation blocks first claim after reopening.
- `expired_lease_reacquisition_fences_unclaimed_launch_and_preserves_claimed_replay`: expiry/reacquisition increments fence even for the same worker; stale unclaimed launch fails, consumed launch stays false, fresh fenced launch registers and claims.
- `eight_independent_claim_connections_have_exactly_one_durable_winner`: eight preopened Stores, barrier-synchronized scoped threads, all joined; exactly one true and seven false, durable claim/event/outbox verified after reopening.
- `event_and_outbox_faults_roll_back_registration_and_claim_then_retry`: four combinations (registration/claim × event/outbox). Assert the named SQL failure, unchanged ledger/event/outbox counts and full event list, unchanged state across reopen, then remove only the fixture-owned trigger. First retry succeeds, replay is false, and the production trigger still inserts exactly one outbox row.

Validation: `scripts/with-local-tools cargo test -p graph-store --test rpc_launch --locked --offline` → **12 passed, 0 failed**, final follow-up run 0.17s. `scripts/with-local-tools rustfmt --edition 2024 --check crates/store/tests/rpc_launch.rs` → passed. Only this Rust file was formatted. Earlier ten-test runs passed; the twelve-test follow-up run is the final acceptance evidence. Inspected the now-present adapter and V11 read-only to align assertions with the assigned port.

Gaps/limits: no explicit release method exists in TaskRepository or Store, so release/reacquire is covered only through lease expiry/reacquisition; no SQL lease-state rewrite was invented. Corruption coverage is limited to the requested host-column/descriptor mismatch. No V10 upgrade duplication (main owns that evidence), full-workspace gate run, production service/account exercise, process launch, host approval validation, or power-loss/crash simulation. These are integration tests of real SQLite transactions and the public ledger API, not evidence of executable host authorization. Per the latest scope instruction, validation stayed on the assigned external target and file formatting. Follow-up edits stop with this report update.

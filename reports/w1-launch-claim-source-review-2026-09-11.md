# W1 durable one-shot launch claim

Source gate ready before code. Re-read Grit clean
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`, src/db/sqlite_store.rs:48–112,
480–515: IMMEDIATE transaction, one competing winner. Adopt atomic claim; avoid
TTL recycling or same-owner refresh because uncertain process execution cannot
be safely repeated. Reference tests read, not run. Local execution-plan candidate
validation, receipt guard, events and outbox inspected.

Design: V10 one-shot launch-claim table keyed by registered plan run ID. Claim
requires exact plan and fresh submitted/policy/candidate/analysis metadata; no
receipt may already exist. Insert claim and launch-claimed event/outbox together.
Only first committed caller gets true; any later identical request gets false,
even after restart or cancellation. No delete/reset/reclaim API. Clock is audit
metadata, never an expiry. Factor shared candidate validation for plan and claim.

This is at-most-one ledger claim, not exactly-once OS execution, command approval
or host authentication. A host dying after commit leaves uncertainty; it must
reconcile before any new run. Runtime must consume true only once, enforce live
cancellation and approval, and never launch from historical reads/false replies.
Tests: first/replay/restart, wrong/missing plan, cancellation/receipt rejection,
concurrent connections one winner, event rollback and migration preservation.
Skills rust-router, m12-lifecycle, m07-concurrency. No source copied, no live
accounts/services launched, no existing migrations rewritten.

The interrupted permission change completed only shared candidate-validator
extraction; its tool result and worktree were checked before continuing. No
duplicate patch applied. V10 implementation subsequently completed and checked.

Validation: cargo check and foundation exited 0. Foundation: 151 Rust + 11 Node
pass; existing subprocess-only helper is ignored by discovery and run by parent.
Five new tests cover one-shot replay/restart/cancellation, wrong/missing/receipted
plan and invalid time, eight separate-connection contenders (one true), event and
outbox insertion failures rolling back the claim, and V9 migration preserving
plan bytes with no fabricated claim. Direct SQL confirms the new event's outbox
row. Existing migration history/current-future assertions advanced to V10/V11.
Format and PLAN mirror checked. No actual launch or launch crash recovery proven.

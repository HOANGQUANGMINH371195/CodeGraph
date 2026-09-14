# W2 registered launch process-death boundary

Source gate recorded before implementation; root owns implementation and review.

Reread Orca `src/main/runtime/orchestration/orchestration-legacy-worker-terminal-recovery.ts`
and its complete adjacent test at revision
`26f9fd8ea152ad6126c005e5ae602c7d201f4a99`; checked MIT license.
`planLegacyWorkerTerminalRecovery` separates durable dispatch state from live
process identity, rejects mismatched handles and ambiguous identities, and its
tests assert these refusals. Adopt that separation; do not adopt a process from
a missing claim or invent runtime recovery authority. No upstream code copied
or upstream tests run.

Read current product `rpc_launch.rs` registration/claim transactions and
`current_origin`, plus `rpc_terminal_tests.rs::fixture` and the entire
`rpc_terminal_kill_tests.rs` helper/parent. Registration and claim are separate
commits. Existing kill tests begin after claim, leaving registered-only death
untested. This test design adapts the existing owned helper, timeout and Drop
guard; rust-router and m12-lifecycle skills apply.

Pre-code decision: parameterize the test fixture to leave registration unclaimed;
add a registered-only SIGKILL stage. Two read-only reopens must preserve exact
descriptor, absent claim/spawn/terminal, events and outbox counts. Then the
still-valid original lease must permit one claim, reject a duplicate, and keep
that claim after reopen. Also test expired-lease refusal before the valid claim.
No production, SQL or migration change. This verifies ledger crash persistence,
not permission to spawn, authenticated host identity, orphan recovery or power
loss. Run focused kill test, foundation and fmt.

Descendant validation correction: direct `--ignored` invocation failed because
`GRAPH_RUN_SUBREAPER_HELPER` was absent, as intended. The public wrapper supplies
the setup and passed; see `validation/descendant-wrapper-2026-09-13.log`.
The wrapper already invokes the ignored helper during foundation runs.

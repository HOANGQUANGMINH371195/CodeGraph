# W1/W2 development dependency correction

Status: source-reviewed sequencing correction, no runtime executed.

Historical implementation inventory below is superseded by
[W1-C development audit](w1-c-development-gate-audit-2026-09-11.md): the missing
contracts now exist and that development gate passed. Original sequencing
rationale and full W1/W2 acceptance requirements remain unchanged.

## Evidence

- PLAN sections 0.3 and 11 required verified integration authority before any
  execution implementation; W2 depends on W1. Section 4.3 assigns running approved
  commands to Executor, while Verifier validates evidence. Authentic check-run
  evidence therefore depends on W2. No JobReceipt/ExecutionReceipt/ProcessReceipt
  type exists in current Rust crates (rg inspected); CheckObservation remains a
  caller claim, and integration is disabled. W1 cannot prove end-to-end execution
  verification solely by adding policy/lease tests.
- Re-read OpenDev clean `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
  `crates/opendev-tools-impl/src/bash/foreground.rs:40–180`: child spawn, two
  readers, wait/timeout/cancel, then reader join timeouts. ToolResult/exit success
  does not independently establish complete drainage or process-tree teardown.
- Read entire `crates/opendev-mcp/src/transport/process.rs` and `process_tests.rs`:
  descendant enumeration shells out to pgrep, failures become missing descendants;
  kill command result is ignored; selected tests assert bounded count, nonexistent
  PID and empty kill-list no-op. Not proof all owned processes terminated.
  Reference tests inspected, not executed. No code copied or process launched.

## Decision

Distinguish development milestones from full package acceptance. W1-C means
versioned execution contracts + task/policy/lease/storage invariants sufficient
to develop W2 against trusted, owned local fixtures. W1-C is NOT yet complete:
execution receipt/command binding and explicit cleanup states are missing.
W2 produces real process/output receipts; W1-I uses them for real verification
and atomic acceptance. Full W1 and W2 gates remain open until joint evidence.
No early native account calls, untrusted workloads, merge, or graph acceptance.
W0 baseline/platform requirements remain required for complete package gates.

Next implementation: execution contract binding task+lease fence, complete source
snapshot, candidate identity, approved command/config and check policy; model
exit, reaping, stdout/stderr completion and cleanup independently. Claims parsed
from JSON must not construct verified authority. Then own-process fixtures for
the W2 adapter; only afterward end-to-end verified integration.

This supersedes the earlier report's instruction to finish all W1 verification
before developing any W2 execution. It does not supersede its transport failure
fixtures or permit live native dispatch before W1/W2 acceptance.

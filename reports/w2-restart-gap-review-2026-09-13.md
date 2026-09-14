# W2 restart recovery: current source review

Status: design/source review, not implemented recovery. Root reviewed current
source; no worker dispatch, process adoption, reference changes or code copying.

## Source evidence

Orca revision `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`, MIT license previously
reviewed: reread the entire orchestration-legacy-worker-terminal-recovery.ts and
adjacent test. planLegacyWorkerTerminalRecovery rejects mismatched handles,
malformed identities and duplicate process incarnations; completed dispatch
status alone does not settle a live worker. Adopt separate accounting and
process identity checks. Avoid using a durable dispatch/PID as live authority.
This reference supplies a planner, not an OS-level containment capability.

Product source examined:
- execution/src/rpc_fixture.rs::launch_bound: registration and one-shot claim
  commit before command.spawn; record_spawn commits after spawn and before
  setup.attach. Observation persistence failure returns the owned Child, but
  host death prevents that caller cleanup path.
- application/src/rpc_query.rs::RpcLaunchLedgerSnapshot: consistent linked
  accounting only. No current process identity is represented.
- execution/src/rpc_output.rs::publish: analysis run, stdout CAS/metadata,
  stderr CAS/metadata, then terminal receipt are staged writes. Replay relies
  on self.terminal retaining both output buffers in memory.

## Crash boundaries and required acceptance

| Boundary | Durable evidence | Required next capability |
|---|---|---|
| Registered, before claim | exact launch only | existing lease/admission checks before a new claim |
| Claimed, before spawn observation | consumed claim; spawn unknown | supervisor-owned launch identity independent of a numeric PID |
| Spawn observed, before attach | historical PID and spec | authenticated supervisor inventory and process incarnation comparison |
| Attached, before terminal preparation | no durable terminal/pending snapshot | disconnect remains uncertain; never fabricate completed requests |
| Prepared, before publication | buffers only in host memory | durable prepared-output journal with exact descriptors and byte hashes |
| Partial CAS publication | possibly run/artifact rows/blobs | recover exact staged publication, verify bytes, never rerun RPC |
| Terminal committed | linked receipt | existing output re-verification; completion is not containment proof |

Do not add a recovery enum that merely renames these ledger states and call it
restart recovery. The substantive next implementation should be a durable
prepared-output journal for owned fixtures, with bounded manifest parsing,
atomic publication, exact receipt/launch matching and CAS hash re-verification.
It must not recreate a live LaunchedRpc or acquire spawn/retry authority.
Before coding, study current CAS atomic-write/reopen tests and upstream durable
journal failure paths; this review alone does not close that source gate.

Acceptance must kill an owned publisher after each durable boundary, reopen,
finish publication without launching a worker, preserve pending uncertainties,
reject changed descriptors/bytes, and keep one terminal event/outbox entry.
An in-memory replay test does not meet this acceptance. Orphan process recovery
remains separate and requires supervisor/containment capability evidence.

PLAN correction: terminal ledger/output publication already exists; replace the
stale instruction to implement it with the actual remaining crash boundaries.

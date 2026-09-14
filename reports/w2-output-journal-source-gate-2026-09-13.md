# Durable RPC output journal: source gate and implementation contract

Status: source study completed for storage ordering; implementation outstanding.
This follows w2-restart-gap-review-2026-09-13.md, not a recovery completion claim.

## Current source study

OpenDev `state_snapshot.rs::SnapshotPersistence::save/load_from_path` and
`state_snapshot_tests.rs::test_save_and_load/test_load_nonexistent` reread.
Revision d32c660e4eed1a8e988d1fd58da88e41ba641d08; MIT license checked.
Save uses create_new, Unix mode 0600, write, rename. Tests prove roundtrip and
missing-file behavior, not crash durability. There is no file/directory fsync;
load reads unbounded text and collapses corruption into absence. Adopt exclusive
staging and private permissions; avoid overwrite-by-rename, missing fsync,
unbounded loading and corruption-as-absence. No source copied or upstream tests run.

Product DirectoryArtifacts::ingest already provides bounded length/hash checks,
exclusive staging, file sync, nonreplacing hard-link publication, verification
on existing destination, staging removal and capability-directory sync. Its
Drop cannot remove staging on SIGKILL. Tests exercise replay, concurrent writes,
corruption refusal and root scoping; they do not establish power-loss behavior.

PreparedRpcReceipt currently borrows BoundRpcTerminal. Its publish sequence
validates spawn/terminal linkage, records the run, ingests both streams, then
records terminal receipt. Reopening must not reconstruct BoundRpcTerminal:
that type owns supervision evidence and possibly a Child.

## Implementation contract recorded before coding

1. Use host-selected private journal storage. Publish stdout and stderr through
   the existing CAS writer, then publish a versioned manifest last. A returned
   journal handle means both streams and manifest passed durability operations.
2. Manifest contains exact protocol RpcTerminalReceipt and references to its
   two descriptors. Preserve pending correlations/input counters exactly. Bind
   caller's expected launch before publication; manifest has an explicit byte
   budget and schema version. Filenames derive from hashes, never wire paths.
3. Reopen requires an expected launch selected by host and a manifest handle.
   Missing, oversized, corrupt and mismatched are distinct failures. Apply
   bounded reads before JSON decoding, validate domain conversion, then verify
   both stream lengths/hashes under a cumulative budget before ledger writes.
4. Recovery object contains receipt/bytes only, no Child, process ID capability,
   spawn method or successful-execution authority. Share a publication routine
   with the existing prepared path without widening its constructor authority.
5. Recovered publication must retain exact spawn lookup and conflicting-terminal
   checks. Replaying a committed terminal is idempotent. Never reset a claim or
   launch a command. Keep journal until commit is verified; defer GC policy.

Storage ownership must respect dependency direction: execution already depends
on application ports, while graph-source is only a dev dependency. Prefer an
application journal port and infrastructure implementation; do not add a
production graph-execution -> graph-source edge merely for convenience.

## Acceptance required

First implementation slice: protocol rpc_journal manifest encode/decode using
existing terminal wire converter; validate byte cap before parsing, independent
journal version, exact expected domain launch, both output descriptors and
cumulative output cap. Return historical receipt only. Reuse current
rpc_terminal_tests fixture to test roundtrip, version/unknown fields, malformed
input, budget limits and mismatched host launch. Source save/load path reread
before patch; full CAS/journal durability remains outside this protocol slice.

- Owned publisher SIGKILL after each stream/manifest/ledger boundary, parent
  reap, fresh process reopen and replay without a worker spawn.
- Missing/changed/truncated manifest or stream, oversized input, foreign launch,
  conflicting terminal and duplicate replay; corruption must never look absent.
- Exact pending uncertainty and event/outbox counts after repeat recovery.
- Foundation and fmt required. Separate OS-crash evidence from power-loss and
  orphan-process containment. This document adds no passing acceptance evidence.

## Manifest implementation result

Outbox assertion pre-patch: read Store pending_events/acknowledge_event and
outbox_replays_after_restart_with_independent_ordered_consumers. ICM memory.rs
delete transaction groups dependent deletes to avoid stranding records; reuse
the consistency principle, not its SQL (same pinned Apache-2.0 source).
Extend owned SIGKILL test: compare fresh-consumer pending events to complete
history, acknowledge in order, reopen and replay again; drained consumer stays
empty and independent consumer sees exact original events. No new SQL/code path.

SIGKILL pre-patch study: reread product terminal_kill_tests owned Child Drop,
ready marker and signal-9 assertions; publication::finish reaps RPC child before
returning. Upstream snapshot roundtrip tests (same source/revision) do not test
process death. Adapt owned helper/parent for journal handle-committed and
terminal-committed boundaries. Helper exports expected fixture receipt, stages
journal, optionally replays, signals readiness then parks. Parent kills/reaps,
loads handle from DB, replays twice, checks exact receipt, event replay and single
worker-start marker. Fixture export is expected test data, not production trust.
No recursive children or live-account processes; no power-loss claim.

Handle registration pre-patch: read Store::record_artifact/artifact and its
immutable restart test, application ArtifactRepository and upstream OpenDev
save/load/missing tests (pinned source above). Existing scoped artifact lookup
can retain a journal descriptor without migration. Add stage_registered_journal:
verify exact stored spawn and no conflicting terminal, stage CAS, register run,
then manifest artifact. Caller persists/selects a stable manifest ID; reopening
uses existing artifact(id, expected snapshot, graph) then replay verifies bytes.
No automatic scan or launch-derived identity invented. Failed registration can
leave orphan CAS blobs; do not claim recovery before manifest row is committed.
Test reload manifest from a reopened Store and replay with that stored handle.

Bounded encode pre-patch: reread current rpc_journal::encode, existing exact-cap
tests, and OpenDev save serialization before write plus snapshot tests at the
recorded revision. Its to_string_pretty allocates full output. Avoid that
allocation pattern: serialize through a capped Write sink; reject a chunk
before extending past cap, retain TooLarge classification. This bounds buffer
length (not serializer temporaries/domain-to-wire clones). Unit test sink
length after failed writes; existing wire roundtrip/boundary tests unchanged.

Replay refusal test pre-patch: reread replay_rpc_journal lookup/write order,
existing damaged-reader fixture and Orca tests refusing disagreeing handles
(same pinned revision). Test failed verification before writes, and a second
valid but conflicting terminal receipt with its own valid manifest after first
commit. Assert no new events and original receipt retained. No production edits.

Replay pre-patch: reread current PreparedRpcReceipt::publish prerequisite
lookups and staged repository writes, and Orca recovery tests refusing
disagreeing/ambiguous handles at the recorded revision. Adopt exact linkage
before writes; avoid deriving live process rights. Add replay_rpc_journal:
reopen/verify bytes, require exact stored spawn and no conflicting terminal,
register same run/descriptors then record receipt. No claim/spawn port bound.
Test actual staged journal replay twice: first insertion, second no new events,
exact terminal/pending state. Full SIGKILL/discovery remains open.

Reopen pre-patch review: reread upstream load_from_path/save-load tests and
current stage_journal/ArtifactReader paths. Implement bounded single-buffer
manifest read, compare its exact length and SHA-256 before parsing that same
buffer (avoid verify-then-reopen race). Validate handle kind/project/graph/run
against expected launch and decoded receipt, then independently verify outputs.
Return historical receipt only, no Child or publication permission. Extend real
CAS test with reopen, missing output and tampered manifest. Deadlines/root
selection remain host obligations; no full crash recovery claim.

CAS staging pre-patch: reread DirectoryArtifacts::ingest publication/sync and
OpenDev exclusive-temp save path; inspected execution publication fixture.
Implement PreparedRpcReceipt::stage_journal through ArtifactWriter, without
new dependency edges: encode/check all descriptors before I/O, write and
independently verify both streams, then manifest last. Return manifest Artifact
as host-retained handle, not ledger row. Test real CAS decode and no ledger
events, plus refusal before manifest on stream failure. Durable handle discovery
and replay remain future steps; caller must retain the handle.

Follow-up pre-patch source review: reread OpenDev load_from_path and
save/load + missing tests. Its read_to_string(...).ok() loses I/O errors and
has no bound. Add protocol decode_reader using cap-plus-one streaming read,
preserve I/O errors, delegate existing validated decode. Test actual bytes
consumed, oversized invalid JSON rejected before parse, missing/I/O errors and
exact-cap roundtrip. This does not open paths or supply storage authority.

Protocol rpc_journal now encodes/decodes v1 metadata and validates the expected
launch, both stream descriptors and combined output budget. Decode checks the
input byte cap before JSON parsing; encode checks serialized length after
serialization (not a peak-allocation bound). Returned value is historical domain
metadata, never a recovered supervisor. Tests cover exact roundtrip including
uncertain requests, size boundary, foreign host, version/unknown fields, invalid
JSON and combined output size. Foundation and fmt check exited 0; log:
`validation/foundation-journal-manifest-2026-09-13.log`.
Storage adapter, durable reopen, crash tests and shared replay publication are
still outstanding. PLAN copies synchronized. No full journal acceptance claim.

Bounded reader follow-up: decode_reader now consumes at most cap+1 bytes and
preserves I/O errors separately. Exact-cap receipt roundtrip, oversize reader
position and retained NotFound tests passed in foundation (exit 0); fmt check
also exited 0. Log: `validation/foundation-journal-reader-2026-09-13.log`.
This bounds consumed bytes, not arbitrary reader latency or JSON object overhead.

CAS staging result: stage_journal writes and re-verifies stdout/stderr before
manifest via ArtifactWriter, returning an Artifact handle without repository
writes. Real CAS test checks receipt roundtrip, all blob hashes, idempotent handle
and unchanged event history; injected first-stream failure stops publication.
Foundation and fmt exited 0. Final failure-test addition rerun in focused test,
exit 0. Logs: `validation/foundation-journal-stage-2026-09-13.log` and
`validation/journal-stage-focused-2026-09-13.log`. No crash test yet. Generic
writer success depends on its durability contract; this is not power-loss proof.

Reopen result: bounded manifest read verifies exact length/hash on the decoded
buffer, then exact launch/run/scope and both output hashes. Real CAS roundtrip
and injected equal-length manifest tampering/missing stderr pass. Foundation
and fmt check exited 0; `validation/foundation-journal-reopen-2026-09-13.log`.
Return is historical receipt only; handle durability/discovery and replay ledger
remain unimplemented. No process crash/restart acceptance is claimed.

Replay result: replay_rpc_journal verifies manifest/output, checks exact stored
spawn and nonconflicting terminal, and stages run/artifact/receipt registration.
Real journal fixture verifies first insertion and duplicate replay after SQLite
reopen, exact receipt and unchanged event history on duplicate. Foundation and
fmt exited 0; `validation/foundation-journal-replay-2026-09-13.log`.
This is independent of a live supervisor but not authenticated execution proof.
No crash injection, handle discovery or exhaustive failure-matrix evidence yet.

Replay refusal validation: corrupted manifest, missing stderr and a separately
staged valid conflicting receipt all cause replay errors; event history remains
unchanged and the committed original receipt survives. Foundation and fmt check
exited 0; log `validation/foundation-journal-refusal-2026-09-13.log`. These are
in-process failure tests, not process-death or full journal acceptance evidence.

Encode cap result: serialization now uses a bounded Write buffer; a rejected
chunk does not extend it. Exact-cap wire roundtrip and rejection tests plus
buffer test pass. Foundation/fmt exited 0; log
`validation/foundation-journal-encode-cap-2026-09-13.log`. This supersedes the
earlier post-serialization length check limitation. Vector capacity growth,
domain-to-wire cloning and serializer temporaries are not a total-memory cap.

Handle registration result: stage_registered_journal checks ledger prerequisites,
stages CAS, then registers the run and manifest handle. Test drops the in-memory
manifest and Store, loads by host-selected ID/snapshot after reopen and replays
the exact receipt. Foundation/fmt exited 0; log
`validation/foundation-journal-handle-2026-09-13.log`. Existing schema and scoped
artifact lookup reused. No automatic discovery, orphan-blob cleanup, SIGKILL or
power-loss evidence; failure before handle registration can leave unindexed CAS.

SIGKILL result: owned publisher test passed for handle-committed and
terminal-committed boundaries. Helper runs/reaps a real owned RPC fixture before
ready, parent kills/reaps publisher with signal 9, reopens DB/CAS and replays
without a second RPC start. Exact receipt (including pending uncertainty) and
idempotent event history survive. Focused test and foundation/fmt exited 0;
logs `validation/journal-kill-focused-2026-09-13.log` and
`validation/foundation-journal-kill-2026-09-13.log`.
The expected receipt export is test oracle data, not authenticated host metadata.
Intermediate CAS/ledger kill boundaries, outbox-specific assertions, power loss,
orphan runtime recovery and automatic discovery remain unverified.

Outbox follow-up: focused SIGKILL test and fmt check exited 0;
`validation/journal-outbox-focused-2026-09-13.log`. Earlier foundation log
`validation/foundation-journal-outbox-2026-09-13.log` ends with passing doc-tests;
its process handle was missing on resume, so no new exit-code claim is made.
Recovered events were acknowledged in order through acknowledge_event, which
checks the next event_outbox sequence. After reopen/replay the acknowledged
consumer stays empty and an independent consumer sees the same history.
pending_events reads events, not event_outbox: equality alone is not a direct
outbox row-count assertion. This proves ordered acknowledgement for this fixture,
not exhaustive outbox integrity or power-loss behavior.

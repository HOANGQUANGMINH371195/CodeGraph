# W1 — durable task cancellation

Implemented `TaskService::cancel` / `TaskRepository::cancel` and the SQLite
adapter, using the existing cancelled state/event and existing outbox trigger.
No shipped migration was edited. New production SQL lives in separate files.

The domain permits cancellation from queued, leased or submitted, never from
integrated or rejected. An already cancelled task returns false without
overwriting the first actor/reason or adding another event. An unknown task
and empty actor/reason are rejected. The application accepts coordinator
requests; actor identity is attribution, not authentication or a capability.

An immediate transaction changes the state, clears owner/expiry and records
the cancellation event. The original fencing token remains for audit; terminal
state revokes submission authority and no transition currently recycles the
task. The event payload includes requested_by, reason, previous_state,
previous_owner and fencing_token. The outbox trigger commits atomically with
that event. Submitted artifact history is retained; cancellation never deletes
the artifact or source files. Dependents remain blocked/queued, not silently
cancelled or promoted.

## Validation

`sh scripts/validate-foundation.sh` passed after the change:

- Architecture: 6 packages / 29 dependency declarations, unchanged policy.
- 10 Node diagnostic/gate tests passed.
- 25 Rust tests passed, 0 failed/ignored, including four new cases:
  cancellation across all three allowed states, reopen/idempotent replay and
  stale submission rejection; injected event-write failure rolling back state
  and outbox; invalid/missing/terminal requests; blocked dependent preservation.
- `scripts/with-local-tools cargo fmt --all -- --check` passed.

`rust-router` and `m09-domain` informed the explicit transition invariant and
separation between durable cancellation and physical process termination.

## Still required

No independent wire request schema, authentication/capability gate, scheduler propagation, executor kill/cleanup
or cancellation acknowledgement is implemented by this patch. W2 must consume
the outbox and reconcile a live process independently; a cancelled database
task does not prove the process stopped. The broader lifecycle, typed verifier
receipt, AnalysisRun/snapshot authority, rejection/retry and W1 acceptance are
still incomplete. The temporary string-based integrate operation remains a
known gap, not a verified merge path.

## CLI and concurrent follow-up

Implemented `cancel TASK --requested-by ACTOR --reason REASON`, with versioned
JSON containing task_id, state, changed and process_termination_confirmed=false.
The two subprocess tests check repeated cancellation across CLI restarts,
stale submit/lease/integrate refusal, one cancellation event, required arguments,
blank attribution/reason, missing task, error stderr/nonzero exit and help text.

A further SQLite concurrency test uses two independent connections and a
barrier, eight trials each for cancel/cancel, cancel/submit and cancel/integrate.
Repeated cancellation has one winning transition; submit may precede cancel
but never follow it; cancel and integrate cannot both commit terminal events.
Each trial checks the final persisted state and outbox/event equality. These
are actual thread races, not exhaustive scheduling or multi-process stress.
The integrate operation used here remains the documented prototype API.

Final `sh scripts/validate-foundation.sh`: **28 Rust tests and 10 Node tests
passed**, no failures/ignored tests. Format check passed. CLI and concurrency
skills informed the output contract and independent-connection race tests.

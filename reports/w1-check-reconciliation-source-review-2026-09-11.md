# W1-C check binding reconciliation

Source gate: ready before implementation.

Re-read clean Grit `0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`,
`src/git/mod.rs:185–219,589–620`: current precondition check before merge and
regression preserving candidate on refusal. Reference tests inspected, not run.
Adopt read-before-action and preserve state on failure; no source copied.

Local CheckRunBinding and submission verification reviewed. Add read-only
application reconciliation against submitted task/owner/fence/sequence/artifact,
registered policy and exact artifact descriptor. Re-read submission afterward to
detect cancellation/change during lookups. Return the observed SubmittedCandidate,
not a new authority type. No writes or content reads; content verifier remains
separate. Missing/mismatch must never fall back to another candidate or policy.

Limits: submission query does not attest original expiry; only origin owner/task/
fence are reconciled. Run ID uniqueness, approved command registry, actual blob
integrity, filesystem snapshot, host authentication and commit-time revalidation
remain required. Matching these metadata fields alone cannot authorize execution.

Tests planned: real SQLite match/no mutations, wrong owner/fence/sequence/policy/
candidate metadata, missing policy/artifact and cancellation. Skills: rust-router,
m04-zero-cost. Reuse existing repository ports, no dependency or migration change.

Continuation source gate: re-read the same clean Grit revision and both ranges
above before adding regression tests. Add scripted repository observations for
each changed submission field, disappearance and read failure on the second
lookup. This tests the application recheck, not concurrent database isolation;
no claim of protection against changes after return. Existing ports suffice.

Validation: `sh scripts/validate-foundation.sh` exited 0: 126 Rust tests and
11 Node tests passed; architecture guard reports 6 packages/30 direct dependencies.
Three check-binding tests cover actual SQLite match without events, mismatched
owner/fence/sequence/task/candidate/policy, absent registrations, cancellation,
and scripted second-read changes across all six submission fields, disappearance
and repository failure. Reference tests were not executed. Full W1-C stays open:
execution receipts and host/runtime verification are not implemented by this step.

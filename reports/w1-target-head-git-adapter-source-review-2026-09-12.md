# W1-I Git target-head adapter

## W2 joint negative source gate (2026-09-13)

Read execution/tests/composition.rs full owned run→CAS→receipt→reopen flow,
owned fixture visibility-mode zero exit, and application integration policy gate.
Re-read OpenDev foreground cwd selection and Ripwire timeout non-success tests
at previously recorded revisions. Reuse the real runner and existing durable
receipt fixture; initialize a Git root only for a new zero-exit case, obtain
ProjectRef through host Git authority, verify the target independently, then
assert integration rejects the actual runner's Unverifiable cleanup before
and after reopen. Do not modify raw completion to obtain an acceptance.
This addresses joint negative coverage, not production positive acceptance.

Joint verification passed: real fixture exits zero, both streams complete,
raw cleanup stays Unverifiable; CAS publication and durable receipt reopen
preserve it. Git target independently matches. verify_integration returns
Decision error before and after reopening DB, with event list unchanged.
First run failed because unknown-mode returns 64; corrected fixture selection
to the inspected visibility mode. No runtime semantics changed.
Foundation exit 0 and fmt check exit 0 after correction. Logs:
[first](validation/foundation-joint-negative-first-2026-09-13.log),
[corrected](validation/foundation-joint-negative-2026-09-13.log).

## Root retarget source gate (2026-09-13)

Read CodeGraph gitWorktreeRoot/gitCommonDir canonicalization and distinct
worktree/non-Git assertions again. Local authority stores a canonical root,
but observation compares freshly canonicalized root only to freshly reported
Git root. Both can move together if the configured path becomes a symlink.
Adopt exact canonical identity comparison against the stored path before Git
probes. Add Unix regression: rename the admitted repo and symlink its old
path to the new location; existing authority must refuse, while a newly
host-configured authority can read the moved repo. No inode/hostile same-UID
or same-path replacement guarantee is inferred; this closes path retargeting.

Root-retarget regression and foundation command exited **0**; fmt check
exited **0**. [Log](validation/foundation-root-retarget-2026-09-13.log).
The comparison is shared by current_project/source binding/materialization;
it rejects canonical-path drift but does not pin directory inode identity.

## Full identity regression source gate (2026-09-13)

Re-read CodeGraph worktree.ts root/error handling and worktree-detection tests
for non-Git and distinct worktrees, plus local project_from_observation and
target adapter comparison. Adopt per-identity negative fixtures and actual
Git mutations; retain fail-closed errors rather than upstream null. Extend
the existing regression to independently alter all six ProjectRef fields,
restore tracked bytes for a positive control, then make an empty commit to
prove real HEAD change is detected even with unchanged file contents. No
source behavior change or new identity algorithm.

Validation: six field mismatches, actual tracked-file drift, restored bytes
positive control and empty-commit HEAD drift all passed. Foundation command
exit **0**, fmt check exit **0**. [Log](validation/foundation-target-identity-2026-09-13.log).
This tests task-to-configured-authority comparison, not cryptographic host
authentication or atomic acceptance of the observed worktree.

## Fresh timestamp source gate (2026-09-13)

Re-read OpenDev worktree.rs unique-name clock conversion and worktree_tests.rs
porcelain fixtures, revision d32c660e4eed1a8e988d1fd58da88e41ba641d08.
Its default-on-clock-error is suitable for a name seed, not a verification
timestamp; avoid that fallback. Local rpc_fixture::now_ms uses checked epoch
conversion, which we reuse conceptually without depending on execution crate.
Remove constructor timestamp; sample SystemTime after successful Git comparison.
Reject epoch/overflow errors using static metadata error. Test two observations
bounded by caller clock samples; no synthetic timestamp, sleep or fresh-proof
claim beyond local wall-clock observation. Clock rollback remains possible.

Fresh-clock implementation and two-call timestamp regression passed foundation
(`scripts/with-local-tools sh scripts/validate-foundation.sh`, exit 0) and
fmt check (exit 0). Constructor no longer accepts a timestamp; it rejects
control characters in verifier version. [Log](validation/foundation-target-clock-2026-09-13.log).
This supersedes the constructor-timestamp limitation recorded below; clock
rollback, host authentication and atomic observation/commit remain open.

## Resume correction gate (2026-09-13, before patch)

The added test does not compile: TaskSpec needs a validated TaskId and ten
arguments, not four. Re-read TaskSpec constructor and Git fixture helpers;
re-read CodeGraph worktree.ts Git root/common-dir bounded calls and
worktree-detection.test.ts real-repository setup/mismatch assertions. Reuse
real temporary Git fixtures; do not adopt its fail-open null behavior. Fix
test construction, then assert exact matching, claimed HEAD mismatch and actual
tracked-file mutation against the original task. Existing graph-source tests
alone did not exercise the new adapter, so earlier adapter coverage claims
must be read as unproven until this regression and foundation complete.

Correction verification: the real Git matching/claimed-HEAD/actual-file-drift
regression passed. `scripts/with-local-tools sh scripts/validate-foundation.sh`
completed exit **0**, and `scripts/with-local-tools cargo fmt --all -- --check`
exited **0**. [Log](validation/foundation-target-2026-09-13.log).
The constructor-supplied timestamp remains a known limitation: it is not a
fresh clock sample at verification time. No production authentication or
end-to-end real W2 acceptance is claimed.

Source gate: re-read local `GitSnapshotAuthority::current_project` and
`SourceSnapshotAuthority` in `crates/source/src/snapshot.rs`, plus the
independent audit of CodeGraph `src/sync/worktree.ts` and OpenDev worktree
tests. Reuse local Git observation to avoid a second fingerprint algorithm.
The adapter accepts only a host-constructed authority; worker task strings are
compared after observation and never select the root or policy.

Implemented `GitTargetHeadVerifier` in `crates/source/src/lib.rs`, forwarding
the exact observed `ProjectRef` into `TargetHeadVerification` only after full
authority comparison. Invalid metadata and project drift fail closed. This is
still an observation adapter: host authentication, atomic worktree snapshot,
and analyzer execution attestation remain open.

Validation: `cargo test -p graph-source --locked --offline` passed (6 source
tests plus doc tests); formatting was applied. End-to-end W1-I with this
adapter and real W2 receipts remains to be added.

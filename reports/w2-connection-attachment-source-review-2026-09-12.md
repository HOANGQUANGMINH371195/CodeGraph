# W2 connection preparation and attachment ownership

status: implemented and locally verified | owner: main
source-gate: ready
scope: graph-execution preparation/attachment and owned Linux process tests

## Source before implementation

- OpenDev clean `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
  `crates/opendev-mcp/src/transport/stdio.rs:86–198`: connect spawns then takes
  stdout with a fallible early return before publishing the child in state.
  Its `stdio_tests.rs` (read completely) checks connection and echo but not
  post-spawn setup failure ownership. This is scoped source analysis, not a
  reproduced upstream leak or a claim about all tests.
- Codex clean `818f1cca8ccf8899f0f4d59336baebaccf358eed`,
  `codex-rs/utils/pty/src/pipe.rs:134–229`: explicit environment reset, cwd,
  stdin mode and pipe extraction after spawn. Parent-death/process-group and
  Windows containment paths are not copied or claimed verified here.
- Product current untracked worktree, no HEAD claim: domain/command.rs requires
  closed stdin; execution/linux.rs checks fixture digests/cwd/env and then
  spawns; connection_input.rs begins protocol/writer after receiving stdin;
  tests/supervisor.rs previously spawned before fallible configuration and
  manually matched child/pipes/epoch. Read actual protocol Connection::begin
  and actual pipe tests/connection_input.rs as well.

## Adopt / avoid / gap

Prepare and validate initial frame and limits without a child. A consumed setup
then takes all three pipes from one owned Child and binds the same epoch. On
missing pipe or input setup error, return the Child explicitly; do not detach,
retry, implicitly wait forever or infer cleanup. No request bytes are written
by attachment. Reject missing pipes before extracting any; on input I/O failure
stdin may close, while ownership of Child and remaining outputs is retained.

Use a prepared input object to avoid generating initialization twice. Preserve
the existing low-level begin/attach APIs for externally owned transports, with
their existing caller obligations. Do not rebrand fixture CheckCommand as RPC
approval or change its closed-stdin semantics. Approved spawn/identity, replay
registry, containment and W1-I are separate unfinished integration requirements.

Expected tests: valid preparation and successful existing supervisor scenarios;
invalid frame/epoch/handshake/pending limits before spawn; each missing pipe
returns exact same live Child and untouched remaining handles for explicit reap.
No global configuration, accounts or external programs. Run foundation + fmt.

## Verification and limits

- Two new tests cover eight invalid configuration cases and three actual missing
  pipe cases. Each failure returns the original live Child ID; other handles
  remain present; the test owns a cleanup guard before asserting and explicitly
  kills/reaps the child. No process-name lookup or unrelated process signals.
- All four supervisor process tests now prepare before spawn and use the new
  attachment path. They assert zero initial bytes written, one unresolved
  initialize, and not-ready state before polling. Existing input tests pass.
- Foundation: 188 Rust + 12 Node pass; two standalone helpers are exercised by
  parent tests. Architecture remains seven packages/41 dependencies, no errors.
  Targeted attachment/input/supervisor tests and fmt check pass.
- Main review: `PreparedConnectionInput` is crate-private; public ConnectionSetup
  owns one validated initialization and binds one epoch to pipes extracted from
  the provided Child. Existing low-level APIs remain explicitly caller-owned.
- OS fcntl initialization failure has not been fault-injected. Its error branch
  returns Child ownership, but this source observation is not runtime evidence.
  Missing outputs fail before extraction; failure to configure nonblocking
  output is still surfaced by existing I/O polling, not an attachment success
  guarantee of operational transport. AttachmentFailure has no implicit reaper.
- No executable/env/cwd approval, spawn factory, durable launch claim, sandbox,
  process-tree containment, native session or W1-I acceptance is established.

## Parallel source review: next launch boundary

Read-only worker Kant (`01a093d2-d06f-7193-94ca-c7df711b6992`) independently
reviewed RPC launch contracts and returned without edits/services/tests. Main
checked domain/command.rs, domain/check_run.rs, store/execution_launch.rs and
the Codex pipe source above plus tests.rs:394–438 before adopting these next steps:

- Keep RPC launch description separate from closed-stdin CheckCommand. Bind
  exact argv/executable/env/cwd and snapshot to connection epoch, initialization,
  JSONL/piped transport and all byte/pending/time budgets.
- A launch description or CheckRunBinding is not approval. Bind host identity,
  policy/approval and attempt to a one-shot launch claim, with current fencing
  and cancellation checks. Existing claim_execution_launch is check-specific;
  its IMMEDIATE transaction plus event pattern is a reference, not an RPC grant.
- Next owned-fixture implementation must compose preflight → one spawn → setup
  attachment. Rejection must have zero spawn; interrupted claims must not be
  reset/replayed into duplicate dispatch. Exercise concurrent claims, reopen,
  claim-before-spawn, spawn-before-attach and partial-delivery uncertainty.

These are pending requirements, not implemented contracts. Main closed the
completed worker; no live agent or account experiments remain from this review.

# W2 sandbox/egress source study — 2026-09-12

status: implementation partial; source-gate and focused acceptance complete
owner: main agent
scope: bounded Linux sandbox admission/launch contract for `graph-execution`
source-gate: ready

## Source and tests read

- OpenSandbox revision `eed301cca02b261256b2c5e5a23bb1a570c90160`.
- `components/execd/pkg/isolation/bwrap.go:36-181,215-342`:
  `buildArgvWithLifecycle` orders namespace flags, read-only root, virtual
  filesystems, declared workspace/binds, environment policy, seccomp,
  `--die-with-parent`, and the command separator. `validateWrapOptions` rejects
  missing workspace, invalid modes, and non-absolute bind paths before side
  effects. Private network is explicit via `--unshare-net`; shared network is
  an opt-in value passed down from session state.
- `components/execd/pkg/isolation/bwrap_linux.go:146-269`:
  lifecycle wrapping creates inherited descriptors, a status pipe and a setup
  gate; the caller owns parent copies and must close them after `Start`.
- `components/execd/pkg/isolation/lifecycle_linux.go:114-255,288-340`:
  `WaitForIdentity` validates bwrap status and native-gate credentials before
  releasing the setup gate; cancellation/error calls `Abort`. Status EOF/error
  is independently drained and a lifecycle failure invalidates the workload.
- `components/execd/pkg/isolation/probe.go:53-118,151-239`:
  capability probing is per mode and returns explicit unavailable states rather
  than assuming that a binary implies usable namespaces/uid switching.
- `components/execd/pkg/runtime/isolated_session.go:235-312,389-553` and
  `isolated_session_lifecycle_test.go`, `isolated_session_cleanup_test.go`:
  process reaping, lifecycle drain, namespace pins and upper-dir cleanup are
  separate ownership obligations. Failed cleanup retains the session/upper for
  retry; a numeric PID/group is not signalled after the pre-reap barrier.
- `components/execd/pkg/runtime/bwrap_test/bwrap_isolation_test.go` and
  `bwrap_filesystem_test.go`:
  tests assert argument ordering, workspace RO/RW/overlay semantics, private
  network flags, environment clearing/blacklisting and rejection of malformed
  bind options. Namespace integration tests skip explicitly when root, bwrap,
  user namespaces or a required backend are unavailable.
- `components/egress/pkg/policy/policy.go`, `domain_index.go`,
  `policy_test.go`: empty/null policy is deny-by-default; exact and wildcard
  domain rules are normalized and first-match; IP/CIDR rules are kept separate
  from domain rules.
- `components/egress/pkg/nftables/manager.go:42-325` and
  `manager_test.go`: static rules are compiled into explicit allow/deny sets,
  default drop/accept is explicit, dynamic DNS-derived IPs use short-lived
  sets, updates are serialized, and missing-table cleanup is idempotent.

## Flow understood

`declared sandbox spec → capability probe/admission → canonical root/cwd and
bounded env/egress policy → owned launcher → direct-child/group supervision →
reap + stream drain + cleanup receipt`. A policy parser or `pid/group gone`
observation is not execution authority and does not prove containment.

## Adopt / adjust / avoid

- Adopt explicit capability probing, deny-by-default egress, absolute-path and
  declared-root validation, a separate launch gate, independent lifecycle and
  cleanup outcomes, and retryable cleanup ownership.
- Adjust the bwrap idea into a small Rust adapter around an argv vector. It must
  use `Command` directly, bind only the declared root, keep the executable
  inside that root, use private network by default, and return a typed
  `Unsupported`/`Unverifiable` state when bwrap or a required capability is
  unavailable. It must not silently fall back to the current host runner.
- Avoid copying OpenSandbox's privileged lifecycle/native launcher, overlay,
  cgroup, nftables or namespace-pinning implementation into the graph store.
  Those remain future production-host adapters. Do not claim a bwrap process
  group proves `setsid`/namespace escape resistance, host crash recovery or
  race-free PID identity.

## Product gap and tests to add

The current Rust runner only canonicalizes `root/cwd` and uses a private
process-group teardown; it has no sandbox admission, executable-in-root check,
or egress policy contract. Add a standalone `sandbox` module with deterministic
policy validation and a Linux bwrap command builder. Test builder/admission on
all platforms and run an opt-in Linux integration fixture when bwrap plus the
required namespace capability is actually available:

1. the workload can see only the declared root/workspace and cannot read a
   host sentinel outside it;
2. private network is requested by default and declared egress is deny-by-
   default; inability to enforce it is `Unsupported`/`Unverifiable`, never an
   allow claim;
3. a malformed path/policy is rejected before spawn;
4. timeout/cancel still returns owned cleanup state and does not grant retry or
   graph-write authority;
5. one failed sandbox job is an error result, not a daemon panic or ledger reset.

receipt command: `git -C /home/minh/projects/outsource/OpenSandbox rev-parse HEAD`
result: exit 0, revision above; no source repository was modified.

## Implementation and verification

- Added `crates/execution/src/sandbox.rs`: exact absolute bwrap discovery and
  namespace probe, canonical root/cwd/executable admission, `/mnt` declared
  root bind, masked common host roots, explicit environment allow-through,
  private-network request, and fail-closed unverifiable allow-list handling.
- Added `run_trusted_fixture_in_sandbox` without changing the existing host
  runner's default behavior. The sandbox plan remains separate from graph,
  ledger, lease and receipt code.
- Added `crates/execution/tests/sandbox.rs` and a bounded `visibility` fixture:
  3 focused tests pass. They cover plan flags/egress disposition, declared
  mount visibility versus masked host temp, and outside-root pre-spawn refusal.
- `cargo test -p graph-execution --test sandbox --locked --offline`: exit 0,
  3 passed.
- `cargo test -p graph-execution --locked --offline`: exit 0, all execution
  tests passed (1 intentionally ignored descendant helper).
- `cargo test --workspace --locked --offline`: exit 0, all workspace tests
  passed (existing intentionally ignored tests retained).
- `git diff --check`: exit 0 for the changed slice.
- `cargo clippy` and `cargo fmt` could not run because those components are not
  installed; no claim is made for either gate.

review: source-to-patch boundary preserved; W2/full acceptance remains open

# W2 RPC sandbox integration source-gate — 2026-09-12

## Scope

- Product slice: connect the existing `SandboxRuntime` plan to
  `TrustedRpcFixtureHost` while preserving `ConnectionSupervisor` ownership and
  RPC JSONL stdin/stdout semantics.
- Reference repository: `/home/minh/projects/outsource/OpenSandbox`
- Reference revision: `eed301cca02b261256b2c5e5a23bb1a570c90160`
- Product files read before implementation:
  `crates/execution/src/rpc_fixture.rs`, `attachment.rs`, `supervisor.rs`,
  `sandbox.rs`, `linux.rs`, and `tests/rpc_fixture.rs`/`tests/sandbox.rs`.

## Source flow read

| Source | Lines | Observation |
|---|---:|---|
| OpenSandbox `components/execd/pkg/isolation/bwrap_linux.go` | 146–167 | Resolve the exact bwrap executable, build argv, and wrap the caller's existing `exec.Cmd`; the adapter does not replace the caller's pipe ownership. |
| OpenSandbox `components/execd/pkg/isolation/bwrap_linux.go` | 170–269 | The lifecycle wrapper prepares gates/status pipes before `Start`, closes parent/child-side descriptors at the correct boundary, and returns an explicit lifecycle object. |
| OpenSandbox `components/execd/pkg/isolation/bwrap.go` | 41–180 | Namespace, read-only root, workspace, environment, die-with-parent and separator/gate arguments are assembled in a deterministic order; lifecycle failure is fail-closed. |
| OpenSandbox `components/execd/pkg/runtime/isolated_session.go` | 127–245, 267–312 | Session startup configures isolation before start, attaches pipes to the same command, starts one process group, waits/reconciles the process and lifecycle separately, and performs a startup health check. |
| OpenSandbox `components/execd/pkg/runtime/isolated_session.go` | 520–553 | Numeric process-group signalling is guarded against post-exit PID reuse; process exit is published before later cleanup decisions. |
| OpenSandbox `components/execd/pkg/isolation/lifecycle_linux_test.go` | 1080–1195 | Tests validate exact lifecycle setup, fail-closed pre-ready behavior, namespace identity, readiness, output and cleanup. |
| OpenSandbox `components/execd/pkg/runtime/isolated_session_runner_close_test.go` | 31–95, 98–165 | Exit and close paths are observable, cleanup is retried/owned explicitly, admission stops after close, and cleanup is not inferred from an idle timeout. |
| Product `crates/execution/src/rpc_fixture.rs` | 189–358 | Existing host flow is exact-spec preflight → register → claim → spawn → durable spawn observation → same-child RPC attachment; observation failures retain the child. |
| Product `crates/execution/src/attachment.rs` | 59–101 | `ConnectionSetup::attach` must receive the exact spawned child and returns it on missing-pipe/input setup failure. |
| Product `crates/execution/src/supervisor.rs` | 93–234 | `ConnectionSupervisor` owns the child, process group, pipes, timeout/cancel and reap; no second runner may be introduced. |
| Product `crates/execution/src/sandbox.rs` | 122–250 | `SandboxRuntime::plan` performs root/executable/cwd/environment/egress admission and produces an immutable absolute-backend argv plan; allow-list egress is intentionally unverifiable. |
| Product `crates/execution/tests/rpc_fixture.rs` | 157–430 | Existing integration tests require handshake/request/output/reap, exact context, cancel semantics, durable observation and no respawn after reopen. |
| Product `crates/execution/tests/sandbox.rs` | 57–153 | Existing sandbox tests cover private network request, explicit environment, `/mnt` visibility, masked host temp and outside-root rejection. |

## Adopt

1. Add a separate sandboxed constructor/builder path; preserve `new` and the
   direct trusted host as the explicit fallback used by existing fixtures.
2. Build the sandbox plan before ledger claim/spawn so missing capability,
   path admission or unverifiable egress fails closed without consuming a
   process. The plan must use the exact absolute backend path and argv.
3. Configure bwrap around the same `Command` that receives piped stdin,
   stdout and stderr, then pass that exact child to `ConnectionSetup::attach`
   and the existing `ConnectionSupervisor`.
4. Keep process-group creation, direct-child reap, timeout/cancel, bounded
   drain and observation ownership unchanged. Document that the observed PID
   is the host-visible bwrap leader, not a workload PID inside the PID namespace.
5. Test successful RPC handshake/request/output/reap through `/mnt`, plus
   fail-closed allow-list and pre-spawn rejection. Keep a guard for every
   launched child in tests.

## Adjust

- The current Rust adapter has no OpenSandbox native lifecycle gate, cgroup,
  seccomp, namespace pinning, credential switching or firewall controller.
  Its receipt must therefore remain a raw host observation with cleanup
  `Unverifiable`; this slice cannot claim production containment.
- OpenSandbox's long-lived session lifecycle is not copied into the one-shot
  RPC fixture. The product supervisor remains the only owner of pipes and
  process cleanup for this slice.
- Sandbox environment values are encoded by the immutable plan after
  `--clearenv`; the host launcher environment stays empty so the wrapper does
  not receive unrelated host variables.

## Avoid

- Do not call `SandboxPlan::configure` if it forces `stdin` to null; RPC must
  attach a piped stdin. Use its validated backend/argv fields or provide a
  dedicated configure method that preserves caller-owned stdio.
- Do not record a synthetic inner-namespace PID, attach from persisted PID, or
  turn a vanished bwrap group into a containment proof.
- Do not retry spawn or reset a claim after an observation/attachment error.
- Do not treat OpenSandbox lifecycle tests as evidence that this product has
  native gate, cgroup, seccomp or egress enforcement.

## Test plan

- Unit: plan selection uses the exact bwrap backend/argv, rejects allow-list
  egress and rejects an executable outside the declared root before claim.
- Integration: copy the fixture into the declared root; use sandboxed RPC
  launch; assert initialize/request response, `/mnt` cwd/context, bounded
  output, `Reaped`, no unreaped child, and stored `Spawned` observation.
- Failure: unavailable runtime/invalid egress does not register/claim/spawn;
  attachment/observation errors retain the same child for caller cleanup.
- Regression: run the focused RPC/sandbox tests and then the locked offline
  workspace suite; architecture checks remain required.

## Decision

Implement only this composition seam now. Full W2/W1-I remains open until
production host identity/authentication, native lifecycle/containment,
crash-recovery and enforced egress have independent evidence.

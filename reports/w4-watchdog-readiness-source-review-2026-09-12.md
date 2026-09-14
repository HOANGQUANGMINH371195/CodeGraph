# W4 verification gate — watchdog readiness

status: component verified; W4 remains open | owner: main | source-gate: ready

Scope: CodeGraph watchdog integration test only; do not weaken production
supervision, modify authentication, or rebuild any user graph.

Source read before patch on resume: CodeGraph revision
`3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty source fingerprint
`3af09d998723b39c855fd63559d5e87d49d1316ee4156988ac84d2d86132fecb`.

- `__tests__/mcp-ppid-watchdog.test.ts`: wrapper reports PIDs after a fixed
  800ms delay; no readiness event. Stdin-holder owns the write end past wrapper
  death. Assert both child exit within 5s and parent-death stderr; retain these.
- `__tests__/mcp-initialize.test.ts:sendInitialize/tagStreams` and
  `__tests__/mcp-daemon.test.ts:sendInitialize/findResponse`: real JSONL initialize
  response is an observable handshake; use that instead of elapsed startup time.
- `src/mcp/index.ts:startDirect/installPpidWatchdog/stop`: transport registration,
  signal handlers and watchdog registration run in one synchronous block before
  stdin I/O can yield an initialize response. Stop exits after releasing state.
- `src/mcp/session.ts:handleInitialize` and `src/mcp/transport.ts`: validate
  JSON-RPC response identity, capabilities and server name, not arbitrary stdout.
- `src/mcp/early-ppid.ts`, `ppid-watchdog.ts`, `startup-handshake.ts`: early
  capture still has a pre-JS gap; handshake timeout covers abandoned launches.
  This test is for an active server losing its parent, not startup-abandonment.
- `src/extraction/wasm-runtime-flags.ts`: the original fixture may insert a
  synchronous re-exec launcher; preserve that case and add direct flagged cases.
  The detached test child owns a process group containing its re-exec descendant.

Adopt: send initialize from the surviving stdin-holder, forward server stdout
through the wrapper, wait for a valid response before SIGKILL. Report PIDs
immediately so startup failures can still clean up owned children. Isolate cwd
in an owned temp directory; record startup stderr on timeout. Keep the pipe
holder alive after parent kill and assert it remains alive when child exits.
Use bounded cleanup of the exact test-owned detached process groups, including
re-exec descendants; remove only the test-created temp directory afterwards.

Regression design: original re-exec launch, direct flagged launch and direct
launch with a 1200ms preload delay (longer than the old readiness guess). Do not
increase the 5s shutdown deadline. Add a negative control using the existing
`CODEGRAPH_PPID_POLL_MS=0` configuration from `parsePpidPollMs`: after a real
handshake and parent death the direct child must still be running for a bounded
600ms observation, with its holder alive and no parent-death shutdown log.
The three enabled cases still require exit within 5s and the watchdog log.
This distinguishes watchdog shutdown from fixture-induced EOF. No shared dist
mutation is needed; the startup-abandonment timer is disabled in this fixture.

Avoid: treating a passing isolated rerun as proof of the historical cause,
adding a longer arbitrary startup sleep, weakening exit assertions, claiming
startup-race diagnosis from empty stderr alone. The full-suite historical
failure remains recorded. This change removes a concrete test precondition race;
does not prove what happened to historical PID 574684.

- [x] Read current source and tests; recorded adoption before code.
- [x] Implement readiness-based integration fixture and delayed-start coverage.
- [x] Focused supervision checks and independent review.
- [x] Full-suite verification at unchanged source fingerprint.

## Independent review before full-suite launch

Godel reviewed current source and found two P2 cleanup gaps: wrapper `exit`
can precede draining its PID records, and sending group SIGKILL is not proof
of termination. Re-read the fixture event handlers and relaunch group ownership
before fixing. Adopt a captured wrapper `close` completion to drain PID output
before enumerating cleanup targets; bound that wait. Handle inner spawn errors
explicitly, retain partial PID records, signal only owned groups and poll group
absence before removing the fixture. Propagate non-ESRCH errors instead of
silently calling permission/probe failures successful cleanup. Keep the existing
readiness and five-second watchdog assertions; reviewer found those sound.
Add an expected child-spawn ENOENT case with the holder already running; the
test must report the spawn failure and its same afterEach must confirm the
holder group is gone. This exercises partial ownership/error cleanup instead
of relying exclusively on successful startup cases.

## Executed checks

Receipts in `.harness/baselines/`:

- `watchdog-readiness-20260912-01.json`: 3 pass / 0 fail, exit 0 (initial
  handshake replacement; original relaunch, direct and delayed direct).
- `watchdog-readiness-20260912-02.json`: 42 pass / 0 fail in five supervision
  suites, exit 0; includes disabled-watchdog control.
- `watchdog-readiness-20260912-03.json`: 4 pass / 0 fail, exit 0 after partial
  PID tracking and failed-wrapper-spawn cleanup refinement.
- `watchdog-readiness-20260912-04.json`: 42 pass / 0 fail, exit 0 after review's
  stdout-drain and confirmed group-cleanup corrections.
- `watchdog-readiness-20260912-05.json`: 5 pass / 0 fail, exit 0; additionally
  exercises expected child ENOENT with surviving holder cleanup.

Only `__tests__/mcp-ppid-watchdog.test.ts` changed in CodeGraph this turn;
production source, emitted dist, extraction version and package version are
unchanged. No rebuild is required for this test-only change. The first full suite
with `CODEGRAPH_KERNEL_EXPECT=1` completed, receipt
`codegraph-watchdog-readiness-20260912-01.json`: **5159 pass, 0 fail, 9 skip /
5168 tests, 293 files**, runner exit 0, source fingerprint before/after
`a97c8c8661cb6f1976aace0ab3f4b8b7bc2090bdb4440da5358aaca0d5bad0fa`.
This result predates the exceptional-cleanup patch below.

## Exceptional cleanup follow-up source gate

Review of the current callbacks found that a wrapper-close timeout could skip
the group cleanup block, and an isAlive exception on a later timer tick would
escape the Promise executor. Before patching, re-read those exact paths.
Collect cleanup errors while still attempting every known group's termination;
report an AggregateError and retain the fixture on unresolved cleanup. Catch
each poll's errors and reject the Promise. Add an injected EPERM regression
after the first successful poll; assert holder liveness in the ENOENT fixture
before cleanup so a dead holder cannot satisfy the coverage claim.

Final focused receipt `watchdog-readiness-20260912-06.json`: **44 pass, 0 fail**
in five suites, exit 0. Godel re-read the corrections and confirmed all three
requested issues addressed, with no remaining findings in that review scope;
read-only review is not a test result. Agent completed and closed.

Second full suite completed on final source fingerprint
`e0b9c4e9b3264d5875948aefbebdea68ae40c984562abbe00bf960a756f2020a`,
unchanged before/after. Receipt `codegraph-watchdog-readiness-20260912-02.json`:
**5160 pass, 0 fail, 9 skip, 0 TODO / 5169 tests, 293 files**, runner exit 0.
All six assertions in the watchdog file passed, including actual direct/re-exec
parent death, slow startup, disabled control, child-spawn ENOENT cleanup and
asynchronous probe rejection. `git diff --check` on the changed test passed.
No live Vitest process remains after completion. Only test-owned temp fixtures
were removed by their cleanup; no user graph, source or configuration was deleted.

This closes the current full-suite verification failure, not the whole W4
package. Historical failure cause remains unproven; the readiness assumption
and observed cleanup gaps have been corrected and tested. The previous Orders
smoke remains the latest Orders result because runtime/dist did not change:
diagram acceptance is still missing. Next implementation must reopen the
source gate for the remaining W4 graph/context work.

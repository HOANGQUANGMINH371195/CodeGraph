# W2 descendant-held pipes

Source gate ready before code. Reread OpenDev clean
`d32c660e4eed1a8e988d1fd58da88e41ba641d08`, bash/helpers.rs:170–195 and
bash/mod.rs:629–648. Its group-kill test does not assert reaping descendants or
pipe drainage. Adopt owned lifetime tracking; do not equate kill with cleanup.
No upstream code copied or test run. Read cached rustix 1.1.4 process/prctl.rs:
838–876, wait.rs:21–46,77–145,407–433 and Pid::INIT for safe subreaper/wait APIs.

Before patch: add Linux dev-only rustix process feature, isolated subprocess
test supervisor with subreaper enabled, existing owned fixture driver spawns one
finite 1-second pipe holder and exits. Runner must return reaped direct child,
incomplete streams and Unverifiable scope on its 100ms drain deadline; dedicated
supervisor then reaps adopted holder before assertions/exit. No host-global
subreaper or untrusted commands. A separate parent bounds helper runtime and
owns its fresh process group. Only fixture mode explicitly marked as running
under this supervisor may create descendants. No production cleanup guarantee.
Skill m12-lifecycle guides explicit separate ownership and reaping responsibilities.

## Verified outcome

- `scripts/with-local-tools cargo test -p graph-execution --test descendant --offline`:
  parent test passes and invokes the ignored helper in an isolated subprocess.
- Runner observes exit zero / direct child reaped, both streams Incomplete,
  elapsed at least 100ms and Unverifiable cleanup. Dedicated helper subsequently
  reaps exactly the PID written by the owned driver, with exit zero; assertions
  run after reaping. Holder sleeps only one second and launches no descendants.
- `sh scripts/validate-foundation.sh`: 168 Rust + 12 Node pass, exit 0; two
  ignored standalone helpers are actually invoked by their parent tests
  (descendant helper once, existing SQLite crash helper twice).
  Architecture: 7 packages, 39 direct dependency declarations, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- Post-test process lookup finds no graph-execution-fixture executable still
  running. No reference changes. Product remains uncommitted development state.

Rustix process feature is Linux dev-only, used by the isolated test supervisor;
production runner still supervises only the direct child. Test coverage proves
bounded drainage and truthful uncertainty, NOT production descendant cleanup,
cgroup isolation, pidfd ownership or cross-platform support. Full W2 remains open.

# W2 sandbox capability revalidation

## Required capability source gate (before patch)

Read OpenSandbox hardening tests' Landlock active checks and pytest.skip paths
at revision eed301cca02b261256b2c5e5a23bb1a570c90160, Apache-2.0 inspected.
Upstream accurately exposes a skipped capability; Rust early return currently
does not. Read local SandboxRuntime::discover, sandbox runtime_or_skip and both
RPC sandbox discovery branches. Retain optional developer probes but add
GRAPH_REQUIRE_SANDBOX presence as strict failure on unavailable runtime.
Share discovery handling across both test binaries. Unit-test unavailable
optional vs required policy without changing process environment or backend.
Add a script that sets strict mode and runs both suites with full output.
This gates these fixture tests only; no production runtime claim is created.

Implemented shared test support and `sh scripts/validate-sandbox.sh`.
Presence of GRAPH_REQUIRE_SANDBOX makes discovery failure panic; unset retains
explicit optional skip. Script rejects non-Linux before tests (avoids cfg-gated
zero-test success). Unavailable required/optional branches have deterministic
tests. Required gate exit **0**: sandbox 5 tests, RPC 16 tests, including the
two policy tests in each binary. Foundation and fmt check exited **0**.
Logs: [required](validation/required-sandbox-2026-09-13.log),
[foundation](validation/foundation-sandbox-required-2026-09-13.log).
Optional developer tests still skip by design; acceptance must invoke the
required script. This supersedes the CI skip design gap below for this gate.

Inspected `tests/sandbox.rs::runtime_or_skip` and the two sandboxed RPC tests:
discovery failure returns early with an explicit stderr skip message. Ordinary
test success alone therefore cannot establish sandbox runtime coverage.

Executed with `--nocapture` and inspected full output:

- `scripts/with-local-tools cargo test -p graph-execution --test sandbox
  --locked --offline -- --nocapture`: exit 0, 3 passed, no skip messages.
- `scripts/with-local-tools cargo test -p graph-execution --test rpc_fixture
  sandboxed_rpc --locked --offline -- --nocapture`: exit 0, 2 passed,
  12 filtered, no skip messages.

Logs: [sandbox](validation/sandbox-capability-2026-09-13.log),
[RPC sandbox](validation/sandbox-rpc-capability-2026-09-13.log).
No implementation changes. Coverage is declared mount visibility/masked temp,
outside-root refusal, private network argv policy, unsupported allow-list
refusal, and actual RPC fixture handshake/supervision under the backend.
No full descendant containment, cgroup/firewall enforcement, authenticated
execution, platform parity or W2 package completion is inferred. Returning
early on unavailable backend remains a validation design gap for CI acceptance.

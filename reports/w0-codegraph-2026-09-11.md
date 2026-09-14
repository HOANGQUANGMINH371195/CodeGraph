# W0 baseline — 2026-09-11

CodeGraph revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf` (clean checkout).
Host: Linux x64, Node 22.22.1, npm 10.9.2, Rust/Cargo 1.93.1.
Reference commits, branches and root license/manifest hashes are recorded in
[repo-lock.json](../repo-lock.json): 19 clean checkouts, including temp-rs-ddd.
License interpretation and compatibility remain pending.

## Reproduce tooling and builds

From the product repository, provision npm in the ignored local cache:

```sh
COREPACK_HOME="$PWD/.harness/corepack" corepack npm@10.9.2 --version
scripts/with-local-tools node scripts/preflight.mjs
```

On this Ubuntu host, rustfmt/clippy were downloaded as packages and extracted
with `dpkg-deb -x` into `.harness/rust-tools`, without a system installation.
`scripts/with-local-tools` adds the extracted `usr/lib/rust-1.93/bin` to PATH.
Other hosts can use the components declared in `rust-toolchain.toml` with rustup.
The supported baseline/MSRV is now 1.93.1, the version tested here; the previous
1.85 declaration had no supporting compatibility test.

| Package | Version | SHA-256 |
|---|---|---|
| rustfmt-1.93 | 1.93.1+dfsg-0ubuntu6 | 5637586a3abb61497418e14b076d75ed9d127cfc0f29f690beb6a304fb684abd |
| rust-1.93-clippy | 1.93.1+dfsg-0ubuntu6 | 1af7f95f039fbe7c12b64240662df45814dc0e96d3148b18881b6032d79c6142 |

For the CodeGraph commands below, put the product's `scripts/tool-bin` on PATH.
Commands were run in the CodeGraph checkout unless specified otherwise:

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm run build
npm run build:lib
cd codegraph-kernel
cargo build --release --locked
```

All four commands succeeded. The native library was staged at
`codegraph-kernel/prebuilds/linux-x64/codegraph-kernel.node` using `install -D`.
Its SHA-256 is
`2b0486e45eaa36f0a908c236c9c85a121c71996e853e7e40515c7e7f8a6218ea`.
The probe's loader diagnostics confirm that this kernel loaded.

## Full upstream suite

```sh
npm test -- --maxWorkers=2 --minWorkers=1 --reporter=json --outputFile=/home/minh/projects/project-graph-agent/.harness/baselines/codegraph-tests-20260911.json
```

Result: **4,598 passed, 16 failed, 11 skipped**, 4,625 tests in 267 files.
Exit code 1. This is a failing baseline, not an acceptance pass.
Raw results: [Vitest JSON](../.harness/baselines/codegraph-tests-20260911.json).

| Failing file | Failed tests |
|---|---:|
| kernel-dart-parity.test.ts | 4 |
| mcp-callers-truncation.test.ts | 1 |
| nextjs.test.ts | 2 |
| object-literal-methods.test.ts | 1 |
| react-native-bridge.test.ts | 1 |
| ui-steps-api-servers.test.ts | 3 |
| ui-steps-api.test.ts | 2 |
| ui-steps-cross-tier.test.ts | 2 |

An isolated rerun of the seven non-Dart files with `CODEGRAPH_KERNEL=0`
produced **102 passed, 12 failed**: the same 12 failures reproduced without
native extraction. This narrows the investigation; it does not identify their
root cause. Results: [WASM triage JSON](../.harness/baselines/codegraph-wasm-triage-20260911.json).
The four Dart parity failures still need a separate extractor comparison.

## Deterministic context probe

The harness reuses upstream `scripts/agent-eval/probe-factory-closure.mjs`:

```sh
node scripts/codegraph-baseline.mjs /home/minh/projects/outsource/codegraph .harness/baselines/factory-closure-native-20260911.json
```

Run this command from the product repo with a new receipt filename on each run.
The recorder preserves command, commit, input hashes, loader diagnostics and
the upstream metrics. Receipt: [native probe](../.harness/baselines/factory-closure-native-20260911.json).

Query: “how does the dashboard store refresh its metrics and apply a filter”.
The 385-line dashboard fixture delivered 189 source lines and the definition
lines of 8/13 inner functions. Both refreshMetrics and applyFilter are present.
This ratio measures which definitions reached the output, not correctness of
every required relationship or completeness of every function body.

Envelope: 15,939 characters; 15,936 is its pre-finalization length, **not a
budget limit**. Follow-up inspection of the pinned implementation establishes
a 13,000-character soft target and 19,500-character hard ceiling for this tier.
`overBudget: true` concerns the soft target; this fixture does not exceed the
hard ceiling. The earlier inference of a three-character budget overrun was
incorrect. The harness context gateway must enforce its own declared limits.
No agent, account, token/cost measurement, MCP transport acceptance or
multi-service benchmark has been performed by this probe.

## Product checks

## Current deterministic probe (2026-09-12)

Re-read upstream `scripts/agent-eval/probe-factory-closure.mjs` init/index,
reopen/explore and delivered-line measurement before running the existing
product wrapper. No reference source modified. Command:
`node scripts/codegraph-baseline.mjs /home/minh/projects/outsource/codegraph NEW_RECEIPT`.
Process exit **0**, source fingerprint before/after equal:
`75d5c57f6d10b630c62dcc020b9ef1aa21585c1ef0826ee102e43ff20bc2d0a8`.
Revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`; native kernel loaded.
Result: **16/32 inner definitions delivered**, **15,796 characters**, above
13,000 soft target and below 19,500 hard ceiling. Neither source/build
correspondence nor full body coverage is proven by this measurement.
Receipt with hashes, raw output and metrics:
[current factory probe](validation/codegraph-factory-current-2026-09-12.json).
`claude`, `jq`, `codegraph` remain absent from PATH in this environment;
P0.T05 live-agent baseline remains unexecuted. No token/cost savings claimed.

### Historical product checks

`scripts/with-local-tools node scripts/preflight.mjs` reports all eight probes
available. Three preflight tests pass. `cargo fmt --all -- --check` passes after
formatting; `cargo test --workspace --locked --offline` passes all 21 Rust tests.
`cargo clippy --workspace --all-targets --locked --offline` runs successfully but
reports warnings (including missing API error docs, must-use suggestions and
unwraps in tests). Strict lint acceptance remains pending.

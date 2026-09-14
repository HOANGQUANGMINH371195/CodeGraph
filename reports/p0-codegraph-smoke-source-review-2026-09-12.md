# P0.T02 CodeGraph smoke — source gate and execution receipt

## Task record

```text
status: done | owner: coordinator
scope: outsource/codegraph build, tests, Orders fixture index/status/MCP/explore/watcher
depends_on: P0.T01, P0.T03, P0.T04
source-gate: ready
review: coordinator self-check; all requested smoke assertions passed
next: P0.T05 baseline không graph; keep W0 gate open until all Phase 0 gates pass
```

## Source-first study before execution

- [x] Reset this task's source-gate to `pending` before starting the pass.
- [x] Read `codegraph/package.json` and `scripts/build-kernel.sh`: build is
      TypeScript/assets/UI plus an optional Rust native kernel; the kernel is
      staged per platform and a missing kernel falls back to WASM.
- [x] Read `src/index.ts` (`indexAll`, `sync`, `watch`) and the watcher tests:
      indexing is mutex/file-lock protected, sync is incremental, and watcher
      changes are debounced then surfaced through completion/error callbacks.
- [x] Read `src/mcp/transport.ts`, `src/mcp/index.ts`, `src/mcp/proxy.ts` and
      `src/mcp/tools.ts`: the transport is newline-delimited JSON-RPC; MCP
      supports initialize/tools/list/tools/call and server-initiated
      `roots/list`; tool responses are query-only and `codegraph_status` reports
      index health/journal mode/pending resolution.
- [x] Read `mcp-roots.test.ts`, `mcp-initialize.test.ts`,
      `mcp-staleness-banner.test.ts`, `mcp-catchup-gate.test.ts`, `watcher.test.ts`,
      `sync.test.ts`, `status-json.test.ts` and CLI index/status tests. The
      subprocess tests are real stdio tests; unit tests are not enough to claim
      transport behavior.
- [x] Reused the existing `scripts/orders-graph-smoke.mjs` and
      `scripts/agent-eval/probe-explore.mjs` instead of inventing a second
      evaluator. The existing Orders smoke deliberately labels itself as
      index/watch + direct ToolHandler only, so an MCP transport probe remains
      a separate check.

## Adopt / avoid decision

Adopt CodeGraph's explicit lifecycle, source-hash capture, isolated temporary
fixture copy, direct status/statistics checks, real watcher callback and
one-shot `codegraph_explore` probe. Add a separate real subprocess MCP JSON-RPC
probe for `initialize → tools/list → tools/call(codegraph_status,
codegraph_explore)` so the requested surface is proven end to end.

Avoid treating the optional native kernel, a green focused test, a README
claim, an upstream ToolHandler invocation, or a warm index as proof of the
entire MCP/product capability. Preserve the distinction between static graph
candidate edges and runtime truth, and retain source/dist/fixture hashes in the
receipt. Any timeout, malformed response, stale warning, incomplete index or
watch error is a disclosed gap rather than a silent pass.

## Planned checks

```text
scripts/with-local-tools bash outsource/codegraph/scripts/build-kernel.sh
scripts/with-local-tools npm test                 # from outsource/codegraph
node scripts/orders-graph-smoke.mjs <codegraph> <new-receipt>
node scripts/codegraph-mcp-smoke.mjs <codegraph> <fixture> <new-receipt>
```

## Execution results

Commands were run after the source gate was ready:

```text
/home/minh/projects/project-graph-agent/scripts/with-local-tools bash scripts/build-kernel.sh
```

Pass. The Linux-x64 kernel was compiled and staged. The compiler emitted three
warnings (unused C expression / unused `mut`), with exit code 0.

```text
/home/minh/projects/project-graph-agent/scripts/with-local-tools npm test
```

Pass: 317 test files, 5,478 tests passed, 11 skipped, 0 failed. The skips are
the upstream opt-in/platform cases shown by Vitest; they are not converted to
passes.

```text
npm run build
node scripts/orders-graph-smoke.mjs /home/minh/projects/outsource/codegraph .harness/baselines/orders-graph-20260912-03.json
node scripts/codegraph-mcp-smoke.mjs /home/minh/projects/outsource/codegraph fixtures/orders .harness/baselines/codegraph-mcp-20260912-01.json
```

The dist build and both smoke commands exited 0. The raw receipts are:

- [`orders-graph-20260912-03.json`](../.harness/baselines/orders-graph-20260912-03.json): isolated index has 18 indexed files, 63 nodes and 143 edges; five direct ToolHandler question probes exited 0; native watcher observed the added symbol, with no watch errors; source/dist/fixture hashes were unchanged.
- [`codegraph-mcp-20260912-01.json`](../.harness/baselines/codegraph-mcp-20260912-01.json): direct index succeeded with 18 files, 63 nodes and 143 edges; real MCP stdio `initialize`, `tools/list`, `codegraph_status` and `codegraph_explore` passed. Explore returned the Orders route, `OrderService` and `createOrder`; source/dist/fixture hashes were unchanged.

The MCP smoke used `--no-watch`, and its stderr explicitly disclosed that live
sync was disabled. Watch behavior is therefore proven by the separate direct
watch smoke, not inferred from the MCP call.

One initial ad-hoc MCP harness attempt failed because the parent Node process
was launched with `--input-type=module`, which leaked into CodeGraph worker
startup. This was a harness error, not a product result; the durable `.mjs`
probe above runs without that flag and passed.

## Acceptance boundary

P0.T02 is complete for the requested CodeGraph smoke. This does not claim that
the full W0 baseline is complete, that static candidate edges are runtime facts,
or that CodeGraph's README benchmark is an acceptance result. `product_acceptance`
remains false in both raw receipts until the remaining Phase 0 tasks and gates
are complete.

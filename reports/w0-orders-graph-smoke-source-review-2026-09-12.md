# W0 orders graph smoke

status: done (diagnostic only); source-gate: ready; owner/reviewer: main agent.

- [x] Reread upstream probe-factory-closure.mjs index/copy/query flow and full
  probe-explore.mjs. Reuse probe-explore as subprocess for each fixed question;
  do not implement an alternative agent evaluator or score symbol hits as correctness.
- [x] Reread CodeGraph.watch/unwatch implementation and real fs.watch test
  (__tests__/watcher.test.ts:904–935). Native watcher must detect a real file change.
  Existing source revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf is dirty; capture
  source fingerprint and dist hashes, do not claim build/source correspondence.
- [x] Plan: isolated mkdtemp copy of fixture, index/stats and raw nodes/callees;
  close DB, run five upstream explore probes with fixed question text; reopen,
  watch actual edit and wait boundedly for exact new symbol. Preserve receipt
  with commands/results/errors, fingerprints and explicit acceptance=false.

No account/native agent, external workload or reference source edits. In-process
index/watch plus upstream ToolHandler probes are not MCP transport acceptance.

## Runs

`node scripts/orders-graph-smoke.mjs /home/minh/projects/outsource/codegraph .harness/baselines/orders-graph-20260912-02.json`
from product root: exit 0. `node --check scripts/orders-graph-smoke.mjs`: exit 0.
No foundation implementation changed; its last run remains 283 Rust + 13 Node.

- [Final raw receipt](../.harness/baselines/orders-graph-20260912-02.json): 51 nodes,
  127 edges, 7 files (6 JavaScript, 1 YAML); no route nodes or SQL files indexed.
- Native watcher changed one file, completed sync in reported 42 ms and found
  exact ordersWatchProbe. onSyncError list empty; source, dist and fixture hashes
  unchanged. Temporary indexed copy removed after stopping watcher/closing DB.
- All five upstream probes exited 0, **not** five correctness passes. Actual
  output preserved: orientation 3119 chars, order-flow 4574, impact 4547, event
  4574, diagram 86. Diagram says no relevant code. Counts are JS string lengths,
  not tokens/UTF-8 bytes; no live-agent cost/latency comparison.
- Initial [receipt](../.harness/baselines/orders-graph-20260912-01.json) is retained:
  symbol appeared before sync fully finished, causing a database-closed warning
  during cleanup. Wrapper now waits for onSyncComplete as well as symbol presence,
  based on reread watcher.ts:913–973. Do not use run 01 as clean watcher evidence.
- Existing dist is fingerprinted but not rebuilt; kernel load identity is not
  separately asserted. Source revision is dirty, build/source correspondence
  remains unverified. No upstream mutation or independent review.

## Concrete coverage gaps

| Boundary | Current evidence |
|---|---|
| ordersServer → OrderService.create | calls edge present |
| notificationsServer → NotificationService.consume | calls edge present |
| OrderService.create → injected store.createOrder | no callees returned |
| NotificationService.consume → injected store.consume | no callees returned |
| OutboxRelay.dispatch → pending/acknowledge and HTTP consumer | no callees returned |
| query(name) → external SQL file | SQL not indexed in this run |

Order/event probes deliver store/http/service source, but not relay, deployment or
SQL source. The graph is useful partial context, not complete request/event
architecture. Next source investigation: constructor-injected receiver resolution
and external SQL/HTTP boundary evidence; preserve unresolved edges rather than
generic name guessing. Full P0.T02 still needs current build/full suite, CLI status
and actual MCP transport; P0.T05 needs shell/graph agent comparison. W0/W4 stay open.

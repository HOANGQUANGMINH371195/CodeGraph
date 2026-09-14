# W0 orders corpus

status: done (P0.T03/T04 corpus task); source-gate: ready; not agent benchmark.

- [x] Read CodeGraph evaluation/test-cases.ts and runner.ts (fixed IDs/symbol
  oracle and measured API calls), probe-factory-closure.mjs:1–95 (isolated fixture,
  index/explore and exact source-line accounting), and sqlite-adapter.ts:1–95
  (DatabaseSync prepared statements). Revision 3ed73bc127323e63153bf6ec8354afa82ce36aaf,
  dirty checkout from prior graph fixes, not clean upstream. No source changed here.
- [x] PLAN P0.T03 requires HTTP/service/database/queue/test/deployment corpus;
  product currently has no fixtures directory. Existing upstream queue fixture
  is a runtime stub, so it is insufficient as executable cross-service truth.
- [x] Before code: add owned Node 22 corpus using stdlib HTTP/SQLite, durable
  outbox delivery over HTTP to independent notification service. Real loopback
  tests use separate DBs, rollback/retry/dedup assertions. Compose is a deployment
  description only; no Docker/image/service download or live agent run.
  Add five fixed questions with symbol/file evidence and explicit unknowns.

Adopt deterministic corpus and exact expected evidence, not an alternative scoring
engine or upstream agent/account policy. No graph correctness/token claim from
fixture tests. JavaScript corpus does not change the product's Rust implementation.

## Verification and scope

- Added `fixtures/orders`: two HTTP listener factories, OrderService, SQLite
  transactions/outbox, HTTP relay, notification deduplication, deployment files.
  SQL is in named files, not inline application strings. No runtime dependencies.
- One end-to-end test covers POST /orders, DB/outbox, explicit /dispatch, real
  loopback /events delivery, duplicate delivery, failed delivery retained for retry,
  invalid input, duplicate order and rollback after the first insert when enqueue
  conflicts. Test listeners run in one process; databases are independent.
- `node --test test/*.test.mjs` in the fixture: exit 0. Added this test to
  foundation. `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0,
  283 Rust + 13 Node passed (12 existing + 1 corpus), architecture 7 packages/41
  Rust dependencies, no errors; cargo fmt --all -- --check exit 0.
- `benchmarks/orders-v1.json` has five unique IDs, required facts, explicit unknowns
  and evidence file/symbol references outside indexed fixture. JSON parses and all
  evidence files exist. These are authored answer keys, not scored graph results.
- Sorted relative paths + NUL + file bytes SHA-256 over fixture plus question JSON:
  `bf1662e34d62044ea6a5992945b8b01ed94a4ecf5fedfd4c91814f886f47628a`.
  Source CodeGraph fingerprint (existing source-fingerprint.mjs):
  `742e34304011a5836d3bb58e0aec9e11783e817aacd71c8ae43235b0e953845e`.
- Main agent review only; product files untracked, no committed baseline pin.
  No outsource edits, account usage, external service or Docker execution.

## Remaining W0 requirements

Run index/status/MCP explore/watch on an isolated copy, preserve per-question
delivered source and missing relations, and reuse upstream agent-eval for comparable
shell/graph metrics. No new evaluator was introduced. Docker build/runtime and
image digest pin remain unverified. This JS/Node corpus is one test stack, not a
decision to exclude other language fixtures. Full W0/W4 remains open.

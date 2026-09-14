# P0.T05 no-graph baseline — source gate and environment receipt

## Task record

```text
status: blocked-by-environment | owner: coordinator
scope: Orders fixture; upstream CodeGraph agent-eval baseline scripts
depends_on: P0.T01, P0.T03, P0.T04, P0.T02
source-gate: ready
review: coordinator self-check; source study and prerequisite probes recorded
blocker: claude CLI and jq are unavailable; codegraph is not on PATH (a local
         dist launcher exists but the exact harness still cannot run)
next: rerun the exact baseline after the pinned agent/tool prerequisites are available
```

## Source-first study before execution

- [x] Reset this task's source-gate to `pending` before starting this pass.
- [x] Read CodeGraph `scripts/agent-eval/run-all.sh`: the A/B is a headless
      Claude `stream-json` session with the same model/effort, strict MCP
      configuration, bounded budget and optional `||`-joined resumed turns.
      The no-graph arm keeps only the normal file-access tools available.
- [x] Read `scripts/agent-eval/no-cli-shim.sh`: PATH sanitization plus a
      `PreToolUse`/`jq` hook blocks both relative and absolute CodeGraph CLI
      invocations. This prevents the control arm from silently using the
      indexed graph through Bash; the parser separately counts contamination.
- [x] Read `scripts/agent-eval/parse-run.mjs` and its `--selftest` fixtures:
      assistant usage is deduplicated by `message.id`; per-request context is
      summed; result cost/duration and tool calls are retained; residual
      occupancy accounts for compaction/FIFO eviction; explore sufficiency and
      citation-based allocation are reported as relative diagnostics.
- [x] Read `scripts/agent-eval/bench-readme.sh` and
      `parse-bench-readme.mjs`: the campaign uses fixed questions, three turns,
      four repetitions by default, medians for throughput, and pooled
      per-call/byte metrics. A single-turn number cannot establish the
      multi-turn occupancy claim.
- [x] Read `scripts/agent-eval/offload-eval.md`: no-graph accuracy must be
      judged against source-verified ground truth; model-mediated offload is a
      separate arm and must not be conflated with raw graph retrieval.
- [x] Read the product `benchmarks/orders-v1.json` and `fixtures/orders`:
      five fixed architecture/flow questions and authored evidence/required
      answers are the first product baseline corpus.

## Three required source-study answers

### 1. What was read?

Source revision: CodeGraph `3ed73bc127323e63153bf6ec8354afa82ce36aaf`.
Relevant source hashes at the start of this pass:

```text
run-all.sh       6df9a9bf7a59e32ad75dff97e8f4aea22148c4c0b03261ee9d5644d1476c77f4
no-cli-shim.sh   c78dce9f9c3d0e69d7acd34295a8753ae3276a716bd0d6fa82a47561122101a4
parse-run.mjs    5e430b735b74be11c3a02ec357e636e293a48e94fde0384bd09f27ded4767b89
bench-readme.sh  ea39934c3ed2af80f68606ff33616f534edfd6febb111cca454f07189a60adfb
offload-eval.md  fd302522a0d9be512731c65e4aaccfce9ba3d48685d5cd5a02117e94578b2269
```

The relevant test-like assertions are the parser's in-file `--selftest`
fixtures for deduplication, compaction, FIFO eviction, multi-turn stitching,
sufficiency and allocation. The product fixture and question ground truth are
the acceptance inputs; no upstream README result is treated as a product fact.

### 2. What flow and failure behavior was learned and adopted?

The baseline flow is:

```text
fixed question(s) → sanitized agent session → find/grep/rg/Read only
→ stream-json receipt → parse-run metrics → ground-truth review
```

The with-graph arm is not part of P0.T05. It is reserved for the later same-
question comparison, where the only retrieval variable is CodeGraph MCP. The
control must not receive `.codegraph` output through the CLI. Missing MCP,
startup races, contaminated shell calls, malformed logs, non-success result
events, or unavailable credentials are disclosed and excluded/blocked rather
than converted into zeros or simulated runs.

This adopts the upstream separation of throughput, persistent context
occupancy and answer correctness. It also adopts measured token accounting and
multi-turn resume semantics; it avoids writing a second evaluator with
different attribution rules.

### 3. What is still product-specific?

The Orders ground truth, evidence paths, and the policy that the baseline is a
control for the future Harness graph/context compiler are product-specific.
The future Harness may use a Codex-native runtime and native subagents, but it
must first preserve an equivalent control/with-graph receipt contract. No
Claude account, Plus/Pro model availability, Luna/Astra routing, or swarm
behavior is inferred from this blocked local probe.

## Prerequisite probe

Probe command, run from `/home/minh/projects/project-graph-agent`:

```text
for x in claude jq node npm codegraph rg grep find; do command -v "$x"; done
node --version
npm --version
claude --version
codegraph --version
find fixtures/orders -maxdepth 3 -type f -print | sort
```

Observed:

```text
claude: MISSING
jq: MISSING
node: /home/minh/.nvm/versions/node/v24.18.0/bin/node
npm:  /home/minh/.nvm/versions/node/v24.18.0/bin/npm
codegraph: MISSING on PATH; /home/minh/projects/outsource/codegraph/dist/bin/codegraph.js exists
node: v24.18.0
npm:  12.0.2 (the pinned local baseline probe records 10.9.2)
Orders fixture: present, 18 files including Dockerfile, compose, JS source,
tests and 11 SQL files
```

The parser self-test was independently runnable and passed:

```text
node scripts/agent-eval/parse-run.mjs --selftest
```

This validates the metric parser only. The agent baseline itself was not run:
without `claude` and `jq` there is no valid upstream harness invocation. The
local CodeGraph dist launcher is not enough: `run-all.sh` requires an explicit
executable and an indexed target, while P0.T05 must still obtain a real
stream-json agent run, tool restriction, token/cost/latency measurement and
ground-truth correctness result.

## Acceptance boundary

P0.T05 remains unchecked. This receipt proves the source gate and records a
reproducible environment blocker; it does not claim a no-graph baseline, a
CodeGraph advantage, W0 completion, or any multi-account/subagent result.

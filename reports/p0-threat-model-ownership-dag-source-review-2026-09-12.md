# P0 threat model, ownership và dependency DAG — source review (2026-09-12)

## Task record

- tasks: `P0.T07`, `P0.T08`
- status: complete for target-stack/ownership/threat-model/DAG artifacts;
  runtime and adapter acceptance remain open
- scope: product workspace plus the 23 pinned outsource repositories
- source-gate: ready
- implementation: no production-code change; this receipt is the pre-code gate
- evidence: source and test reading only; no live account, external service,
  sandbox, browser, analyzer or subagent runtime was launched by this task

## Source-first record

### 1. Sources re-read for this task

| Decision area | Source and revision | Flow/tests used |
|---|---|---|
| Native subagent identity, depth, capacity and wait | [`codex`](https://github.com/openai/codex), `818f1cca8ccf8899f0f4d59336baebaccf358eed`; [`spawn.rs`](/home/minh/projects/outsource/codex/codex-rs/core/src/tools/handlers/multi_agents/spawn.rs:47), [`control/spawn.rs`](/home/minh/projects/outsource/codex/codex-rs/core/src/agent/control/spawn.rs:603), [`wait.rs`](/home/minh/projects/outsource/codex/codex-rs/core/src/tools/handlers/multi_agents/wait.rs:60) | depth refusal, capacity/reservation before thread creation, inherited environment/exec policy, status subscription and deadline wait |
| Execution isolation, readiness and cleanup | [`OpenSandbox`](/home/minh/projects/outsource/OpenSandbox), `eed301cca02b261256b2c5e5a23bb1a570c90160`; Python sandbox adapter, `execd`, egress policy and secure-container guide | create → provision → Running → endpoint/readiness; cancellation and partial-create cleanup; network/mount/capability policy |
| Context scout, cache freshness and advisory partition | [`ripwire`](/home/minh/projects/outsource/ripwire), `48222d62f41c6e15f60855127c1d9ee06b3aed4c`; [`packtask.h`](/home/minh/projects/outsource/ripwire/src/packtask.h:1), [`ingest_cache.h`](/home/minh/projects/outsource/ripwire/src/ingest_cache.h:1), [`partition.h`](/home/minh/projects/outsource/ripwire/src/partition.h:1), [`lanes.h`](/home/minh/projects/outsource/ripwire/src/lanes.h:1), MCP/serialization tests | final serialized budget, ambiguity/refusal disclosure, content/parser/config/checksum cache rejection, dirty handoff, partition/lane as advisory |
| Event loss, endpoint ownership and remote execution | [`orca`](https://github.com/tt-a1i/orca), `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`; worker launch, status authority, reconnect and SSH-boundary sources/tests | lost transcript/event → unverifiable; generation/identity guards; host owns process and artifacts |
| Terminal concurrency and background lifecycle | [`opendev`](https://github.com/EntropyLabsAI/opendev), `d32c660e4eed1a8e988d1fd58da88e41ba641d08`; spawn manager/runner, interrupt and background tests | separate task lifetime from spawn call, cancellation token mismatch risk, bounded work must include detached lifetime |
| Graph/evidence authority and deep-analysis boundary | [`codegraph`](https://github.com/tt-a1i/codegraph), `3ed73bc127323e63153bf6ec8354afa82ce36aaf`; [`joern`](https://github.com/joernio/joern), `7c1163d96705d354d7c1957531487a63af34dda6`; [`codepropertygraph`](https://github.com/ShiftLeftSecurity/codepropertygraph), `e7b6e8da670e4b58a64ba153d197041c25fd798a` | fast index/context path, CPG pass lifecycle, partial/unresolved deep query and artifact publication |
| Security execution and scoped analysis | [`T3MP3ST`](/home/minh/projects/outsource/T3MP3ST), `29824d5625ede419ac8cdae418c8f4c72c6270f7`; [`browser`](/home/minh/projects/outsource/browser), `d693f49872fc72676baecfb3626b1876eadaf57d`; [`OpenSandbox`](/home/minh/projects/outsource/OpenSandbox) | authorized scope, redaction, output/budget caveats, URL/search extraction, sandbox and license boundaries |
| Persistence, worktree and retrieval | [`grit`](/home/minh/projects/outsource/grit), `0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`; [`icm`](/home/minh/projects/outsource/icm), `2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42`; [`zvec`](/home/minh/projects/outsource/zvec), `67ea1fa65ff99ee4c3a5bf4f2c0a6799c0390ead` | symbol claims/locks/worktrees, scoped memory and invalidation, local vector index/rebuild and crash/reopen concerns |

Existing detailed flow receipts remain the primary evidence for the source
read: `outsource-runtime-audit-2026-09-11.md`,
`outsource-infrastructure-audit-2026-09-11.md`,
`outsource-graph-audit-2026-09-11.md`, `outsource-retrieval-audit-2026-09-11.md`,
`outsource-codex-audit-2026-09-11.md`, and
`ripwire-source-review-2026-09-12.md`. Their test names are source evidence,
not tests executed by this task.

### 2. Adopt / avoid decisions

- Adopt a Rust-owned control plane for ProjectRef, graph version, evidence,
  task DAG, admission, budgets, leases, worktrees, event ledger and merge
  policy. Domain and application crates do not depend on an analyzer, model,
  terminal UI or database implementation.
- Adopt Codex native subagents under one root account. Codex owns native thread
  lifecycle; the Harness owns task admission and acceptance. `Luna` is a
  worker-model policy value only after runtime capability probing; it is not an
  authority or a credential selector.
- Adopt process boundaries for Ripwire, Joern/CPG, browser and T3MP3ST. Their
  result is bounded, versioned, hostile input, candidate evidence. Every path,
  symbol and range is re-captured and rebound to the accepted snapshot before
  it enters a context pack or graph publication.
- Adopt OpenSandbox as an optional execution-isolation adapter. `Running`,
  `Ready`, `Completed`, `Destroyed` and `Unverifiable` remain separate states;
  a lost response cannot trigger blind create retry or host execution.
- Adopt SQLite + versioned Refinery migrations for the current local ledger,
  Zvec as an optional retrieval index, and CodeGraph as the fast graph path.
  The choice is reversible behind ports; changing storage is not part of P0.
- Avoid importing a second scheduler, memory authority, agent runtime,
  account-rotation pool, raw analyzer cache, or full-body Ripwire task pack
  into the core. Avoid treating score/rank/quality, `exitCode=0`, missing
  events, a warm cache or a lane forecast as proof of success or graph truth.

## Target stack and ownership

### Target stack v1

1. Rust 2024 workspace: domain/application/protocol/store/source/system/
   execution/CLI crates already present in the product repository.
2. `graph-domain` owns value objects and state invariants; `graph-application`
   owns use cases and ports; adapters own SQLite/Refinery, filesystem, process,
   native Codex, browser, analyzer and terminal concerns; `graph-cli` is the
   composition root.
3. SQLite bundled locally is the durable task/event/evidence ledger, with
   versioned Refinery migrations, transaction/CAS and recovery receipts.
   CodeGraph remains the fast graph adapter; Joern/CPG is an optional deep
   query adapter; neither is embedded into the domain.
4. Codex app-server/native subagents are the only required agent runtime.
   One account is selected by the host. Root keeps its current model; workers
   use the configured Luna default only if the runtime probe says it is
   available. No multi-auth rotation is required for correctness.
5. OpenSandbox is the isolated execution backend for untrusted/deep jobs;
   trusted local execution is allowed only under an explicit policy and must
   never be silently substituted when an isolation capability is required.
6. Ripwire is an optional `RipwireContextScout` CLI process. It can improve
   orientation and candidate ranking but is outside graph authority, leases,
   scheduler, memory and merge decisions. Its default output is metadata/ranges,
   followed by Harness re-capture under the final byte budget.
7. Zvec/embedding, ICM memory, browser/search, Archify, Joern/CPG and T3MP3ST
   are opt-in adapters with independent manifests, license records,
   capability probes, time/byte limits and fallback/unknown states.

### Workstream owner map

Owners are stable responsibilities, not model names. The root coordinator is
the escalation owner for cross-cutting, security, merge and graph-publication
decisions; Luna workers may prepare scoped artifacts but cannot self-approve
them.

| Workstream | Owner | Primary responsibility | First gate |
|---|---|---|---|
| W0 | Root / release | repository lock, fixtures, toolchain and baseline | reproducible source/tool manifest |
| W1 | Domain + persistence owner | protocol, ProjectRef, assertions, task/event store, migrations | schema/CAS/crash/replay |
| W2 | Execution-isolation owner | process/PTY, sandbox, mounts, egress, cancellation and cleanup | receipt + teardown + no host fallback |
| W3 | Codex runtime owner | app-server/native subagent binding, identity, model/capability probe | native lifecycle and shared quota |
| W4 | Graph/context owner | CodeGraph, graph versions, source rebind, ContextBroker and Ripwire scout | precision/freshness/budget/fallback |
| W5 | Fleet/merge owner | admission, task DAG, leases/fencing, mailbox, worktrees and merge queue | deps/conflict/retry/recovery |
| W6 | Terminal/UI owner | headless event projection, terminal host, TUI and reconnect | event parity/backpressure |
| W7 | Browser/evidence owner | scoped search/navigation/extraction and citations | egress/scope/hash/fallback |
| W8 | Retrieval/memory owner | FTS/vector, Zvec, embedding provider and ICM invalidation | namespace/recall/rebuild |
| W9 | Boundary-adapter owner | manifest/API/IaC/runtime/SQL links and capability receipts | cross-boundary fixtures |
| W10 | Deep-analysis/security owner | Joern/CPG and scoped T3MP3ST findings | query mapping/timeout/verifier/license |
| W11 | Diagram owner | Archify IR, architecture/data-flow/sequence views and navigation | evidence-linked artifact |
| W12 | Graph-improvement owner | worker candidates, critic, regression, reindex and feedback | candidate → verify → publish |
| W13 | Root / release | platform, license, benchmarks, packaging and final compatibility | product DoD + published receipts |

## Threat model

The primary assets are source/worktree contents, accepted graph facts and
evidence, task/attempt state, write authority, native session identity, model
quota, terminal output, memory and generated diagrams. The following are
security and integrity boundaries, not optional quality improvements.

| ID | Threat / untrusted input | Required control | Authority / acceptance evidence |
|---|---|---|---|
| T01 | Source, README, docs, logs, memory and analyzer output contain prompt injection | label all external text as data; structured parse; escape before prompts; never replay retrieved text as control instructions | ContextBroker + adapter; hostile-output fixture |
| T02 | Path traversal, symlink escape, TOCTOU or dirty checkout drift | bind `ProjectRef`/root; reject symlink/non-regular escape; capture hash/size/range; re-read and rebind before acceptance; pin dirty fingerprint | Source/evidence verifier; containment, race and stale tests |
| T03 | Build/install/dependency or repository command executes with host authority | classify tool risk; sandbox untrusted jobs; explicit argv/cwd/env/mount/network policy; no arbitrary install in the control-plane worktree; no silent host fallback | Execution owner; sandbox/capability/teardown receipts |
| T04 | Credentials or model/account state leaks through child prompt, cache, logs or egress | same-account opaque binding; no credential bytes in context/graph/receipt; redact output; explicit egress and secret-vault policy; no account rotation in core | Codex/execution owner; secret non-export and egress tests |
| T05 | Terminal escape sequences, output flood or malicious stderr exhausts UI/ledger | retain bounded raw stdout/stderr/exit/argv/hash; separate control/data channels; cap bytes/lines/queue; neutralize display; preserve truncation | Terminal/execution owner; raw-before-projection and backpressure tests |
| T06 | Lost event, fake completion, timeout or replay duplicates side effects | separate agent/thread, job, process, worktree and graph states; idempotency key; durable event receipt; timeout/lost host = `Unverifiable`; reconcile before retry | Runtime/ledger owner; crash/lost-event/cancel/reconcile tests |
| T07 | Worker or child escalates write scope, lease, capability or merge authority | immutable task scope; monotonic child permissions; epoch/fenced lease; independent worktree; CAS merge; critic and GraphWriter are separate | Fleet/merge owner; recursive-spawn, overlap and stale-fence tests |
| T08 | Stale/corrupt/cache-poisoned graph or context is accepted as current | cache key includes project/snapshot/content/parser/config/graph version; checksum/trailer/bounds; reject or reparse corrupt cache; freshness gate and last-good preservation | Graph/context owner; cold/warm, drift, corruption and dirty tests |
| T09 | Ranking, quality, partition, dynamic/name-based edge or deep-analysis claim becomes fact | candidate/observation/accepted assertion types; provenance, coverage, ambiguity and unknowns; verifier-owned publication; Ripwire lanes advisory only | GraphWriter; golden ambiguity/partial/unresolved fixtures |
| T10 | SQL/migration/store corruption, injection or partial multi-writer update | bind parameters; named query identity; versioned immutable migrations; transaction/CAS; generation/tombstone; recovery and backup policy | Persistence owner; crash/rollback/concurrency tests |
| T11 | Browser/search creates SSRF, cookie leakage or unscoped exfiltration | explicit URL/domain scope; isolated session when identities differ; egress allowlist; bounded extraction; URL/time/hash citation; no visual-capability overclaim | Browser/evidence owner; scope/egress/citation tests and AGPL packaging review |
| T12 | Autonomous swarm creates a spawn storm, quota starvation or duplicate task | global/depth/active/attempt/deadline caps; semantic dedup; root-owned admission; same-account shared quota; child delegation re-enters the same gate | Fleet + Codex owners; spawn-storm/starvation/quota tests |
| T13 | Copyleft or unpinned upstream code is accidentally shipped in core | manifest every source/binary/license/NOTICE; process boundary does not alone settle distribution obligations; legal gate before packaging AGPL/ELv2 material | Release owner; license inventory and artifact SBOM/receipt |

### Non-negotiable state rules

- `exit_code == 0` is not `Completed` without the expected receipt and
  verifier result.
- `Running` is not `Ready`; `TimedOut`, `Cancelled`, `Crashed` and
  `Unverifiable` are not successful completion.
- A cache hit is not evidence freshness. A rank/quality/lane forecast is not
  a lease, permission, dependency readiness, merge approval or graph fact.
- Any required isolation/capability that is unavailable fails closed. An
  optional adapter may degrade to a named coverage gap and the last accepted
  graph, never to an apparently complete answer.
- Every state transition is scoped by `ProjectRef`, graph revision, task
  attempt, owner and generation; late events cannot resurrect terminal work.

## Dependency DAG

### Package DAG

```text
W0 Repo/toolchain/fixtures/baseline
 └── W1 Rust protocol + domain + task/event/evidence store
      ├── W2 execution host + sandbox + PTY + cancellation
      │    └── W3 Codex app-server/native subagents
      ├── W4 fast graph + source rebind + context compiler
      │    ├── W7 browser/search evidence (also W2)
      │    ├── W8 memory/vector retrieval (also W1)
      │    ├── W9 boundary/runtime/IaC adapters (also W2)
      │    └── W11 Archify diagrams (also W9)
      └── W2 + W3 + W4 ──> W5 fleet/admission/worktree/merge
                              └── W6 terminal/event UI (also W3)
W2 + W4 + W5 ───────────────> W10 Joern/CPG + scoped security
W5 + W7 + W9 + W10 ─────────> W12 swarm-driven graph improvement
W6 + W7 + W8 + W9 + W10 + W11 + W12 ──> W13 release/compatibility/benchmark
```

The package table in `PLAN.md` remains the normative dependency list. A
package can prepare interfaces before all predecessors finish, but it cannot
pass its integration gate or publish runtime capability before its listed
predecessors pass.

### Mandatory execution order for the Ripwire path

```text
P4.T07 operation profiles + one final budget/fallback
   → P4.T08 CLI process adapter + hostile output + rebind
   → P4.T09 cache/freshness + CodeGraph/FTS fallback + parity
   → P4.T10 one computation/front-door renderer parity
   → P5.T13 verified/heuristic swarm handoff + advisory partition/lane
   → P9.T17 quality/trace/test feedback through verifier
```

This path is not a replacement for W1/W4 graph authority. If Ripwire is absent,
malformed, stale, capped or license-blocked, ContextBroker returns a disclosed
coverage gap and uses accepted CodeGraph + FTS/source ranges. Ripwire cannot
change W5 worker identity, lease, write scope, quota, dependency, approval or
fan-in.

### Critical-path and parallelism rules

- W0→W1 is the foundation gate. W2 and W4 may develop in parallel after the
  relevant W1 contracts; W3 waits for W2 plus the Codex capability probe.
- W7/W8/W9 can develop as independent adapters after their listed scope and
  evidence ports exist. They must not introduce a second scheduler or graph
  writer.
- W5 is the first package allowed to admit a swarm. It requires native identity,
  execution cleanup, graph context and durable task state together.
- W10 runs only after scope/approval/sandbox/graph mapping gates; security
  findings remain candidates until independent verification.
- W12 runs only after candidate output, critic, regression fixture, graph
  reindex and merge publication are all available. W13 is the only release gate.

## Acceptance and status boundary

P0.T07/T08 are complete as planning artifacts because this receipt names the
target stack, single owner for every workstream, trust boundaries, controls,
package DAG, Ripwire sub-DAG and acceptance evidence. This does **not** mark
W0–W13, any external adapter, native Codex runtime, sandbox enforcement,
license closure or benchmark as complete. Those remain gated in `PLAN.md`.

Validation performed after writing the artifact:

```text
cmp -s /home/minh/projects/outsource/PLAN.md /home/minh/projects/project-graph-agent/PLAN.md  PASS
bash scripts/validate-foundation.sh                                      PASS (existing workspace gate)
python3 scripts/validate-context-schemas.py                              PASS (existing workspace gate)
cargo test --workspace --locked --offline                                  PASS (isolated rerun)
```

The first validation invocation accidentally ran `validate-foundation.sh` and
another full Cargo test concurrently. One pipe test then observed the other
process's timing and failed; the isolated foundation rerun and the independent
full Cargo run both passed. This is recorded as a verification-harness race,
not silently counted as a product pass.

The source repositories were not modified and no external tests are claimed
as executed here.

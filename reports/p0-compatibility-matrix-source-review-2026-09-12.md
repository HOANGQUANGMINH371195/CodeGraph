# P0.T06 compatibility matrix — source review receipt (2026-09-12)

## Task record

- status: complete for the compatibility-matrix artifact; adapter execution gates remain open
- scope: all 23 pinned repositories in `repo-lock-20260912.json`
- depends_on: P0.T01, P0.T03, P0.T04, Ripwire source study
- source-gate: ready
- tests executed: product foundation/schema/workspace gates pass; no external integration was
  installed or launched by this task

## Source-first checklist

- [x] Reset the task gate to `pending` before this review.
- [x] Read the live repository lock, root manifests/licenses and existing source-flow audit
      reports before assigning a boundary or license status.
- [x] Re-read the Ripwire paths used by its row: `src/packtask.h`, `src/handoff.h`,
      `src/partition.h`, `src/lanes.h`, `src/prcontext.h`, `src/situ.h`, `src/tracein.h`,
      `src/quality.h`, `src/mcpserver.h`, and the related cache, handoff, partition, lane,
      test-gate, trace, situational-awareness and CLI/MCP parity checks.
- [x] Record whether a capability is observed source behavior, a product target, or only a
      reference pattern. No README performance claim is treated as an acceptance result.

## Three required source-study answers

### 1. What was read?

The repository names, commits, status and observed root licenses come from
`repo-lock-20260912.json`. Detailed execution-flow evidence comes from:

- `reports/outsource-graph-audit-2026-09-11.md` for CodeGraph, Joern, Code Property Graph,
  Archify and T3MP3ST;
- `reports/outsource-runtime-audit-2026-09-11.md`, `reports/outsource-infrastructure-audit-2026-09-11.md`,
  and `reports/outsource-codex-audit-2026-09-11.md` for Codex, Orca, OpenDev, T3Code,
  OpenSandbox, Ghostty, GRIT and RTK;
- `reports/outsource-retrieval-audit-2026-09-11.md` for context-mode, ICM, Zvec,
  SQLite-vector, Ruflo and LLMRouter;
- `reports/ripwire-source-review-2026-09-12.md` plus the Ripwire source/tests listed above
  for the newly added structural-context scout.

The product itself is Rust 2024 with `refinery` migrations, a domain/protocol boundary that does
not depend on SQLite or external analyzers, and CLI/system adapters that own process, parsing and
filesystem concerns. That boundary is a product constraint, not an assumption about any source
repository.

### 2. What flow and failure behavior was learned and adopted?

The matrix separates each integration into an explicit boundary and assigns one failure posture:

- fast graph and source slices remain the primary local path;
- deep analyzers, browser, vector retrieval, memory, terminal UI and security tools are optional
  adapters with versioned receipts;
- model/agent/orchestration repositories are references or native-runtime adapters, never a
  second task/lease/GraphWriter authority;
- untrusted or remote process loss is `unverifiable`, not success or clean exit;
- malformed, stale, capped, unavailable or license-blocked integrations fail closed or degrade
  with a coverage gap while preserving the last accepted graph;
- raw tool output is first stored as bounded evidence, then projected into context; no integration
  may turn its own ranking, score, cache, model label or prose into an accepted fact.

Ripwire specifically adds the operation-profile and one-call terminality lesson: a fixed section
bundle can combine ranking, selected bodies, callers, notes and tests under one final budget. Its
`shown/total/capped`, `ambiguous/unresolved/declined`, dirty snapshot and heuristic-vs-verified
markers become fields in Harness contracts. Its `partition` and `plan-lanes` predictions remain
advisory and cannot mutate worker identity, lease, capability, quota, scope or merge state.

### 3. What still requires product-specific work?

No repository supplies the exact product compatibility contract. The Harness must still implement
and test: adapter manifests, process argv/environment/cwd pinning, source and artifact hashes,
license/NOTICE closure, time/byte/resource limits, capability probes, schema/version negotiation,
snapshot rebind, cancellation and crash recovery, and a single verifier-owned publication path.
The matrix therefore records expected gates rather than claiming those integrations are available.

## Compatibility matrix

| Repository | Observed capability / entry boundary | License observed | Harness mode and owner | Failure / required gate |
|---|---|---|---|---|
| `codegraph` | Node/TypeScript indexer, Rust kernel, CLI/MCP, watcher and context queries | MIT | Fast graph adapter; W4 owns graph snapshot and source rebind | Dirty checkout and kernel/JS ABI drift are blocked; P0 smoke, schema/parity, freshness and full-suite gates |
| `codepropertygraph` | CPG schema/protobuf and loader/pass lifecycle | Apache-2.0 | W10 interchange adapter; Harness IDs are namespaced by backend+snapshot | Unknown kinds/endpoints and loader persistence are reported; loader fixture, compatibility and license gates |
| `joern` | JVM CPG frontends and typed data-flow/reaching-definition queries | Apache-2.0 | W10 optional deep-analysis process | Timeout, partial tasks and semantic configuration become gaps; sandbox, query/source mapping and cancellation gates |
| `archify` | JSON architecture IR validation, layout and staged HTML delivery | MIT | W11 presentation process/adapter | Invalid candidate never overwrites last-good artifact; IR/evidence mapping and browser visual gates |
| `codex` | App-server protocol, native subagent spawn/wait/resume and shared auth thread state | Apache-2.0 + NOTICE | W3 native runtime; Harness owns admission and ledger | Capability mismatch, lost events and child identity uncertainty do not fabricate completion; native mock/live opt-in gates |
| `codex-multi-auth` | Shadow-home/account selection, affinity, refresh and quota routing patterns | MIT | Reference only for telemetry/refresh-lock failure modes | No account rotation or credential authority in core; one-account policy and secret non-export gate |
| `opendev` | Rust agent fleet, terminal jobs, parallel dispatch and TUI events | MIT | W2/W5 execution/reference adapter | Unbounded queues, no-op callbacks or remote loss are not completion; bounded stream, teardown and receipt gates |
| `orca` | TypeScript orchestration, transcript reconciliation, status feed and terminal host | MIT | W5/W6 reference for observer/reconnect behavior | SSH/socket loss is unverifiable; host ownership, generation, reconnect and no-resurrection tests |
| `grit` | AST symbol/function claims, worktrees and serialized merge | Apache-2.0 | W5 optional claim/worktree adapter | Claims are not Harness leases; dirty base, overlap, merge CAS and user-edit preservation gates |
| `ruflo` | Router, planner/swarm, memory/Graph-RAG patterns and handwritten HNSW | MIT | W5/W8/W12 reference or narrow adapter | No second scheduler/memory authority; recall/namespace/fallback benchmark and license gates |
| `browser` | Zig Lightpanda MCP/CDP, search providers and semantic-tree extraction | AGPL-3.0-only | W7 optional browser process | Unsupported visual/page/search capability is disclosed; sandbox, citation/hash, egress and legal gates |
| `OpenSandbox` | Sandbox argv/mount/network/capability lifecycle | Apache-2.0 | W2 execution isolation adapter | Missing capability/egress enforcement is `Unsupported`/`Unverifiable`, never host fallback; containment and teardown gates |
| `T3MP3ST` | Authorized whitebox ingest, reachability, packing and red-team decomposition | AGPL-3.0-only | W10 scoped security process | No unscope execution or finding promotion; approval, sandbox, redaction, verifier and copyleft gates |
| `t3code` | Codex server/UI session runtime, child registration and event/control surface | MIT | W6 UI/reference adapter | UI must not own graph semantics; event parity, queue bounds, stop/reconnect and receipt gates |
| `ghostty` | Zig terminal/VT implementation and `libghostty-vt` ABI | MIT | W6 optional VT backend; currently disabled | ABI is not assumed from source; pinned ABI probe, headless parity and platform gate |
| `rtk` | Rust shell-output compaction/proxy and hooks | Apache-2.0 | W2/W6 optional output projection | Raw stdout/stderr/exit/argv/hash retained before compaction; passthrough and output-integrity gates |
| `context-mode` | MCP intent/batch search, FTS5/BM25 output artifacts and compact resume | Elastic-2.0 | W4/W8 optional artifact retrieval | ELv2/package review pending; actor/fleet quotas, byte caps, session isolation and stale-artifact gates |
| `icm` | Local memory MCP with episodic/permanent records, feedback and decay | Apache-2.0 | W8 rebuildable memory adapter | Memory is not graph authority; namespace, TTL/source hash, dedup, invalidation and restart gates |
| `zvec` | Local vector/full-text/hybrid collection and retrieval engine | Apache-2.0 + NOTICE | W8 optional semantic backend | Embedding/provider identity and rebuild are explicit; namespace, recall, crash/reopen and license gates |
| `sqlite-vector` | SQLite extension with exact/quantized vector scan operators | Apache-2.0 | W8 optional benchmark/storage adapter | Extension load/ABI and quantization are not called HNSW; exact-oracle, update/delete and bundled-SQLite gates |
| `LLMRouter` | Offline KNN/RACER routing and held-out evaluator patterns | MIT | W3/W5/W12 research-only policy seam | Deterministic fallback, capability/privacy/quota filtering and OOD abstention; no live routing before shadow eval |
| `temp-rs-ddd` | Rust DDD/Clean Architecture project structure and migration conventions | MIT OR Apache-2.0 | Product code-style/reference only | Do not copy application semantics; dependency-direction, migration rollback and clippy/test gates |
| `ripwire` | C++23 deterministic ranked context, task packs, cache, quality/trace/lane CLI and optional MCP | Apache-2.0 | W4 optional process-boundary context scout | Name-based graph and ranking are candidates; schema/exit/byte-time/corrupt-cache/parity/rebind gates |

## Required adapter-test catalog

The matrix is not complete until each enabled row has a fixture and receipt for:

1. manifest/license/NOTICE and pinned binary/source identity;
2. exact input scope, argv/environment/cwd, output byte cap and timeout;
3. malformed, unavailable, stale, partial and cancellation behavior;
4. source/artifact hash and revision rebind before acceptance;
5. deterministic replay and no cross-project/worktree contamination;
6. last-good preservation for mutating artifact paths;
7. candidate-only output until the Harness verifier/GraphWriter accepts it.

Current product validation for this document:

```text
cargo test --workspace --locked --offline                  PASS
bash scripts/validate-foundation.sh                         PASS
python3 scripts/validate-context-schemas.py                 PASS
cmp -s PLAN.md /home/minh/projects/outsource/PLAN.md         PASS
```

No external row is marked runtime-ready by this source review. P0.T05 remains blocked by missing
`claude`, `jq` and the expected baseline launcher; P0.T07/P0.T08 and all adapter-specific gates
remain open.

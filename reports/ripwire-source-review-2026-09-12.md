# Ripwire source-study receipt — 2026-09-12

## Scope

- Repository: `/home/minh/projects/outsource/ripwire`
- Revision: `48222d62f41c6e15f60855127c1d9ee06b3aed4c`
- License observed: Apache-2.0 (`LICENSE`)
- Status: source/doc audit only; no build, benchmark, installation, MCP
  connection, cache write, or claim of compatibility/execution.

## Sources read

| Source | SHA-256 | Lesson used |
|---|---|---|
| `README.md` | `d1fcbbdea2098c0f38a678a9a0be3f4202cdd9a004a107be7c32cdca2132075b` | structure-first context, bounded task packs, explicit limits/calibration |
| `skills/ripwire-before-you-build/SKILL.md` | `9e667a9606eed312f0dfcae36204943d453eb5c19e8597c900ea8313272d857` | reuse/seam/impact/test planning before edits |
| `skills/ripwire-handoff/SKILL.md` | `1974ef9fafdf18dc0a8528e2915651197f196f39a47c2543e1835e5e2f7170e2` | verified-versus-heuristic handoff; dirty commit stamp |
| `skills/ripwire-graph-query/SKILL.md` | `ba50688fad6987f5df324a3c658c3199bee94230fccdf5661c08375a21395414` | closed, bounded graph-query language and cap disclosure |
| `src/serialize.h` | `8d4115cf6e0b655bdc86c99cf8c7976060c212fbc93658fad741df1b13bc7d1d` | pre-priced serialization and non-silent byte/token-cap disclosure |
| `src/graph.h` | `5663e2cb92fd9f192a3c9077197576b6f02e074058ad11e43f49d741a09e07a0` | ambiguity/unresolved/declined resolver accounting and selector refusal |
| `src/partition.h`, `src/verbs_change.h`, `docs/COMMANDS.md` | `7bddf6f88056f3e11960e35db8c2e6a8057b4462fab046c9d222d15e490703b2`; `6165347d4eef0bf4b3cdcfe86869f1ecb3d6ed5e6f55c899ba398354e35a9959`; `110e990fc16f9f4f8a2f3a460457fe6e7d7541b48c1b31b7345333af3b97f6d9` | `--partition` is deterministic candidate-context carving (common core + call-graph-community slices); `--plan-lanes` is a structural-only collision/landing forecast, explicitly not task semantics or runtime behavior |
| `src/ingest.cpp`, `src/ingest_cache.h`, `src/mcpserver.h`, `test/` fixtures | inspected in working tree | content-hash cache/version rejection; MCP is optional and needs contract tests |
| `docs/ARCHITECTURE.md`, `docs/COMMANDS.md` | inspected in working tree | deterministic ingest/rank/serialize pipeline; task lenses may include bodies and must not become a Harness default |
| `skills/ripwire-orient/SKILL.md`, `skills/ripwire-orient/map-before-you-read.md` | inspected in working tree | task partition/lane plans are orientation inputs, never execution authority |

## Adopt

1. An optional, process-isolated `RipwireContextScout`: it contributes ranked
   context candidates only after Harness binds paths/ranges to accepted evidence.
2. Structured disclosure for ambiguity, unresolved/declined extraction, caps,
   dirty snapshot and fallback condition.
3. Context/Handoff separation between verifier-backed references and heuristics.
4. Pre-serialization final envelope budget and CLI/MCP semantic-parity tests.
5. Cache identity must include extractor/parser/config/ignore/version and reject
   corrupt or stale artifacts rather than silently reusing them.
6. Retain only candidate metadata/range from the scout. The Harness re-captures
   evidence and owns bounded range composition; Ripwire's full-body task-pack
   posture is incompatible with the product's no-long-file-by-default goal.
7. Treat partition/lane output as untrusted advisory text: its pre-trim overlap,
   structural claims and model/effort recommendation cannot select workers or
   alter capability, lease, quota, write scope, dependency or integration.

## Avoid

- Treating name-based structural edges, ranking, churn/co-change, docs/notes or
  `tested` signals as runtime/semantic/accepted graph facts.
- Embedding Ripwire's MCP schema, global skills, cache blob, or scheduler into
  the Harness authority path.
- Repeating README performance/token claims as product results.
- Falling back from a failed/capped query with an apparently complete response.

## Plan impact

`PLAN.md` §3.8 and `P4.T06` define the bounded spike and its required contract,
golden, cache, dirty-snapshot, parity and benchmark gates. They now additionally
forbid forwarding Ripwire full bodies, docs/notes, or quality scores as Context
facts, and constrain partition/lane output to advisory briefs. `P9.T16` now
covers 23 audited outsource repositories. No implementation checkbox was marked
done. The follow-up source read adds explicit hostile-tool parsing, digest-only
resume and an invariant test that advisory partition/lane fields cannot mutate
Harness swarm authority.

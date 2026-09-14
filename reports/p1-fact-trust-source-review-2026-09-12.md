# P1.T07 fact assertions, evidence and trust — 2026-09-12

status: done (scoped) | owner: coordinator | source-gate: passed
scope: `crates/domain`, `crates/application`, `crates/protocol`, `crates/store`
depends_on: P1.T01–P1.T06
review: parent self-review complete | independent review: pending | blocker: none | next: P1.T08 context pack contracts

## Source-first checklist

- [x] Reset this task's source-gate to `pending` before the source pass.
- [x] Select sources by mechanism: evidence freshness, provenance, candidate
  disclosure, schema validation, graph mutation and repository boundaries.
- [x] Read the relevant source and tests in CodeGraph, Ripwire, CPG, Archify
  and `temp-rs-ddd` before implementation.
- [x] Record the main flow, failure branches, adopted mechanisms and gaps.
- [x] Decide implementation and regression tests; the gate is no longer pending.
- [x] Recheck the implementation against each source behavior after coding.

## Source study and decisions

### CodeGraph — evidence is bounded observation, not trust

At commit `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, read:

- `src/types.ts` (`Node`, `Edge`, `ExtractionResult`, `Subgraph`): nodes and
  edges carry provenance separately from identity; unresolved references and
  low confidence remain explicit instead of becoming edges silently.
- `src/graph/parameter-call-evidence.ts`
  (`parseMarker`, `parameterCallEvidence`): a producer marker is accepted only
  when schema/producer/status/coverage/source hashes/config and exact edge
  identity match; stale or malformed state returns a named refusal. The result
  is explicitly `observed-source-only` and still requires runtime assumptions.
- `__tests__/parameter-call-evidence.test.ts` and
  `__tests__/parameter-call-evidence-mcp.test.ts`: exact coordinate joins,
  foreign collision handling, stale source/config refusal, bounded output and
  no mutation; the MCP surface says “not runtime proof”.
- `src/db/schema.sql`, `src/db/migrations.ts`, `src/db/queries.ts`: provenance
  and unresolved status are persisted as separate fields and migrations; a
  heuristic edge is replaceable within an owned source scope, not silently
  mixed with extractor edges.

Adopt named evidence status/refusal, exact scope/hash binding, explicit
`candidate`/unresolved state and provenance orthogonal to confidence. Avoid
using retrieval confidence or a cache hit as acceptance authority.

### Ripwire — per-edge provenance and freshness disclosure

At commit `48222d62f41c6e15f60855127c1d9ee06b3aed4c`, read:

- `src/model.h` (`Symbol`, `Reference`, `FileHealth`, `IngestResult`): the
  ingest output is deterministic; parse degradation, skipped content and
  reparse counts are disclosed as facts with explicit not-measured sentinels.
- `src/graph.h` (`Graph`, resolution/edge accumulation): `outProv` is a
  per-edge axis independent of rank and ambiguity; `ambOut`/`unresolvedOut`
  remain separate aggregate honesty signals. Split, binding, import and SCIP
  edges are marked at the exact commit point after resolution, and unresolved
  or external calls are not fabricated as edges.
- `src/ingest_cache.h`, `test/cacheidentitycheck.sh`,
  `test/artifactcheck.sh`: cache format/parser/architecture/checksum refusal
  reasons are explicit; a valid artifact is not the same as fresh source and
  mismatch reparses rather than becoming evidence.
- `test/isolateprovenancecheck.sh`, `test/freshnesscheck.sh` and
  `test/emittertruthcheck.sh`: mutually exclusive provenance buckets must sum
  to the exact total, output distinguishes indexed-stat drift from content
  change, and truncation/refusal must be disclosed without emitting a false
  success payload.

Adopt immutable producer provenance, per-edge evidence/quality labels, explicit
unknown/partial/refusal states and deterministic ledgers. Avoid dense runtime
IDs, aggregate confidence averaging, and treating parser output as accepted
fact without verification.

### CodePropertyGraph — validated mutation boundary

At commit `e7b6e8da670e4b58a64ba153d197041c25fd798a`, read:

- `schema/.../Finding.scala`: findings are a graph layer with explicit
  evidence-node relations and key/value details, rather than an implicit
  mutation of ordinary code edges.
- `cpgloading/ProtoCpgLoader.scala`: loading uses a two-pass node mapping,
  then applies properties/edges through one `DiffGraph`; unknown node labels
  are warned/skipped and missing endpoints are not invented.
- `passes/CpgPass.scala` and `CpgPassNewTests.scala`: passes accumulate changes
  and apply them together; schema violations fail at apply time, lifecycle
  cleanup runs on failure, and parallel work merges deterministic side results
  before mutation.
- `cpgloading/CpgLoader.scala` and `CpgLoaderTests.scala`: format detection is
  explicit and unsupported/missing input fails; supported legacy formats are
  converted through named adapters.

Adopt validation-before-projection, evidence attached to findings and one
mutation boundary. Avoid making a CPG/analysis pass itself an authority or
allowing arbitrary unknown graph data to enter the accepted view.

### Archify — repository evidence is pinned and independently verified

At commit `18911058008f17dc065af23a2cdc9bfeff6d3f7a`, read
`archify/references/authoring-contract.md`,
`archify/renderers/shared/repository-evidence.mjs`, repository-evidence tests
and the lifecycle/workflow examples. Architecture output requires a full
revision, credential-free origin, repo-relative paths and valid line ranges;
the renderer reads the pinned Git blob locally and emits structured diagnostics
for mismatches. It never infers runtime causality from file proximity.

Adopt pinned revision/path/span evidence and fail-closed verification. Keep
documented/human architecture overlays distinct from static/runtime facts.

### `temp-rs-ddd` — dependency direction

At commit `12398c3c6ebbc67998c4ac10a60332f281eeb793`, read
`crates/domain/src/repository/healthy_repo.rs`,
`crates/infrastructure/src/repository/pg_healthy_repo.rs` and
`crates/infrastructure/src/connection/postgres_conn.rs`. The domain declares a
repository port and the infrastructure implements it; SQL/connection details
stay outside the domain. The concrete example is a health probe, so it is not
copied as a trust model or as permission to put SQL in domain code.

Adopt domain-private invariants plus application ports and adapter-owned
migrations/query files. Avoid embedding SQL, serde, SQLite or transport types
in `graph-domain`.

## Contract before implementation

- `FactAssertion` is a scoped entity with validated subject/predicate/object,
  assertion kind, producer and one or more typed evidence references. It starts
  as `candidate`; wire input cannot construct an accepted assertion.
- `EvidenceRef` records an evidence ID and kind (`source`, `artifact`,
  `analysis_run`, `runtime_trace`, `document`, `human`). The reference is not
  itself verification; the GraphWriter must resolve every referenced record in
  the same project/worktree/revision/graph snapshot.
- `AnalysisRun`, `Artifact` and `SourceEvidence` remain immutable metadata and
  observations. Registration/content verification alone never changes fact
  trust. Existing artifact protection and execution flags remain separate.
- `GraphWriter` is the only public transition surface for
  `candidate → accepted`; it requires a verifier-owned receipt binding the
  candidate, exact evidence set, snapshot and verifier. Rejection/supersession/
  expiry are explicit terminal decisions with a reason and never imply accept.
- The assertion repository stores immutable candidates and accepted decisions
  with composite scope checks and idempotent replay/conflict behavior. Normal
  graph views select only accepted assertions and expose rejected/unknown gaps
  separately; no candidate or security finding is promoted by ranking.
- Tests will cover state transition legality, evidence kind/scope binding,
  missing/stale evidence, strict wire rejection of accepted/unknown fields,
  immutable replay/conflict, SQL foreign-key scope isolation, and absence of
  candidate facts from the accepted projection.

source-gate: ready; implementation is authorized only for this contract and
its domain/application/protocol/store tests.

## Implementation and verification

Implemented after the source gate:

- `crates/domain/src/fact.rs`: typed evidence/producer/receipt, candidate-only
  construction, explicit terminal state machine, strict decision invariants,
  and `GraphWriter` as the only constructor of `AcceptedFact`.
- `crates/protocol/src/fact.rs`: strict schema-v1 DTOs with `deny_unknown_fields`;
  `try_into_domain` rejects accepted/terminal wire state while a separate
  persistence conversion is used only after scoped store selection.
- `crates/application/src/fact.rs`: `FactAssertionRepository` port plus
  `record_fact_candidate` and `accept_fact` use cases. The repository receives
  the domain `AcceptedFact` marker and the exact receipt, never a caller string.
- `crates/store/migrations/V17__fact_assertion_ledger.sql` and SQL query files:
  assertion and decision rows are immutable, composite-scope keyed, replayable
  and conflict-detecting. Acceptance checks registered source/artifact/
  analysis evidence in the same project/graph snapshot before commit; normal
  reads join only accepted decisions for the accepted projection.

Verification completed:

- `cargo test -p graph-domain --locked --offline`: 30 passed, 0 failed.
- `cargo test -p graph-protocol --locked --offline`: 71 passed, 0 failed.
- `cargo test -p graph-application --locked --offline`: 15 passed, 0 failed.
- `cargo test -p graph-store --locked --offline`: 64 passed, 0 failed, 1
  ignored (owned subprocess fixture).
- `cargo test --workspace --locked --offline`: passed, including CLI, source,
  system and execution regressions.
- `sh scripts/validate-foundation.sh`: passed architecture/dependency and
  subprocess foundation checks after the workspace build.
- Changed P1.T07 files were rustfmt-checked individually. Whole-workspace
  `cargo fmt --all -- --check` remains red on pre-existing unrelated
  CLI/execution formatting diffs; no unrelated mass reformat was applied.

Scoped limitations kept explicit:

- The current store resolves only registered `Source`, `Artifact` and
  `AnalysisRun` evidence. `RuntimeTrace`, `Document` and `Human` references
  fail closed until their own immutable registries/verifiers exist.
- The durable adapter currently persists the accepted decision path; generic
  rejected/superseded/expired decision projection, CLI/MCP submission and
  outbox/projector integration remain later work. Domain legality is covered.
- This task creates no code graph nodes/edges by itself. `FactAssertion` is a
  trust boundary that future extractors and Ripwire/Joern/CPG/runtime adapters
  must use; ranking, cache hits and provenance labels still cannot self-accept.

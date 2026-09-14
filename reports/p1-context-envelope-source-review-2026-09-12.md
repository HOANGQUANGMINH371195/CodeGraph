# P1.T03–P1.T05 ContextEnvelope — source gate

## Task record

```text
status: done | owner: coordinator
scope: crates/protocol ContextEnvelope wire contract and validation tests
depends_on: P0.T01, P0.T03, P0.T04, P0.T02
source-gate: ready
review: coordinator self-review; focused and workspace checks passed
next: P1.T10 independent JSON Schema/golden validator; ContextGateway wiring
```

## Source-first study before coding

- [x] Reset this task's source-gate to `pending` before the source pass.
- [x] Read CodeGraph `src/types.ts` (`Node`, `Edge`, `Subgraph`, `CodeBlock`,
      `TaskContext`, `BuildContextOptions`): the upstream context combines
      ranked entry nodes, a bounded traversed subgraph and source blocks with
      repository-relative file paths and inclusive line ranges.
- [x] Read CodeGraph `src/context/index.ts` and `src/context/formatter.ts`:
      FTS/ranking selects entry points, graph traversal expands relationships,
      output is rendered as markdown or JSON, and code block/node counts are
      bounded. The formatter is an agent convenience projection, not a
      snapshot/provenance authority; its JSON does not bind every edge to a
      source hash.
- [x] Read `__tests__/context.test.ts`, `context-ranking.test.ts` and
      `explore-output-budget.test.ts`: tests cover search/traversal limits,
      markdown/JSON shape, source line numbering, generated-file ordering and
      hard serialized output budgets. A passing context result alone does not
      prove source freshness or correctness of every relationship.
- [x] Read CPG schema `Base.scala`, `Method.scala` and `CpgSchema.scala` at
      CodePropertyGraph revision `e7b6e8da670e4b58a64ba153d197041c25fd798a`:
      canonical nodes retain `FULL_NAME`, `FILENAME`, line/column start/end,
      optional `HASH`, and external/declaration status; method and source-file
      edges are separate schema facts. These fields guide identity/evidence
      mapping but are not copied as an acceptance claim about runtime flow.
- [x] Read product `graph-domain::SourceEvidence`, deployment context and
      protocol serialization: evidence already validates repository-relative
      paths, lowercase SHA-256, inclusive one-based ranges, project and graph
      scope; protocol uses `deny_unknown_fields` and rejects future schemas
      before domain conversion.

## Required source-study decisions

### Adopt

Use a separate agent-facing context schema version, explicit `view`, bounded
nodes/edges/slices, repository-relative paths, inclusive line evidence and
snapshot binding (`ProjectRef` + `graph_version`). Preserve explicit
`resolved|partial|unresolved|stale` resolution and `complete|partial|unknown`
coverage. Mark source data as untrusted so source text cannot become an
instruction or authority. Reuse `SourceEvidence` validation rather than
duplicating hash/path rules.

### Adjust

CodeGraph's `TaskContext` and formatter have no complete product snapshot or
per-edge evidence requirement, so the Harness envelope wraps every
source-derived node and edge with one or more `SourceEvidence` citations and
checks all citations against the envelope project/graph version. A code slice
must be covered by a citation for the same path and line range. System,
runtime and knowledge projections may be represented without source citations,
but their provenance and resolution state remain explicit.

### Avoid

Do not make ranking score, CPG presence, a cached index, a README claim or a
CodeGraph output string authoritative. Do not deserialize an envelope directly
into domain scheduling/GraphWriter authority. Do not estimate tokens from
characters in this contract; serialized byte limits are separate from model
usage receipts.

## Implementation and test plan

Add a typed `graph-protocol::context` module with:

- versioned `ContextEnvelope`, `ContextNode`, `ContextEdge`, `CodeSlice`,
  `Coverage` and `UnknownContext` wire records;
- bounded text/item counts and `source_is_untrusted = true` invariant;
- required evidence for source-derived nodes/edges/slices;
- project/graph/path/range consistency checks and dangling-edge rejection;
- serde round-trip and negative fixtures for missing evidence, mixed snapshots,
  invalid ranges, duplicate IDs, dangling edges, future schema and explicit
  unknown coverage.

The independent JSON Schema validator and golden fixture remain P1.T10 work;
this task must not claim that gate complete merely because Rust validation
passes.

## Source fingerprints

```text
CodeGraph revision: 3ed73bc127323e63153bf6ec8354afa82ce36aaf
CodeGraph types.ts: bc220fb0b82a614277c5d9b792d045705144da0b0f3ed759e5f0ee3b4325f11b
CodeGraph context/index.ts: 028cbe205d076f0e39641d5f5a57ba03f9bfc7354c5442a803e54d44f4bacc4f
CodeGraph context/formatter.ts: 91a8ab80fed68e548d5b2aaf64b601ba4352a88ef5d5c13f4c66c1e2de9d4083
CodePropertyGraph Base.scala: 1553bf86adb331409c8f7123f3a91320c854aabe2990203d17d38d32ea5607ab
CodePropertyGraph Method.scala: 7643ca415b9218aa0d30d49dca34ee9613ef71c755f5f607994a482d26bf90b6
CodePropertyGraph CpgSchema.scala: 92a8940dc1aee7df51aa23acb818446a612c4f59bb6f72ee981342315c5743bd
```

This source receipt authorizes only the scoped ContextEnvelope implementation;
it does not authorize graph ingestion, Joern publication, swarm capability or
runtime truth.

## Implementation receipt

Added `crates/protocol/src/context.rs` and exported it from
`crates/protocol/src/lib.rs`. The contract now validates the schema version,
project/revision/graph snapshot, bounded node/edge/slice/evidence counts,
source-derived evidence references, same-path slice coverage, dangling edge
endpoints, explicit coverage unknowns, budget usage and the mandatory
`source_is_untrusted` label. Serde rejects unknown fields and future context
schema versions before any application/domain use.

Checks:

```text
scripts/with-local-tools rustfmt --edition 2024 crates/protocol/src/context.rs crates/protocol/src/lib.rs
scripts/with-local-tools cargo test -p graph-protocol
```

Pass: 63 protocol unit tests and 3 protocol integration tests. The new context
module contributed 6 passing tests.

```text
sh scripts/validate-foundation.sh
scripts/with-local-tools cargo test --workspace --locked --offline
```

Pass: foundation architecture/preflight checks 15/15; product fixture test
1/1; workspace test run completed with all executed tests passing (the existing
single ignored migration/fixture cases remain ignored by their test policy).
The full `cargo fmt --all -- --check` remains red on pre-existing formatting
diffs in unrelated CLI/execution files; the two files changed by this task
were formatted directly. This is recorded as a repository hygiene gap, not
converted into a false pass.

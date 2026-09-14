# W9 SQL-link graph source-study receipt — 2026-09-12

## Sources read before implementation

| Source | Role adopted |
|---|---|
| `crates/domain/src/deployment.rs` | immutable aggregate, scope identity, citation consistency, bounded child collections |
| `crates/application/src/lib.rs` | repository port and strict generation-CAS shape |
| `crates/application/src/source.rs`, `crates/source/src/lib.rs` | host-root-bounded byte verification before parsing/publishing |
| `crates/store/src/deployment.rs`, `V14__deployment_graphs.sql` | transaction-only persistence, header generation and corruption checks |
| `crates/system/src/sql.rs`, `crates/cli/src/sql.rs` | SQLite parser is offline syntax-only and full-file evidence is mandatory |
| `crates/protocol/src/deployment.rs`, `crates/cli/src/compose.rs` | output-only projections and pre-budgeted receipt before mutation |
| `reports/w4-file-read-coordinate-source-review-2026-09-12.md` | CodeGraph extents are zero-based half-open UTF-16 code units |
| `reports/w4-sql-source-review-2026-09-12.md`, `reports/w9-sql-cli-source-review-2026-09-12.md` | code→file candidate remains distinct from SQL syntax, table identity and runtime DB proof |

## Adopt

- A dedicated `SqlLinkGraph`, scoped by adapter + complete `ProjectRef` + graph
  version + code path, with a separate full-file code citation and one
  independently verified full-file SQL citation per link.
- Explicit `candidate_only=true`, `relationship_verified=false`,
  `runtime_verified=false`, `semantic_verified=false` in stored/output model.
- CAS replacement/invalidation; source I/O and parser work occur before the
  database transaction; output budget is checked before mutation.
- Reject mixed scope/snapshot, duplicate link identity, non-full-file citations,
  invalid coordinate encoding and invalid UTF-16 extent boundaries.

## Avoid

- Treat CodeGraph JSON, authored Orders truth, source hash, parser relations or
  a cached graph as proof the file read executes, the relationship is semantic,
  or a DB instance/table is physically resolved.
- Execute SQL, import graph-system into protocol, or embed SQL strings in Rust
  migration/query source.
- Reuse deployment graph as a different domain aggregate.

## Planned acceptance

Orders: 11 code→SQL candidates and 13 syntax statements only after emitted
extraction then independent verification. Negative tests: path/root escape,
symlink, missing/drift/non-UTF8 target, invalid coordinate encoding or surrogate
boundary, CAS conflict/reopen, output exact/one-under budget and SQL-only change.

## Implemented follow-up and verification

- Domain: `SqlLinkGraph` and strict CAS repository port; V15 stores only
  candidate link/statement/relation observations, with a distinct invalidation
  tombstone.
- Transport: `graph_protocol::file_reads::Report` rejects unknown top-level
  transport fields and binds consumed candidate UTF-16 invocation extents to
  the report's source hash. Nested extractor-owned discovery remains untrusted.
- CLI: `publish-sql-links` reads report JSON under the existing 8 MiB cap,
  re-verifies the registered full code citation and re-captures every SQL target
  beneath `DirectorySource`; it checks path/hash/bytes/lines, one-to-one index,
  UTF-16 character boundary, parses offline SQLite syntax, pre-budgets receipt,
  then performs CAS. `sql-links` is historical-only; `invalidate-sql-links`
  creates a CAS tombstone without source I/O.
- Tests: domain 3 focused tests; store roundtrip/CAS/reopen test; protocol 3
  report/extent tests; CLI black-box publish/read/invalidate plus exact and
  one-under output cap. `sh scripts/validate-foundation.sh` exited 0.

Still not claimed: runtime execution, relation/table or DB-instance semantics,
atomic repository snapshot, whole-program/cross-file coverage, authorization,
or an SQL diagram projection.

## SQL-context follow-up — source gate reopened 2026-09-12

status: doing | source-gate: ready

### Sources reread

| Source | Flow/tests reread | Decision |
|---|---|---|
| `crates/application/src/deployment_context.rs`, `crates/cli/src/deployment_context.rs`, `crates/protocol/src/deployment.rs` | deterministic bounded selection → historical snapshot read → typed, capped output; unknown/invalidated owner and seed fail | Reuse the historical-only, typed-output and final-byte-cap shape, but do not force SQL links into deployment nodes/edges or invent a runtime data-flow edge. |
| `crates/domain/src/sql_link.rs`, `crates/store/src/sql_link.rs`, `crates/cli/src/sql_links.rs`, `crates/cli/tests/sql_links.rs` | full-file code/target citations + candidate links/ordinal syntax observations → strict CAS/tombstone → historical read | Select one persisted `SqlLink` by stable link ID, include only its code citation, target citation and syntax observations; never read/emit SQL text. |
| `outsource/ripwire` @ `48222d62`, `docs/ARCHITECTURE.md`, `docs/COMMANDS.md`, `skills/ripwire-orient/map-before-you-read.md` | ranked context must disclose cap/ambiguity and not silently claim terminal coverage; packs can include full bodies | A bounded context projection must report total/omitted links/statements and explicit candidate/runtime/semantic flags; output no bodies and use no ranking as graph authority. |

### Adopt / avoid / tests

- Adopt a deterministic direct-link selector rather than BFS: a SQL-link owner is
  one code file and its independently captured SQL targets; traversal would
  imply a relation that this candidate model does not prove.
- Return only historical data, require an exact link ID, and fail for absent,
  invalidated or unknown owners/seeds. Citations are output references, not a
  current-source verification token.
- Test publish → context → exact/one-under final output cap; unknown seed and
  invalidated owner errors; target citation present; serialized response has no
  SQL body and keeps all verification flags false.

### Implemented and verified

- `graph_application::sql_link_context::select` selects one exact persisted
  link ID. It deliberately has no traversal: the owner grouping is not a
  data-flow edge.
- `sql-link-context ID TASK_SPEC --link LINK_ID` returns only historical code
  and target citations, UTF-16 extent, candidate path and parser observations.
  It emits no SQL source body and has no runtime, semantic, relationship or
  current-source verification claim.
- `cargo test -p project-graph-agent --test sql_links` passed: publish →
  context, target citation/flags/no body, exact and one-under output budget,
  unknown seed, and invalidated owner. `cargo check -p graph-application -p
  graph-protocol -p project-graph-agent` and `git diff --check` passed.
- `cargo fmt` could not run because this workspace's Cargo installation has no
  `fmt` subcommand and no `rustfmt` executable on PATH; this is a local tooling
  gap, not a formatting-pass claim. `cargo clippy` is likewise unavailable in
  this Cargo installation, so no lint-pass claim is made.

## SQL-candidate diagram follow-up — source gate reopened 2026-09-12

status: doing | source-gate: ready

| Source | Flow/tests reread | Decision |
|---|---|---|
| `crates/cli/src/deployment_diagram.rs`, `crates/protocol/src/archify.rs`, `crates/cli/tests/deployment_diagram.rs` | historical typed context → architecture IR → evidence manifest plus deterministic local bindings; output budget precedes no mutation | Mirror the IR/manifest split and bounded output, but use a distinct `SqlLinkDiagram`; do not reuse deployment node/edge bindings. |
| `outsource/archify/archify/schemas/architecture.schema.json`, `renderers/architecture/render-architecture.mjs`, `test/diagram-guide.test.mjs` | schema validates component/connection IDs and renderer mechanically rejects unreadable/invalid geometry; `external` is a neutral component category and `dashed` is available | Emit exactly two neutral external components and one dashed **static file-read candidate** connection with stable local IDs; no database/cloud/runtime icon or traffic label. |
| `scripts/validate-deployment-diagram.mjs`, `outsource/archify/archify/bin/archify.mjs` | an owned temp fixture → typed bundle → raw IR input → Archify `validate`/`deliver`; rejected candidate must retain last-good HTML | Add a separate local SQL-candidate gate with the same ownership and last-good rule, without treating renderer success as source/runtime verification. |
| `crates/cli/src/sql_link_context.rs`, `crates/protocol/src/sql_link.rs`, `crates/cli/tests/sql_links.rs` | exact historical candidate selection, full code/target citations, no bodies, all verification flags false | Use the exact same context as manifest and preserve every false verification flag; diagram reads no current source and performs no mutation. |

Adopt: diagram labels are bounded path labels, connection meaning is candidate-only,
and cards explicitly distinguish parser relation names from physical DB/table facts.
Avoid: deriving a database component from `.sql`, exposing source/SQL text, or
calling this an architecture runtime/data-flow diagram. Tests will cover direct
manifest equality, two components/one dashed candidate edge, no SQL body,
exact/one-under byte cap, unknown link and invalidated owner.

### Implemented and verified

- `sql-link-diagram ID TASK_SPEC --link LINK_ID` projects the same historical
  `SqlLinkContext` into the existing typed Archify-compatible architecture IR.
  IDs are local (`source`, `target`, `candidate`), components are neutral
  `external`, and the only edge is dashed and labelled `static file-read
  candidate`.
- The manifest is byte-for-byte the selected context object; cards disclose
  candidate-only meaning, historical state and lack of physical table/database,
  authorization, runtime, semantic and relationship verification. SQL bodies
  are neither read nor emitted.
- `cargo test -p project-graph-agent --test sql_links` passed after the diagram
  addition: direct manifest equality, two neutral components, candidate edge,
  no body, exact/one-under output cap, unknown link and invalidated owner.
- `node scripts/validate-sql-link-diagram.mjs
  /home/minh/projects/outsource/archify/archify` passed after a local binary
  build. It created `/tmp/graph-sql-diagram-6OpPXu`, where Archify validation
  and delivery accepted the raw IR, the delivered HTML contained no SQL body,
  and a schema-invalid candidate left the last-good HTML unchanged.
  This proves that pinned local Archify accepted this bounded IR; it does not
  prove the depicted candidate executes or resolves a table/database instance.

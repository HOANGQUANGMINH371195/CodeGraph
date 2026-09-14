# P1.T09–P1.T10 context budget and schema receipt

status: done (scoped) | owner: coordinator | source-gate: ready
scope: `crates/protocol/src/context.rs`, `schemas/context-*.schema.json`,
`scripts/validate-context-schemas.py`, `fixtures/context-{envelope,pack}/`
depends_on: P1.T01–P1.T08
review: complete | blocker: none | next: P2 indexer and P4 context compiler

## Source-first study

- CodeGraph commit `3ed73bc127323e63153bf6ec8354afa82ce36aaf` was read at
  `src/mcp/explore-output-limit.ts`, `src/mcp/explore-diagnostics.ts`,
  `src/context/formatter.ts`, `docs/design/explore-budget-allocation.md` and
  the output-budget, context, stale-slice and allocation tests. The adopted
  mechanism is a final-output hard ceiling with explicit truncation, while
  allocator diagnostics and model-token estimates remain separate metadata.
- Ripwire commit `48222d62f41c6e15f60855127c1d9ee06b3aed4c` was read at
  `src/serialize.h`, `src/recall.h` and budget/truncation tests. The adopted
  mechanism is pre-priced serialized output and disclosed omission state; XML
  body/line truncation and Ripwire's token estimate are not copied into the
  Rust contract.

## Implementation

- `ContextBudget` now declares and validates node, edge, source-range, source
  bytes, traversal depth, serialized bytes, characters and optional
  tokenizer-specific tokens. Emitted counters cannot exceed their declared
  limits; token fields must be all absent or consistently measured with a
  bounded tokenizer name.
- Added strict Draft 2020-12 schemas for `ContextEnvelope` and
  `ContextPack`. Nested pack claims are candidate-only and the schemas require
  the `source_is_untrusted`/`untrusted_content` markers.
- Added independent Python `jsonschema` validation with semantic checks for
  evidence references, dangling endpoints, same-snapshot bindings, budgets and
  scope/grant/handoff equality. It validates the two valid golden fixtures and
  expects two invalid compatibility fixtures to fail.
- No schema or validator was added to the domain crate; no SQLite, tokenizer,
  embedding engine or runtime authority was introduced.

## Verification

- `python3 scripts/validate-context-schemas.py` → 4 fixtures and 2 schemas
  validated.
- `cargo test -p graph-protocol --locked --offline` → 78 unit tests, 3
  integration tests and doc tests passed.
- Workspace test and foundation gates had already passed for the same product
  implementation before the final budget-only regression test was added; the
  protocol suite was rerun afterward and passed.
- `cargo fmt`/`cargo clippy` remain unavailable because this environment lacks
  those Rust components; this is recorded as an environment limitation rather
  than a green lint claim.

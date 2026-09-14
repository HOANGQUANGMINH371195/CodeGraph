# W11 deployment-diagram CLI tests source review

status: done | owner: gpt-5.6-luna worker (agent ID not independently exposed)
scope: `/home/minh/projects/project-graph-agent`, owned files `crates/cli/tests/deployment_diagram.rs` and this receipt
depends_on: parent CLI deployment-diagram command implementation
source-gate: ready

model: requested `gpt-5.6-luna`; effective runtime model not independently exposed

## Source identity

Reference repository: `/home/minh/projects/outsource/archify`

- git revision: `18911058008f17dc065af23a2cdc9bfeff6d3f7a`
- `archify/archify/schemas/architecture.schema.json` SHA-256: `94568c7ca72c07ac0ee02ee76bd39392874b3a13bb796a127962be68d0f5bc90`
- `archify/archify/schemas/common.schema.json` SHA-256: `2630c958314eeebb92e0f6f6330475f1a2bc48f8c1b53201ba3b58889b319c80`
- `archify/archify/test/repository-evidence.test.mjs` SHA-256: `a078539c8b61f26f3694c44dced2e83eb58d3f5ad16556d23d6bcf3ea594fd0b`

Read before patch:

- `archify/archify/schemas/architecture.schema.json` (architecture envelope, metadata, grid layout, components, connections, cards)
- `archify/archify/schemas/common.schema.json` (IDs, component types, variants, points, cards)
- `archify/archify/test/repository-evidence.test.mjs:412-430` (evidence is opt-in for ordinary artifacts; no embedded source-evidence beacon)
- `archify/archify/test/repository-evidence.test.mjs:518-585` (evidence delivery fails closed without root, on origin mismatch, outside/malformed/missing paths, impossible lines, and shape limits)

Product patterns read for adaptation:

- `crates/cli/tests/deployment.rs:1-337,339-588` for isolated `tempfile` fixtures, real binary invocation, JSON-line helpers, Compose publication, historical-root deletion, exact byte-budget checks, bounded context assertions, and non-mutating invalid/unknown cases.
- `crates/application/src/deployment_context.rs:1-125` and `crates/cli/src/deployment_context.rs:1-111` for deterministic selection semantics, original edge orientation, citation deduplication, historical/untrusted flags, and output budgeting.

## Adopt / avoid / adapt

Adopt:

- Use the real `CARGO_BIN_EXE_project-graph-agent` binary in each isolated fixture, with a temporary SQLite database and Orders Compose source.
- Publish once through the existing CLI, query a node from the returned full graph, and assert diagram mappings preserve the selected context node/edge order and original edge orientation.
- Treat the context evidence manifest as the authoritative citation set: deduplicate citation IDs and compare each diagram binding back to the original node/edge evidence and endpoints.
- Check exact serialized output-byte limits including the trailing newline, and require rejected calls to emit no stdout.
- Assert historical and coverage cards remain explicit, including after deleting the source root.

Avoid:

- Do not invoke Archify or require any external renderer; the Rust CLI test validates the producer contract only.
- Do not infer runtime/network/source claims: deployment components stay `external`, sublabels are only `service`, `volume`, or `network`, and edge labels derive only from declared `mounts`, `depends_on`, or `attached_to` facts.
- Do not share helpers or mutate `crates/cli/tests/deployment.rs`; this test owns an isolated fixture and assertions.
- Do not treat a missing/tombstoned/unknown seed as an empty architecture; require a nonzero exit, stderr, and empty stdout.

Adaptation and test plan:

- Adapt Archify's schema boundaries to the parent command's wrapper contract: top-level `kind=deployment_diagram`, `diagram` architecture payload, existing-shaped `evidence_manifest`, and explicit component/connection bindings.
- Assert defaults and maximums required by this task (`both`, depth `2`, nodes `12`, edges `24`, output `65536`; depth max `16`, nodes `1..12`, edges `0..24`, output max `16777216`) without constraining unrelated formatting.
- Cover real publish → fullgraph seed → diagram mapping/deduplication/citations/generation; root deletion; exact/under output budget; depth/node/edge caps and omission flags; invalid args; missing/tombstone/unknown seed; and repeated deterministic queries with unchanged graph state.

## Pre-code checklist

- [x] Source revision and fingerprints recorded.
- [x] Actual outsource schemas and requested evidence-test ranges read.
- [x] Existing product fixture and deployment-context behavior read.
- [x] Source flow, adopt/avoid decisions, adaptations, and planned tests recorded.
- [x] `crates/cli/tests/deployment_diagram.rs` implemented and targeted test run recorded.

receipt: `scripts/with-local-tools cargo test -p project-graph-agent --test deployment_diagram` | exit 0 | 7 passed, 0 failed, 0 ignored | final run after `scripts/with-local-tools rustfmt --edition 2024 crates/cli/tests/deployment_diagram.rs`
review: worker self-review complete; assertions cover the requested public contract and remain disjoint from parent files | blocker: none | next: parent may integrate both owned files

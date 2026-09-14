# Deployment graph persistence boundary — source gate

Main reread current store migrations (V3 immutable citations, V13 terminal
receipts), evidence repository reads/transactions, migration history tests,
domain SourceEvidence/AnalysisRun, and actual CodeGraph replacement source/tests
before implementation. Product remains an unborn working tree.

- CodeGraph revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`;
  `src/db/queries.ts` SHA-256
  `e49e1f4b76afa3b832611668b15db67af45b5512287b67ea8c74412d503048db`.
- `__tests__/parameter-reconciliation.test.ts` SHA-256
  `1e54ec9356db14271cfa9a06ae75615c1939e465f7cc4ddaec895f96b99a003f`.
- Product domain analysis.rs SHA-256
  `53b4c36ef8bea03c3540817eb5410cdfa683142d51557623777f2d1afc24cf8b`;
  evidence.rs `1abbd2090921b1b7bd0ac377f3a42ceeb7c5fb189be34bc15a9e5a1ee684768d`.

CodeGraph freezes input before mutation, checks producer ownership/endpoints,
replaces only owned edges in a transaction, preserves foreign/static edges,
and includes previous sources even when new candidates are empty. Its actual
reconciliation tests exercise rebinding/removal and foreign collision behavior.
Adopt validation-before-mutation, explicit owner/version and empty replacement
semantics. Do not copy its nested transaction assumptions into rusqlite, infer
source freshness from stored metadata, or store CLI booleans as capabilities.

The existing Compose adapter/CLI has public candidate DTOs but no validated
aggregate suitable for a persistence port: dangling endpoints, mixed snapshots
or reused evidence IDs could otherwise be accepted by a future graph store.
First add a pure domain DeploymentGraph boundary with private aggregate fields,
bounded inputs, per-kind endpoint/mount constraints and citation identity checks.
Then make the actual adapter and CLI pass through that boundary. This is a
prerequisite for transactional graph publication, not a substitute for it.

Candidate node/edge fields remain public inputs; constructor consumes them and
exposes immutable slices only. SourceEvidence retains its existing meaning:
shape validation is not byte verification, source freshness, analysis-run
attestation, running deployment evidence or graph-write authorization. Explicit
adapter/version ownership will scope subsequent replacement. A valid empty
graph must remain representable so removed declarations can be retracted.

Next storage work must add a new migration (never modify old migrations),
per-owner/snapshot atomic replacement with a stale-writer fence, scoped reads,
corruption checks, invalidation/tombstone handling, reopen/rollback/concurrency
tests and real CLI publication/query. No SQL graph persistence is claimed by
this constructor. Skills: rust-router, m09-domain, m12-lifecycle,
m06-error-handling. Luna independently owns domain invariant tests.

Parent review before follow-up tests: overflow fixtures repeat identical node/
edge IDs, so generic is_err could pass on duplicates if the budget guard were
removed. Require the exact item-budget error and independently accept each
exact limit with unique identities. Add edge-citation scope/ID conflicts too:
node-only scope mutation tests would not detect a missing edge citation check.

## Storage follow-up design (not implemented by this increment)

Use the full caller-scoped ProjectRef, graph version, source path and adapter as
the replacement owner key; keep adapter version in the generation descriptor so
a new producer version can retire its predecessor, not silently coexist as a
second owner. Require an expected generation for replacement/invalidation, with
one transaction covering the header, citation checks and relational node/edge
rows. Do not hold the transaction across source reads or parsing. Empty success
and failed/deleted-source invalidation need distinct states, with stale nodes
and edges unavailable to current reads. Preserve immutable citation history and
foreign owners. Do not represent this merely as a cache of CLI verification flags.

Existing Store citation methods open their own transactions, so the publication
implementation must share an explicit transaction helper or execute its SQL in
the owning transaction, not nest Store.record_source calls. Use a new migration
after V13; verify old refinery history remains unchanged. Add rollback, exact
scope/generation, reopen and concurrent-writer tests before a CLI write path.
This design still needs implementation and acceptance evidence; no store method,
schema migration, writer fence or invalidation is claimed complete here.

## Implemented and verified boundary

Added `graph_domain::deployment::DeploymentGraph`: consumed candidate vectors
become immutable aggregate slices after full-source/scope/hash/run/range checks,
node uniqueness, per-kind endpoint/mount checks, edge uniqueness and within-batch
citation-ID consistency. Owner adapter/version and valid empty graphs are retained.
Caps: 10,000 nodes / 50,000 edges / 50,000 unknowns; adapter/version 128 bytes,
node ID 512, name 1024, mount path 4096, unknown reason 512. These field/item
limits are not a total allocator cap, source-byte verification or DB-global ID
consistency proof.

`graph_system::analyze_compose_graph` converts actual parsed candidates into that
aggregate, with producer `compose` version `1`. CLI analyze-compose now uses
this checked path before encoding. Protocol node/edge/unknown conversions take
domain references and remain independent of the adapter. The CLI wire shape
and candidate/unverified-runtime flags are unchanged. Duplicate same-evidence
reference candidates now fail the graph boundary instead of passing to output.
The old `analyze_compose` API still returns raw candidates, not publication-ready
domain data. No new dependency, SQL or migration was added.

Verification performed by parent:

- Luna delivered 14 domain tests; parent added exact positive collection limits
  and edge-citation checks, and strengthened overflow error identity: **16 domain
  integration tests passed**. The domain's existing 15 unit tests remain covered.
- **20 graph-system tests passed**, including actual adapter-to-domain ownership,
  empty graph, duplicate-edge rejection and the earlier parser/source controls.
- **6 actual Compose CLI tests passed** after routing through the aggregate.
- `scripts/with-local-tools sh scripts/validate-foundation.sh`, session `91779`:
  terminal exit 0; **326 Rust passed, 0 failed, 3 ignored; 15 Node tests passed**.
  Architecture remains 8 packages / 47 declared dependencies.
- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy -p graph-system --lib --locked --offline --no-deps -- -D warnings`:
  exit 0 after final integration.
- Domain Clippy JSON review found six new must_use suggestions; added attributes.
  Final domain review reports zero diagnostics in deployment.rs but 164 existing
  diagnostics elsewhere. This is not a strict whole-domain/workspace lint pass.

Final SHA-256:

- domain/src/deployment.rs: `c65a2fcbf34253a0cb1cf90e8c501023ecda237d60b55c545f2d60e6c0b5ebd7`
- domain/tests/deployment.rs: `86f75e5e7727709c64e4239ee87d689451cd261730d2ec839d62928311f498e9`
- system/src/deployment.rs: `f6ba872d459cc7bad1a42b912f81f9da03e153d764b6d3d79d9308beb70d80e0`
- protocol/src/deployment.rs: `f7732d85b320ce871abda082d7d44837e70e93f5fe8bbb54ae2ac1d0c4ac7e96`
- cli/src/compose.rs: `8760478943ffba07f146c4423d7587afce35951f9945e46995328c4cb4b5adac`

DDD/ownership skills informed the private aggregate and borrowed validation maps;
no mutation handle or serialization-derived verification capability is exposed.
Storage follow-up above and complete W9/W11 acceptance remain unimplemented/open.

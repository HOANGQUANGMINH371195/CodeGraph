# W9 relational deployment persistence — source receipt

Before implementation, parent read CodeGraph `src/db/queries.ts:1909–1971`
(`replaceSynthesizedEdges`, `replaceSynthesizedSnapshot`) and
`__tests__/parameter-reconciliation.test.ts:88–151` (foreign ownership,
retraction on failed extraction, receipt rollback).
HEAD: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`; dirty source fingerprints:

- queries: `e49e1f4b76afa3b832611668b15db67af45b5512287b67ea8c74412d503048db`
- tests: `1e54ec9356db14271cfa9a06ae75615c1939e465f7cc4ddaec895f96b99a003f`

Adopt explicit producer ownership, empty replacement removing previous owned
rows, and atomic data/receipt mutation with propagated errors. Avoid copying
flattened nested transaction semantics or promoting declared edges to runtime
facts. Original Rust implementation; no upstream code copied.

Current local evidence repository was inspected: immutable citation ID replay
must compare full content within the same transaction as graph replacement.
Migrations V1–V13 remain unchanged; new schema is V14 through refinery.

Contract chosen before patches: owner = full ProjectRef + graph version + path
+ adapter. Adapter version and content hash are replacement values, not owner
keys. Strict expected-generation CAS: zero means absent, every successful
replacement/invalidation increments, no implicit retry idempotency. Invalidating
an absent scope at generation zero creates tombstone generation one. Empty valid
graphs differ from tombstones. Reads reconstruct validated domain aggregates
within a read transaction; stored provenance remains historical. No source I/O
under DB locks. Child row counts detect missing rows, not arbitrary coherent
database tampering. CLI publication remains separate pending integration.

## Implementation and verification

Implemented immutable `DeploymentScope`, the application repository port, V14
relational schema, external SQL query files and transactional Store adapter.
Shared evidence helpers allow graph/citation writes under one immediate
transaction, without nested transactions. Reads pin a SQLite snapshot until all
rows and immutable citations have been reconstructed and domain-validated.

Luna (`gpt-5.6-luna`, native worker Helmholtz) delivered seven integration tests
and its own source receipt. Parent reviewed both, strengthened foreign-owner
checks from absent reads to actual stored graphs surviving invalidation, and
added nonempty→empty→nonempty→tombstone plus absent invalidation checks (eight
tests total). Parent also added three corruption/late-write rollback/overflow
tests and the V13 upgrade test. Corruption fixtures explicitly disable foreign
keys on their own separate connection; production foreign keys remain enabled.
The first corruption fixture attempt was correctly blocked by the FK constraint;
the corrected fixture now exercises read-time rejection.

Validation on the final implementation:

- `scripts/with-local-tools sh scripts/validate-foundation.sh`: exit 0,
  **338 Rust passed, 0 failed, 3 existing ignored; 15 Node passed**.
  Architecture gate: 8 packages / 47 declared dependencies, unchanged.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- Scoped Clippy inspection: zero diagnostics in store `deployment.rs` after
  fixing a checked-conversion warning. This is not a whole-workspace strict
  Clippy pass; existing unrelated diagnostics remain outside this claim.
- All existing V1–V13 migration files left unchanged. Latest-history assertions
  advanced to 14; old migration prefix/checksum assertions remain intact.
- Tests cover full kind roundtrip/reopen, adapter upgrade, generation races,
  every owner dimension, immutable citation conflict rollback, late edge failure
  rolling back header/children/new citation, empty/invalidation recovery, and
  rejection of missing rows, wrong kinds/endpoints/citations/ordinals.

Final SHA-256:

- store implementation: `29ce9a3fc367e3df239f06e07e8602228f263b038c998d53b0c0fdc4043312b4`
- V14: `b65463ed4123064be7787ae7294427345823b054aeb44d8cc3a36146295a67f8`
- integration tests: `0f4e3596a29f76300bfa19230ac37f7b79d54bee1c1d4f54991af193ab28aa95`
- corruption tests: `f629c78a6d522b06a19523db2467bf7cf40c3b088b74d7d5fd37fb9134faebf0`

Open work: wire explicit CLI publication and invalidation/reindex, then connect
build/module/API/SQL/context/compiler/diagram paths. No runtime deployment or
whole-project understanding acceptance is claimed. W0–W13 remain incomplete.

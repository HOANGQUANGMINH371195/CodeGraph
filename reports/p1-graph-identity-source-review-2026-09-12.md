# P1.T01 graph node/edge identity — 2026-09-12

status: done | owner: coordinator | source-gate: ready → complete
scope: `crates/domain` identity value objects plus `crates/application`
deterministic derivation only
depends_on: P0.T01, P1.T06
review: coordinator self-review | blocker: none | next: P1.T02 schema versioning/compatibility

## Source-first checklist

- [x] Reset this task's source-gate to `pending` before the source pass.
- [x] Read the relevant implementation and tests in `outsource` again for this
  task, rather than relying on the earlier audit receipt.
- [x] Record the final source-study decision and tests before the first code
  patch.
- [x] Recheck the implementation against source behavior after coding.

## Source study

- `outsource/codegraph` revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`:
  reread `src/extraction/tree-sitter-helpers.ts:generateNodeId`,
  `src/extraction/tree-sitter.ts:createNode`, `src/types.ts:Node`,
  `src/db/schema.sql` and `src/db/queries.ts` node/edge writes. The current
  node digest is a short hash of file path, kind, name and line/column; edge
  rows use an auto-increment database ID while relation columns are source,
  target, kind and optional call-site coordinates. `NODE_ID_VERSION` rejects
  old populated stores and requires a full rebuild when the format changes.
- `outsource/ripwire` revision
  `48222d62f41c6e15f60855127c1d9ee06b3aed4c`:
  reread `src/model.h:Symbol`, `src/graph.h` relation construction,
  `src/ingest_cache.h` cache identity/version checks and the identity/cache
  tests. Dense symbol IDs are deterministic only for one run and are restamped
  after cache load; canonical `path::scope::name` strings are useful selectors
  but do not distinguish every overload/anonymous declaration. The cache
  rejects parser/format/architecture/checksum mismatches and reparses.

## Decision before implementation

Adopt explicit versioned opaque IDs, deterministic ordering and reject-on-invalid
input from both sources. Use length-prefixed canonical bytes before SHA-256 so
field boundaries cannot collide through delimiter ambiguity. Namespace IDs by
stable repository/worktree identity, and include language, kind, qualified name,
signature fingerprint, declaration path and a non-line-only byte-span anchor.
Keep source line/column as evidence, not the sole identity. Keep semantic/body
content fingerprint as a separately validated node fingerprint: changing a
function body must not silently create a different logical node ID, while the
fingerprint still participates in equality/change detection later.

Edge identity is a relation key over source ID, target ID, edge kind and an
optional canonical discriminator (for example parameter index or route verb).
Call-site spans and producer provenance belong to evidence/assertions, not the
edge key; reverse relations such as `called_by` are query projections. IDs are
scoped values and are not GraphWriter permission or snapshot freshness claims.

The source does not provide a safe cross-snapshot identity contract, rename/move
matching, or a typed Rust value object. Those remain product-owned gaps. Rename
and move matching will later be an evidence-backed relation, as required by
the plan, not an implicit ID rewrite.

## Adopt / avoid and test plan

- Adopt CodeGraph's full-rebuild/version-guard posture, Ripwire's deterministic
  restamping/cache rejection, and the existing domain's private validated value
  objects.
- Avoid CodeGraph's line/column-only digest, Ripwire's per-run dense IDs, raw
  canonical names as durable IDs, truncated hashes, delimiter-ambiguous
  concatenation, and edge auto-increment IDs at the graph contract boundary.
- Test same-input byte stability; project revision/working-tree/config changes
  do not change the namespace ID; path/kind/language/qualified-name/signature/
  anchor changes do; body fingerprint changes without logical ID change;
  delimiter collision resistance; invalid paths/fingerprints/spans; edge
  discriminator distinction; and strict ID parsing.

source-gate: ready after this source study; implementation was limited to the
scoped identity modules and their tests.

## Implementation receipt

Added `crates/domain/src/identity.rs` and
`crates/application/src/identity.rs`, exporting their APIs from the respective
`lib.rs` files. The domain remains free of crypto dependencies: it owns the
validated identity inputs and rendered-ID invariants, while the application
layer owns SHA-256 derivation. The implementation provides:

- `NodeId` and `EdgeId` rendered as versioned opaque SHA-256 IDs;
- validated `IdentityNamespace`, `NodeIdentityInput` and
  `EdgeIdentityInput` value objects;
- length-prefixed canonical framing, snapshot-independent repository/worktree
  scoping, signature/path/kind/language/byte-anchor inputs and edge
  discriminator support;
- a separately validated content fingerprint so body changes are detectable
  without silently changing the logical symbol ID;
- strict parse/format checks and fail-closed path, span, text and fingerprint
  validation.

Focused tests cover byte stability, snapshot changes, same-line collisions,
body fingerprint changes, delimiter framing, invalid values, edge relation
discriminators and strict parsing. The modules do not write graph state,
accept assertions, or claim rename/move matching.

The first implementation attempt exposed a forbidden `graph-domain → sha2`
dependency in the architecture gate. It was corrected before acceptance by
moving hashing/canonical digest construction to `graph-application`; no
architecture exception was added.

Validation:

```text
scripts/with-local-tools rustfmt --edition 2024 crates/domain/src/identity.rs crates/application/src/identity.rs crates/domain/src/lib.rs crates/application/src/lib.rs — pass
scripts/with-local-tools cargo test -p graph-domain -p graph-application --locked --offline — pass: 24 domain unit, 14 application unit, 16 deployment, 3 sql-link, 9 source-capture, 8 deployment-context, 0 doc failures
node scripts/check-architecture.mjs . — pass: 8 packages, 49 declared dependencies, 0 errors
sh scripts/validate-foundation.sh — pass: architecture/preflight, Orders fixture and current workspace tests
```

The repository's strict Clippy binary is unavailable in the active toolchain;
the existing workspace warning baseline remains tracked separately. The next
task is P1.T02 schema versioning/compatibility, which must add a new source
receipt before its implementation.

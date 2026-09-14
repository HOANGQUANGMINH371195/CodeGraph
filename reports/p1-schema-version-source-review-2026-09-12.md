# P1.T02 schema version and compatibility — 2026-09-12

status: done | owner: coordinator | source-gate: ready
scope: `crates/protocol` schema policy and explicit extension fields
depends_on: P1.T01, P1.T03–P1.T06
review: coordinator self-check passed | blocker: none | next: P1.T07 source-gate

## Source-first checklist

- [x] Reset this task's source-gate to `pending` before the source pass.
- [x] Read the current product protocol and its negative schema tests.
- [x] Read `outsource/codegraph` schema/migration and extraction-version flows.
- [x] Read `outsource/ripwire` cache format/parser/architecture version guards
  and refusal tests.
- [x] Read `outsource/codepropertygraph` schema validation and loader/pass tests.
- [x] Record the decision and planned tests before implementation.
- [x] Recheck the implementation against the source behavior after coding.

## Source study and decisions

- CodeGraph separates SQLite table migration (`CURRENT_SCHEMA_VERSION`) from
  extracted-content/parser version. A migration upgrades known storage in
  order; extractor changes invalidate/rebuild facts. Adopt this separation;
  protocol schema must not be reused as storage or extractor version.
- Ripwire binds cache reuse to format, parser family and artifact architecture,
  gives every refusal a reason, and reparses on mismatch. Adopt fail-closed
  future-version rejection and explicit refusal classification. Do not treat a
  cache hit as evidence that source content is fresh.
- CodePropertyGraph validates schema violations when applying a diff and its
  loader explicitly converts supported formats. Adopt validation before domain
  conversion and format-specific compatibility adapters; do not use raw numeric
  graph IDs as cross-snapshot identity.
- The product already uses required numeric `schema_version` plus
  `deny_unknown_fields` on wire DTOs. Keep rejection of arbitrary unknown keys
  in v1 so misspellings and authority fields cannot pass silently. Define
  forward compatibility as an explicit, bounded `extensions` object with
  namespaced keys; a future schema may add fields, while an old reader only
  preserves/ignores declared extensions after validating their envelope.

## Implementation contract

Add a protocol-owned schema policy with:

- one current numeric wire version and a typed check used before every DTO's
  domain conversion;
- `SchemaCompatibility`/error classification for current, legacy and future
  versions, with no silent downgrade;
- bounded namespaced extension fields whose keys and serialized bytes are
  validated, while unknown top-level fields remain rejected;
- a round-trip fixture proving extensions survive, a typo/unknown-field
  rejection fixture, future/legacy-version rejection and duplicate-key decode
  checks.

The policy must remain independent from SQLite migration version,
`GraphVersion`, extractor version and cache format version. It grants no graph
write authority and does not make an extension semantically trusted.

source-gate: ready; implementation is authorized only for this protocol policy
and its compatibility fixtures.

## Implementation and verification receipt

- Implemented in `crates/protocol/src/schema.rs` and applied to every current
  protocol DTO converter before domain conversion; `ContextEnvelope` carries
  optional bounded `extensions` with `#[serde(default)]`.
- `SchemaCompatibility` classifies current, legacy and future versions;
  `require_current_schema` rejects both legacy and future versions without a
  downgrade. Unknown top-level fields remain rejected, while only explicitly
  namespaced extension keys are accepted.
- No dependency from `graph-domain` to `sha2`, SQLite, or protocol serialization
  was added. This keeps schema policy in the wire adapter and keeps storage,
  extractor, graph, and cache versions independent.
- Source behavior recheck: CodeGraph's migration/extractor-version split,
  Ripwire's format/parser/architecture cache guards, and CPG's validation-before
  conversion remain represented as separate boundaries; extensions do not grant
  graph-write or trust authority.
- Commands: `scripts/with-local-tools rustfmt --edition 2024 ...` (exit 0);
  `scripts/with-local-tools cargo test -p graph-protocol --locked --offline`
  (exit 0, 69 unit + 3 integration tests, doctests pass);
  `sh scripts/validate-foundation.sh` (exit 0, architecture/preflight,
  fixture, workspace tests and doctests pass).
- Tests cover current/legacy/future classification, bounded namespaced
  extension round-trip, invalid keys and oversized values, missing legacy
  extensions, unknown top-level fields, duplicate keys, and all DTO conversion
  boundaries.

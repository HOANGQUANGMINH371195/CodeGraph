# W1 — declared Cargo dependency gate

Added `scripts/check-architecture.mjs` and a local foundation command,
`sh scripts/validate-foundation.sh`. The latter stops on a failed architecture
check, diagnostic test, or Rust workspace test. No CI service was configured.

The checker consumes `cargo metadata --format-version 1 --no-deps --locked
--offline`, not a regex approximation of Cargo manifests. Its explicit policy
keeps domain independent of adapters, application dependent only on domain
and its approved utility libraries, and protocol independent of persistence.
Store/source implement the inward-facing ports; the CLI composes adapters.
All declared optional, target-specific, dev and build dependencies are checked,
including declarations inactive on the current host. Actual package names,
not dependency aliases, are checked. Internal dependencies must refer to the
expected workspace manifest path; external path/git replacements are rejected.
An unclassified or missing workspace package fails the check.

## Evidence

On 2026-09-11, `sh scripts/validate-foundation.sh` completed with exit 0:

- 6 workspace packages and 29 direct dependency declarations accepted.
- Cargo metadata SHA-256:
  `bab9f73b972138acfc7fe31d7bd939b7e1dbe684d116b339f3ee78a4e45e9cf5`.
- 10 Node tests passed: 6 architecture cases plus the existing 4 diagnostic
  tests. Negative cases cover reversed layers, renamed/optional/platform/build/
  dev edges, internal package impersonation, external git origin, unknown or
  missing workspace members, and dev-only dependency leakage into production.
- 21 Rust workspace tests passed, 0 failed or ignored; existing source
  verification, SQLite transactions/migrations, fenced leases/outbox, domain,
  protocol and CLI evidence tests remain green.

An initial test-fixture aliasing bug shared the package-name array between
fixtures; it was corrected by creating a fresh member array for each fixture
before the successful combined run. No application dependencies were changed.

`rust-router` and `m11-ecosystem` informed use of Cargo's resolved manifest
metadata and explicit dependency review, without adding a TOML parsing library.

## Limits

This is a direct-manifest dependency policy, not transitive supply-chain
approval or a runtime sandbox. It does not inspect module-level imports,
macro expansion, build-script behavior, filesystem capabilities or semantic
layering inside a crate. The gate was validated on Linux, not Windows/macOS.
The validation command does not include strict clippy/rustfmt or whole-product
acceptance, and it does not make the temporary string-based task `integrate`
API a real verification gate. Snapshot authority, AnalysisRun identity,
assertion decisions/projectors and the remaining W1 contracts remain open.

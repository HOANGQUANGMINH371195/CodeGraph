# W4 file-target worker test receipt

## Source study before edits

Studied the current source and assertions before adding tests:

- `/home/minh/projects/outsource/codegraph/src/graph/route-flow.ts`
  - SHA-256: `c9f96e322f8a8fd60e01c7c7f28da21719f28c4d25cbb2258c484f3a62b7ea69`
  - Adopt: bounded descriptor capture, descriptor-only success metadata, and
    a non-runtime-verified result.
  - Avoid: route-flow's route-source-only scope, parser/body work, and its
    fixed `MAX_PARSE_BYTES` policy; this sidecar must exercise the parent API's
    separate source and target limits.
- `/home/minh/projects/outsource/codegraph/src/utils.ts`
  - SHA-256: `634a86b3dc0241a00460d062934c3854e7f1839b77202f1ddd7f6c56a88265ce`
  - Adopt: `validatePathWithinRoot` as the strict lexical and realpath-aware
    containment chokepoint, with ordinary missing paths treated separately
    from unsafe resolution failures.
  - Avoid: changing the containment implementation or duplicating its symlink
    escape matrix in this worker-owned test file.
- `/home/minh/projects/outsource/codegraph/__tests__/security.test.ts`
  - SHA-256: `94f51dd77424211e4cdcfc0b58244892c380762bb68cdc6aedcb8cf47e68c908`
  - Adopt: real temporary roots, cleanup after each case, and explicit
    assertions for file/directory/path safety outcomes.
  - Avoid: symlink creation and symlink-boundary assertions; the parent owns
    that boundary file.

## Test scope

The sidecar covers a concrete JavaScript `readFileSync(new URL(...))` target:
good capture with hashes/line counts and no body, source drift rejection,
target mutation re-capture, exact/one-under source and target limits, and
missing/directory target failure. It initializes and loads all grammars and
does not edit production code.

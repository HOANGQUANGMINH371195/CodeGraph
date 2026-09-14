# W4 string call-chain worker tests — 2026-09-12

## Source gate (before test edits)

Source study: CodeGraph `3ed73bc127323e63153bf6ec8354afa82ce36aaf`; reread
`__tests__/receiver-hazards.test.ts`, `__tests__/member-this-effects.test.ts`,
`__tests__/construction-sites.test.ts`, and
`__tests__/string-substitution.test.ts` before creating the worker tests.

Source hashes captured before editing:

- `receiver-hazards.test.ts`: `8e8d87eee625092590bff4255cb76d36345369ca8cca3086a648bd30893b1c1b`
- `member-this-effects.test.ts`: `7f7a52faa3e196b08ddec87eaf34b4285c9b68a0ce067eed31dde48cf041ddfd`
- `construction-sites.test.ts`: `0b412b1bfa91f5eda3422cb43b573ec5a910cc9f50b38a609346e7b2a1bb764d`
- `string-substitution.test.ts`: `598b60f3af633b39cc8ab1b400f2e0a8758503cdb3714f66b9b4cf19276a5f6b`

Source flow and decisions:

- Adopt exact source extents and detached, source-bound assertions from string
  substitution and construction-site tests; call-chain inputs must use the
  complete invocation ranges because nested calls can share a start offset.
- Adopt lexical binding discipline from construction sites: immutable local
  aliases may resolve in declaration scope, while shadowed or written bindings
  remain unresolved.
- Adopt receiver evidence boundaries from receiver hazards/member-this effects:
  lexical arrows retain outer `this`, ordinary nested functions do not, nested
  classes have separate receivers, and instance method calls are only
  class-local evidence.
- Avoid asserting reason strings or runtime/file authority. Candidate results
  must expose only the value plus `runtimeVerified:false`,
  `fileBindingVerified:false`, `receiverCoverage:'class-local-only'`, complete
  hop extents/binding labels, and parameter bindings.
- The references do not provide a multi-hop string-call API. The tests therefore
  define the missing contract: outermost-to-innermost invocation order, exact
  final template extent, two-hop local receiver/function propagation, and
refusal of mutation, foreign same-name classes, and ordinary-function `this`.

## Verification

Focused command:

`node_modules/.bin/vitest run __tests__/string-call-chain.test.ts`

Output: `1` test file passed, `6` tests passed; Vitest duration `1.74s`.
`git diff --check` also passed. The initial `npx vitest ...` probe could not
run because `npx` is not installed; the repository-local Vitest binary was
used successfully.

Final paths:

- `/home/minh/projects/outsource/codegraph/__tests__/string-call-chain.test.ts`
- `/home/minh/projects/project-graph-agent/reports/w4-string-call-chain-worker-tests-2026-09-12.md`

## Test scope

Owned files: `codegraph/__tests__/string-call-chain.test.ts` and this report.
The parent owns `src/resolution/string-call-chain.ts`; no production or other
test files are changed. Six focused tests cover candidate provenance, two
independent query names, immutable aliases, lexical-arrow versus ordinary
function receivers, foreign same-name classes, and mutation rejection.

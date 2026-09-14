# W0 — retained TS/JS member-call evidence

CodeGraph reference revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`,
with local accumulated changes; this is not an upstream-clean baseline.

## Cause and change

Both extractors intentionally discarded identifier-rooted member-chain calls
to avoid matching their terminal method name to unrelated project symbols
(upstream #1566/#1707). Consequently `prisma.user.findMany()` disappeared
before insertion into `unresolved_refs`; Steps could not classify its database
effect even though the source contained the call.

TypeScript/WASM and the Rust kernel now retain plain static dotted chains as
unresolved references, including their existing caller and source coordinates.
The TS resolver explicitly declines these nested TS/JS calls before generic
name/import guessing. This preserves an unknown boundary, not a proven target
or a new internal edge. Existing `this` field and `window` namespace handling
is unchanged. Computed/subscript receivers remain unsupported; optional dots
are normalized for the reference name, with the original source still authoritative.

Tests now assert that host calls remain available in the database without
self-edges or same-named project edges. Kernel parity expectations retain
static chains and independently visited argument calls; computed receivers
are still omitted. Temporary Next.js diagnostic logging was removed.

## Verification so far

From `/home/minh/projects/outsource/codegraph`, with the product's pinned npm
wrapper prepended to PATH:

- `CODEGRAPH_KERNEL=0 npm test -- __tests__/ts-chained-receiver.test.ts __tests__/nextjs.test.ts __tests__/resolution.test.ts --maxWorkers=1 --minWorkers=1`:
  **250 passed / 3 files**. All 36 Next.js tests pass, including the two
  previously failing Steps cases.
- `CODEGRAPH_KERNEL=0 npm test -- __tests__/ui-steps-api.test.ts __tests__/ui-steps-api-servers.test.ts __tests__/ui-steps-cross-tier.test.ts --maxWorkers=1 --minWorkers=1`:
  **41 passed, 2 failed / 3 files**. Servers (18) and cross-tier (12) are green;
  the two remaining API failures concern the missing `setZipUri` store action.
  Its source uses `useCaptureStorage((s) => s.setZipUri)`, a selector binding,
  not the `.getState()` binding currently handled by the store resolver.
- `npm run build`: **passed**, engine plus viewer and asset checks.
- `bash scripts/build-kernel.sh`: **passed**, release kernel staged; existing
  unused-mut and Scala scanner warnings remain. Kernel SHA-256:
  `9412a1cdd39f003b5757eaeb9d2a1bbbc2e80a0e4079f7f9c22249c85e0838a1`.
- After both builds finished, native-enabled tests for kernel TS/JS parity,
  nested receivers, Next.js, Steps servers and Steps cross-tier: **91 passed,
  0 failed, 0 skipped / 5 files**. This includes all 21 parity cases (four
  JS/TS variants, torture fixtures, real source and CRLF variants).
  Receipt: `.harness/baselines/codegraph-chain-evidence-targeted-20260911.json`.

The initial WASM runs overlapped builds (WASM extraction explicitly disabled
the kernel); they are diagnosis/regression evidence, not a hermetic full-suite
receipt. Full-suite status must be re-measured after the kernel is staged.
No W0/W4 completion is claimed. The Rust router skill prompted verification
of both extraction implementations rather than claiming WASM-only coverage.

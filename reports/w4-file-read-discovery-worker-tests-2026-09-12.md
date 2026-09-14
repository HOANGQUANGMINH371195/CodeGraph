# W4 file-read discovery worker tests — 2026-09-12

Scope: tests-only sidecar for `discoverLocalFileReads(source, language, limits?)`.

Parent relocated this worker-created report from codegraph/reports to the
authorized product reports directory on resumption; original evidence retained.

## Pre-edit receipt

Observed source inputs before edits:

- `src/resolution/string-call-chain.ts` — `f736cb78b0875abb8918c07b5275700ef0db24a6991ed3bd7fdcfe3304dc51d1`
- `src/resolution/file-read-binding.ts` — `8caec57f75cc755e25832526cb8d0c0c013f801a79d90e5bc70996dd2e369523`
- `__tests__/file-read-binding.test.ts` — `1c2ba874c6e937c5d24f0b75a322116a489e225ce4b40d70b18047706fe32c05`
- `__tests__/file-read-target.test.ts` — `2b032f0a14dee605e2c08c6e77294cdcf2920f7c9f61033e27770ffc1906735e`
- `__tests__/file-read-target-boundary.test.ts` — `2e5cf3623a6c768d0b9ec61d109e17d14bea96411d03d9bc7218ae26bbeab076`

Adopted: the existing binding vocabulary and exact source extents; imported `node:fs`/`fs` recognition; substitution values from the binding.

Avoided: production edits; filesystem reads; caller-offset inputs; parent-owned limits, truncation, cycles, and boundary tests.

## Coverage added

The sidecar covers a positive two-hop class-method chain, a direct top-level literal, imported aliases, repeated equal values at distinct calls, fake-fs rejection, and an uncalled wrapper reported as incomplete. It also checks the stable observation metadata without requiring runtime verification.

## Validation

The repository-local Vitest binary passed the focused suite: 1 test file and 6 tests passed.

```sh
./node_modules/.bin/vitest run __tests__/file-read-discovery.test.ts
```

# W0 follow-up: store-owner resolution and source identity

The original clean-checkout baseline remains in
[the baseline report](w0-codegraph-2026-09-11.md). This follow-up uses the local
patch on CodeGraph revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`.
It is not a clean upstream revision and is not whole-product acceptance.

## Build and deterministic probe

From the CodeGraph repository, with the product's `scripts/tool-bin` on PATH:

```sh
npm run build
```

Build succeeded on Linux x64, Node 22.22.1. TypeScript, asset copy and viewer
build completed; the packaging check found the viewer and all 29 grammars.
This was an incremental build, not a clean-machine reproducibility test.

From the product repository:

```sh
node scripts/codegraph-baseline.mjs /home/minh/projects/outsource/codegraph .harness/baselines/factory-closure-store-owner-20260911.json
node --test scripts/source-fingerprint.test.mjs scripts/preflight.test.mjs
```

The probe completed with exit 0 and confirmed the native kernel loaded.
[Receipt](../.harness/baselines/factory-closure-store-owner-20260911.json)
records source fingerprint
`f5f339ac7ebdb39c91fba1b05dd9e6348925c51cde90189c6cd0c514c5fb9c2a`,
unchanged before/after execution. It hashes tracked and non-ignored untracked
source, including the new resolver module, and the entire built `dist` tree.
Schema v2 distinguishes a stable dirty worktree from a clean checkout; it
explicitly does not prove build/source correspondence. Ignored dependencies,
toolchain and external configuration are not a hermetic execution snapshot.

The diagnostic result is unchanged: 189/385 source lines, 8/13 inner definition
lines delivered. Both requested actions are present, but the envelope is
15,939 characters with 15,936 before finalization. Correction: the latter is
not the budget. The soft target is 13,000 and hard ceiling 19,500; the fixture
exceeds the soft target but stays below the hard ceiling. A three-character
hard-budget overrun cannot be inferred from these fields.
Exit 0 here means the diagnostic ran and recorded stable inputs, not acceptance.

The four Node tests passed, including detecting changed bytes under identical
Git porcelain status, new files, deletion, executable-mode changes, symlink
identity and a source file named `__proto__`. POSIX mode/link checks have not
been validated on Windows. Existing preflight tests remained green.

## Full-suite comparison

```sh
npm test -- --maxWorkers=2 --minWorkers=1 --reporter=json --outputFile=/home/minh/projects/project-graph-agent/.harness/baselines/codegraph-after-store-owner-20260911.json
```

Run from the CodeGraph repository. Exit code 1: **4,601 passed, 14 failed,
11 skipped**, 4,626 tests in 267 files.
[Full result](../.harness/baselines/codegraph-after-store-owner-20260911.json).
Comparison by failing test identity found no newly failing tests. The previous
store-action cross-file caller test and per-definition callers truncation test
now pass; the new lexical-owner regression case passes as well.

| Remaining file | Failed tests |
|---|---:|
| kernel-dart-parity.test.ts | 4 |
| nextjs.test.ts | 2 |
| react-native-bridge.test.ts | 1 |
| ui-steps-api-servers.test.ts | 3 |
| ui-steps-api.test.ts | 2 |
| ui-steps-cross-tier.test.ts | 2 |

The full suite and incremental build overlapped. Source fingerprinting was
introduced during this run, not before its launch; the probe's before/after
identity must not be presented as a pre-launch fingerprint for the full suite.
This run provides regression triage, not hermetic timing or build reproducibility.

## Remaining gates

Full-suite acceptance remains failing. No live agent,
account entitlement, MCP session, token saving or completed architecture
understanding is proven by this deterministic probe. W0 and W4 remain open.

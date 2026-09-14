# W4 file-read CLI worker receipt — 2026-09-12

## Source fingerprint

- CodeGraph repository revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`.
- `__tests__/cli-route-context.test.ts` SHA-256: `06060aad2f18f8f013ba32fb966036ab7551aae9abea3ae3bcad30d14b833e90`.
- `src/bin/codegraph.ts` SHA-256: `271e125ea147f3cf65fe2d95fb775d9a552cbc23eb1f8ee432005b57769536f6`.
- `src/resolution/file-read-target.ts` SHA-256: `20afc8cc3a0ea390033ca3bba248c4dc7c09a7a795dba87ab91c9546caffaa0f`.
- Guidance read: `/home/minh/projects/project-graph-agent/AGENTS.md` and `/home/minh/projects/outsource/codegraph/AGENTS.md`.

## Source study and adoption

Adopt from `cli-route-context.test.ts`:

- Use a temporary root and subprocess invocation of the built `dist/bin/codegraph.js`.
- Suppress daemon, relaunch, telemetry, and update-check side effects with `CODEGRAPH_NO_DAEMON=1`, `CODEGRAPH_WASM_RELAUNCHED=1`, `CODEGRAPH_TELEMETRY=0`, and `CODEGRAPH_NO_UPDATE_CHECK=1`.
- Assert exit status, stderr, and stdout explicitly; require exact output-byte behavior and no partial stdout on invalid input.
- Keep fixtures small and source-body-free in assertions.

Adopt from `src/bin/codegraph.ts` and `file-read-target.ts`:

- Exercise the emitted JSON contract through `file-reads SOURCE --path ROOT` with an exact root.
- Cover bounded integer parsing, source SHA-256 verification, target identity fields, containment/refusal outcomes, and the distinction between observed, incomplete, and unavailable.
- Treat `runtimeVerified`, `atomicSnapshotVerified`, and `raceFreeContainmentVerified` as false and preserve `requiresStableFilesystem: true`; these are observation results, not stronger guarantees.

## Avoid and scope boundaries

- Do not modify production CLI or resolver code; the parent owns implementation and source symlink/byte-cap boundary tests.
- Do not initialize or index fixtures, include function bodies, recurse into `.codegraph`, auto-select `index`, or use an inexact project root.
- Do not write config, accounts, snapshots, or any file other than this receipt and the requested sidecar test.
- Do not compile/build. Validation, if possible, is limited to the requested local Vitest command and may depend on the parent-provided build.

## Test design committed after this receipt

The sidecar will cover: wrapper-plus-`this` positive discovery to SQL, no bodies and no `.codegraph` output, exact and one-byte-under output caps, source-hash mismatch, malformed numeric and unsupported-language rejection, and a missing-target incomplete result with an explicit failure. It will use the requested default and bounded option values and assert the complete JSON shape on incomplete results.

## Validation

After the parent reported the build ready, ran from `codegraph`:

`/home/minh/projects/project-graph-agent/scripts/with-local-tools node node_modules/vitest/vitest.mjs run __tests__/cli-file-reads.test.ts`

Result: 1 file passed, 11 tests passed, exit 0. No compilation was performed by this worker.

The product guidance requested `gpt-5.6-luna` for new workers. No native child
worker was spawned for this bounded sidecar, so this receipt makes no claim of
Luna execution; the effective coordinating-model identity is not exposed by
the local test runner.

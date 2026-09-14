# W0 — Dart extension-type ownership

Both parsers already recognize `extension_type_declaration`, but neither
extractor dispatched it as a class-like container. Its members fell into the
generic method-to-function fallback; native and WASM differed on getters and
sibling body ranges. Four original parity failures exposed this gap.

## Change and scope

Add extension-type declarations to the existing class-container dispatch and
enclosing-type-name lookup in both the TypeScript Dart extractor and Rust
kernel. Members now take the established class-method path, retaining
`Meters::km` and `Meters::report` instead of orphan top-level functions.
The method's existing sibling-body handling preserves the full line range.

This uses the graph's existing `class` category for a class-like source
container; it does not assert runtime class identity for Dart extension types.
Representation-field modeling, type erasure, generated constructors and all
Dart semantic rules are not claimed implemented. No route/navigation/WHEN
support was added. Framework/language coverage guidance was read; the three
coverage axes are unchanged. Re-indexing is required for changed node kinds,
IDs and containment to appear in an existing database.

## Verification

- `CODEGRAPH_KERNEL=0 npm test -- __tests__/extraction.test.ts -t Dart --maxWorkers=1 --minWorkers=1`:
  15 selected tests passed; 640 unrelated tests filtered/skipped.
- TypeScript compiler passed after the source change.
- Native `bash scripts/build-kernel.sh` release build passed and staged the
  kernel; pre-existing unused-mut and Scala scanner warnings remain.
  Kernel SHA-256:
  `16437fff86616321789e9c5352886aa34fc67faee7719d858b1ba46660327575`.
- Dart parity, TS/JS parity and React Native bridge: **65 passed, 0 failed,
  0 skipped / 3 files**. All four old Dart failures now pass. Two new LF/CRLF
  parity cases additionally assert two extension-type containers, same-named
  getter ownership, return type, method body lines, a call owned by that method,
  and absence of orphan top-level getter/method functions.
  Receipt: `.harness/baselines/codegraph-dart-extension-type-20260911.json`.
- `git diff --check` passed.

The Rust router skill informed the mirrored implementation and dual-backend
verification. No real-repository agent A/B, Dart SDK execution or complete
language-coverage acceptance is claimed by these fixture tests.

## Full-suite follow-up

Launched after compilation and targeted tests completed:

`npm test -- --maxWorkers=2 --minWorkers=1 --reporter=json --outputFile=/home/minh/projects/project-graph-agent/.harness/baselines/codegraph-after-dart-extension-20260911.json`

Pre-run source SHA-256:
`742e34304011a5836d3bb58e0aec9e11783e817aacd71c8ae43235b0e953845e`.
Revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty local modifications.
The full suite finished with exit 0: **4,629 passed, 0 failed, 11 skipped /
4,640 tests in 268 files**. Post-run source SHA-256 matches the pre-run hash.
All five failures from the previous completed full suite are gone; the initial
16 failing tests have now been repaired across the accumulated local patches.

The 11 skipped tests are eight host-platform-specific cases (Windows/macOS),
two presence assertions enabled only by `CODEGRAPH_KERNEL_EXPECT=1`, and one
100ms UI query check that requires this reference repository to be indexed.
The actual kernel/parity tests ran; a separate run with
`CODEGRAPH_KERNEL_EXPECT=1` for kernel scaffold/deep-nesting completed with
**18 passed, 0 failed, 0 skipped / 2 files**, including both opt-in presence
assertions. Receipt: `.harness/baselines/codegraph-kernel-required-20260911.json`.
The reference repo has not been
indexed solely to turn on the optional performance test. Cross-platform and
real-agent/retrieval benchmarks remain unproven.

This is a green Linux full suite on the dirty local CodeGraph revision, not
hermetic build attestation, an upstream release result or completion of W0/W4.

The current built factory-closure probe was also rerun with
`scripts/codegraph-baseline.mjs`; receipt:
`.harness/baselines/factory-closure-final-fixes-20260911.json`.
It exited 0 with unchanged source and the kernel loaded. Eight of thirteen
inner definition lines were delivered. Output exceeds the 13,000-character
soft target but not the 19,500-character hard ceiling; product acceptance
remains false. The receipt hashes built files but explicitly does not prove
build/source correspondence.

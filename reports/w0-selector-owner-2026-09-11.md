# W0 — Zustand selector ownership

The two remaining Steps API failures after member-chain preservation were
missing the action selected by `useCaptureStorage(s => s.setZipUri)`.
The store resolver recognized `.getState()` but not hook selector bindings.

## Implementation

The existing lexical binding pass now recognizes a constant identifier bound
to a synchronous, single-parameter arrow selecting a direct property. A block
body is supported only when its sole statement is the return; parentheses and
typed required parameters are handled structurally. The selector's member
receiver must be that exact parameter. No string/comment matching establishes
the binding.

The selected owner is resolved using the existing local/import path, then
verified against a top-level constant initialized with the actual named
`create` import from `zustand` (including an import alias). Direct and empty
curried factory call syntax are recognized when the initializer returns an
object literal. An unrelated factory with the same method names is declined.
Unknown or mutable owners do not fall through to global action-name guessing.
Binding caches are cleared on sync. `var` declarations now enter the nearest
function scope, preventing block syntax from hiding their hoisted shadowing.

This is a limited static rule, not general execution of selectors or complete
Zustand support. Computed selectors, async selectors, multi-statement selectors,
middleware-wrapped factories, namespace/default factory imports and other
dynamic factory shapes remain unsupported. Existing `.getState()` handling
is unchanged except for the corrected `var` scope bookkeeping.

## Verification

- Initial Steps API and object-literal tests: 20/20 passed.
- Expanded object-literal suite: 11/11 passed, including TS/TSX/JS/JSX selector
  fixtures with two same-action stores, default-exported stores, aliased
  `create`, captured bindings, mutable/shadowed bindings, unrelated factories,
  async/computed/wrong-parameter/multi-statement selectors and sync invalidation.
- Final targeted run also includes hoisted `var` shadowing, Steps API and
  all resolution tests: **234 passed, 0 failed, 0 skipped / 3 files**.
  Receipt: `.harness/baselines/codegraph-selector-targeted-20260911.json`.
- Engine/viewer build passed; after the final `var` change, TypeScript compiler
  passed again. The unchanged viewer assets were already built.
- Full suite launched only after builds and targeted tests completed, with
  `--maxWorkers=2 --minWorkers=1 --reporter=json` and receipt destination
  `.harness/baselines/codegraph-after-selector-20260911.json`.
  Pre-run source fingerprint:
  `c77fd1304e59f96ac48ce1768052e95872c37c9a02d61ee86f4768c67c0621b7`.
  Reference revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty local
  modifications. The run finished with exit 1: **4,622 passed, 5 failed,
  11 skipped / 4,638 tests in 268 files**. The post-completion source hash
  exactly matches the pre-run hash above.

Compared by failing test identity with
`.harness/baselines/codegraph-after-store-owner-20260911.json`, nine old
failures are fixed and there are **no newly failing tests**: Next.js (2),
Steps servers (3), Steps API (2), Steps cross-tier (2). The remaining failures
are the four Dart parity cases (`torture.dart` and `TortureCtors.dart`, each
LF/CRLF) and React Native Android/iOS cross-platform pairing (1).

The full run started after builds and targeted tests completed; no subsequent
CodeGraph source/build writes occurred during it. Source identity is verified,
but ignored dependencies, process environment and build/source correspondence
are not a hermetic execution attestation. Test runtime is not a controlled
performance benchmark. Further root-cause details are in
`reports/w0-remaining-triage-2026-09-11.md`.

The handwritten HNSW benchmark is independent of these changes. Neither this
repair nor its targeted tests complete W0/W4 or the full product plan.

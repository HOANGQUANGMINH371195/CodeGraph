# W0 — read-only triage while the selector full suite runs

These probes ran while the full suite was live, without CodeGraph source/build
edits. The run subsequently finished: 4,622 passed, 5 failed, 11 skipped in
268 files, with no new failing test identities versus the prior 14 failures.
The post-completion source hash equals the pre-run
`c77fd1304e59f96ac48ce1768052e95872c37c9a02d61ee86f4768c67c0621b7`.
The five remaining failures are precisely the Dart and React Native cases
investigated below. Receipt: `.harness/baselines/codegraph-after-selector-20260911.json`.

## Dart parity: concrete delta

Read the built extraction APIs directly, initialize/load Dart grammars, extract
the same fixture path/content once with `tryKernelExtract` (`KERNEL_LANGS=all`)
and once with `extractFromSource` (`KERNEL=0`), and compare serialized nodes
excluding `updatedAt`, as the existing parity tests do.

- `fixtures/torture.dart`: native alone emits function `km` on line 213,
  signature/return type `double`; WASM has no differing replacement node.
- `fixtures/TortureCtors.dart`: native alone emits `km` on line 12. Both emit
  `report` with the same ID and start line 13, but native ends on line 15
  (the body end), whereas WASM ends on line 13 (the signature).
- The fixture source is an `extension type Meters(double value)` containing
  `double get km => value / 1000` and `void report() { print(km); }`.
- A direct WASM parser probe recognizes `extension_type_declaration` with a
  `class_body`, `method_signature > getter_signature` for `km`, and a separate
  sibling `function_body` for each member. This is not simply an unparseable
  grammar feature.
- Neither current Dart extractor's class-kind dispatch includes
  `extension_type_declaration`; members therefore pass through the generic
  non-class method-to-function path. Native/WASM differ in that fallback.

Next: establish the correct class/member ownership for extension types and
body ranges, then test both extraction paths and downstream references. Do not
delete native getter evidence or shorten a true body range just to match WASM.
Language-coverage instructions must be read before extending the Dart rules.

## React Native: precision gate and bridge ordering

The old failing end-to-end fixture explicitly imports `NativeModules` from
`react-native` and calls `NativeModules.RNThing.uniquePingMethod()`. It declares
both Java `@ReactMethod` and ObjC `RCT_EXPORT_METHOD` implementations. Its
assertion expects cross-platform pairing edges.

After the static-chain change, extraction retains the call. However,
`ReferenceResolver.resolveOneInner` declines all plain nested JS/TS chains
before the framework loop, so this call still cannot reach the existing
`react-native-bridge` resolver. The bridge itself can resolve a named module
at 0.95 confidence and declines a known module missing the requested method;
its general bare-name fallback is only 0.6 and must not be used to bypass the
new precision gate indiscriminately.

Next: a narrowly evidence-checked bridge route for imported, unshadowed native
module receivers, with same-name decoys, missing modules/methods and lexical
shadowing negatives. Preserve the unresolved-chain guard for unrelated calls.
The existing bridge's broad regex alias collection is not sufficient proof of
lexical receiver ownership.

These findings identify the next implementation work; neither fix is claimed
implemented or verified here. W0 and W4 remain open.

# W0 — evidence-gated React Native receiver route

The last full suite (before this patch) had 4,622 passed, 5 failed and 11
skipped tests. One failure was Android/iOS cross-platform pairing: the retained
`NativeModules.RNThing.uniquePingMethod` reference was rejected by the generic
nested-chain precision gate before reaching the bridge resolver.

## Change

The gate now permits only a verified native-module call to the existing
`react-native-bridge` resolver. The cached AST binding pass checks a runtime
named `NativeModules` import from the literal `react-native` module, supporting
an import alias. Local declarations, parameters, destructured bindings,
hoisted `var` and a function's own name block the imported receiver in their
lexical scope. Type-only imports do not establish runtime evidence.

The bridge receives a canonical receiver but the resolution result retains
the original reference/alias and source coordinates. Only a module-scoped
result with at least 0.9 confidence and React Native bridge metadata is
accepted; the bridge's low-confidence bare-name fallback cannot bypass this
gate. Unrelated nested chains and missing modules/methods remain unresolved.
The existing bridge pairing pass then links the actual Java/ObjC methods.

This does not overhaul the older two-segment native alias regex collection or
prove runtime object immutability. Namespace imports, require/re-export forms,
computed chains and dynamic receiver mutations are not newly supported.

## Verification

- Original bridge/chain/store group: 39/39 tests passed after the fix.
- Expanded end-to-end fixture verifies direct and aliased import calls, native
  cross-platform pairing, and negative parameter/destructured/block/hoisted-var
  shadows, function-expression self-name, unrelated receiver/import,
  type-only import and missing module/method.
- Final regression run: **249 passed, 0 failed, 0 skipped / 4 files**:
  `react-native-bridge`, `ts-chained-receiver`, `object-literal-methods`, and
  all `resolution` tests. Receipt:
  `.harness/baselines/codegraph-native-receiver-20260911.json`.
- `npm run build` passed; TypeScript compiler was rerun successfully after
  the final function-self-name change. `git diff --check` passed.

The targeted run overlapped compilation; it is not a hermetic benchmark.
No full-suite rerun after this patch is claimed. Four known Dart parity
failures remain to be addressed; full-suite acceptance remains open. The
249 passing tests do not complete W0/W4 or the product plan.

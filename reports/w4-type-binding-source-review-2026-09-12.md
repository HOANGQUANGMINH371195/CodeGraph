# W4 exact TypeScript type binding

Source-gate: ready after current source reads; component implemented and focused
verification complete. Runtime resolver integration remains pending.
Baseline CodeGraph HEAD `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty
fingerprint `0e9466c671a94c12511325f5814046cc5a39cc65fce3e0d0ca38d6898e0f62cc`.

## Sources read before code

- `class-exports.ts`, complete: exact AST/node extent joins, module-local
  bindings, explicit exports before stars, all-branch ambiguity and bounded
  cycle traversal, source hashes and finally-disposed trees. Its runtime-only
  import rejection and const-value aliases must not be reused as type semantics.
- `import-resolver.ts:resolveImportPath` and uncached dispatch: existing relative
  and project-alias path resolution; no new filename guessing. Resolver context
  owns memo lifetime; caller must supply a stable index/snapshot context.
- `types.ts:ResolutionContext`, complete: node and file access is available;
  simple-name/supertype helpers do not prove lexical identity.
- `injected-call-mapping.test.ts`: real temporary project/index, exact ID and
  negative-decoy assertions, read-only mapper checks, ambiguity and source
  freshness tests. Reuse that test setup, not mocks of SQLite.
- Current class-field evidence receipt/PLAN: retain full annotations; resolving
  a type declaration does not prove runtime receiver allocation or writes.

## Decisions before implementation

Implement a separate read-only type-namespace binding component. Input is a
file, exact source offset and a supported simple/qualified type name; output
is a unique class/interface node plus source evidence, or explicit
missing/unknown/ambiguous. Preserve lexical scopes and generic parameter
shadowing. Follow type-only named/default/namespace imports and explicit/star
reexports through indexed files, with bounded cycles. Direct aliases may follow
simple/qualified type names, never truncate unions, arrays, conditional or generic
applications. Values do not become types merely because they share a name.

Unsupported namespaces/merges, ambiguous declarations, unavailable/invalid
source and non-unique node joins must stop resolution. This component is not
yet runtime-edge authority; integration still requires exact member lookup,
field write/escape policy, inheritance and index/sync lifecycle verification.
No SQL/schema/package/native ABI changes. A separate module keeps runtime
class/function export semantics unchanged.

Tests: actual imports with aliases, type-only forms, default and namespace;
same-named decoys and block-local definitions; class/function type parameters;
forward/simple aliases versus unsupported compound aliases; export diamonds,
ambiguity and cycles; exact node mapping and invalid source. Record actual
supported cases and gaps after verification, not inferred full coverage.

- [x] Current sources and adoption/avoidance recorded before implementation.
- [x] Type binding component verified within the supported scope below.
- [ ] Typed-field closure resolver integrated and lifecycle verified.
- [ ] End-to-end W4 acceptance.

## Additional source gate during implementation

Re-read `constructor-join.test.ts` export identity/diamond/default/class-expression
fixtures, `tree-sitter.ts:createNode/extractClass/extractInterface`, and
`import-emitted-specifier.test.ts` before refining these cases. Read-only AST
probes with the bundled grammar confirm `export type *` and `export type * as`
produce parse errors, whereas named type reexports and ordinary namespace
reexports parse. Keep explicit unknown results for unsupported syntax; the
grammar gap remains open, not silently rewritten to another syntax.

The same probes show import-equals uses `import_require_clause` and exported
ambient classes have an `ambient_declaration` wrapper. Before patching those
paths: block unsupported import-equals bindings at their lexical scope; unwrap
only an exact single class/interface/type declaration for ambient exports.
Preserve alias binding identity separately from the terminal node. Add negative
tests for changes to the file inventory and cached target joins. These checks
do not replace a caller-owned source/index snapshot verification.

## Reviewer gate — lexical blockers

Avicenna reread the new component and found two simple-field false-positive
paths: a class self binding bypasses an enclosing class/interface merge, and a
reopened namespace can incorrectly fall through to an outer type. Before fixes:
add regression fixtures; remove redundant declaration self bindings (retain
class-expression self bindings), and make unsupported namespace scopes stop
outer lookup. Conditional/infer/mapped type scopes also need explicit blockers
for queries within their syntax. Exact declaration-name checks reject obvious
source/index mismatches with equal extents, but cannot prove full snapshot
coherence; stable source/index collection remains a caller precondition.

## Verification and remaining integration work

Implemented `src/resolution/type-binding.ts` and 54 tests in
`__tests__/type-binding.test.ts`. The API returns exact class/interface node and
binding identity with source hashes, never writes graph edges. It supports
lexical class/interface bindings, nongeneric simple/qualified aliases, type-only
named/default/namespace imports, named type reexports, ordinary star/namespace
reexports, actual default declarations and single ambient declaration wrappers.
Unsupported namespaces (including lookup from their nested scopes), merges,
compound aliases, import-equals and generic/mapped/inferred binders stop lookup.

Results retained in `.harness/baselines/`:

- `type-binding-20260912-01.json`: 21 pass / 3 fail. Tests exposed unavailable
  class-expression nodes and the bundled grammar's type-star export rejection.
  These remain explicit coverage gaps; supported alternatives have separate
  positive fixtures, not claims that the missing forms now work.
- `type-binding-review-red-20260912-01.json`: 45 pass / 7 fail before the
  reviewer-driven fixes (merge/self-binding, reopened namespace, mapped/infer
  shadowing and same-extent declaration-name mismatch).
- `type-binding-20260912-06.json`: 116 pass / 0 fail, four files, exit 0:
  54 type-binding, 15 field-evidence, 40 constructor-join and 7 emitted-specifier
  tests. These checks preserve existing runtime constructor/export behavior.
- Fresh final `tsc --noEmit`: exit 0. No full suite, native rebuild or Orders
  diagram smoke was run for this read-only component.
- Final CodeGraph source fingerprint:
  `d50245947faea360e37f8eac23552099ef103cf9692f5507244a14899bc0f2e7`.

Snapshot guards reject observed source changes, changed file inventory and a
removed/ambiguous cached target join. They do not prove that preexisting index
nodes correspond to all source bytes, compiler configuration or import-path
resolution. That remains an integration precondition; existing path resolution
is reused, not represented as a full TypeScript compiler implementation.

Next: connect field annotation evidence to this binding API at the field's
declaration offset, preserving unknown outcomes; implement exact member and
inheritance lookup with a separate runtime-write/receiver policy. `typeof`
value fields and JS constructor initializer evidence need their own value
binding path. No old regex field resolver was replaced in this component;
typed-field closure calls, namespace merging, class-expression extraction,
type-star grammar coverage and end-to-end W4 flows/diagrams remain open.

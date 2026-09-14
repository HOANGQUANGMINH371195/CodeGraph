# W4 exact class-field declaration evidence

Status: collector implemented; integration pending. This is a prerequisite to
closure field-call resolution, not a replacement for the full W4 flow goal.
Baseline HEAD 3ed73bc127323e63153bf6ec8354afa82ce36aaf, dirty fingerprint
2e381dad998b149f929bbd7aadc41108535cd626bc4353b3a6ab10311f8630e9.

## Source before implementation

- name-matcher.ts matchTsThisFieldCall selects the class by truncating the
  caller qualified name and then scans all its declaration lines with regex.
  Returned scopes break that owner selection; whole-class line scanning can
  pick local variables or nested classes, ignores static/instance distinction,
  truncates unions and chooses same-name targets by directory proximity.
- returned-callable-this.ts already derives exact lexical-this class spans
  and static mode, resetting ordinary function receiver. Reuse its identity
  discipline, not another qualified-name guess.
- constructor-facts.ts and its tests provide the existing AST collector pattern:
  parser availability/error is null, successful no-facts is [], source offsets,
  direct class/constructor children and tree deletion in finally.
- ts-this-field-call.test.ts preserves annotated constructor properties, JS
  constructor new-initializers, builtin rejection and typeof object namespaces.
- class-exports.ts provides exact ES definition identity, but is a runtime
  class/function export resolver, not a general TypeScript type namespace
  resolver. Do not silently reuse its rejection of type-only bindings as proof
  that typed fields have no target. Reviewer Sagan is checking integration.

## Decisions before coding

First implement a read-only AST field-evidence collector, retaining full type
syntax/text (including unions, typeof, generics and unknowns), class and member
extents, static/instance, optional/readonly, and explicit constructor parameter
properties. Plain constructor parameters and body-local declarations are not
fields. Record initializer source/extent separately; do not infer a runtime
target from an annotation or first regex match. Unknown computed keys remain
explicit unknown field names, not invented property names. No graph writes,
new SQL, schema, package version or native ABI changes in this component.

Tests must cover nested/same-name classes, Unicode/CRLF, static/instance same
field, constructor parameter-property modifiers vs plain parameters, full union
and typeof text, computed/quoted/private keys, malformed/unsupported language
and successful empty collection. Integration still needs scoped binding/type
resolution, member lookup, writes/ambiguity handling and index/sync evidence.

- [x] Current source and tests read; adoption/gaps recorded before code.
- [x] Collector implemented and initial focused tests verified (30 passed).
- [ ] Closure field-call resolver integration and lifecycle verified.
- [ ] End-to-end W4 flows/diagrams verified.

## Resume source gate — additional negative coverage

Source-gate reset on resume, then ready after rereading the current collector,
constructor-facts implementation/tests, returned-callable-this implementation/tests,
matchTsThisFieldCall and its integration tests. Current dirty fingerprint:
0cca331fdef113bc714e8b507cb95ab28bbf5310f09b3067671319f5b580cd24.
The existing collector receipt reports 30 passing tests; a fresh standalone
`tsc --noEmit` completed with exit 0. No old process remained live in the process
inventory; the earlier typecheck's lost handle is not treated as success.

Before the next test patch: adopt constructor-facts' negative-evidence fixtures
to exclude comments, method/accessor bodies, static blocks and ordinary constructor
parameters from declaration facts. Assert that unsupported type annotations and
initializers remain separate, and invalid syntax cannot leak partial facts.
No graph resolver behavior changes in this patch.

Sagan's read-only review highlights remaining integration constraints:

- Preserve exact owner/class and declaration-scope identity, then resolve type
  or value binding to a unique node ID; never return to nearest/simple-name lookup.
- Type-only imports, aliases and wildcard-export ambiguity require dedicated
  binding support; the runtime-only class export resolver is insufficient.
- Distinguish receiver field staticness from instance versus constructor-valued
  field contents. Resolve inherited fields and methods with node-based traversal.
- Constructor body assignments are not field declarations and remain outside
  this collector. Writes/escapes must be handled before claiming runtime edges.
- Abstract-class lexical-this support and deferred TS field-call retry are gaps.
  Full typed-field closure support and W4 acceptance remain unchecked.

## Verification after resume

- `class-field-evidence-20260912-02.json`: 43 passed, 0 failed, four test
  files, runner exit 0. Includes 15 collector tests, 21 constructor-facts
  tests, three lexical-this tests and four existing field-call integration tests.
- Fresh `tsc --noEmit` after the added tests: exit 0.
- Final source fingerprint:
  `0e9466c671a94c12511325f5814046cc5a39cc65fce3e0d0ca38d6898e0f62cc`.
- Only focused checks were run for this component. The historical 5055-test
  full suite is not a full-suite result for this fingerprint. The collector
  is not yet consumed by the resolver; no improved runtime graph coverage,
  native parity change or completed end-to-end diagram is claimed.

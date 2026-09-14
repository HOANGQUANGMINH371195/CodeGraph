# JS string-argument evidence — before-code source gate

Parent reread CodeGraph AGENTS.md completely, construction-sites.ts declaration,
write invalidation, lexical lookup, origin/functionOrigin and materializeCall
flows, construction-sites.test.ts assertions and function-parameter-uses tests.
Current reference revision: 3ed73bc127323e63153bf6ec8354afa82ce36aaf.
Pre-edit construction-sites.ts SHA-256:
26c8e5448e57737f87bac7ef428efcce31a6e039909fad20df27a0717fa63e0b.

Adopt existing declaration-scope alias resolution, all-write invalidation,
parameter identity, dynamic-scope refusal, spread position uncertainty and
detached AST spans. Do not use abbreviated display arguments, match method
names globally, evaluate JavaScript, or turn query names into table identities.

Next implementation: opt-in string expressions on call/constructor arguments,
default callers unchanged. Bounded literals, immutable aliases, symbolic plain
parameters and untagged template interpolation preserve query-key/path inputs.
Only literal+literal binary concatenation is folded (parameter arithmetic must
not be mistaken for string concatenation). Escaped literals and unsupported
expressions stay unknown initially, explicitly not complete JS evaluation.
Parameter expressions carry function/parameter source identity, not a runtime
value; spread/unknown callee still cannot imply a dispatch or file reference.
Per-argument step, depth and text limits bound this additional evidence.

Tests must cover positive literals/aliases/templates, lexical shadowing,
mutation, spread, dynamic scope, unsupported coercion/escapes, bounds and
default-output compatibility. No graph edges/persistence/schema changes here.
Wrapper substitution, import-bound fs/URL recognition, path containment and
actual query-file hash binding remain subsequent integration requirements.

## Implementation and focused verification

Implemented opt-in includeStringArguments in the existing lexical pass; default
evidence shape unchanged. Literal/parameter/template/unknown forms are detached
from the WASM tree. Template CRLF/CR normalization is explicit. Limits per
argument: 4096 UTF-16 code units of expanded literal text, 256 recursive steps,
depth 64. These are expression evidence limits, not source-read/parse limits.

23 new tests plus construction/function-parameter/constructor-join/parameter-call
regressions: 191 passed across five files, terminal exit 0. tsc --noEmit and
tsc emit both exit 0; git diff --check exit 0. Current source hashes:
- construction-sites.ts: 0cc0c74322f893eefb56d9f5c22372d4cb055eedb409d6dac2f6c0526f0b5138
- string-argument-evidence.test.ts: f4eceaacdb40a14d4878ff13775c743491c816b7c277e07897b7f2f71b79e289

Orders emitted build probe: 11 literal query keys match authored expectations;
URL argument preserves symbolic name parameter at functionStart=97. Receipt:
.harness/baselines/orders-string-arguments-20260912-01.json. Initial probe used
a prefix filter and incorrectly selected chained outer .run()/.all() calls;
it failed before writing a receipt. Corrected fixture-only selection matches
whole simple sql/this.query calls; this is not a production binding resolver.
linkage_verified and product_acceptance remain false.

Full native-required CodeGraph suite completed in exec session 44649: exit 1,
5380 passed, 1 failed, 9 skipped, 307 files, 313.51 seconds. Receipt:
.harness/baselines/codegraph-string-arguments-20260912-01.json; source fingerprint
before/after matches. Only failure: ui-server-api busiest-symbol timing,
127.923386ms against unchanged <100ms assertion; correctness assertions before
the timing assertion passed. Isolated rerun subsequently passed 61 tests with
1 skip (session 74823), without edits to UI implementation/test or threshold.
This does not erase the full-suite failure or establish full-suite green.

Subsequent parameter-initialization guard changes the source fingerprint;
see w4-string-parameter-order-source-review-2026-09-12.md. Full regression for
that newer snapshot remains open; the previous process is terminal, not live.

Luna Jason was read-only and returned no findings before parent requested
shutdown. Shutdown confirmed by native notification. Parent authored this
implementation/tests; no delegated research completion or token saving claimed.

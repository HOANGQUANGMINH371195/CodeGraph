# W4 construction-site lexical evidence

status: done (lexical construction-origin component only); source-gate: ready;
owner/reviewer: main, no independent review.

- [x] Reset gate on resume; reread constructor-facts implementation and its
  complete 21-case test file, store-accessor scope creation/lookup, and the
  exclusive this-field name-matcher gate. Read ResolutionContext/ResolvedRef
  interfaces and CodeGraph/project guidance.
- [x] Source before patch: CodeGraph revision
  `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty fingerprint
  `ed6370fb8fe8a2e83175953a67eb07252703e40c7b1ccd9258d7e973cba0a17b`.
- [x] Adopt lexical scope parent lookup and AST field-based parsing; extend
  evidence to `new` sites and immutable aliases instead of matching callee names
  globally. Preserve import module/export identity rather than resolving it by
  directory proximity. Reject unsupported/mutable/shadowed bindings explicitly.

Before-code contract: plain-data construction origins, not inferred runtime
types. Record every new-expression, exact offsets and argument positions; after
spread, parameter positions are unknown. Local class identity is its extent;
import identity is the actual runtime import spelling, awaiting import resolution.
Constants may forward a syntactically known class/instance initializer. Parameters,
destructuring, catch variables, var/let, and functions block outer names. Assignment
to a class binding invalidates that origin. With/eval conservatively invalidate
origins; do not follow arbitrary factories or computed members. Keep unknown sites
alongside known ones, rather than silently filtering them from candidate coverage.

Tests planned: all four JS-family grammars; positional inline new and aliases;
renamed/default/type-only imports; function/block/catch/var shadowing; reassignment;
forward references/cycles; spread; multiple sites and unsupported expressions.
This component precedes graph integration. Constructor returns, instance mutation/
escape, cross-file module binding and incremental invalidation remain required for
the full resolver; syntax origins alone must not authorize call edges.

## Implementation and verification

Added `codegraph/src/resolution/construction-sites.ts` and
`codegraph/__tests__/construction-sites.test.ts`. Origins preserve class extents,
import module/export spelling, allocation sites and explicit unknown reasons.
Two-phase lexical lookup establishes declarations before evaluating references;
const aliases resolve in their declaration scope, not the consumer's scope.
Temporary parser trees are disposed; output contains only detached data.

The initial 32 cases passed. Four added for-in/of regression tests then failed:
the grammar puts declaration names directly in `for_in_statement.left`, with no
`variable_declarator`. Inspected actual grammar AST via existing dist grammar
loader, added the direct binding/assignment path, and reran. Added TypeScript
runtime enum shadow protection as well. No source-to-dist correspondence is
claimed by the AST inspection (the final tests run against source).

Final checks from CodeGraph cwd using product `scripts/with-local-tools`:

- `node node_modules/vitest/vitest.mjs run __tests__/construction-sites.test.ts
  __tests__/constructor-facts.test.ts __tests__/ts-this-field-call.test.ts
  __tests__/call-receiver-no-fabrication.test.ts`: exit 0, **65 passed** across
  four files (37 construction-site, 21 assignment, 7 integration regressions).
- `node node_modules/typescript/bin/tsc --noEmit`: exit 0.
- `git diff --check`: exit 0.
- Post-code source fingerprint:
  `18a527bd7705473e86ee807f13df9b9d59a06d5bc93e4aaa488b7e7b76834fc2`.

No full suite/foundation rerun, dist rebuild, account activity, graph edge changes
or W4 acceptance claimed. In particular an `instance` origin only identifies a
syntactic allocation through a lexical class/import binding; it does NOT prove
the class of the returned runtime value or absence of later mutation.

Next task: combine assignment and construction evidence with exact project import
resolution; model receiver writes/constructor returns/escapes, multiple known and
unknown sites, provenance and resolution-pass invalidation before enabling edges.
Rerun the unchanged orders corpus after integration; do not replace it with a
typed/direct-initializer fixture to obtain a passing flow.

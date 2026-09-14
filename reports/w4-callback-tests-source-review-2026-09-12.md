# W4 callback extraction tests — source review, 2026-09-12

Status: source study recorded before test implementation; test file written;
execution pending parent.

## Source-study

Read both project AGENTS.md files and project-graph-agent PLAN §0.4.1.
CodeGraph HEAD: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`; working tree is dirty
and shared with main. project-graph-agent has no resolvable HEAD. Source SHA-256
fingerprints captured before this test patch:

| File in CodeGraph | SHA-256 |
| --- | --- |
| `__tests__/returned-callables.test.ts` | `8595acdd6daedd3e5d6c3eb5267fbca6a0fcf21fb11977ee7f3b9e48f9ad19af` |
| `src/extraction/returned-callable.ts` | `10e9234bc5e1f4dbad35b4b0b59d28c793fe02b063759492e1f54e0dab6be756` |
| `src/extraction/tree-sitter.ts` | `1c65cf7396a6c63155a5f097349bd79850e07f349249bb30d36d2beba95cb1f8` |

Read the complete returned-callables test and naming helper. The tests use real
`extractFromSource`, initialize all four grammars, force WASM, restore kernel
environment state after each case, and compare canonical nodes/edges/unresolved
references against native when available. `CODEGRAPH_KERNEL_EXPECT=1` makes a
missing native kernel a failure. Timestamps are excluded from comparison.

Read `tree-sitter.ts` `extractFunction` (1593–1706), nested
`visitForCallsAndStructure` function handling (5711–5761),
`reactHookBoundName` (5580 onward), and direct receiver handling (4685–4730).
Existing names and declarator/CommonJS bindings precede synthetic returned
names. Named, hook-bound, declarator-bound and returned functions enter their
own body scope; other anonymous bodies currently fall through to outer scope.
Returned scopes explicitly preserve `this.send`, suppress export inheritance,
and get contains edges without invocation edges. The naming helper walks only
transparent/value-producing wrappers and rejects unsupported positions, notably
call callees. These are the mechanisms this test work must exercise.

Read main's
[ownership receipt](w4-callback-ownership-source-review-2026-09-12.md).
Main owns TS/native extraction and rebuilds. This task changes tests only.

## Adopt / avoid / adaptation decided before code

- Adopt the existing real extraction/grammar/environment/native parity harness
  within this repository; no external implementation or dependency is copied.
  Read the repository MIT license (copyright Colby Mchenry, 2026).
- Adapt location assertions to `<callback@line:UTF16column>` using fixture string
  offsets (UTF-16), independent of the implementation's naming helper. Check
  node kind, qualified name, contains owner, non-export, and each body call owner.
- Cover JS/JSX/TS/TSX with CRLF and an astral Unicode character before callbacks;
  direct call and constructor arguments; same-line siblings and nested callbacks;
  anonymous arrows, ordinary functions and generators; transparent wrappers,
  sequence tails, and conditional/logical value branches.
- Preserve explicit names, local declarator bindings and React useCallback
  binding names. Reject callback labels for IIFE callees and functions hidden
  in object/array arguments. Do not assert resolver invocation or reachability
  from a contains edge. Direct deferred receiver refs must remain `this.send`.
- Avoid mocked extraction, source-text assertions, hardcoded node IDs, broad
  native-result truthiness, and assertions that accept outer-owned deferred calls.
  Native results must exist before canonical comparison when a kernel is loaded.
- New work is the callback-specific fixture/assertion matrix: existing returned
  tests cover value-position traversal but not anonymous call/new argument nodes.
  No new extraction algorithm is designed in this task.

## Constraints and validation

Only authorized writes: `codegraph/__tests__/callback-callables.test.ts` and this
receipt. No tests/builds, recursive agents, global configuration, commits, or
TS/native source changes. Parent owns scheduling and native rebuild. No passing
test claim is made; runtime and parity acceptance remain pending.

## Written result and static review

Added `__tests__/callback-callables.test.ts` with the real extraction harness and
native comparison on every fixture. All four grammars cover direct call/new
arguments with anonymous arrow/ordinary/generator functions, Unicode/CRLF and
same-line siblings, nested callback ownership, `this.send` for all three forms,
named/declarator/React hook preservation, IIFE callee exclusions, and file-level
containment. TypeScript cases cover transparent wrappers, conditional/logical
branches, sequence tails, unsupported value positions, and a returned function
inside a callback. Both extraction paths must report no extraction errors;
negative node-count assertions therefore cannot silently accept a parser failure.

During static review, read `extract()` file-node setup and
`declaratorBoundFunction` (5535 onward). The latter currently recognizes only
arrow/function-expression local bindings, so preservation tests use those actual
supported binding forms. Anonymous and explicitly named generators are covered
as direct arguments; this task does not demand a new local generator-binding
feature. Test files are excluded by the repository's production tsconfig.

Changed files are exactly the two authorized paths. Reviewed written assertions
against current source; no tests, compiler, or native build executed. The
read-only `git diff --check -- __tests__/callback-callables.test.ts` emitted no
diagnostics, but the new file is untracked, so this is not validation of its
contents. Parent must run the focused suite after rebuilding native, preferably
with `CODEGRAPH_KERNEL_EXPECT=1`, then integrate broader regression results into
the main ownership receipt. Runtime behavior and parity remain unverified here.

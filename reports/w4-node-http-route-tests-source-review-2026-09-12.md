# W4 node-http route-test source review — 2026-09-12

Reviewed before coding:

- `codegraph/AGENTS.md`: framework coverage is judged through indexed graph output; route nodes should bind to handlers with framework-emitted references, and incremental sync must retract stale derived rows.
- `codegraph/docs/design/framework-coverage.md`: framework additions are self-contained route extraction work; unsupported/dynamic shapes must remain unresolved rather than becoming false route coverage.
- `codegraph/src/resolution/construction-sites.ts`: import origins retain `{ module, imported, local }`; declaration/use analysis invalidates mutable, shadowed, ambiguous, or otherwise unsafe bindings. Function origins distinguish local functions from imported functions and unknown values.
- `codegraph/src/resolution/frameworks/index.ts`: framework resolvers are registered in the central list and looked up by exact `name`; the new adapter is expected to register as `node-http`.
- `codegraph/__tests__/callback-callable-lifecycle.test.ts` and `callback-callables.test.ts`: real `CodeGraph.initSync(...).indexAll()` tests inspect synthetic callback nodes and derived edges, then use `sync({ paths })` to verify stale callback/edge removal and convergence.

Test scope derived from the parent contract: named immutable ESM `createServer` imports from `http`, `node:http`, `https`, and `node:https`; inline arrow/function callbacks with a first plain `req` parameter; exact POST `/x` predicates, including continuation after a direct terminal negative guard; same-line route uniqueness; exact callback linking without outer-factory leakage; and full index → incremental change → removal behavior. Negative cases cover unsupported or shadowed/mutated/local/wrong imports, strings/comments/nested-function guards, and request binding/property writes.

This receipt records bounded test intent only. Unsupported cases are documented coverage gaps to preserve as non-false-positive boundaries, not evidence of full framework support.

## Honest revision record

The source-study receipt was written before the parent adapter existed, but no repository revision or content hashes were captured at that moment. I do not retrospectively label the following values as pre-patch state.

At receipt finalization, the current repository revision was `3ed73bc127323e63153bf6ec8354afa82ce36aaf`.

Current SHA-256 content hashes at finalization:

- `AGENTS.md`: `478680c9e9672325245cd46f2625f4eb435e376d972f7121fba7753d02a75ee4`
- `docs/design/framework-coverage.md`: `04c07f4f579f6f2ab784a81393965300af1e93e4e27212e663231f7dec3dbf70`
- `src/resolution/construction-sites.ts`: `26c8e5448e57737f87bac7ef428efcce31a6e039909fad20df27a0717fa63e0b`
- `src/resolution/frameworks/index.ts`: `936270504d8dd985fd09c929090d61947e5dada165be96e047c7328475837a98`
- `src/resolution/frameworks/node-http.ts`: `ba521e1cb44769f8f482abb172567574450efbf28f8addd907859756d0f9acf2`
- `__tests__/callback-callable-lifecycle.test.ts`: `cdac35e84b9c835d3d25eaebe7b623b82ced98a40f66581c935b4a10be091dee`
- `__tests__/callback-callables.test.ts`: `f52fc3db318da254b0d63164bcfaf38b616340869df2d7553d5ec7865bc0623f`

Limitation: these hashes describe the current post-parent state and cannot prove the exact source bytes studied before coding.

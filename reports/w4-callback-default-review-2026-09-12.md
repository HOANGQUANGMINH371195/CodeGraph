# W4 callback-parameter initializer review — 2026-09-12

## Scope and evidence

Read-only review of current source. I read `/home/minh/projects/project-graph-agent/AGENTS.md`, `PLAN.md` §0.4.1, codegraph guidance/design notes, current TS/native extraction and resolver code, and the requested tests. This review ran no builds, tests, agents, or commits and made no writes except this report. Findings are source-derived; execution results below are supplied by main and were not rerun here.

Revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf`.

Current source fingerprints:

| File | SHA-256 |
|---|---|
| `src/extraction/returned-callable.ts` | `e6928b39fc8986464910747d91ddac378710714f501373400c4dfc2f8df7a701` |
| `src/extraction/tree-sitter.ts` | `55614faca1ead092d3de12af5c58cfb3c0c0f00276420425382a804952f00790` |
| `codegraph-kernel/src/tsjs/mod.rs` | `5793338212c0fd65a53c3f2449dc430823b80a99348f7fa066b89bf51d2d032b` |
| `codegraph-kernel/src/tsjs/extractors.rs` | `3304dce83050f5052eca9386dce0727b20c1740c6c198d503b2d83b5c1a2d83b` |
| `src/resolution/returned-callable-this.ts` | `a13a0db8f6a78ae7569570cc78a2f31a12b86a518f2d19ef29ef6aae24f6c9c5` |
| `src/graph/dead-code.ts` | `6cdf1f4bc235960cf83656cb9f099ff187aa106c2775d0572ac5e824b75eaa62` |
| `__tests__/callback-parameter-defaults.test.ts` | `4811a9b7bda8a6c537798919e8eed5b72d7348fb91620da36a8ac16c04654fc2` |
| `__tests__/callback-callable-lifecycle.test.ts` | `1bb1b71df801b0dae4f0fd82ca0a1e55bfa68fbf7674950609bb56cd144ad70d` |

Main reports: six additional default Unicode/CRLF/type-syntax cases, 24 pass / 0 fail; independent field red-case run, 24 pass / 8 fail before the field fix; post-fix `fielddefaults`, 51 pass / 0 fail across 3 files; native rebuild `29230` live. These are not independently asserted here.

## Findings

### 1. P1 — new false attribution through discarded function bodies in parameter RHS

Classification: **new regression in the newly exposed parameter-initializer path**. There may be analogous older weakness in generic anonymous-function recursion, but parameter RHS expressions were not previously entered by this path.

`src/extraction/returned-callable.ts:46-77` correctly rejects the left side of `&&` as a deferred callable value. But `src/extraction/tree-sitter.ts:5615-5645` sends the whole default RHS to `visitFunctionBody`; `visitFunctionBody` (`:5651-5800`) recursively descends through anonymous function nodes when no named/callback/returned/default owner is recognized. The native path mirrors this at `codegraph-kernel/src/tsjs/mod.rs:724-827`.

Fixture:

```ts
function f(x = (() => hidden()) && fallback) {}
```

The arrow is a discarded, never-invoked operand, so no `<default@...>` owner should be assigned to it. Generic recursion can nevertheless record `hidden` under `f`, making a body that is not executed look like an invocation-time default call. Similar risk shapes are `function f(x = { cb: () => hidden() }) {}` and `function f(x = (() => hidden()) + suffix) {}`.

Action: retain the value-position gate, but stop initializer traversal at function nodes that are neither immediately invoked nor recognized deferred callables. Add a negative assertion for the owner of `hidden`, not only an assertion that no synthetic node exists. Avoid labeling every nested function value; that would undo the existing conservatism.

### 2. P1 prior gap — class-property arrows are addressed by the current patch

Classification: **prior uncovered capability, now source-addressed; keep as a regression guard, not a duplicate implementation task**.

The original gap was in TS/JS `field_definition` / `public_field_definition`: `resolveBody` selects the nested arrow body (`src/extraction/languages/typescript.ts:63-90`, `javascript.ts:31-59`), while `visitParameterInitializers` was initially called on the outer field node. Current `extractMethod` uses the already-selected body’s immediate callable parent at `src/extraction/tree-sitter.ts:1923-1933`; native mirrors it at `codegraph-kernel/src/tsjs/extractors.rs:217-226`.

Fixtures that should remain in the guard set:

```ts
class C { handler = (x = prepare()) => consume(x); }
class C { send() {} handler = (x = () => this.send()) => consume(x); }
```

By inspection, the current TS/native source now passes the actual callable node for direct and supported call-wrapper field shapes. Main’s reported post-fix `fielddefaults` result is consistent with that repair; this review did not execute it.

Action: keep the current selected-body/`body.parent` logic covered by the eight field red cases, including direct `this` and wrapper forms. Do not regress to reading parameters from the outer field definition or walking type-only children.

### 3. P2 — HOC/component body-only path still bypasses defaults

Classification: **prior uncovered capability**, not a new false edge in an existing default path.

`extractReactComponentNode` at `src/extraction/tree-sitter.ts:1760-1771` pushes the component owner and walks only `innerFn`’s body. The HOC branch in `extractVariable` (`:2751-2767`) routes there instead of `extractFunction`, where initializer traversal is attached.

Fixture:

```ts
const C = forwardRef((props = makeProps()) => render(props));
```

The component callable’s `makeProps()` initializer is not reached by this specialized body-only path. The same source shape applies to `memo` and other configured HOC wrappers.

Action: after resolving `innerFn`, visit its parameter initializers with the component node as owner, while preserving the existing component body ownership and anonymous-callable policy. This remains unverified until main’s scheduled coverage includes it.

### 4. P2 decision — ArkTS remains outside the new gate

Classification: **prior uncovered capability**, unless scope is intentionally limited to the four whitelisted languages.

`returnedCallableName/inParameterPattern` (`src/extraction/returned-callable.ts:39-41`) and `visitParameterInitializers` (`src/extraction/tree-sitter.ts:5615-5617`) accept only `javascript`, `jsx`, `typescript`, and `tsx`. `src/extraction/languages/arkts.ts:11-18` reuses the TypeScript extractor, but these helpers reject ArkTS before syntax is considered. Fixture if ArkTS is in scope: `function f(x = prepare()) { consume(x); }` under the ArkTS language.

Action: either add ArkTS to one shared TS-family predicate in TS/native and add parity coverage, or document the limitation. This is an unresolved scope possibility, not an executed failure claim.

## Review checks with no current finding

- **Type syntax:** the visitor follows pattern/value fields and excludes parameter `type` fields (`tree-sitter.ts:5615-5645`; native `mod.rs:724-761`). This is the correct execution boundary. Keep the supplied typed/type-syntax cases as guards.
- **Nested defaults:** object/array patterns, pairs, rests, required/optional parameters, and computed keys are explicitly traversed in TS/native. The remaining concern is expression recursion after entering a value, covered by Finding 1.
- **`this` scopes:** no direct resolver regression found. `returned-callable-this.ts:18-66` lets arrows inherit lexical class scope and resets at ordinary functions; `index.ts:2533-2581` applies class/static filtering. The lifecycle arrow-versus-ordinary cases match this conservative design. The field-arrow `this` fixture belongs to Finding 2’s regression guard.
- **Same-line Unicode/CRLF:** the current JS parser and native `col16` convention are aligned, and synthetic labels/node IDs use columns on both sides. Main’s supplied 24/0 Unicode/type-syntax result is consistent with the source design; parity was not rerun here.
- **Dead-code:** `src/graph/dead-code.ts:478-491` excludes returned/callback/default synthetic functions before candidate filtering. No concrete exclusion bug found.
- **TS/native parity:** callable naming, parameter-pattern traversal, synthetic labels, and deferred `this` handling are structurally mirrored (`mod.rs:724-902`, `extractors.rs:17-80,217-226`). No obvious feature-parity drift found by inspection; execution remains unverified.

## Adopt / avoid

Adopt the current field selected-body/`body.parent` repair, existing value-position conservatism, UTF-16 columns, and resolver-side lexical-`this` validation. Add the remaining HOC owner walk and a negative discarded-body ownership fixture.

Avoid broad recursive descent through function bodies in initializer expressions, treating “no synthetic node” as proof of no call attribution, or widening ArkTS support without a shared TS/native decision and parity fixture.

# W4 parameter-call mapping source review — 2026-09-12

## Pre-patch receipt

- Product checkout revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf` (`codegraph/`).
- The product checkout was already dirty before this task; unrelated tracked and
  untracked worker changes were preserved. No files outside the task ownership
  are being edited.
- Source hashes recorded before the first implementation patch:

  - `PLAN.md`: `003a9ad288a032792f1868ee16b19230dcf5cb5f87b50dee861d20d7e1425310`
  - `codegraph/src/resolution/construction-sites.ts`:
    `26c8e5448e57737f87bac7ef428efcce31a6e039909fad20df27a0717fa63e0b`
  - `codegraph/src/resolution/constructor-join.ts`:
    `d983606bf0b34efcb28457a5672426f6a23107fa4339104799e5e5bb0cccafe5`
  - `codegraph/src/resolution/receiver-decision.ts`:
    `7ded8b02dc03432ec01975d02dd276013084a50371947be19c8bc28ff1df473f`
  - `codegraph/src/resolution/injected-call-mapping.ts`:
    `eff3381822a6948e86062d6d0fbd03e95afb1b213d5e0f27892823591905a878`
  - `codegraph/src/resolution/injected-reconciliation.ts`:
    `ebdc16c3db36f1b078a78a69592eeade9b3dee7bfaa1202d726db3e2cbb4415b`
  - `codegraph/src/db/queries.ts`:
    `e49e1f4b76afa3b832611668b15db67af45b5512287b67ea8c74412d503048db`

## Source-study

1. `PLAN.md` §0.4.1 requires a source-first receipt with source, decision, and
   tests before implementation. The W4 brief is a read-only candidate mapper;
   it must not activate reconciliation or write graph edges.
2. `construction-sites.ts` and `constructor-join.ts` provide the exact lexical
   evidence needed here: `callInventory` includes calls without tracked
   construction arguments; resolved calls carry target and parameter spans;
   allocation arguments retain `origin` and allocation offsets; parameter
   effects carry method-call paths, coordinates, and `executionFunctionStart`.
   Unknown, missing, spread, mutable, dynamic, and unsupported shapes remain
   explicit evidence rather than guesses.
3. `receiver-decision.ts` provides the bounded `reviewReceiverRoots` contract:
   roots must match indexed allocation/function/parameter spans and the walk
   reports writes, escapes, prototype boundaries, reassignment, and unresolved
   handoffs. The mapper will bind it to the current join input and use one
   bounded review pass for its collected roots, without a persistent cache or
   a new mutation engine.
4. `injected-call-mapping.ts` and its tests provide the exact UTF-16/CRLF span,
   source-hash, current-file-inventory, node-identity, provenance, and
   coordinate-aware dedup validation. Those gates will be reused for parameter
   calls, while the new mapper will not require a constructor field.
5. `injected-reconciliation.ts` and `queries.ts` show the later lifecycle seam:
   producer-scoped replacement identity is `(source,target,kind,line,column)`
   and `replaceSynthesizedSnapshot` is a writer concern. This task only returns
   candidates and receipts; it does not call either writer.
6. Orders sources in `/home/minh/projects/project-graph-agent/fixtures/orders`
   establish the target shape: imported HTTP factory callbacks receive service
   allocations, capture a parameter, and call an instance method. The mapping
   must follow resolved invocation argument → exact factory parameter → method
   effect execution function → exact callable node → exact instance method.

## Adopt / avoid / self-designed gap

- Adopt exact indexed-span and source-hash validation, `callInventory`, import
  identity from the join, bounded receiver roots, observed-source metadata,
  heuristic provenance, and coordinate-aware identity deduplication.
- Adapt injected-field mapping from owner-field candidates to every resolved
  factory invocation; aggregate all invocation evidence before deduplication so
  known sites can remain candidates while aggregate coverage is incomplete.
- Avoid field-name or class-name guessing, `contains => calls`, one-known-
  receiver exhaustiveness claims, fake invocations, graph writes, lifecycle
  integration, persistent caches, and changes to shared worker files.
- Self-design only the new parameter-call result contract and its explicit
  inventory/receiver issue metadata because no current producer maps a
  resolved call argument into a captured callback method effect.

## Planned tests

Parent verification receipts: parameter-mapping-parent-20260912-01.json,
11 pass / 0 fail before the extra controls; -02.json, 64 pass / 0 fail in
three files (14 mapper, 36 receiver, 14 injected mapper), required native,
runner exit0 (8.61s). New controls verify per-coordinate observation contents,
independent safe/unsafe receiver candidates, missing indexed class rejection,
and unresolved calls without known arguments. Producer integration remains a
separate in-progress worker task; no end-to-end graph acceptance inferred.

Parent Orders source-loaded real-index probe (owned temporary fixture, cleaned,
terminalexit0; not emitted-build/MCP/agent acceptance): stats60nodes/140edges.
New lifecycle marker reports incomplete, edgeCount0, issueCount129; replacement
counts removed0/inserted0/conflicts3/duplicates0. Crucially mapper evidence has
six observations supporting three exact callback-to-method pairs: create,
dispatch, consume, from main wiring and test wiring. No receiver-specific issues.
Existing identical graph edges are foreign null-provenance instance-method
resolution with confidence0.7/0.65, so the producer correctly does not overwrite
them. New evidence persists in the producer receipt, but graph retrieval still
needs to consume that corroborating evidence. Do not report0newedges as0mapped
relations, or claim the old heuristic edge has automatically been upgraded.
129 issues include ordinary unresolved library/member invocations; coverage is
explicitly incomplete, not129 proven bugs. Registration/routes and diagram
end-to-end remain open.

Orders probe extension source gate: parent reread the entire current product
orders-graph-smoke.mjs (SHA256
7fb1c29a1f3467988fd1b37e0698b58c7ab168a0a8c79cc9fbce0bcc1991686a),
current mapper contract and producer receipt/write path (parameter-reconciliation
b8a408a83d59b9c88c6c68ddf27ffc580eda32fa82c2c9114dc9698984980259).
Adopt its existing emitted-build imports, one joined-evidence input, owned copy,
source/dist/fixture hashes and cleanup. Add mapper output and raw durable
producer marker, with summary distinguishing candidates, observations, inserted
and conflicting edges. Do not infer acceptance or change existing watch/probe
gates. Syntax check now; execution requires a fresh emitted build after workers
finish, so no end-to-end run is claimed by this patch alone.

Probe extension node --check exit0. It now records mapper candidates and raw
producer reconciliation evidence/counts separately; acceptance remains false.

Read-only Luna review received: no demonstrated wrong edge; focused execution
was not performed by that reviewer. Confirmed coverage work still open:
function-wide unsupported-parameter conservatism; whole-input dynamic/alias
hazard conservatism; multi-hop parameter forwarding (the bounded engine can
visit forwarded parameters, but the mapper selects method effects only from
the directly allocated argument's target factory); duplicate manually forged
join evidence is not a demonstrated production AST case. Do not weaken global
mutation guards merely because a syntactically separate file appears unrelated:
an imported prototype mutation can affect the candidate. Broader coverage needs
its own source-study, positive/negative fixtures and evidence-scoped analysis.

Parent post-handoff review gate: reread current mapper in full, its real-index
tests, receiver factory and source/exact-span mapping checks. Worker closed;
parent owns mapper/test. Current mapper SHA256
4f32339ce575857bc8a0f95a785585c65a37e118a5b9227f4f0cfcb55c3bf702.
Per-candidate root review and coordinate-qualified provenance aggregation are
implemented following parent review. Add direct controls for independent unsafe
receivers, absent indexed class nodes and unresolved no-known-argument calls;
strengthen same-target multi-site test to verify each evidence coordinate, not
only array lengths. Adopt existing setup and keep assertions/source fixtures
semantically scoped. No mapper behavior change is assumed necessary until tests.

The new native-index tests cover imported factories, real source/target nodes,
Unicode/CRLF coordinates, same-named unrelated classes, multiple allocations
and call sites, aggregate metadata, unknown/missing/spread/unresolved calls,
reassignment/mutation/escape/dynamic-scope hazards, stale source/removal,
and rejection of fabricated factory invocations. The package typecheck is run
with `noEmit`.

## Patch/test receipts

- Added only `codegraph/src/resolution/parameter-call-mapping.ts` and
  `codegraph/__tests__/parameter-call-mapping.test.ts` in the product checkout.
- The mapper now imports the authoritative `createReceiverReviewer` directly,
  constructs it once per joined input, and invokes it with each candidate's
  exact allocation/parameter root group. It validates exact factory callable,
  receiver class, and instance method nodes before emitting a candidate.
- Focused native-index receipt: `./node_modules/.bin/vitest run
  __tests__/parameter-call-mapping.test.ts` — **11 passed, 0 failed**.
  The CRLF/Unicode test uses JavaScript `\r\n` escapes, checks line/column,
  and checks the absolute UTF-16 source offset including the emoji.
- A full `./node_modules/.bin/tsc --noEmit` was attempted. It reaches the new
  mapper without errors but is currently blocked by an unrelated pre-existing
  `src/mcp/tools.ts:4180` `SetIterator.some`/implicit-`any` error in the dirty
  checkout; no unrelated file was changed to hide that failure.
- Post-patch hashes: mapper
  `4f32339ce575857bc8a0f95a785585c65a37e118a5b9227f4f0cfcb55c3bf702`, tests
  `74602ceda964504e5260d30c9191bb2aa2d6a97538b3274054e8b991338cd1c2`.

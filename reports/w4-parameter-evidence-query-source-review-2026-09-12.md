# W4 parameter-call evidence query source review — 2026-09-12

## Pre-implementation receipt

- Product revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf` (`codegraph/`).
- The checkout was already dirty; all unrelated changes are preserved.
- Source study hashes recorded before implementation:
  - `src/resolution/parameter-call-mapping.ts`: `4f32339ce575857bc13c2a76d41c531dda26d7a3ff9b52f08939d5ed8b3a34f5`
  - `src/resolution/parameter-reconciliation.ts`: `b8a408a83d59b9c88c6c68ddf27ffc580eda32fa82c2c9114dc9698984980259`
  - `src/resolution/injected-reconciliation.ts`: `bdd8bbd8f72179fe539f90c014db1656ddec874cc923a4dba463c2c663d983c1`
  - `src/resolution/config-observation.ts`: source-study read at this revision
  - `src/db/queries.ts`: producer snapshot/edge identity read at this revision
  - `src/index.ts`: `7274507fa0aaaa0cdb60e6004cc1d0bef36cc22bea6d31d61f6a71240d6ffefb`
  - `src/mcp/tools.ts`: current dirty-checkout source read before this slice

## Source study and decisions

`PLAN.md` §0.4.1 requires this receipt before implementation. The parameter
mapper's receipt is source evidence only: six observations support three
coordinate-qualified candidate identities on Orders, while three insertions
collide with foreign null-provenance instance-method edges. The query must not
upgrade, replace, or relabel those edges.

Adopt the existing producer marker, `(source,target,kind,line,column)` edge
identity, indexed node endpoint checks, source hashes, config observations, and
the shared snapshot freshness/config observation model. Validate source,
inventory, and config once per request; do not rerun the mapper or keep a
persistent cache. Accept only `observed`/`incomplete` producer receipts and
make missing, malformed, running, failed, stale, or drifted receipts explicit
errors. Return bounded, coordinate-specific evidence labelled
`observed-source-only` with `requiresRuntimeAssumptions: true`; never claim an
invocation occurred.

For MCP, annotate only the already-selected callers/callees edge rows. Keep
existing labels, limits, filters, grouping, and disambiguation unchanged;
deduplicate by displayed callee/caller while aggregating all applicable
parameter observations honestly. Existing method-return mocks may not expose
the new API, so any compatibility branch will be narrow and test-explained.

## Planned verification

New native-index tests cover foreign-collision visibility, same-endpoint
different-coordinate joins, source/hash/config drift, malformed/running/failed
markers, explicit absence/incompleteness, endpoint validation, bounds, and no
graph mutation. MCP tests cover actual observation locations and assumptions,
preserved existing labels, same-callee aggregation, and the absence of runtime
proof. Lifecycle hooks remain unchanged.

## Parent reread and implementation receipt

The parent review identified and the implementation corrected these issues
before handoff:

1. Invocation location is derived from `invocationStart` in the validated
   invocation source; effect location is derived from `effectStart` in the
   source node's file. A multiline cross-file test asserts both locations.
2. Config/source/inventory validation now uses the shared
   `snapshotResolutionInputs` boundary-checked snapshot and validates its
   observed config operations once per request. An empty fresh observer is not
   treated as equivalent to a persisted non-empty receipt.
3. Marker offsets and indices require safe non-negative integers, line numbers
   are one-based, invocation paths must be in the recorded source inventory,
   and effect offsets must land inside the indexed source node span.
4. Requested identities must match an exact existing DB edge, including
   coordinates; endpoint existence alone is insufficient. No graph or receipt
   mutation occurs during the query.
5. Duplicate edge/identity inputs are canonicalized before exact-edge
   validation; missing identities are computed from validated observations
   before the evidence output bound is applied. Line-column conversion now
   rejects columns beyond the actual line terminator.

Post-fix hashes:

- `src/graph/parameter-call-evidence.ts`:
  `eeffd6a40fa81fe5dd08cee06de6b007f1f66f03d663fbe9405e2c56031f7e11`
- `__tests__/parameter-call-evidence.test.ts`:
  `3fbee2c1d774cfd030d7e43b9cf739d03a7c6b11b3eca894b245990220025f8e`
- `__tests__/parameter-call-evidence-mcp.test.ts`:
  `f69fd866c05710de88ff103709ab9584a7b187dd45b8c5dff2c66265c33d41dc`
- `src/index.ts` (dirty checkout includes inherited changes):
  `35255f3fe05af8032c53a787924a7b920ec9ae103c1986273ac2a27e3aafe3e4`
- `src/mcp/tools.ts` (dirty checkout includes inherited changes):
  `ef10187373fc3482e651be9b51895ac1939fcc0ecf45c91b1f06b35909e08ad1`

Verification: `./node_modules/.bin/vitest run
__tests__/parameter-call-evidence.test.ts
__tests__/parameter-call-evidence-mcp.test.ts` — **5 passed, 0 failed**;
`./node_modules/.bin/tsc --noEmit --pretty false` — **pass**.

Final parent-resume correction verification: duplicate requests, invalid
effect-span omission, and beyond-line columns are covered by the native reader
suite; no lifecycle or unrelated files were changed in this correction pass.

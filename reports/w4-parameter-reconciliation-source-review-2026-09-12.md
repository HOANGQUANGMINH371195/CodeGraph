# W4 parameter-call reconciliation source review — 2026-09-12

Status: source study was recorded before implementation; final digests below were
recomputed from the current filesystem during handoff audit. The historical
source-study and implementation receipt below are retained; the verification
addendum records the later lifecycle-test strengthening.

## Source revision and hashes

- Study revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf` (`codegraph`).
- `src/resolution/injected-reconciliation.ts`: `bdd8bbd8f72179fe539f90c014db1656ddec874cc923a4dba463c2c663d983c1`.
- `src/resolution/parameter-call-mapping.ts`: `4f32339ce575857bc13c2a76d41c531dda26d7a3ff9b52f08939d5ed8b3a34f5`.
- `src/db/queries.ts` snapshot replacement: read at the same revision; producer-scoped replacement is `replaceSynthesizedSnapshot` / `replaceSynthesizedEdges`.
- `src/resolution/config-observation.ts`: `source-study` read at the same revision; config fingerprints are revalidated before commit.
- `src/resolution/parameter-reconciliation.ts`: `b8a408a83d59b9c88c6c68ddf27ffc580eda32fa82c2c9114dc9698984980259`.
- `src/resolution/index.ts`: `77342fe67b793c0487e14056e3e2bac79d376677c973316b4fc93685aef54b44`.
- `src/extraction/index.ts`: `2badf738db8415c304015cabae426650875cc27be21664abf8e9c68d0cf382c5`.
- `src/index.ts`: `7274507fa0aaaa0cdb60e6004cc1d0bef36cc22bea6d31d61f6a71240d6ffefb`.
- Existing lifecycle test: `__tests__/injected-reconciliation.test.ts`, hash `db1ebfcc806c97db4b7f347fbf1c47243eaa7b0addeae560e1043f37707d531e`.
- Mapper tests: `__tests__/parameter-call-mapping.test.ts`, hash `e91dd57f73a842b0d03d3893a509c99812446a5aeb816058cde1ef8cb91b7838`.
- Lifecycle tests before this addendum: `__tests__/parameter-reconciliation.test.ts`, hash `2cee91e7097505099c09f82e97d1910f22cd96faaa825cce57c17c8024f68d9c`.

## Source study

1. `injected-reconciliation.ts` owns the complete fail-closed lifecycle. It
   checks abort before work and before replacement, pairs disk bytes with the
   indexed file hash and extraction errors, uses an observed `ResolutionContext`
   backed by those exact snapshots, validates resolution configuration, and
   commits the producer snapshot plus receipt atomically. Its public begin,
   fail, and reconcile API must remain unchanged.
2. `queries.ts` replaces the union of previously owned sources and candidate
   sources inside one SQLite transaction. It deletes only heuristic edges whose
   `metadata.synthesizedBy` matches the producer, rejects missing endpoints,
   preserves foreign/static edges, and rolls back edge and receipt writes
   together.
3. `parameter-call-mapping.ts` is read-only and already returns the required
   `status`, `coverage`, `sourceHashes`, `edges`, `issues`, and `evidence`.
   Unknown/missing/spread arguments and receiver hazards make the mapping
   `incomplete`, including the zero-edge case; its mapper must not be edited.
4. `injected-reconciliation.test.ts` verifies wiring-only rebind/removal,
   no-change retry, stale source and stored extraction failure, abort/reopen,
   receipt and edge rollback, foreign/static preservation, config invalidation,
   and unavailable evidence. These are the lifecycle cases to mirror for the
   second producer. The mapper tests verify exact callback/target edges,
   Unicode/CRLF coordinates, unknown arguments, and no graph mutation.
5. `PLAN.md` §0.4.1 requires this source-to-decision receipt before each
   implementation slice. The lifecycle skill (`m12-lifecycle/SKILL.md`) adds
   the same ownership rule: define scope, owner, creation/use/cleanup, and
   error behavior; the producer snapshot is the transaction-scoped resource.

## Adopt / adjust / avoid

- **Adopt:** shared snapshot reading, indexed hash validation, observed context,
  abort checks, config observation/revalidation, and atomic producer-scoped
  replacement. Extract only these mechanics so the injected public API and its
  receipt shape remain compatible.
- **Adjust:** parameter reconciliation uses `PARAMETER_CALL_PRODUCER` and
  `mapParameterCallGraph`, reports mapper `incomplete` even with zero edges,
  and exposes additive `parameterCalls` fields on index/sync results. Its
  begin/fail hooks clear only the parameter producer.
- **Avoid:** runtime invocation claims, changing `calls`/`contains` semantics,
  mapper edits, duplicated freshness logic, global snapshot invalidation,
  dependency/config/version changes, and assertions about invocation itself.

## Remaining design gap and tests

The existing lifecycle has no reusable source snapshot helper and no parameter
producer activation path. The minimal addition is a shared internal helper in
`injected-reconciliation.ts`, a parameter reconciliation adapter, thin resolver
and CodeGraph hooks, and additive result fields. Tests will cover real indexed
callback edges; rebind/removal, unknown argument, stale source, snapshot error,
abort/retry, foreign/injected/static preservation, receipt rollback, and config
invalidation. The mapper's incomplete status is preserved even when no edge is
produced. Validation is native Vitest plus `tsc --noEmit`; broader inherited
regressions remain the main branch gate.

## Historical patch/test receipt

- Added only `codegraph/src/resolution/parameter-call-mapping.ts` and
  `codegraph/__tests__/parameter-call-mapping.test.ts` in the product checkout.
- The mapper imports the authoritative `createReceiverReviewer` directly,
  constructs it once per joined input, invokes it with each candidate's exact
  allocation/parameter root group, and validates exact factory callable,
  receiver class, and instance method nodes before emitting a candidate.
- Focused native-index receipt: `./node_modules/.bin/vitest run
  __tests__/parameter-call-mapping.test.ts` — **11 passed, 0 failed**.
- A full `./node_modules/.bin/tsc --noEmit` was attempted and was blocked by
  an unrelated pre-existing checkout error; no unrelated file was changed.
- Historical post-patch mapper hash:
  `4f32339ce575857bc5a0f95a785585c65a37e118a5b9227f4f0cfcb55c3bf702`.

## Verification addendum

The two owned-file hashes recorded before this addendum were:

- `codegraph/__tests__/parameter-reconciliation.test.ts`:
  `2cee91e7097505099c09f82e97d1910f22cd96faaa825cce57c17c8024f68d9c`
- This report:
  `c564e5815e8e19a5fd525f6a58a5c0e05b7a8ead3a46ec7fd183d4b90b102fac`

The strengthened lifecycle test asserts real callback source/target ownership,
true A→B wiring-only rebind with stable callback/source, removal, independent
constructor-injected-field preservation, foreign/static preservation, malformed
source and stored extraction-error retraction, abort/no-change recovery,
foreign collision counts/evidence, receipt rollback, config drift, and indexed
symlink retarget outside the project root. It keeps `incomplete` honest when
the unresolved `install` inventory remains; mapper semantics were not changed.
Empty owned edges are not accepted vacuously: evidence/counts or independent
foreign/injected edges are asserted.

Required native command, exit **0**:

`./node_modules/.bin/vitest run __tests__/parameter-reconciliation.test.ts __tests__/injected-reconciliation.test.ts __tests__/parameter-call-mapping.test.ts --reporter=dot`

Result: **3 files, 40 passed, 0 failed** (parameter lifecycle 8, injected
lifecycle 18, mapper 14; runner duration 8.98s).

Required no-emit command, exit **2**:

`./node_modules/.bin/tsc --noEmit`

Result: one unrelated existing error at
`src/graph/parameter-call-evidence.ts:5`: `Node` is declared but never used.
No production file was edited to hide it.

Post-addendum test hash:
`b871f25a47943b20e47b9dca650e3b3a5b7410a0bb0f8b49cacd47bb7332c974`.
This report was restored at the exact owned path and retains the earlier study
and receipt history rather than replacing it.

## Follow-up source study before evidence-quality patch — 2026-09-12

Reread `__tests__/parameter-reconciliation.test.ts`,
`src/resolution/parameter-reconciliation.ts`, the shared snapshot/freshness
helpers in `src/resolution/injected-reconciliation.ts`, the transactional
replacement in `src/db/queries.ts`, and the analogous cases in
`__tests__/injected-reconciliation.test.ts`.

Decision: the malformed-source case must return the fixture to its indexed,
valid source before injecting `upsertFile(...errors...)`; otherwise the second
assertion can succeed using the first case's stale source-hash mismatch rather
than independently proving the stored extraction-error gate. The abort retry
must assert the actual owned parameter edge (callback source to `A::send`), not
only receipt evidence, because `replaceSynthesizedSnapshot` is the edge
authority and a receipt alone does not prove graph restoration. No production
semantics or mapper behavior needs changing.

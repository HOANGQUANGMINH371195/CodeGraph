# Orders parameter-evidence consumption

## Pre-patch source study

Parent reread the current Orders smoke wrapper, upstream emitted-build probe,
CodeGraph.getParameterCallEvidence, ToolHandler.parameterEvidenceAnnotations,
parameter reconciliation receipt writer, and the cross-file MCP test.
CodeGraph revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf` (dirty).
Observed SHA-256 before this diagnostic change:

- `src/graph/parameter-call-evidence.ts`: `1c509632e052ff91c23a013ca020326622aba61b3f03f0c6196ff3d9dcef90d0`
- `src/index.ts`: `35255f3fe05af8032c53a787924a7b920ec9ae103c1986273ac2a27e3aafe3e4`
- `src/mcp/tools.ts`: `ef10187373fc3482e651be9b51895ac1939fcc0ecf45c91b1f06b35909e08ad1`
- product `scripts/orders-graph-smoke.mjs`: `9d1ce82ca1dadba16d839edcc8434c5a2b64fde8ded75994d7b427a6e3584cc8`

Adopt the existing owned temporary fixture, source/dist fingerprints and actual
ToolHandler execution. Add the public read-only evidence result for the exact
candidate identities plus caller/callee responses for their endpoint symbols.
Keep raw responses so absent annotations cannot be hidden by aggregate counts.
Do not treat candidate edges as inserted edges, corroboration as runtime proof,
ToolHandler as MCP transport, or a smoke exit code as architecture acceptance.
The Luna reader is still being corrected; execution must follow a fresh build.

## Agent-facing guidance source gate

Reread `src/mcp/server-instructions.ts` completely and the current annotation
formatter. The instructions intentionally name only the default explore tool;
preserve that policy. Add a short source-evidence caveat to both initialized
project variants, without advertising disabled tools or implying runtime proof.
Pre-patch instructions SHA-256:
`ea0f173340febec1e072e3d6140871811b8e60c31b49ec85727269f181292522`.

## Parent verification

- Smoke script syntax check: exit 0.
- Required-native reader/MCP focused run: 5 passed, 0 failed in 2 files,
  receipt `.harness/baselines/parameter-evidence-parent-20260912-01.json`.
  This focused run alone does not prove every planned negative/limit case.
- `tsc --noEmit`: exit 0. Full `npm run build`: exit 0, including viewer and
  copied grammar checks. No native source was changed in this turn.
- Emitted Orders smoke: exit 0, receipt
  `.harness/baselines/orders-parameter-consumption-20260912-01.json`.
  60 nodes, 140 edges, 7 files. Mapper has 3 candidates and 6 observations;
  producer inserted 0 with 3 foreign-identity conflicts. Public query returns
  all 6 observations with no missing identity; status remains incomplete.
  Inspected all 5 ToolHandler responses: create/dispatch/consume callers and
  both HTTP callback callees display invocation/effect locations and explicit
  source-only/runtime-assumption caveats. This is not an MCP transport test.
- Watch refresh observed; source, dist and fixture unchanged during smoke.
  Source SHA-256: `5a0098c34f5018b16340834fad4da7da255538ba203f52758912755525b674b7`.
- All five explore processes exit 0, but the diagram response is literally
  `No relevant code found`. Process success is not semantic acceptance.
  Diagram, deployment/SQL boundaries and whole-product acceptance remain open.
- Full required-native suite started against the above source fingerprint,
  session `32502`, target receipt
  `.harness/baselines/codegraph-parameter-evidence-20260912-01.json`.
  It is still running; recover this exact handle and compare final fingerprint
  before claiming a stable full-suite result. Previous full-suite failure is
  not superseded merely by starting this run.

## Full-suite completion

Recovered session `32502` to terminal exit 0. JSON receipt independently
inspected: **5,335 passed, 0 failed, 9 skipped, 0 TODO; 5,344 tests in 300
files**. Runner duration 265.44 seconds. All file statuses passed, including
the prior failing explore oversize/reservation suites, parameter lifecycle,
mapper and evidence readers. This was one full run, not summed focused runs.

Post-run source fingerprint equals the recorded pre-run value:
`5a0098c34f5018b16340834fad4da7da255538ba203f52758912755525b674b7`.
This supersedes the earlier full-suite regression failure for this exact
source state. It does not close W4 coverage or diagram acceptance, and is
not proof of unimplemented test cases listed in a source-study plan.

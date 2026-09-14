# node:http conditional-route adapter — source gate

Parent reread framework coverage (all 253 lines), Express extract/resolve,
framework registry and contracts, extraction hook and detection/worker paths,
React Router's AST binding reader, returned-callable naming, construction call
inventory/origin binding, and current Orders HTTP source before implementation.
Revision: `3ed73bc127323e63153bf6ec8354afa82ce36aaf` (dirty).

Current reference hashes:

- construction-sites.ts: `26c8e5448e57737f87bac7ef428efcce31a6e039909fad20df27a0717fa63e0b`
- frameworks/express.ts: `616a074d38aeabb8e448a7674a9864cb8958b9b4174fdd37fb3416b03b565800`
- frameworks/index.ts: `181d4a4c05df99cb47081be68e872e2cbdd52d83e2fd674f7fa45d4b8c042153`
- extraction/tree-sitter.ts: `598d77263811ee7d7614f01f47ffc7ab8cfd59642e980f122c943312fd07caf8`

Adopt existing framework nodes/unresolved references, exact callback execution
spans, lexical import-origin inventory, normal extraction/re-index ownership,
and heuristic evidence metadata. Avoid Express's broad body-call regex: a
route must point to its real callback, not duplicate every nested body call.
Do not infer invocation from contains, claim listen/deployment from factory
creation, or register arbitrary createServer names.

Implementation: identify statically imported http/https createServer calls via
the existing lexical inventory; inspect inline callbacks with an unmodified
plain request parameter. Recognize literal method+url predicate conjunctions
and fail-fast negative disjunctions followed by unconditional return. Keep
each conditional route distinct by source position and link to an exact
callback span through a framework-owned reference. These are inferred request
conditions, not runtime traces or proof the server listens. Named/namespace/
CommonJS handlers, computed routes, richer path predicates and middleware
require follow-up source-backed coverage, not guesses.

Test genuine import aliases versus local/shadowed/mutated imports; positive
and negative guards; comments/strings/nested functions, unsafe request mutation,
source coordinates and re-index/removal. Preserve W4/W9/W11 gates until wider
coverage and Orders architecture acceptance are actually verified.

Initial typecheck exposed a contract mistake: Node has no metadata field.
Reread `src/types.ts:133-218` and FrameworkExtractionResult/UnresolvedRef.
Keep the existing DB schema: serialize the framework-owned handler evidence
in its synthetic unresolved reference, validate it against route identity and
source hash during resolution, then persist it as ordinary edge metadata.
Do not hide internal JSON in a node docstring or add an unsupported property.

## Incremental discovery gate

Current-source inspection of `ExtractionOrchestrator.ensureDetectedFrameworks`
and `indexFileWithContent` shows project detection is cached until indexAll.
New built-in imports introduced by sync need file-local opt-in detection.
Reread raw-kernel fast path in parse-worker and framework merge in
extractFromSource: both must use the same file-level decision, otherwise the
raw buffers bypass extra route nodes. Add optional detectFile to the existing
framework contract and a shared registry selector; no whole-project rescan
per changed file, no unconditional parsing of every JavaScript file.
Pre-change hashes: types.ts
`95cdc8cad05ca31e90a413274cb7baeec4ce879d21dc9ca85a69f4834b658fa5`,
parse-worker.ts
`9bcba24b6ebdafecbfefe0aed6a7fba5d9ca0aef462582b67f27c91344f73351`.

The existing extraction-version contract requires a shape-version increment
for new framework nodes; increment 30→31 (not an npm version bump). Coverage
documentation and Unreleased note must distinguish conditional request routes
from listen/deployment knowledge and broader Node routing patterns.

Parent focused run 02 exposes the second half of discovery: 172 passed,
1 failed. The new route now exists after sync, but its callback edge does not.
Reread ReferenceResolver.initialize, pre-filter and framework strategy loop:
sync does not initialize the project framework list. Keep project detection
reporting unchanged; retain a separate registry list of file-local resolvers
and consult only those that claim the exact synthetic reference. Source/hash
and route/handler identity checks remain inside their resolver. This avoids
rescanning the project or treating every Node project as detected HTTP.

## Native decode gate control

Using m10-performance's measure-before-change rule: parent reread shared
registry selector and native worker raw/decode branch. Add a direct gate test
counting selected hooks for an ordinary JS file in a detected HTTP project.
The intended count is zero; HTTP source still selects one hook when the cached
project detection is empty. This measures work admission, not wall-clock speed.
Make detectFile authoritative for opt-in resolvers instead of letting stale or
broad project detection override its per-file decision. Other frameworks retain
their original project-based selection. No algorithmic speedup claim is made.

Admission red receipt `node-http-admission-red-20260912-01.json`: 2 passed,
1 failed, exit 1; ordinary JS selected one HTTP hook instead of zero. Selector
then changed to honor detectFile as authoritative. Initial build passed before
this final selector adjustment; rebuild before the emitted Orders check.

Parent reread the product Orders smoke wrapper before adding route receipts.
Script hash before the addition:
`88cfbadd0d65179f0f779dcf7f3c8faa020de75aeb0f1252aa4f63eec8c38157`.
Adopt existing graph API and owned fixture/fingerprint checks: record each route
and actual outgoing edges without inventing a diagram score or runtime proof.

## Parent integration receipts

- Initial focused run 01: 5 passed. Initial typecheck failed on unsupported
  Node.metadata and nullable AST node; both corrected, then noEmit passed.
- Lifecycle red: new HTTP file after a no-HTTP index produced zero routes.
  File-local extraction fixed route creation; parent run 02 still failed the
  actual route→callback edge assertion (172 passed, 1 failed). File-local
  reference dispatch fixed that second gap.
- Run 03: 174 passed across 4 files. New-file and existing-file discovery are
  now independently asserted; both require a real heuristic callback edge.
- Run 04: **186 passed, 0 failed across 7 files**, including 8 route tests,
  3 native-admission controls, framework/kernel/parse-pool compatibility and
  parameter lifecycle/MCP controls. JSON:
  `.harness/baselines/node-http-routes-parent-20260912-04.json`.
- Final `npm run build`: exit 0, including viewer and grammar asset checks.
  Script syntax check and git diff --check pass. No Rust native source changed.
- Source frozen after Luna test handoff; full required-native suite started
  with SHA-256 `5dabb5dbf0101a3e67253f3f95f229787a0589136d2f507dc80dcbcd78a29763`,
  session `2665`, receipt `.harness/baselines/codegraph-node-http-20260912-01.json`.
  Recovered terminal exit 1: **5345 passed / 1 failed / 9 skipped, 5355 tests**.
  The sole failure is `ui-server-api.test.ts:647`, the busiest-symbol API request
  taking 112.90653199999906 ms against the unchanged <100 ms threshold. Main
  verified the after-run source fingerprint matched the frozen hash above.
  Later diagnostic report files are separate worktree changes, not test inputs
  retroactively included in this receipt. No new full-suite green is claimed.
- Emitted Orders smoke started in session `78696`, target receipt
  `.harness/baselines/orders-node-http-20260912-01.json`; inspect the route and
  edge records, not only process status. Previous full-suite green applies
  only to its older parameter-integration snapshot, not this new source.

Remaining: namespace/CommonJS/named handlers; complex predicates and per-route
path-sensitive continuation; raw HTTP registration/listen/deployment mapping;
real small/medium/large repository and agent sufficiency validation. An edge to
the callback does not imply every conditional call inside it is reachable for
that endpoint. W4/W9/W11 and whole-product acceptance stay open.

## Emitted Orders result

Session `78696` terminated exit 0. Inspected receipt routes and actual edges:
`POST /orders` (http.mjs:15) and `POST /dispatch` (:19) both link to
`ordersServer::<callback@13:22>`; `POST /events` (:28) links to
`notificationsServer::<callback@26:22>`. All three carry heuristic provenance,
the exact predicate text, registration location and HTTP source hash
`9e645ee86d8c64aefd77605305f0543e861e5635c633634b1b2f494d8dbe02b6`.
The events predicate is explicitly a reject-guard continuation, not a positive
branch. Counts are 63 nodes / 143 edges / 7 files (+3 routes/+3 calls).

The 3 parameter candidates / 6 observations / 3 foreign conflicts and 6 public
query observations remain unchanged. Watch refresh passed; source, dist and
fixture were unchanged during smoke. Diagram query still returns `No relevant
code found`. The full-suite process `2665` is terminal with the threshold
failure detailed above; the node-http full-suite gate remains open.

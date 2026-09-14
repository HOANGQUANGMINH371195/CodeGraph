# Deployment graph to Archify — source gate

Before coding, parent reread PLAN P7.T01–T03 and current Archify revision
`18911058008f17dc065af23a2cdc9bfeff6d3f7a`:

- Complete architecture/common schemas: strict additionalProperties, conservative
  identifiers, typed components/connections/cards; connections cannot carry
  arbitrary evidence fields. SHA-256 respectively
  `94568c7ca72c07ac0ee02ee76bd39392874b3a13bb796a127962be68d0f5bc90`,
  `2630c958314eeebb92e0f6f6330475f1a2bc48f8c1b53201ba3b58889b319c80`.
- renderers/shared/cli.mjs loadDiagram and validateRelationshipIds; validator.mjs
  schema validation; architecture/render-architecture.mjs:1–180 and complete
  architecture/grid.mjs (SHA
  `f6de99c32436feed3378b0ddef516bb4732ff27a89c543b50635d4d14a15a55a`).
  Grid is fixed placement, NOT graph auto-layout. Render validation may reject
  routing/layout; schema-conforming output alone does not prove readability.
- shared/repository-evidence.mjs:68–119 (SHA
  `ce3c358093aee12790d5567de6c887ce7b9ce7961bfc742d2cf9f36eab2d4dd5`),
  test/repository-evidence.test.mjs:412–430,518–585 (SHA
  `a078539c8b61f26f3694c44dced2e83eb58d3f5ad16556d23d6bcf3ea594fd0b`):
  source links require verified Git root/origin/commit, not caller-supplied
  historical graph citations. Do NOT emit meta.repository or component.sources
  from existing metadata. Preserve complete citations in a separate manifest;
  pinned Git navigation remains open until actually verified.
- test/legend-contract.test.mjs:1–90,135–168 (SHA
  `99abc8fb61a3b8fa1971378ba0107205554b16ca5bbe3f068a6a8900f9e26197`):
  legend kinds follow authored types. Use neutral external component type for
  Compose service/volume/network, with explicit sublabel and custom legend;
  never imply volume=database or network=cloud provider.
- bin/archify.mjs:900–1040 (SHA
  `cb00a6a60cff395f1728722ce6c41b7721b484c5349ae82111ee16719a672721`):
  deliver freezes input and stages beside destination, checks artifact before
  replacing last-good. Exercise actual validate/deliver locally; do not copy a
  renderer into Rust or launch browser/network/brand capture automatically.

## Chosen contract before implementation

`deployment-diagram ID TASK --node NODE_ID` shares stored context selection,
depth/direction, with overview defaults/max nodes 12, edges 24; no source reads.
Stdout bounded JSON bundle (default 64KiB, hard 16MiB) containing typed Archify
architecture `diagram` and `evidence_manifest` (the complete selected context),
plus component/connection bindings. The manifest preserves exact endpoints,
kind, mount target, evidence IDs, full project/revision/hash/run and generation.
Snapshot-local n0/e0 IDs map to manifest array indices: no cross-revision stable
identity or collapse claimed. One-to-one projection, no invented boundaries.
All edges dashed/declared, no runtime animation. Cards explain historical state,
omitted facts, owner-wide unknowns, and that dependencies are not network traffic.
Labels may be shortened for layout; full names remain in manifest. No new DB
schema/dependency. CLI produces a bundle, not a rendered/verified artifact.

Tests: real CLI stored fixture, exact mapping/citations and limits, no source
root requirement, tombstone/missing owner failures, actual pinned Archify schema
validation/delivery of fixture and last-good preservation on a bad candidate.
Five-view compiler, collapse/delta and verified clickable Git navigation remain
required future work, not replaced by this deployment architecture projection.

## Implementation and verification

- Added output-only typed protocol IR, CLI `deployment-diagram`, shared context
  report construction and the explicit local `validate-deployment-diagram.mjs`
  integration gate. No external Rust dependency or migration. `coding-guidelines`
  informed separating evidence-card construction from projection; no async/shared
  state added. Missing endpoints propagate errors, never invented connections.
- Luna worker `01a09643-d269-7b80-8d80-4ceaf2c0a2be` requested model
  `gpt-5.6-luna` (effective model not independently exposed by runtime) supplied
  seven real CLI tests. Parent read source receipt and all assertions, requested
  explicit baseline success and absent meta.repository checks, and removal of an
  unrelated absent-field assertion. Parent also added Unicode-boundary unit
  coverage and extended the existing 8MiB task-input test.
  After worker follow-up, parent confirmed those edits and reran the owned suite:
  7 passed, exit 0. Final test SHA-256
  `28b14acb8d41abffaef85dbe91f1dba83ad9cc5c961434b77c96429a166a363c`.
- Foundation: exit 0, **384 Rust passed / 0 failed / 3 ignored; 16 Node passed**.
  Architecture check remains 8 packages / 48 declared dependencies. Rustfmt check
  exit 0; all-target CLI Clippy exit 0 with zero diagnostics scoped to the new
  production modules (not a warning-free workspace claim).
- Actual pinned Archify validate and deliver succeeded on the stored Orders
  projection: 2 of 4 owner nodes, 1 of 2 relationships, 7 owner-wide unknowns.
  Invalid-schema delivery failed and preserved the last-good HTML byte-for-byte.
  Persistent [bundle](../.harness/w11-deployment-diagram-2026-09-12/bundle.json),
  [HTML](../.harness/w11-deployment-diagram-2026-09-12/architecture.html), and
  [integration receipt](../.harness/w11-deployment-diagram-2026-09-12/receipt.json).
- Browser gate initially skipped (Chrome absent from PATH); using cached Chromium
  failed due to missing libasound.so.2. Downloaded distribution libasound2t64
  1.2.15.3-1ubuntu1.1 and extracted ONLY in `/tmp/graph-archify-libs-ySBdUT`, no
  system installation or global configuration. Package SHA-256
  `96e20cf5c9095ef54ba46fccffea5c01cf2de17a1012c5683a67e43504a6d726`.
  Per-process LD_LIBRARY_PATH plus ARCHIFY_CHROME selected existing cached
  Chromium; actual visual-check then passed containment/readability/capture,
  diagnostics empty, 4 desktop viewport checks and 4 light/dark captures.
  [Automated visual receipt](../.harness/w11-deployment-diagram-2026-09-12/architecture.visual-check.json).
- Parent inspected light and dark 1440×900 screenshots of the same HTML hash:
  service/volume names and mounts direction readable, dashed relation visible,
  coverage and historical caveats visible, no apparent clipping/overlap. This
  separate parent review does not rewrite Archify's `visualReview: pending` field
  or imply human acceptance. HTML SHA-256:
  `d4d7f6fc4246aab9aa9b9dbf7e46ba8f1196306843e0b8938a4c1b9ace706ffe`.

Final production/source SHA-256:

- cli/src/deployment_diagram.rs:
  `edccacfbcb3cadf4f236d9a5ffe3344e9f0e058d6e6a731363e2def5eb1cb37d`
- cli/src/deployment_context.rs:
  `6f3d1a24a353df1800275c1ef6abcc1669152558d4d9398410d5fddbfc7cfcd3`
- protocol/src/archify.rs:
  `27e7bafd6140e68286e9ddc566cc1ff76114579fa76065d36ee97d787b45e30b`
- scripts/validate-deployment-diagram.mjs:
  `a244d9b5dc3fd09cd0f76f78315978cccee16019f6b380664fdd9cf12959a8a8`

Remaining acceptance is unchanged: one-to-one Compose neighborhood only; no
cross-provider collapse, graph-aware auto-layout, evidence navigation UI,
revision delta or workflow/sequence/dataflow/lifecycle compiler yet. Limits cap
output/traversal, not full owner reconstruction cost. No source/runtime proof
or permission is derived from a diagram, renderer result or historical manifest.

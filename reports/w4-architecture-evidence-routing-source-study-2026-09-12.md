# W4/W9 architecture evidence routing — Archify source study

Date: 2026-09-12  
Scope: bounded, read-only local study of /home/minh/projects/outsource/archify. No CodeGraph edits, network/clone, or full-suite invocation.  
Archify revision: 18911058008f17dc065af23a2cdc9bfeff6d3f7a (clean worktree).  
Target revision: /home/minh/projects/project-graph-agent has no resolvable HEAD in this checkout (git rev-parse HEAD failed). The target report is the only file written.

## Source-first receipt

Read /home/minh/projects/project-graph-agent/AGENTS.md and PLAN.md section 0.4.1, then the Archify implementation and tests below. Parent full-suite status supplied for context: 5335 pass, 0 fail, 9 skip, 300 files; no suite was run here.

Commands run:

    git -C /home/minh/projects/outsource/archify rev-parse HEAD
    git -C /home/minh/projects/outsource/archify status --short
    node archify/bin/archify.mjs validate architecture archify/examples/production-deployment.architecture.json --json
    node archify/bin/archify.mjs validate dataflow archify/test/fixtures/v1-baseline/product-analytics.dataflow.json --json

Both probes exited successfully. They establish renderer/schema/layout checks only, not that a generated diagram is verified architecture.

## Actual workflow and evidence selection

Archify consumes typed JSON. It does not select symbols from a repository. The architecture schema at archify/schemas/architecture.schema.json:1-198 accepts authored components, optional boundaries/connections, and optional per-component sources containing path, line, end_line, and label. There is no symbol ID, AST query, crawler, index lookup, or source-to-component selection step.

The executable flow is:

1. archify/bin/archify.mjs:112-147 extracts --repo-root and passes ARCHIFY_REPO_ROOT; :424-457 starts the architecture renderer and runs final artifact checks.
2. archify/renderers/shared/cli.mjs:19-38 resolves/parses the JSON, runs schema, guided-view, relationship-ID, and engineering-profile validation, then calls verifyRepositoryEvidence before output resolution. loadDiagramWithBrandMarks at :41-47 only adds optional brand preparation.
3. archify/renderers/shared/repository-evidence.mjs:68-75 enables evidence only for architecture diagrams when meta.repository or component sources exist. :81-159 fail-closed checks full SHA, URL/link mode, local root, Git top-level, matching origin, and commit availability. :160-223 checks every authored repo-relative path, blob, and requested line range from the pinned commit. :225-238 returns a receipt with repository, revision, count, and verified node sources.
4. archify/renderers/architecture/render-architecture.mjs:55-62 loads the already-selected facts. :92-108 measures authored components; :110-130 derives boundary rectangles from authored wraps lists; :908-949 computes connection sides/routes. :1037-1078 renders authored boundaries, connections, components, labels, legend, and evidence payload.
5. validateArchitecture at render-architecture.mjs:336-530 checks IDs, placement, containment, route geometry, and unknown connection endpoints. It does not compare relationships with source code or infer them from evidence.

### No named symbols

With no named symbols, Archify has no fallback symbol-selection behavior. The caller must author component IDs/labels and may omit sources. With no repository metadata/sources, hasRepositoryEvidence at repository-evidence.mjs:68-72 returns false and the source-free artifact path proceeds. With metadata/sources but no valid root, it fails repository-evidence/root-required at :114-119; it does not guess a file, symbol, or line. Unknown IDs in authored boundaries/connections are validation errors at render-architecture.mjs:397-401 and :498-506, not inferred nodes.

## Deployment and data-flow relationships

Deployment ownership is an opt-in semantic profile, not topology discovery. engineering-profiles.mjs:23-145 derives only mechanical facts from authored membership:

- non-external components require owner tag (:43-53);
- each non-external component belongs to exactly one region (:56-76);
- databases belong to a security-group (:80-91);
- each private boundary contains members from one region (:95-117);
- a cross-boundary connection must have a non-empty authored label (:119-141).

production-deployment.architecture.json:32-64 authors 12 components, four boundaries, and 12 connections. HTTPS, mTLS, SQL, publish, and cross-region WAL are authored labels. The crossing calculation follows wraps membership, not geometry or labels; engineering-profile.test.mjs:90-119 tests that distinction.

Data-flow is a separate explicit IR. dataflow.schema.json:1-190 requires stages, nodes, and flows. renderers/dataflow/render-dataflow.mjs:44-49 loads it, :108-116 indexes authored node IDs, :118-180 checks endpoints, :296-363 computes visual routes, and :450-476 writes authored flows. It does not derive data movement from architecture connections, source calls, deployment files, or runtime traces. The local dataflow probe passed with a container-border warning; that is visual composition feedback, not semantic data-flow evidence.

## Local codemap result

No local Archify codemap, symbol index, code-map JSON, or generated symbol manifest was found under the checkout (excluding .git, dependencies, and rendered assets). The map-like files found were experiment/documentation indexes or manifests, not an executable source map; archify.zip had no codemap/symbol-index entries.

The local research note docs/research-repo-evidence-passport-2026-07-23.md:14-42 records the bounded design: revision-pinned component source links and local verification before rendering, while explicitly excluding an AST parser, repository crawler, knowledge graph, or indexer (:23-28).

## Reusable mechanisms for Orders

Keep source selection in the Orders/CodeGraph context compiler and reuse/adapt these Archify seams:

- typed explicit IR with stable node IDs and relationship endpoints; reject unknown endpoints;
- one-to-three repo-relative path/line evidence references per selected node, pinned to a full revision;
- fail-closed local Git root, origin, commit, blob, and line-range verification before publication;
- structured diagnostics containing code, subject, evidence, and supported repair controls;
- explicit boundary membership and cross-boundary mechanism labels, preserving ambiguity as diagnostics;
- evidence metadata outside canonical SVG, via renderers/shared/cli.mjs:53-70.

Repository-evidence tests at repository-evidence.test.mjs:175-208 and :518-567 cover failure/preservation behavior. This is useful for Orders receipts, but it is not a graph extractor.

## Gaps and Orders adaptation boundary

Archify does not provide the missing W4/W9 layer: no symbol-aware selection, CodeGraph query, call/data-flow extraction, manifest/API/IaC/runtime adapter, freshness model, or graph-evidence join. It has no generic unknown-node representation in architecture IR: unknown IDs fail validation, while uncertainty in labels/tags remains caller-authored text. deployment-ownership is a truthfulness gate after the context compiler has produced memberships and crossings.

Orders should therefore pass Archify only a bounded graph-selected IR: selected symbols/files and evidence spans, explicit deployment/data-flow edges, and explicit unresolved records/diagnostics when the graph cannot prove a relationship. Renderer validation must not be presented as verified architecture. Do not replace selection/extraction with keyword-only or label-only fake edges; preserve unknowns. Parent owns CodeGraph query-failure diagnosis, route implementation reading, and full-suite critical path.

## Source-study decision

source-gate: ready for bounded reuse; gap-documented for W4/W9 discovery.

Adopt the evidence passport and fail-closed verification seams. Adapt the authored-IR boundary so Orders supplies symbol- and relationship-backed facts. Do not copy Archify as a repository analyzer, and do not claim generated diagrams are verified architecture.

# W9 deployment-context traversal tests — source review and hash receipt

Date: 2026-09-12
Owned files: `crates/application/tests/deployment_context.rs`, this report
Source gate: complete before test implementation

## Source identity

The product worktree has no committed baseline (`git rev-parse HEAD` has no
commit), so this receipt records SHA-256 fingerprints for the reviewed files.
The reference CodeGraph worktree is dirty at the requested snapshot.

- Reference `/home/minh/projects/outsource/codegraph/src/context/formatter.ts`:
  `91a8ab80fed68e548d5b2aaf64b601ba4352a88ef5d5c13f4c66c1e2de9d4083`
- Reference `/home/minh/projects/outsource/codegraph/src/context/markers.ts`:
  `78914262dea5355a85caa7adf810646dedf1ba0901d09aff934770d097a9077a`
- Reference `/home/minh/projects/outsource/codegraph/__tests__/context.test.ts`:
  `585a0bfc0e3e9115331cde269974f1cc64b53d054c98007a150105906aa10d7e`
- Product `crates/application/src/deployment_context.rs`:
  `d2de37c71665a2a1ef053d60760d06dabf7e93f9b0d3b1c206b590958974cd19`
- Product `crates/domain/src/deployment.rs`:
  `a56013b910b6400ca165d01ca8d6bf9a7f08e1810fb35d94bac7a6ff6f208ca7`
- Product `crates/domain/tests/deployment.rs`:
  `86f75e5e7727709c64e4239ee87d689451cd261730d2ec839d62928311f498e9`

## Read source and test flows

- `formatter.ts:132–168` builds an outgoing adjacency list, tracks printed
  nodes, and walks in stable input order. The application adaptation should
  retain stable graph-edge order while returning input indices, not serialized
  copies or source bodies.
- `markers.ts:1–19` is a dependency-free shared sentinel leaf. It has no
  traversal behavior; no marker or formatting dependency belongs in these
  application tests.
- `__tests__/context.test.ts:372–387` only asserts that a markdown result is a
  string and conditionally notices a truncation marker. It is not evidence for
  exact graph neighborhoods, cap admission, or deterministic traversal.
- `deployment_context.rs:5–55` exposes `Direction`, private validated
  `ContextLimits`, index-based `Selection`, and BFS adjacency for outgoing,
  incoming, or both directions. The tests must prove original edge indices and
  orientation indirectly through graph fixtures, with no access to private
  limits.
- `domain/src/deployment.rs:110–199,236–260` validates node identities,
  endpoint kinds, edge kinds, citations, and exposes immutable node/edge slices.
  Fixtures therefore use `SourceEvidence::new` and `DeploymentGraph::new`,
  reusing one valid source citation and no filesystem/network data.
- `domain/tests/deployment.rs:1–145` demonstrates the existing validated
  `ProjectRef`/citation/node/edge fixture style and the allowed deployment
  shapes: service, volume, network, mounts, depends-on, and attached-to.

## Contract and test decisions before coding

Cover exact neighborhoods for outgoing, incoming, and both traversal on a
chain/branch graph; preserve original edge indices and orientation for reverse
traversal; terminate cycles and self-links without duplicate nodes or edges;
exclude disconnected nodes and edges; reject unknown seeds and every invalid
limit boundary; verify depth-zero and zero-edge seed-only behavior; verify
exact and over node/edge caps plus budget flags; verify depth-frontier and
budget omissions set only the corresponding limitation flag; assert every
returned node/edge index is valid and every returned edge has both endpoints
selected; and call the same selection repeatedly to prove determinism.

Adaptation: the reference uses symbol IDs/maps and markdown formatting, while
the product contract requires validated deployment IDs, bounded BFS, original
graph indices, and explicit omission flags. No reference code is copied, no
new dependency is needed, and the parent-owned production module remains
untouched.

Planned validation: `cargo test -p graph-application --test deployment_context
--locked --offline`, then focused rustfmt/checks for the owned test file only;
do not run whole-workspace formatting.

## Validation receipt

- `./scripts/with-local-tools rustfmt --edition 2024
  crates/application/tests/deployment_context.rs` — exit 0.
- `cargo test -p graph-application --test deployment_context --locked --offline`
  — exit 0; **8 passed / 0 failed**.
- Target test path: `crates/application/tests/deployment_context.rs`.
- No protocol, CLI, production-module, dependency, or whole-workspace-formatting
  changes were made by this test task.

Final SHA-256 before handoff:

- `crates/application/tests/deployment_context.rs`:
  `6cf6b454925585bb00a7280e73e52feb82e6252e0da7804121c1e53d5ab50182`

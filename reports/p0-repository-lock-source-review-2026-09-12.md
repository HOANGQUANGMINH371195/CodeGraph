# P0.T01 repository-lock source review — 2026-09-12

## Task record

```text
status: done | owner: coordinator
scope: /home/minh/projects/outsource top-level repositories + product toolchain/analyzer pins
depends_on: P0.T03, P0.T04, Ripwire source review
source-gate: ready
review: coordinator self-check; manifest validation passed
blocker: license interpretation, dependency-license closure and integration compatibility remain pending
next: P0.T02 CodeGraph smoke and capability receipt
```

## Source-first checklist

- [x] Reset the task gate to `pending` before this source pass; no implementation
      work was started before the source study.
- [x] Read the root manifests and root license files of all 23 repositories;
      the resulting file hashes, declared versions and license observations are
      in [`repo-lock-20260912.json`](../repo-lock-20260912.json).
- [x] Read the product pin sources: `rust-toolchain.toml`, root `Cargo.toml`,
      `scripts/preflight.mjs` and `scripts/with-local-tools`.
- [x] Read CodeGraph `package.json` and `scripts/build-kernel.sh`; checked the
      local CLI and Linux kernel artifacts without treating a dirty build as a
      release artifact.
- [x] Read the Ghostty README/library boundary; no Ghostty ABI is consumed by
      the product, so its pin is explicitly `not-enabled`.
- [x] Chose the manifest schema and validation commands before recording the
      snapshot.

## Three required source-study answers

### 1. What was read?

The live top-level Git repositories under `/home/minh/projects/outsource` were
enumerated and each repository's `HEAD`, branch, porcelain status, root
manifest(s), and root `LICENSE`/`NOTICE`/`COPYING` files were read. The newly
added Ripwire revision is `48222d62f41c6e15f60855127c1d9ee06b3aed4c`.

For the product, the Rust toolchain is declared in
`/home/minh/projects/project-graph-agent/rust-toolchain.toml`; the effective
Node/npm/Git probes are implemented by `scripts/preflight.mjs` and selected by
`scripts/with-local-tools`. CodeGraph's package/build flow is declared in
`codegraph/package.json` and `codegraph/scripts/build-kernel.sh`. Ghostty's
embeddable boundary is documented in `ghostty/README.md`.

### 2. What flow and failure behavior was learned and adopted?

The lock is a provenance input, not a capability grant. A repository is
identified by commit plus branch and its working-tree status is independently
hashed; `clean: false` is retained rather than normalized away. Root manifest
and license hashes are captured as files, while version absence is recorded as
`not-declared`. A dirty analyzer source tree and local binaries are pinned by
both source/status and artifact hash, with `pin_kind` stating that they are not
release-signed. Compatibility and license decisions remain `pending` until
their own source/test review.

This adopts the source-first and snapshot-boundary lessons already recorded for
Ripwire: cached or ranked output cannot replace fresh evidence, and a tool's
presence cannot prove its runtime behavior. It also preserves the existing
OpenSandbox/CodeGraph boundary: availability and source pins do not authorize
execution, graph publication, or sandbox policy.

### 3. What is still product-specific?

No outsource repository provides the exact product release manifest that must
bind 23 heterogeneous repositories, a dirty local analyzer build, the product
Rust/MSRV policy, and an explicitly disabled Ghostty ABI. That small manifest
adapter is therefore product-specific. It is validated as JSON and checked
against live Git/file hashes; it does not copy upstream code or infer
compatibility from README claims.

## Snapshot results

- 23 top-level Git repositories were found.
- 22 repositories were clean; CodeGraph was dirty and its status hash is
  `f85fed5507e19026a2dce4d9ef99b018e7d64c150948104ee14d587a89e96625`.
- Ripwire was included in the new snapshot and is clean at commit
  `48222d62f41c6e15f60855127c1d9ee06b3aed4c`.
- Declared licenses are recorded as observations only. `license_review` and
  `compatibility` are `pending` for every repository.
- Product Rust is pinned to 1.93.1 / edition 2024; local preflight selected
  Node `v24.18.0` and npm `10.9.2` and returned ready, but that probe is only a
  tool-availability check.
- CodeGraph is pinned to source commit `3ed73bc…`, the dirty status hash and
  local Linux-x64 CLI/kernel artifact hashes. No analyzer image is used.
- Ghostty ABI is `not-enabled`; no `libghostty` capability is inferred.

## Validation receipt

Commands run after recording the snapshot:

```text
node -e 'JSON.parse(require("fs").readFileSync("repo-lock-20260912.json"))'
/home/minh/projects/project-graph-agent/scripts/with-local-tools node /home/minh/projects/project-graph-agent/scripts/preflight.mjs
```

Results:

```text
JSON parse: pass; repository_count=23; repositories.length=23
preflight: exit 0; ready=true
Rust: rustc/cargo 1.93.1; Node: v24.18.0; local npm: 10.9.2; Git: 2.53.0
```

Manifest SHA-256 after the final content check:

```text
f2b3c07b5428ae1d5a215351cf0b93af4a5d8c83e14e19eb754fc5e65ba46f56  repo-lock-20260912.json
```

This receipt does not claim P0.T02, P0.T05, P0.T06, or the W0 gate complete.

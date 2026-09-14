# Automatic same-file read discovery — before-code design/source gate

Parent reread current string-call-chain.ts and file-read-binding.ts in full.
Use existing exact callee/receiver gates, never create a second name-based method
resolver. Extract the local call-target derivation for reuse within a single
source snapshot, then build a reverse caller index. Existing receiver hazards,
function identity, mutation and extent checks remain unchanged.

Recognized scope: imported node:fs/fs readFileSync source patterns. Start with
direct literal URL expressions (including top-level); otherwise locate the
exact smallest enclosing function, walk known callers backward, and let the
existing binding/chain evaluation approve each candidate. An unresolved foreign
parameter can require another caller hop; malformed/unsupported source patterns,
missing callers, cycles and depth/state/candidate caps remain explicit issues.
Do not imply absence of other fs APIs or dynamically imported reads.

Cap source256KiB and call inventory, states, chain depth and candidates. Keep
candidate/issue ordering deterministic, preserve every distinct invocation-site
chain (even same target text), and expose incomplete/truncated status. No target
I/O or graph publication in discovery. Later capture rechecks actual source SHA.

Tests: discover fixture-shaped two-hop this.query without caller-provided offsets;
direct top-level literal reads; aliases; duplicate path values at different
calls; unresolved dynamic arguments/no caller; incorrect fs/this bindings;
cycles and exact/over budgets. Existing explicit-locator APIs remain compatible.

No discovery implementation edited yet. Preceding full capture snapshot still
running in41248; source is held unchanged until that receipt completes.

## Resumption source gate — 2026-09-12

The paragraph above is historical pre-code state, not current process status.
Old focused process96671 is now missing; worker Dalton is not found/closed.
Parent reread current discovery, binding, shared method resolver, target capture,
both discovery test files and the product Orders SQL CLI smoke script before
writing the reproducible automatic-discovery probe. Current source SHA256:

- discovery: `e7c3c651bd213eb87fa928f6d893608b30ab203792a8094252cfe84f2f10c7b8`
- binding: `12077ec0e26c45329fa55a421722571360ddf3626ca3e51b6b2307a81c7860a6`
- target: `20afc8cc3a0ea390033ca3bba248c4dc7c09a7a795dba87ab91c9546caffaa0f`
- method resolver: `e46623e6a088f474f0984b7799c0bd7a3ffb4566288e26a75afbf11f16a9ab18`
- chain: `6c5935d11e70d0c660976e5d7eca0a434b0a98fd58bac01da20c7f6c41bf3110`

Adopt: source-only discovery and rederived hash-bound capture; compare authored
ground truth only after discovery, never feed fixture offsets/chains to it.
Reuse smoke receipt exclusive creation and before/after input fingerprints.
Avoid: claiming captured candidates are executed SQL, atomic snapshots or
published graph relationships. Bounded parsing is not a latency guarantee:
binding currently reparses the source for each attempted chain.

Validation so far: process82072 exited0, 65/65 tests across7 focused files
(discovery, binding, capture, chain and boundary suites). No full-suite success
inferred from this check. No Rust changes in this slice.

## Verified result

- TypeScript emit process77638 exited0 after the current discovery, shared
  resolver and invocation-specific parameter-binding changes.
- Native-required focused process33821 exited0: **207 passed/11 files**.
  It includes construction/receiver evidence, argument extraction, substitution,
  chains, read bindings, capture and both discovery test suites. Four workers,
  normal assertions/timeouts. This does not prove the full regression gate.
- `scripts/orders-file-discovery-smoke.mjs` ran against the emitted local
  CodeGraph build: **11 candidates, 13 visited states, 11 captured targets**,
  no issues/truncation; all target paths/hashes match authored Orders truth.
  The probe reads truth only after automatic discovery and target capture;
  no authored caller offsets/chains enter either stage.
- Receipt: `.harness/baselines/orders-file-discovery-20260912-01.json`, SHA256
  `38415b57f548b62b1d009429665ff5af65edd14d72f3c47050f133a1be5a956d`.
  Script SHA256:
  `9acebd52ab03796583be81b679542facfaef348171cd07e6c414fa567249cc02`.
- Reusing the receipt path exited1 and preserved its exact bytes. Missing
  CodeGraph build exited1 with a failed receipt and zero captures:
  `.harness/baselines/orders-file-discovery-missing-20260912-01.json`.
- Dalton's six discovery tests were inspected and independently rerun; its
  worker-created report was relocated to the authorized product directory,
  preserving its content. Its handle is no longer present.

Remaining: reference helper is not yet in the shipped CLI/MCP/indexer. Integrate
discovery and fresh target evidence with SQL analysis and durable graph edges,
with invalidation and bounded context output, before claiming code→query→table
retrieval. Source candidates do not prove runtime execution, whole-program
coverage, atomic snapshots or physical SQL table identity. The probe explicitly
does not assert build/source closure verification. Previous full capture suite
remains **5449 pass/1 fail/9 skip**; this source has no new full-suite receipt.

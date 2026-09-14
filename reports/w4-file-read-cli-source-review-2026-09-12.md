# File-read CLI — before-code source gate

Read current CodeGraph CLI context registration/action, lazy import pattern,
stderr error helper, root resolution and telemetry hook; full CLI route tests,
file-read target capture and discovery implementation. Revision:
`3ed73bc127323e63153bf6ec8354afa82ce36aaf` (dirty checkout).

Pre-edit source SHA256:

- bin/codegraph.ts: `271e125ea147f3cf65fe2d95fb775d9a552cbc23eb1f8ee432005b57769536f6`
- cli-route-context.test.ts: `06060aad2f18f8f013ba32fb966036ab7551aae9abea3ae3bcad30d14b833e90`
- file-read-discovery.ts: `e7c3c651bd213eb87fa928f6d893608b30ab203792a8094252cfe84f2f10c7b8`
- file-read-target.ts: `20afc8cc3a0ea390033ca3bba248c4dc7c09a7a795dba87ab91c9546caffaa0f`

Adopt: actual emitted CLI subprocess tests; strict integer arguments; serialize
complete JSON including LF before output-budget check; bounded/no-symlink
source capture before discovery and hash-bound rederivation for each target.
Avoid: ancestor index root selection for explicit file operations, auto index,
opening a graph DB just to inspect source, forwarding raw filesystem errors or
source bodies. No claim of atomic filesystem snapshot or runtime verification.
Skill domain-cli informs stdout/stderr separation and nonzero failures.

Implementation contract: `codegraph file-reads SOURCE --path ROOT`, default
language javascript, optional typescript; JSON only. Limits: source262144,
target131072 bytes, depth8, states256, candidates128, output65536 including LF;
upper bounds follow existing helpers (output16MiB). Explicit partial results
use status incomplete, unavailable input fails with no stdout. Source is
recaptured after enumeration; drift discards the report. Source hash optional
CLI precondition, when given must match before discovery. No graph write/SQL
execution. Global CLI telemetry/compile-cache behavior remains unchanged.

Parent owns production and library boundary tests. Luna owns only emitted CLI
tests and its own source receipt. No recursive delegation or model fallback.

Before adding library boundary tests, reread file-read-target-boundary.test.ts:
adopt owned temporary roots, platform-gated symlinks and exact-byte assertions;
avoid implying those pathname checks prove race-free containment. New tests
will verify the inspector applies those existing gates to its initial source
read even when discovery finds zero candidates.

Probe integration source gate: reread the complete existing
scripts/orders-file-discovery-smoke.mjs (SHA256
`9acebd52ab03796583be81b679542facfaef348171cd07e6c414fa567249cc02`).
Replace internal helper invocation with emitted file-reads CLI, retaining
exclusive receipts, input hashes and post-discovery truth comparison. Adopt
subprocess timeout/maxBuffer and exact/one-under JSON budget assertions from
the previously read SQL CLI smoke. Avoid keeping a separate duplicated probe
for the same fixture or claiming emitted-build/source closure verification.

Output refinement before editing: first emitted Orders CLI probe passed but
JSON was44161 bytes; inspection shows every binding duplicated in discovery
candidates and captured targets. Retain the canonical binding only in discovery,
and make each target carry candidateIndex. This preserves provenance without
duplicating the entire caller chain. Standalone capture API remains unchanged.

## Verified result

- Current emitted source SHA256: `file-read-target.ts`
  `83892ff2bfa47b50f0be751125c64435bcdb5069c049d8a4ccb2f3e347bcc140`;
  `bin/codegraph.ts`
  `034f534f95c468574567f20c41d701e4a574b865827c18ed80ebdeca6093fe22`.
- TypeScript emit completed after the output refinement. A focused validation
  outside the filesystem sandbox passed **60 tests/7 files**: CLI, inspector,
  target/binding/discovery and boundary suites. The sandboxed equivalent could
  not be used for the CLI tests because nested `spawnSync` returns EPERM; this
  was diagnosed with a direct child-process probe, not hidden by test edits.
- CLI tests verify explicit nested root beats an initialized ancestor, typed
  TypeScript parameters, exact/one-under JSON bytes, source hash failure,
  malformed limits, unavailable targets and the incomplete exit2 result.
- Emitted Orders probe receipt
  `.harness/baselines/orders-file-reads-cli-20260912-02.json` passed: 11
  candidates, 11 captures, 13 states, input hashes unchanged. It confirms
  exact output cap and one-under rejection in the same CLI invocation.
  Receipt SHA256: `7d196befd50875c1e1d6be8bfa34f3f4e83e67d6eeeb078746bf246a19bde4a1`.
  Canonical candidate references reduce output from the earlier 44161 bytes
  to 25136 bytes without dropping a target.

The command remains a read-only development capability: it does not index,
publish graph edges, parse SQL statements, prove runtime import identity,
whole-program coverage, atomic snapshot consistency or race-free containment.
The prior full CodeGraph regression failure remains open and is not replaced
by this focused result.

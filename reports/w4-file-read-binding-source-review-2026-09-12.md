# File-read syntax binding — before-code source gate

Parent reread outsource construction-sites.ts import registration, declaration
shadowing, write invalidation, functionOrigin and argument materialization;
construction-sites tests for import aliases/type-only/shadowing. Revision
3ed73bc127323e63153bf6ec8354afa82ce36aaf. Reuse its lexical import origins and
exact call/constructor extents rather than matching names or abbreviated text.
Also reread current source-bound string-call-chain composition.

Implement a no-I/O adapter for an exact imported node:fs/fs readFileSync call
whose first argument is an exact new URL site and second argument a known UTF-8
encoding. URL must be an import of URL from node:url/url, or the identifier URL
with no lexical binding/shadow or observed direct identifier assignment. The
latter is explicitly an unbound-global assumption, NOT verified builtin
identity; globalThis writes and host monkey-patching are not disproved.

Require a direct import.meta.url base; this is a source expression, not proof
of an unmodified runtime module URL. Compose the existing exact call chain to
the URL's first argument. Preserve source hash, call/new/base/import extents
and uncertainty, no realpath/read/stat/graph write and no file-binding authority.
Later source-root containment and target-content citation are separate gates.

Add constructorReference only in opt-in evidence, default outputs unchanged.
Distinguish absent lexical binding from a shadow/duplicate; direct assignments
to the unbound URL identifier invalidate the assumption. Tests: aliases,
shadowed/fake fs/URL, wrong base/encoding, mismatched extents, spread, direct
global write and no actual source execution. Do not silently infer namespace
imports, require(), arbitrary helpers, buffers or other fs APIs.

## Implementation and verification

Added file-read-binding.ts and opt-in constructorReference metadata. Global
identifier writes are collected once per pass, not scanned per construction.
No target file I/O or publication added; both builtin runtime identity and
runtime import.meta.url value remain explicitly unverified. Direct URL
identifier reassignment blocks the unbound assumption; property/reflection or
host mutation is not disproved. Namespace imports and alternate source shapes
stay unsupported, not guessed.

17 new tests. Final focused CODEGRAPH_KERNEL_EXPECT=1 run: **218 passed/9
files**, terminal exit0 (82342); tsc emit exit0 (26154), git diff --check exit0.
Actual Orders emitted-build probe preserves all11 candidate path strings and
node:fs import origin, with unbound-global URL assumptions explicit. Receipt:
.harness/baselines/orders-file-read-binding-20260912-01.json. It checks the
fixture source hash against authored ground truth; no target file is opened.

Source SHA-256:
- file-read-binding.ts: 8caec57f75cc755e25832526cb8d0c0c013f801a79d90e5bc70996dd2e369523
- construction-sites.ts: c59d2872b6b13941d985d5178d6bde078628f10bd1216d40ecc93774b9cbd505
- file-read-binding.test.ts: 1c2ba874c6e937c5d24f0b75a322116a489e225ce4b40d70b18047706fe32c05

No workers/processes remain live. Full regression for this newer snapshot is
not run; earlier full-suite timing failures remain open. Next prerequisite is
source-root/target containment and captured target hash, then durable SQL graph
facts/context integration. Static builtin observations are not authority to
execute repository code or its SQL.

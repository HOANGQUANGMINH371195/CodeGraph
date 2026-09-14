# W4 strict class export identity

status: done (bounded static class-export gate); source-gate: ready;
owner/reviewer: main, no independent review.

- [x] Reset source gate and reread constructor-join and its six real-index tests,
  import-resolver export indexing/re-export traversal, store-exported-later tests,
  grammar loading and AST shapes of JS/TS export statements.
- [x] Pre-code CodeGraph revision `3ed73bc127323e63153bf6ec8354afa82ce36aaf`, dirty
  fingerprint `cba6432f5af7a38e2ddb1703c0a1307abfbbeaea7ab76caa8ff0ff4873e2ee64`.
- [x] Adopt existing module-path resolver and exact class extent matching. Replace
  the candidate join's use of first-exported-class/default and first-wildcard-hit
  fallbacks with an AST export resolver scoped to this new feature; leave legacy
  general resolution unchanged pending broader compatibility work.

Before-code design: explicit exports precede wildcard exports; wildcard never
forwards default. Traverse all wildcard sources, detect conflicting binding
identities (not just differing class IDs), and allow diamond paths to the same
binding. Resolve named/default class declarations, local export renames, immutable
class aliases and import/re-export chains. Preserve missing, ambiguous, unsupported
and unavailable results; unknown wildcard branches must not be silently ignored.
Use cycle/depth/work limits and per-collection parsed-data caches; delete trees.
Return source evidence for module traversal. Runtime receiver/constructor behavior
and stale graph-edge replacement remain outside this static export-identity gate.

Tests planned over the real CodeGraph index: absent default, misleading first
export, default/local renames, named re-exports, wildcard conflicts/diamond/default
exclusion/explicit precedence, type-only exports, cycles, mutable aliases and
comment/string spoofing. Then rerun constructor suites and unchanged orders probe.

## Delivered

Added `codegraph/src/resolution/class-exports.ts`; wired it into constructor-join
in place of `resolveViaImport`. Existing generic import resolution is unchanged.
The new path reuses module path resolution but validates class export identity
using actual AST statements and indexed class extents. It records per-import
status and source-hash traversal evidence. Only supported unambiguous bindings
yield imported class IDs; missing/unknown/ambiguous paths stay unresolved.

The result now reports `requiresImportValidation: false` for this bounded static
gate; `requiresReceiverValidation: true` remains. This is not module evaluation,
proof that a constructor executes, or proof of a returned object's runtime type.
Caches are collection-local; recursion stops after 64 active entries/256 work
steps and returns unknown. Escaped/string-named exports, unsupported expressions,
unindexed modules and unavailable grammar/source do not acquire guessed targets.
Writes are conservatively checked by name, including nested scopes, so recall
can be lower than a full lexical mutation analysis.

## Verification

- Six source suites: **95 passed**, exit 0 (26 constructor-join, 37 construction
  sites, 21 constructor assignments, 4 this-field, 3 no-fabrication, 4 later-export
  regressions). Commands used product `scripts/with-local-tools node
  node_modules/vitest/vitest.mjs run ...` from CodeGraph.
- `node node_modules/typescript/bin/tsc --noEmit`, then emitted build via `tsc`:
  exit 0. Latest source implementation compiled; final test-only adjustment does
  not change engine output. Full npm/UI/kernel/foundation suites were not rerun.
- `git diff --check`: exit 0.

A newly added positive class-expression test failed because the real index emits
no class node for `const Store=class Store {}; export default Store;`. Kept an
explicit regression proving this index gap and the unresolved result; did not
coerce the const node into a class. **Class-expression extraction/parity is still
outstanding**, not an accepted substitute for the full graph coverage requirement.

`node scripts/orders-graph-smoke.mjs /home/minh/projects/outsource/codegraph .harness/baselines/orders-strict-exports-20260912-01.json`
from product root: exit 0.
[Raw receipt](../.harness/baselines/orders-strict-exports-20260912-01.json).

Unchanged orders corpus retains 10 candidate store-wiring sites (3 main/7 test),
3 unknown endpoint sites and zero unavailable files. Eight project import checks
resolve with AST evidence; external `node:sqlite` DatabaseSync remains unknown.
Source/dist/fixture unchanged during probe; watcher observed sync without errors.
Source fingerprint `508a7a7f4cbad900a39d8b7aa07a23beb85b5ec2ff22a47a33fda3dc051072c8`.

Graph remains 51 nodes/127 edges: no constructor-derived call edges activated.
Next: receiver mutation/escape/constructor-return checks, class-expression
extraction parity, and scoped stale-edge replacement before graph activation.
W0/W4 and full-package acceptance remain open; probe exit codes are not flow
sufficiency or live-agent benchmark scores.

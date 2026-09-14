# W1 Git snapshot authority — source-study receipt — 2026-09-12

## Scope

- Work item: concrete host-side Git/worktree snapshot binding for W1-I.
- Product repo: `/home/minh/projects/project-graph-agent`.
- Reference revisions: CodeGraph `3ed73bc127323e63153bf6ec8354afa82ce36aaf`,
  Ripwire `48222d62f41c6e15f60855127c1d9ee06b3aed4c`.
- Source gate: ready before implementation.

## Source and tests read

| Source | SHA-256 | Lesson |
|---|---|---|
| `codegraph/src/sync/worktree.ts` | `4c4878f2960fecd5934c01810092dcb9457fce84a677f3c8b429a630bedf2d5d` | resolve Git worktree/common-dir at the host boundary; distinguish linked worktrees from nested repositories |
| `codegraph/src/sync/index.ts` | `afeb472b1b20e1ec78fce425440b23b71b00e27e6b56d9f95667045e86c6f506` | source changes trigger bounded re-indexing and freshness is not inferred from a query result |
| `codegraph/__tests__/sync.test.ts` | `8e1c87992878a15fbba60eaf6d1ce0d28fc2b4e9dfae3e543cb74f8f0ccd2b74` | added/modified/untracked/deleted files and clean-state behavior are independently asserted |
| `scripts/source-fingerprint.mjs` | `66c31d18dec05822c30b5e1f1fe447ab55334e5a8e2296495e6e054ffb2f1d4f` | fingerprint tracked + untracked + deleted entries, file bytes, executable bit and symlink target; include revision/status |
| `scripts/source-fingerprint.test.mjs` | `63a00ceafa581141f2286f4d1f06925bc5dcfb3825a43f4f070713cf4ac06eac` | dirty content with identical porcelain status, mode changes, symlink and deletion must change identity |
| `ripwire/src/ingest_cache.h` | `d6f81ac639b407704169d7cdb19c1cc1ac7b48488444da9caa7593ac0ad807be` | cache format/parser/content identity is versioned and stale/corrupt cache self-heals rather than passing silently |
| `ripwire/test/cacheidentitycheck.sh` | `7405a5d97f583b86ee3c3d85a7df7c6a726dfefdc75e96025b3567916b27e846` | changing any cache identity input is a negative case |

## Decisions before patching

- Provide a host adapter `GitSnapshotAuthority` in `graph-source`; the
  application port remains independent of Git and SQLite.
- Resolve the canonical worktree root and Git `HEAD`, then enumerate Git's
  tracked plus non-ignored untracked paths. Hash every regular-file body,
  executable mode, symlink target and deleted tracked entry in a versioned,
  length-delimited digest. Include raw porcelain status only as disclosure and
  an additional identity input; it is never the sole freshness check.
- Bound the total bytes and reject command failure, non-UTF-8 paths, unsupported
  filesystem entries, root mismatch, head mismatch or fingerprint mismatch.
  Never return a partial fingerprint as a successful binding.
- Keep repository/worktree/config/ignore identifiers host-configured and compare
  them exactly; the authority does not invent these values from a caller path.
- Use bounded, non-shell Git invocations and avoid exposing Git stderr or source
  content in errors. The authority proves a point-in-time observation only; the
  source verifier still performs a later file-hash check.

## Planned tests

- clean repository binds and produces a stable digest;
- modified tracked bytes, untracked file, deletion, executable-bit and symlink
  changes are detected;
- wrong root/project/head/fingerprint, unsupported entry, malformed Git output,
  and budget overflow fail closed;
- CLI/application composition can use the binding without relabeling legacy
  caller-supplied output until its full flow is migrated.

## Implementation and verification

The source gate remained scoped to this adapter. `graph-source` owns the
streaming SHA-256 implementation because it hashes host-observed Git/filesystem
state; `graph-domain` remains free of crypto and the application port remains
Git-independent. The architecture allowlist was updated explicitly for this
adapter boundary, while the duplicate dev-only `sha2` declaration was removed.
The resulting manifest has 49 reviewed direct dependencies and the architecture
test includes a positive source-adapter/negative-domain hashing case.

Implemented in `crates/source/src/snapshot.rs`:

- canonical configured/requested root checks and Git common-directory resolution;
- non-shell, bounded Git commands with timeout, stdout cap, child cleanup and
  joined reader threads;
- tracked, deleted and non-ignored untracked path enumeration;
- file bytes, executable bit, symlink target, HEAD and porcelain status in a
  versioned length-delimited fingerprint;
- exact host identity comparison and opaque binding ID generation.

The reader/child race was corrected after the first compile check: a reader that
finishes before `Child::try_wait` no longer loses its bytes, and output/error/
timeout paths all reap the child and join the reader. `git_head` accepts the
currently implemented SHA-1 form only; support for Git SHA-256 repositories is
left as an explicit follow-up rather than silently weakening validation.

`crates/source/tests/snapshot_authority.rs` covers clean stability and exact
binding, tracked content, non-ignored untracked files, deletion, executable
mode, symlink targets, linked worktrees/common-dir resolution, unsupported
index entries, root/project/graph-version/budget mismatch, non-Git roots and
invalid metadata. Malformed Git output and CLI composition remain open tests;
the current implementation still fails closed on invalid Git text and the CLI
has not been relabeled.

Validation:

```text
cargo check -p graph-source --locked --offline          # passed
cargo test -p graph-source --locked --offline           # 19 passed, 0 failed
node scripts/check-architecture.mjs .                  # passed: 8 packages, 49 direct dependencies
node --test scripts/check-architecture.test.mjs         # 12 passed, 0 failed
cargo test --workspace --locked --offline               # passed, 0 failed
bash scripts/validate-foundation.sh                     # passed, current workspace green
```

`cargo fmt --all -- --check` could not run because this environment has no
`cargo-fmt`/`rustfmt` component. No CLI output was relabeled and no semantic
relationship or accepted fact was created. The concrete authority is still not
wired into the CLI, does not prove an atomic filesystem snapshot during
concurrent mutation, does not attest analyzer execution, and does not verify
semantic CodeGraph/Joern relationships.

# W1 CLI snapshot-authority wiring — source-study receipt — 2026-09-12

## Scope

- Work item: opt-in CLI composition of the concrete Git snapshot authority.
- Product repo: `/home/minh/projects/project-graph-agent`.
- Reference revisions: CodeGraph `3ed73bc127323e63153bf6ec8354afa82ce36aaf`,
  Ripwire `48222d62f41c6e15f60855127c1d9ee06b3aed4c`.
- Source gate: ready before implementation.

## Source and tests read

| Reference | SHA-256 | Relevant behavior |
|---|---|---|
| `codegraph/src/sync/worktree.ts` | `4c4878f2960fecd5934c01810092dcb9457fce84a677f3c8b429a630bedf2d5d` | resolve per-worktree root and common Git directory with bounded host probes; linked worktrees must not silently reuse another tree's index |
| `codegraph/__tests__/cli-index-explicit-path.test.ts` | `a2a32f31b6472b3071f8ced44fb397683d5ed6c11039679b909630a9b8f023ca` | explicit path is honored, uninitialized path fails without mutating an ancestor index, and bare CLI resolution is a separate compatibility path |
| `codegraph/__tests__/mcp-require-project-path.test.ts` | `383357691ce734050eab6f3959a92d2ea23a81b30773b5e10edf4f57af5faa4e` | project path is made explicit when no default exists; schema/runtime guidance must not silently select a project |
| `codegraph/__tests__/cli-context-command.test.ts` | `8dd0540bdae8d465d099565af238bf6688612240ef58199e642657dae2107045` | registered CLI command has bounded machine-readable output, clean failures and a documented compatibility shape |
| `ripwire/src/cli.h` | `2fd2a7a9165d981130c2a4e37621dece7834127c138bbce001661ae942d33f8c` | linear argv parser keeps root positional, explicit flags and validation in one command contract; output modes are opt-in and disclosed |
| `ripwire/test/clicheck.sh` | `256ee801dea1f2f2d88cd80b348bd5c09eda5fa5cd835d307af7ce10ab453d6e` | broad argv compatibility matrix catches unknown/invalid flags and requires deterministic nonzero failures |
| `ripwire/test/mcpstrictschemacheck.sh` | `52a6fc0c88d7b97d80b06ea7959db1545882ee5bce5fc11861b71f3abdbc67af` | machine-facing schema/transport must be strict, bounded and explicit about required root/arguments |
| `ripwire/test/argvdiffcheck.sh` | `7e192fec51b84629f5a6a4fe227bb285b982cb98f0e2931446dc3816c5149410` | accepted argv surface is independently enumerated so documentation cannot drift from parser behavior |

## Decisions before patching

- Add an explicit `--snapshot-config PATH` mode to `verify-evidence`; keep the
  existing invocation and `caller_supplied` output unchanged for compatibility.
- Parse the config strictly with schema version, host-selected root, repository/
  worktree/config/ignore identities, total-byte cap and bounded command timeout.
  The task JSON remains a scope claim that must match the authority observation;
  it is never used to construct authority identity.
- Require an exact registered `AnalysisRun` through the existing application
  port before the source reader is called. Output remains source-untrusted and
  relationship-unverified even when the Git identity and content hash pass.
- Keep config parsing in the CLI composition root, Git/process/filesystem logic
  in `graph-source`, and identity/run orchestration in `graph-application`.
  Do not add a daemon, MCP server, cache fallback or model/account routing.
- Reject unknown config fields, root mismatch, invalid limits, missing analysis
  registration and all authority failures without success JSON. Do not expose
  config contents or Git stderr.

## Gap documented

The upstream CLI sources do not provide a trusted host configuration channel or
an analyzer-run ledger. A config file is therefore an explicit host composition
input, not cryptographic authentication; production embedding must supply it
from the host launch boundary and protect its path/contents. Atomic filesystem
snapshotting, analyzer execution attestation and semantic CodeGraph/Joern
verification remain separate work items.

## Planned tests

- strict config parsing and unknown-field/limit rejection;
- successful bound verification after recording the matching analysis run;
- changed Git bytes, root, task snapshot or missing run fail before source read/
  success output;
- legacy `verify-evidence` retains its caller-supplied compatibility marker;
- output remains bounded and does not include raw config or Git diagnostics.

## Implementation and verification

Implemented in the product repo:

- `crates/cli/src/snapshot_config.rs` strictly accepts the eight-field version 1
  host config, rejects unknown/missing fields, relative roots, invalid identity
  text and out-of-range byte/timeout limits;
- `verify-evidence --snapshot-config CONFIG.json` checks the config root against
  the command root, composes `graph_source::GitSnapshotAuthority`, and invokes
  the existing application `verify_source_identity` flow;
- the authority binding and exact registered `AnalysisRun` are required before
  `DirectorySource` is allowed to read source bytes;
- the strict success projection discloses `snapshot_binding=git_authority`, an
  opaque binding ID and the verified run ID, while relationship verification stays
  false; the legacy path remains `caller_supplied` for compatibility.

The subprocess integration test
`crates/cli/tests/evidence_snapshot.rs` passed **1/1**. It covers legacy output,
missing-run rejection, successful Git-bound verification, unknown config fields,
invalid limits/root and stale source bytes with empty stdout on failure. The
workspace command `cargo test --workspace --locked --offline` passed, and
`bash scripts/validate-foundation.sh` passed with 8 packages and 49 reviewed
direct dependency declarations.

This remains an opt-in CLI composition slice, not full W1 or W1-I. It does not
yet provide an atomic concurrent filesystem snapshot, analyzer execution
attestation, semantic CodeGraph/Joern verification, accepted fact/outbox wiring,
Git SHA-256 repository support, or equivalent binding for the other source
commands. The focused rustfmt check for the changed snapshot/config/test files
passes. The workspace-wide format check still reports pre-existing formatting
drift in other application/protocol/execution files, so it is not claimed as a
full formatting gate. A strict scoped Clippy run also remains red on existing
diagnostics in the dependency/package surfaces; it is not claimed as a clean
workspace lint gate.

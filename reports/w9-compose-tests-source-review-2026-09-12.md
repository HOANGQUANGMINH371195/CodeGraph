# W9 Compose sidecar tests — source review

Date: 2026-09-12

## Revision and fingerprint receipt

The product repository is an uncommitted working tree: `git rev-parse HEAD`
fails with “unknown revision”, and `git status --short --branch` reports
`## No commits yet on master` plus untracked files. The source baseline is
therefore recorded by file SHA-256 rather than a commit SHA.

Relevant baseline hashes before this test-only patch:

- `crates/system/Cargo.toml`: `ccefed3bd7f3d2950dce5ac6e4a9757cf31deed353397e121bad83c2f98a2f84`
- `crates/system/src/lib.rs`: `f8217b65c9dc96d7cfa281cf161ef6ab7e1c68d8630413656abb548c508cfb28`
- `crates/system/src/compose.rs`: `e40931fe0f374fdbaa3ae2141d3096e87905fb37c1fb7ed75a15308164bcea76`
- `crates/system/src/yaml.rs`: `25fbfad9fc4aa71f54d59946daa9b50b160d5a03946dbb98c06960a533da7e5f`
- `crates/application/src/source.rs`: `8cb050b18786a160747db6869b25dbdff8c3afc94dade55cadce935ca69596cf`
- `crates/source/tests/verification.rs`: `1767a759b08883980c4f780a224f2417e84841cef0e645bb2ae5ba40d3e5f98`
- `crates/domain/src/evidence.rs`: `1abbd2090921b1b7bd0ac377f3a42ceeb7c5fb189be34bc15a9e5a1ee684768d`
- `fixtures/orders/compose.yaml`: `a2a29afb353a588cc28759f281819f5cfda82fd5044f8fd5e38247959fa37701`

The existing Compose architecture review is
`reports/w9-compose-source-review-2026-09-12.md` (baseline SHA-256
`2d08400392169dc8c69663fc6aa1f8c1467950c8d07987436a7ced8748a7a889`).

## Source and test study

The relevant verification flow is `graph_application::verify_source` in
`crates/application/src/source.rs`: the reader returns the complete file,
the verifier enforces the configured source byte limit, compares the complete
file SHA-256 with `SourceEvidence`, rejects invalid UTF-8/NUL bytes, then
returns only the inclusive cited line range in an owned `SourceSlice`.
`SourceEvidence` in `crates/domain/src/evidence.rs` makes the hash a full-file
citation and includes project/worktree/path/line/run identity.

`crates/source/tests/verification.rs` was reread for the expected failure
semantics: exact CRLF/unicode slicing, edits outside a cited slice causing a
hash mismatch on re-verification, invalid ranges, binary text, output limits,
scope mismatch, symlink escape, and non-regular files. The new tests use a
small in-memory `SourceReader` so they exercise the same verifier boundary
without constructing the private `SourceSlice`.

## Adopted contract and deliberate gaps

The tests target the requested public API:

```text
graph_system::analyze_compose(&SourceSlice)
    -> Result<ComposeProjection, ComposeError>
```

They require public projection nodes for services/volumes/networks, explicit
mount/depends-on/network-attachment edges, stable declaration IDs scoped by
project and Compose path, source evidence carrying actual lines and the full
source hash, bounded unknowns, no runtime claims, no interpolation expansion,
and no environment-secret leakage. The orders fixture acceptance is two
services, two volumes, two mounts, and nonempty unknowns for currently
unmodeled `build`, `environment`, and `ports` declarations.

Error cases cover duplicate keys, malformed YAML, multiple documents,
anchors/aliases, parser depth beyond 64, a source over 1 MiB, partial verified
slices, and secret-safe error/debug formatting. The partial-slice test is
important: verification authenticates the complete source but the adapter
must not mistake a cited subset for a complete Compose document.

No production source, manifest, policy, fixture, or private constructor was
changed. The current production parser API is only a skeleton, so these tests
are intentionally red until the main implementation is ready. The expected
red is an API/implementation readiness gap, not evidence that the contract
has passed.

## Validation receipt

- `cargo test -p graph-system --test compose`: exit `101`; compilation stops
  in the existing `crates/system/src/lib.rs` re-export because the requested
  Compose symbols are not yet defined in the empty `src/compose.rs`.
- `./scripts/with-local-tools rustfmt --edition 2024
  crates/system/tests/compose.rs`: exit `0`.
- `./scripts/with-local-tools cargo fmt --all -- --check`: the new test is
  formatted, but the check reports the pre-existing unformatted production
  re-export in `crates/system/src/lib.rs`; that file was not changed.

Post-format hashes:

- `crates/system/tests/compose.rs`:
  `926be37e8f8ef667fe84d62679d81d3157582262f335d5c1461974c350582376`

Skills read for this task: `rust-router`, `m06-error-handling`, and
`m09-domain`. No delegation, CLI authentication, network access, or runtime
truth was used.

## Production-adapter source gate — 2026-09-12

This is a resumed task, so the source gate was reset and reread before the
implementation patch. The product remains an uncommitted tree with no `HEAD`.
Current hashes before coding are:

- `crates/system/src/compose.rs`: `e40931fe0f374fdbaa3ae2141d3096e87905fb37c1fb7ed75a15308164bcea76`
- `crates/system/src/yaml.rs`: `25fbfad9fc4aa71f54d59946daa9b50b160d5a03946dbb98c06960a533da7e5f`
- `crates/system/tests/compose.rs`: `926be37e8f8ef667fe84d62679d81d3157582262f335d5c1461974c350582376`
- `crates/application/src/source.rs`: `8cb050b18786a160747db6869b25dbdff8c3afc94dade55cadce935ca69596cf`
- `crates/source/tests/verification.rs`: `1767a759b08883980c4f780a224f2417e84841cef0e645bb2ae5ba40d3e5f98`
- `crates/domain/src/evidence.rs`: `1abbd2090921b1b7bd0ac377f3a42ceeb7c5fb189be34bc15a9e5a1ee684768d`

The current tests are the direct acceptance contract: they require a full
hash gate and line-accurate `SourceEvidence`, stable namespaced declaration
IDs, declared-only relationship targets, explicit declaration edge kinds,
unknown diagnostics without secret values, and fail-closed malformed/budget
behavior. `verify_source` is the only source acquisition path; the adapter
must not construct a private `SourceSlice` or perform I/O.

Source references reread:

- Product `crates/application/src/source.rs` (`verify_source`,
  `SourceSlice`, `SourceVerificationError`) and
  `crates/source/tests/verification.rs`: the verifier authenticates the
  complete bytes and returns only an owned cited range; edits outside the
  range are detected on re-verification, so Compose adds its own complete
  text/hash gate before parsing.
- Product `crates/domain/src/evidence.rs` (`SourceEvidence::new` and
  accessors): evidence hashes the complete file, while line ranges are
  inclusive and one-based. Derived declaration evidence must retain project,
  graph, path, hash, and analysis-run identity and only narrow the line range.
- Existing `reports/w9-compose-source-review-2026-09-12.md`: the adapter is
  offline declared-fact extraction, not runtime topology; parser budgets and
  unsupported YAML constructs belong at the marked YAML boundary.
- Outsource `temp-rs-ddd` revision
  `12398c3c6ebbc67998c4ac10a60332f281eeb793`,
  `crates/telemetry/src/trace/exporter.rs` SHA-256
  `ec4f6fef6d1207a583b8e9b82333a11be786ee6d52c9f9f6e7645f3223253fd3`, and
  its `unsupported_protocol_returns_error` test (lines 94–97) were inspected
  after the initial source receipt. The source uses `anyhow::bail!` and echoes
  raw `{other:?}` configuration; the test only checks an unsupported-protocol
  substring. It is not a typed or secret-safe reference. The adapter's static
  `thiserror` reasons and secret-safe formatting are an adaptation of the
  product contract, not copied behavior.

Adopt: typed `thiserror` errors with static reasons and line numbers; an
explicit projection assembled from validated declaration maps; stable IDs
based on project/worktree/path/kind/name rather than line numbers; source
citations derived from the verified evidence; and unknowns for unsupported
fields without retaining their scalar values. Avoid: regex/YAML reparsing,
runtime or environment lookup, interpolation expansion, raw YAML in errors,
best-effort edges to undeclared targets, and pretending unmodeled long-volume
options are mounts. The sibling `yaml.rs` API and policy tests are owned by
the main agent, so this patch will not alter them.

Implementation gap now addressed: `compose.rs` was only a module comment and
the public re-export could not compile. This patch supplies the agreed public
types and adapter against the main agent's `yaml::{Node,Value,parse}` API.

## Handoff timing and state

The initial test-only readiness run was historically red because the public
Compose re-export existed while `compose.rs` was still a module comment. That
historical result is retained and is not a current acceptance claim. The
production adapter was subsequently written, and the parent now owns final
integration of `compose.rs`, its tests, and remaining compiler/clippy fixes.
This report makes no additional final test claim after the parent's latest
handoff edits.

For honest post-source-study identity, the product remained commitless
(`git rev-parse HEAD` had no revision). The implementation-time file hashes
observed before this handoff were:

- `crates/system/src/compose.rs`:
  `93bbd586cc0748d792b522f2d8751d2b7f0e9a2d7152d7a44812c3f24235ae11`
- `crates/system/src/yaml.rs`:
  `205af54e689da2363db193117c8d1e58ab4094a33ebe2fdd342f6e8a835c5f69`
- `crates/system/tests/compose.rs`:
  `926be37e8f8ef667fe84d62679d81d3157582262f335d5c1461974c350582376`

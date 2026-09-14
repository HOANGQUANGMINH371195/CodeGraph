# W9 source-capture tests — source review and hash receipt

Date: 2026-09-12

## Revision and fingerprint receipt

The product repository is an uncommitted working tree: `git rev-parse HEAD`
returns `3ed73bc127323e63153bf6ec8354afa82ce36aaf` for the referenced
outsource repository, while the product repository has no commit yet. The
product baseline was therefore recorded by file SHA-256 where this task adds
tests.

Relevant source-study hashes before the test patch:

- outsource `src/resolution/parameter-reconciliation.ts`:
  `b8a408a83d59b9c88c6c68ddf27ffc580eda32fa82c2c9114dc9698984980259`
- outsource `__tests__/parameter-reconciliation.test.ts`:
  `1e54ec9356db14271cfa9a06ae75615c1939e465f7cc4ddaec895f96b99a003f`
- product `crates/application/src/source.rs`:
  `8cb050b18786a160747db6869b25dbdff8c3afc94dade55cadce935ca69596cf`
- product `crates/domain/src/evidence.rs`:
  `1abbd2090921b1b7bd0ac377f3a42ceeb7c5fb189be34bc15a9e5a1ee684768d`
- product `crates/source/tests/verification.rs`:
  `1767a759b08883980c4f780a224f2417e84841cef0e645bb2ae5ba40d3e5f98`

## Source and test study

The current application source exposes `SourceReader`, `SourceLimits`,
`SourceSlice`, and `verify_source`. The existing verifier calls the reader
once with the source budget, checks the returned length independently, hashes
the complete returned bytes, rejects invalid UTF-8 and NUL bytes, preserves
newline bytes, and treats a final newline as terminating the last line rather
than creating a phantom EOF line. Its existing evidence is historical and
must not be treated as a fresh capture constraint.

`SourceEvidence::new` validates the repository-relative path, lowercase
SHA-256 shape, inclusive one-based line range, and all six `ProjectRef` fields.
The capture tests therefore use the public constructor and an in-memory
instrumented `SourceReader`, without filesystem access or private
`SourceSlice` construction.

The outsource reconciliation implementation records a running/failed/observed
lifecycle, replaces only its owned observations, and rechecks freshness before
publishing. Tests `parameter-reconciliation.test.ts:110–156` specifically
cover malformed source, abort/recovery, and receipt-write rollback. The
adaptation here is the same fail-closed boundary discipline at the byte
capture edge: reader errors propagate, over-budget returned data is rejected
even when a reader lies about the requested limit, and no old locator claim is
verified.

After the parent exposed `capture_source`, its implementation was reread before
the test patch. Current `crates/application/src/source.rs` SHA-256 is
`fcdfca00d73dca0633606bf5ff127d91ecee5ccb6ad37fd566d8bbbcb58f6303`. It
rejects blank analysis runs before reading, checks both returned byte budgets,
hashes and decodes the one returned buffer, rejects empty/NUL/non-UTF-8 input,
counts full-file lines with `split_inclusive`, and derives a length-delimited
SHA-256 identity with the `capture:` prefix from project/version/path/content
hash/run and the full line count. It maps constructed-evidence failures to
`InvalidEvidence(DomainError)`.

## Contract and test decisions before coding

`capture_source(reader, locator, analysis_run, limits)` must read exactly once
with `max_source_bytes`, reject empty/NUL/non-UTF-8/over-budget input, preserve
raw CRLF bytes, count the complete file without a final-newline phantom line,
and reject the complete capture when it exceeds `max_slice_bytes`. It creates
fresh evidence from the returned bytes using the locator’s project, graph
version, and path plus the supplied analysis run. The new content hash,
full-file line range, and deterministic scope+hash+run identity are the
authoritative values; locator ID, old hash, and old range are not constraints.

The test matrix covers:

- one read and the exact bound passed to the reader;
- raw CRLF preservation and final-newline line counting;
- reader-error propagation;
- empty, NUL, and non-UTF-8 rejection;
- a lying reader returning more than the source budget;
- the full-capture slice budget;
- equality for identical content despite changed old locator ID/range/hash;
- ID changes for each of the six project fields, graph version, path, and run;
- fresh hash, full range, project/version/path/run, and changed IDs in the
  returned citation.

No production source, manifest, or existing test was changed by this test task.
The parent owns the `capture_source` implementation; these tests exercise its
public API and the missing-analysis-run case confirms that no reader call is
made before that validation.

Skills read: `rust-router` and `m06-error-handling`. No delegation,
recursive agents, network access, or external plugin was used.

## Validation receipt

- `cargo test -p graph-application --test source_capture --locked --offline`:
  final targeted run exit `0`; **8 passed, 0 failed**.
- `./scripts/with-local-tools rustfmt --edition 2024
  crates/application/tests/source_capture.rs`: exit `0`.
- Focused rustfmt check for the owned test file: exit `0`.

Final test-file SHA-256:

- `crates/application/tests/source_capture.rs`:
  `7aa355fbcb6e5a3e0b498d7130110e568dd5f217871a1ac4e843f5039aa09312`

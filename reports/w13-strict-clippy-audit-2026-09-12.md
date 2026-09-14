# W13.01 strict Clippy baseline

## Parser API follow-up source gate (before patch)

Re-read RTK `src/core/filter.rs` `FilterLevel::from_str`, Display and
`test_filter_level_parsing` at revision
`79347d5e0e20a61002b8dffe190d377846cb1e59`; file SHA-256
`d46b3c32ee0385fb14f8b1e79287e166644b58cbe1c9e618e5534c31979af55e`.
Apache-2.0 inspected; no implementation copied. The trait exposes the enum
through Rust parsing and rejects unknown values. Its case normalization and
input-echoing errors are unsuitable for our exact wire tokens and static errors.
OpenDev search found no corresponding manual FromStr implementation.

Local review: context enums already implement FromStr, but AssertionKind,
EvidenceKind, ClaimState and EventKind only suppress its lint. Add actual
trait implementations forwarding to the existing exact parser. Preserve
source compatibility and test all seven enums through generic string parsing,
every canonical token, unknown/whitespace/case/control-character negatives,
and exact error parity. This verifies decoding only, never authority.
Earlier lint batches lacked a fresh source-study receipt; this is a new
forward-looking gate, not retroactive evidence for them.

### Implemented and verified

Added the four missing trait implementations. Seven integration tests exercise
all canonical tokens for all seven enums through `str::parse`, legacy-parser
parity and noncanonical input rejection. The trait is a decoding convenience;
parsing `accepted` or a capability does not grant authority.

`scripts/with-local-tools sh scripts/validate-foundation.sh` completed with
exit **0** after the patch, including all seven parser tests.
`scripts/with-local-tools cargo fmt --all -- --check` exited **0**.
Full output: [foundation parser integration](validation/foundation-parser-integration-2026-09-12.log).

Correction to earlier conversational claims: the seven local
`allow(clippy::should_implement_trait)` annotations suppressed diagnostics;
they did not independently fix parser design. They remain compatibility
exceptions, with real trait implementations and tests now provided for all
seven types. The earlier 296 count included the final compiler summary line;
it was not 296 independent lint defects. No strict-lint gate is marked passed.

Command: `scripts/with-local-tools cargo clippy --workspace --all-targets
--locked --offline -- -D warnings`.

Observed process exit: **101**. Captured output is retained in
[validation/clippy-2026-09-12.log](validation/clippy-2026-09-12.log).
No lint levels were relaxed and no source was changed during this run.

The run stopped at graph-domain: the compiler reports 304 errors for its
library and 404 for its library test target. These are overlapping target
diagnostics, not 708 independent defects. Other workspace crates are not
covered by a successful strict check.

Diagnostic links in the captured output classify as follows (counts are
occurrences in this output, not deduplicated defects):

| Lint | Occurrences |
| --- | ---: |
| must_use_candidate | 223 |
| unwrap_used | 90 |
| missing_errors_doc | 59 |
| should_implement_trait | 7 |
| doc_markdown | 5 |
| manual_string_new | 5 |
| collapsible_if | 4 |
| semicolon_if_nothing_returned | 3 |
| expect_used | 3 |
| struct_field_names | 1 |
| ptr_arg | 1 |
| nonminimal_bool | 1 |
| unused_self | 1 |
| trivially_copy_pass_by_ref | 1 |

The follow-up inspection of `AnalysisRun` removed its constructor error-doc,
accessor `must_use`, and domain-level `doc_markdown` diagnostics without
altering its validation or registration contract. A targeted domain test run
remained green. The remaining lint output still includes the wider domain
surface and must be rerun from a clean command after each bounded batch.

Next implementation pass should distinguish public API documentation and
return-value-use contracts from panic assertions in tests. Review each
contract against its implementation and source-study gate; do not mechanically
replace panic assertions or suppress the workspace lint policy. Then rerun
the full command to reveal diagnostics in later crates. Current foundation
test success does not satisfy this gate. W13.01 and the PLAN strict-lint item
remain open; portable CI and clean-machine installation remain separate.

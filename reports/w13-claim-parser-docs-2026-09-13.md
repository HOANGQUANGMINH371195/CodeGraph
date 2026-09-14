# ClaimState parser documentation

Pre-patch source review: read fact.rs::ClaimState as_str/from_str and
tests/wire_parsing.rs::claims, including exact canonical spellings and refusal
of case changes, whitespace, NUL, unknown and empty input. Read ICM
context_snapshot.rs forced-entry regression (lines 437–467), at previously
recorded revision 2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42, Apache-2.0.
Adopt documenting actual exceptions rather than implying stronger guarantees;
this is a documentation review reference, not a claim-parser implementation.
No upstream code copied. Rust-router and coding-guidelines apply as already read.

Decision before patch: document exact case-sensitive parsing, no normalization,
and no authorization implied by parsing Accepted. Mark the pure string
conversion must_use. No wire or behavior change; existing parser tests suffice.
Validate foundation and fmt; W13 remains open.

Result: foundation and fmt check both exited 0. Foundation log:
`validation/foundation-claim-parser-docs-2026-09-13.log`. Reviewed final diff:
only ClaimState documentation/attribute and plan/report changes. No new parsing
behavior or acceptance authority. Strict Clippy not rerun for a new count.

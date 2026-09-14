# W1 submitted content verification — source-first receipt

Source gate: ready, recorded before implementation.

- Re-read `outsource/grit/src/git/mod.rs:185–220` and test
  `merge_worktree_refuses_dirty_main_tree_and_preserves_branch:589–622`.
  HEAD `0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`, clean worktree.
  Grit checks tracked target changes before merge and preserves the candidate
  on failure. Test assertions were inspected, not executed.
- Adopt precondition checks and preserve submissions on verification failure.
  Do not adopt Git merge success as harness acceptance authority; no code copied.
- Existing harness artifact verifier reads bounded bytes and verifies SHA-256;
  submission query observes one ledger snapshot. Neither binds the two today.
  Gap adaptation: bind the complete expected TaskSpec and registered artifact
  identity/snapshot, verify actual content, then re-query the submission to detect
  cancellation or replacement during I/O. This is a content observation only;
  commit-time atomic revalidation, checks and target-head verification remain open.
- Tests planned: real SQLite submission + artifact registration; success without
  ledger writes; missing/mismatched scope; byte budget/hash failure; cancellation
  during artifact read. No transport, auth or integration endpoint added.
- Skills: rust-router, m05-type-driven, m04-zero-cost. Private result fields;
  reuse repository/reader ports without adding a framework or dependency.

## Implementation and validation

- Added application `submission.rs` and three store integration tests using real
  SQLite and a controlled ArtifactReader (not a filesystem integration test).
  Success emits no events; missing artifact, changed contract, budget overflow,
  hash mismatch and cancellation via a second SQLite connection are covered.
- Initial test placement in CLI exposed its deliberate lack of a domain
  dependency; moved tests to store instead of weakening dependency policy.
  Corrected fixture to existing `Store::open(":memory:")` API.
- Foundation validation passed: 95 Rust tests + 11 Node tests, 6 crates and
  30 direct dependencies. No external reference tests or live agents executed.
- Remaining: adversarial repository substitution coverage, semantic artifact
  parsing/check evidence, target-head binding and atomic integration authority.
  A caller-supplied reader/root and expected scope are not authentication.

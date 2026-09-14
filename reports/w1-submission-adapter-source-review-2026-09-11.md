# W1 submission adapter boundary tests

Source gate: ready, before test implementation.

Re-read Grit `src/git/mod.rs:185–220,589–625`, clean revision
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`. Its merge guard refuses dirty
tracked targets; regression asserts error, preserved agent commit and non-bare
repository. Assertions inspected only, not executed. Adopt refusal on invalid
preconditions and candidate preservation; no upstream code copied.

Local read: application `submission.rs`, store `tests/submission_content.rs`,
protocol `artifact.rs`. Gap: current tests exercise real SQLite but cannot make
the adapter violate filters or replace individual submission binding fields.
Add a scripted test-only repository to return substituted descriptors and
changed observations. Explicitly distinguish this from production SQLite races.

Planned cases: each artifact identity/snapshot field mismatch rejected before
reader open; invalid fence/sequence/time/reference rejected before artifact
lookup; each candidate binding change detected by second observation; repository
errors propagate without success. Keep actual-store cancellation test intact.
No new dependency, integration authority or source changes planned.
Skills: rust-router and m05-type-driven; validated result remains private.

## Results

Four tests added to `crates/store/tests/submission_content.rs`: eight descriptor
substitution cases, four invalid bindings, six second-observation changes and
repository recheck failure. Existing real SQLite cancellation test retained.
All 99 Rust + 11 Node tests pass; 6 crates / 30 direct dependencies unchanged.
No production code adjustment was needed for these cases. These tests establish
defensive checks against individual inconsistent adapter responses, not security
against a consistently lying trusted adapter or post-return races.
Next W1 work remains semantic check evidence, target binding and atomic commit.

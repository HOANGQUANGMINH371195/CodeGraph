# Lint evidence reconciliation

Prior accessor/doc batches after the four-constructor receipt did not consistently
record source study before patching or run the required foundation gate. This
receipt records a current review, not retroactive compliance. Domain tests alone
did not satisfy that gate. The prior statement that ExecutionPlan changes
"enforce" a contract overstated documentation/attribute changes.

Current pre-patch review: ScopeSelector::new validates individual paths/node IDs
and rejects duplicate selectors in addition to its documented errors. Read its
full validation loop and graph-version validation. Revisited ICM
context_snapshot.rs regression for forced-entry budget overflow at revision
2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42 (Apache-2.0 previously reviewed).
Adopt matching documentation to all actual branches; no code copied.

Next patch: correct the missing scope error cases and the PLAN enforcement
claim. No constructor behavior changes. Run foundation, fmt check and capture
an unfiltered strict domain Clippy result with its real exit status.

Results: foundation exited 0, fmt check exited 0. Strict domain library Clippy
exited 101 with 199 remaining diagnostics. Logs are
`validation/foundation-lint-reconciliation-2026-09-13.log` and
`validation/clippy-lint-reconciliation-2026-09-13.log`. Foundation was launched
before the documentation-only correction; it covers the preceding code edits.
No full W13 or workspace strict lint claim. PLAN copies compare equal.

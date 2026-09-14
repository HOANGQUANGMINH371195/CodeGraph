# Constructor error documentation source gate

Before editing: reread the complete product validation paths in
`artifact.rs::Artifact::new`, `check_run.rs::CheckRunBinding::new`,
`command.rs::validate_descriptor`, and `checks.rs::RequiredChecks::new`.
Scope: document their current errors, without changing validation or claiming
execution/host authority. Apply rust-router and coding-guidelines.

Outsource reference: ICM `crates/icm-core/src/context_snapshot.rs`, especially
`over_budget_flag_set_when_single_forced_entry_alone_exceeds_budget`: its
regression explicitly identifies a documentation guarantee contradicted by
the forced-first-entry path. Adopt branch-by-branch documentation review;
avoid describing intended guarantees as implemented behavior. This is a
documentation-method reference, not an implementation of these constructors.
No code copied. ICM license is Apache-2.0; revision
`2ac87e8fc6c6fd0b5a6dc6d910446d95c65d3f42`.

Plan: add precise Errors sections for four constructors; preserve distinction
between syntax validation and runtime verification. Existing tests and required
foundation/fmt checks validate no behavior change. Strict Clippy remains open.

Validation: foundation exited 0 (log
`validation/foundation-constructor-docs-2026-09-13.log`); fmt check passed.
Domain strict Clippy exited 101 with 255 remaining diagnostics, logged in
`validation/clippy-constructor-docs-2026-09-13.log`. No strict gate completion
is claimed. Four constructor Errors sections now match the inspected branches.

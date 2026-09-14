# Project Graph Harness

## Architecture and delegation

The current product target is one Codex account with native subagents. Per the
user's 2026-09-12 instruction, explicitly select `gpt-5.6-luna` for new workers;
the root keeps its current model for coordination and final verification.
Existing workers may finish their assigned tasks without restarting. Check
runtime availability and record the effective model; if Luna is unavailable,
report it and let the root handle or queue the task, with no silent fallback.
Escalate difficult work with evidence to the root; children must not upgrade
their own model or permissions. Measure whole-fleet tokens, retries and quality
before claiming savings. Do not introduce cross-account routing, credential
copying, shadow homes or account rotation. Do not change global
Codex configuration or authentication. Native Codex owns agent lifecycle;
the harness owns domain jobs, graph context, admission and integration checks.

The project author authorizes proactive native subagent delegation for work
within an explicitly requested project task. Delegate independent, bounded work
when useful; do not spawn merely because parallelism is available. This does
not authorize unrelated external actions, live-account experiments or changes
outside the requested scope.

- Keep the immediate critical path on the parent; delegate disjoint side tasks.
- Use at most four concurrent children initially, within runtime limits. Do not
  allow recursive delegation unless the parent assigns it with an explicit
  remaining shared budget and scope. No child may expand permissions.
- Give each child a concrete deliverable, repository/snapshot, read/write scope,
  validation requirements and reporting format. Prefer scoped context over a
  full history fork. Reuse a child for closely related follow-up work.
- Shared-workspace edits require disjoint write sets. A native thread fork is
  not filesystem isolation. Use owned worktrees when isolation is required.
- Parent reviews evidence and integrates results. An agent's completion text
  does not prove tests, process termination, semantic correctness or graph facts.
- Preserve unresolved gaps and source references. Do not broadcast transcripts
  or entire long files when symbol/range evidence is enough.
- Track live handles and reconcile before retrying; timeout is not proof that
  an agent/process died. Close unneeded completed children after integration.

## Implementation and validation

Before each implementation task, return to the relevant repositories in
`/home/minh/projects/outsource` and read actual source flows, failure/recovery
paths and test assertions. Prior audits are navigation aids, not substitutes for
current source. Record revision/fingerprint, file/symbol references, what to
adopt/avoid and adaptations in the task receipt BEFORE coding. Follow PLAN.md
section 0.4.1. Apply this to subagents and review their source receipts before
integration. If no suitable reference exists, inspect outsource alternatives and
document the gap and proposed design first. Do not invent attribution, tick
source-study retroactively, copy without license review, or modify references
merely to study them. Revisit code lacking source-study when continuing that area.
Reading relevant skills remains mandatory.

Preserve user edits and reference repositories. Use apply_patch for source edits.
SQL belongs in migration/query files; do not rewrite applied migrations. Domain
code must not depend on filesystem, network or SQLite. Do not promote historical
metadata into a fresh verification capability.

Run `sh scripts/validate-foundation.sh` and
`scripts/with-local-tools cargo fmt --all -- --check` for foundation changes.
These gates do not prove W0–W13 complete. Keep unchecked requirements open until
their actual acceptance evidence exists. Synchronize this repository's PLAN.md
with `/home/minh/projects/outsource/PLAN.md`; task IDs remain stable through plan
revisions. Update source-backed reports without overstating coverage or tests.

# P1.T08 context pack — source gate and implementation receipt

status: done (scoped) | owner: coordinator | source-gate: ready
scope: `crates/domain/src/context_pack.rs`, `crates/protocol/src/context_pack.rs`
depends_on: P1.T01–P1.T07
review: complete | blocker: none | next: P1.T09 output limits and P1.T10 independent schema/golden gate

## Source-first checklist

- [x] Reset P1.T08 source-gate to `pending` before this source pass.
- [x] Select sources by mechanism: bounded context retrieval, per-agent scope,
      capability restrictions, worker handoff and claim disclosure.
- [x] Read the relevant source and tests in Ripwire, context-mode, Orca and
      OpenDev before implementation.
- [x] Record the main flow, failure branches, adopted mechanisms and gaps.
- [x] Decide implementation and regression tests; the gate is no longer pending.

## Source study and decisions

### Ripwire — structure-first packs and honest handoff

At commit `48222d62f41c6e15f60855127c1d9ee06b3aed4c`, read `src/packtask.h`,
`src/handoff.h`, `src/serialize.h`, `src/graph.h` and the relevant
`test/callformcheck.sh`, `test/declinecheck.sh`, `test/cacheidentitycheck.sh`,
`test/handoffcheck.sh`, `test/partitioncheck.sh`, `test/planlanescheck.sh` and
budget/disclosure checks. Ripwire assembles task-oriented context in one
terminal response, pre-prices the serialized payload, discloses truncation and
keeps `verified` disk facts separate from heuristic co-change/docs/notes. Its
name-based graph, ranks, partition and handoff suggestions remain incomplete
projections, so ambiguous/unresolved/declined results are surfaced rather than
promoted.

Adopt operation-oriented packs, a disclosed budget/omission record, stable
snapshot identity and separate verified/heuristic handoff sections. Avoid
making ranking, partition, cache hit, notes or raw tool text an authority or a
capability grant. The product contract is typed JSON, so it will not copy
Ripwire's XML/CDATA emitter or embed its process.

### context-mode — bounded retrieval and per-agent isolation

At commit `ad7ef27106ee9ebbe9a75d0b75106c9187506e09`, read
`src/search/unified.ts`, `src/search/flood-guard.ts`, `src/session/snapshot.ts`,
`src/truncate.ts`, plus `tests/core/search-flood-guard.test.ts`,
`tests/core/search-project-filter.test.ts` and snapshot pipeline tests. The
search path resolves a project allow-set once, merges only the requested
sources, and tolerates one source failing with partial results. The flood guard
is keyed by agent-context, preserving single-actor protection without starving
parallel agents. Resume snapshots are a table of contents with runnable
references; XML escaping and explicit truncation protect the transport.

Adopt project/worktree/graph snapshot scope, per-agent subject identity,
reference-based handoff/context retrieval and explicit omissions/unknowns.
Adjust the product to fail closed on scope mismatch instead of returning a
cross-project legacy surface, and make all grant/claim text data-only.

### Orca — capability preflight and parent-run binding

At commit `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`, read
`src/main/runtime/rpc/methods/orchestration/worker/workers.ts`,
`worker-start-schema.ts`, `runs/dispatch-methods.ts`,
`src/cli/handlers/orchestration/worker-launch-handler.ts`,
`message-inbox-handlers.ts` and their worker-start/dispatch/messaging tests.
Orca validates worker input before effects, binds a worker to the current run,
checks runtime capability support before honoring model/effort preferences,
keeps residual resources observable, and defaults inbox output to a compact
header with full body only by explicit request.

Adopt explicit subject/parent/run binding, preflighted capability names,
compact ID-based handoffs and residual/unknown disclosure. Do not treat Orca's
runtime capability list or dispatch receipt as graph evidence, and do not copy
its UI/runtime-specific terminal fields into the domain pack.

### OpenDev — restricted child context

At commit `d32c660e4eed1a8e988d1fd58da88e41ba641d08`, read
`crates/opendev-agents/src/subagents/spec/types.rs`, `permissions.rs`,
`subagents/manager/spawn.rs` and their type/permission tests. A child gets a
separate task, restricted tools, optional worktree isolation and inherited
model/context defaults; disabled agents and denied tools are rejected before
the ReAct loop. The source also shows why task descriptions and tool output
need bounded handling rather than a full transcript fork.

Adopt monotonic, explicit capability grants and subject-scoped context. Adjust
model policy to the product decision: one root account with Luna workers;
model selection is runtime admission, not a context-pack permission. The
current task defines the contract only; native spawn and enforcement remain
P9 work.

## Contract before implementation

- `ScopeSelector` binds project, graph version, repository-relative path
  prefixes, graph node IDs and traversal depth. It cannot be empty or contain
  duplicate/out-of-tree selectors.
- `CapabilityGrant` binds issuer, subject, role, exact scope and an explicit
  time window. It is a host-issued claim; serializing it never authenticates,
  grants a lease, permits writes or escalates a child.
- `ClaimBundle` has separate `verified` and `heuristic` candidate sections,
  explicit unknowns and no accepted facts. Every claim remains a candidate and
  must later go through `GraphWriter`/verifier.
- `Handoff` binds sender/recipient, project, graph version, scope and the claim
  bundle. It carries references/IDs and bounded data, not raw transcript; it
  cannot widen the grant or change WorkerTree/lease/capability state.
- `ContextPack` binds one `ContextEnvelope`, selector, grant, optional handoff
  and a mandatory `untrusted_content` label. Envelope/selector/grant/handoff
  project and graph scopes must be identical.

## Implementation and test plan

Add pure domain value objects for scope, capability grant, claims and handoff;
add strict protocol DTOs and a `ContextPack::validate` boundary. Tests cover
round trips, duplicate/out-of-scope selectors, expired grants, capability
duplicates, candidate-only claims, mixed snapshot rejection, handoff scope
mismatch, grant escalation attempts, untrusted-label enforcement, unknown-field
rejection and deterministic field ordering through repeated serialization.

P1.T09 remains responsible for the final serialized output/token/character
budget and deterministic context selection. P1.T10 remains responsible for
independent JSON Schema and golden fixtures; Rust validation here is not that
gate.

## Implementation receipt

- [x] Added the pure domain model in `crates/domain/src/context_pack.rs` for
  scope, capability grants, candidate claim bundles, handoffs and mandatory
  untrusted-content labels.
- [x] Added strict protocol DTOs in
  `crates/protocol/src/context_pack.rs` under
  `project-graph/context-pack/v1`, with `deny_unknown_fields`, bounded
  extensions, nested conversion and snapshot binding to `ContextEnvelope`.
- [x] Rejected future pack schemas, mixed project/graph snapshots, duplicate
  capabilities, accepted claims, invalid origins, widened handoffs/grants and
  missing/unknown fields.
- [x] Added deterministic serialization, round-trip and negative tests.
- [x] Verification passed:
  `cargo test -p graph-domain --locked --offline`,
  `cargo test -p graph-protocol --locked --offline`, and
  `cargo test --workspace --locked --offline`.
- [ ] `cargo fmt` could not run because this environment does not provide the
  `rustfmt` component; formatting was reviewed manually. This is an environment
  limitation, not an implementation acceptance claim.

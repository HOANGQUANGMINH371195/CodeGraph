# P1 acceptance closure — source gate and verification receipt

```text
status: done (scoped) | owner: coordinator
scope: ContextEnvelope/ContextPack protocol, schemas and independent golden gate
depends_on: P1.T01–P1.T10
source-gate: ready
review: coordinator self-review; Rust, independent schema and foundation gates passed
next: W1-I joint verification, then P2/P4 context compiler integration
```

## Source-first study

The source gate was reset to `pending` before this task. The following current
source and tests were read by mechanism, not by repository name:

- CodeGraph `src/context/index.ts` and `src/context/formatter.ts`, plus
  `__tests__/explore-output-budget.test.ts`: ranked search and graph expansion
  are bounded projections; JSON field order and formatter output are not
  snapshot/evidence authority.
- Ripwire commit `48222d62f41c6e15f60855127c1d9ee06b3aed4c`,
  `src/packtask.h`, `src/handoff.h`, `src/serialize.h` and budget,
  disclosure, cache-identity and handoff tests: task packs disclose omission
  and keep verified facts separate from heuristic suggestions.
- context-mode commit `ad7ef27106ee9ebbe9a75d0b75106c9187506e09`,
  `src/search/unified.ts`, `src/search/flood-guard.ts` and project-filter/
  flood-guard tests: resolve the project allow-set once, isolate limits per
  actor and degrade one source to partial results.
- OpenDev commit `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
  `opendev-agents/src/subagents/spec/{types,permissions}.rs` and permission
  tests: validate child scope/capabilities before effects.
- Orca commit `26f9fd8ea152ad6126c005e5ae602c7d201f4a99`, worker-start/dispatch
  runtime handlers and tests: bind workers to a run and preflight capabilities;
  a dispatch result is not graph truth.

Relevant source fingerprints:

```text
codegraph/src/context/index.ts       028cbe205d076f0e39641d5f5a57ba03f9bfc7354c5442a803e54d44f4bacc4f
codegraph/src/context/formatter.ts   91a8ab80fed68e548d5b2aaf64b601ba4352a88ef5d5c13f4c66c1e2de9d4083
ripwire/src/packtask.h               0c9cd7ed9e1096467c38f890707e9de33aee91f276eabeff708357f059f7ea31
ripwire/src/handoff.h                 cc216f5272b883f372043d053efb94f8afe48b3d5dda4874a9f209502c6d04f7
context-mode/src/search/unified.ts   ff04bf05515bc62952d4eede4b025ad8badb393a97cd67266257f55681cebbe7
context-mode/src/search/flood-guard.ts c646dabf80040a619073f06765dd19670f24c392709e804a257e1dc25b3a85d2
```

## Decisions before patching

- Adopt explicit omission/unknown states and actor/snapshot scope checks.
- Adjust the reference designs into a strict JSON contract: a resolved item
  must carry evidence; an item without evidence may only be partial, unresolved
  or stale. This leaves room for future runtime/document evidence adapters
  without presenting an unsupported resolved edge.
- Add a canonical Rust serialization path that sorts set-like collections and
  evidence references. The ordinary derived serializer remains available for
  round trips; producers that need byte identity use `canonical_json`.
- Keep candidate claims outside the normal context envelope. ContextPack
  validates that both `verified` and `heuristic` worker sections remain
  `candidate` with no decision; GraphWriter remains the only acceptance path.
- Avoid copying upstream XML, ranking scores, cache hits, dispatch results or
  model hints into graph authority.

## Implementation

- `ContextEnvelope::canonical_json` validates first, normalizes node/edge/slice/
  evidence/unknown ordering and emits deterministic JSON bytes.
- Rust protocol validation rejects resolved nodes/edges without evidence while
  preserving explicit unresolved/partial/stale projections.
- `context-envelope.schema.json` mirrors the resolved-evidence invariant.
- `validate-context-schemas.py` now runs semantic checks for schema-valid
  `invalid-*` fixtures and checks nested envelope/candidate semantics for packs.
- Added `fixtures/context-envelope/invalid-resolved-edge-without-evidence.json`
  as an independent negative fixture.

## Verification

```text
python3 scripts/validate-context-schemas.py
validated 5 context schema fixtures and 2 schemas

cargo test -p graph-protocol --locked --offline
80 unit tests + 3 integration tests passed; 0 failed

bash scripts/validate-foundation.sh
architecture/preflight, fixture and full workspace Cargo checks passed
```

The full product remains below W1/W0 package completion: this receipt closes
the six P1 context acceptance conditions only. It does not claim snapshot
authority integration, AnalysisRun semantic verification, ContextGateway
runtime wiring, W1-I, or W4 output compilation.

# W1 — immutable AnalysisRun registration

Added a validated domain descriptor, versioned JSON contract, application
repository port and immutable SQLite registration table through migration V4.
V1–V3 were not edited. Production SQL remains in separate files.

The descriptor binds id, full ProjectRef, GraphVersion, analyzer identity and
version, configuration SHA-256 and input-manifest SHA-256. Digests require
lowercase 64-character hex; they are not proof of possession of those inputs.
Registration is idempotent for identical content and rejects ID conflicts.
Update/delete triggers prevent accidental mutation. Scoped lookup validates
the decoded descriptor against the selected key/project/version as well.

`AnalysisRepository::source_analysis_run` looks up registration using the
provided citation's run ID and exact snapshot. It does not attest that the
citation was persisted, the analyzer executed, inputs matched their declared
digests, or any relationship follows from the source. Missing or differently
scoped registration returns None. Legacy unregistered citations remain
readable and unverified; migration never fabricates their missing runs.

## Validation

`sh scripts/validate-foundation.sh` passed: 31 Rust tests and 10 Node tests,
0 failures/ignored tests. Three new cases cover:

- immutable/idempotent replay and conflict, reopen, every ProjectRef component,
  graph-version/missing-ID isolation, citation registration lookup and no
  automatic task/acceptance events;
- actual V3 → V4 upgrade preserving citations without making up runs;
- JSON roundtrip, future schema rejection, blank analyzer and invalid hashes.

Both migration-count assertions were advanced from three to four versions to
match the newly applied migration. Format and diff checks passed. No dependency
policy changes were needed. Rust/domain skills informed the distinction
between immutable registration and trusted execution evidence.

## CLI registration and lookup

Added `record-analysis-run RUN.json` and `analysis-run ID TASK.json`.
Both output versioned JSON and explicitly report `execution_verified: false`;
lookup wraps the descriptor in `run`, or null for missing/different scope.
Caller-supplied task scope is not authentication or host snapshot attestation.
Registration replay reports inserted=false, while conflicting content fails
with a nonzero exit, stderr diagnostic and no success JSON.

Two CLI subprocess tests cover persisted roundtrip across processes, replay,
ID conflict, all six project scope fields, graph version and unknown ID;
future schema, unknown fields, blank analyzer and malformed digests; source
verification remaining untrusted even with matching registration; and help
not creating a database. No new async runtime or dependencies were introduced.
The CLI skill informed stdout/stderr separation and explicit help semantics.

After this addition, `sh scripts/validate-foundation.sh` passed **33 Rust tests
and 10 Node tests**, zero failures/ignored. The direct-dependency architecture
gate passed for six packages and 29 declarations. This does not establish
execution verification, platform parity or completed W1 acceptance.

## Remaining work

This is the registration portion of AnalysisRun, not a completed execution
receipt/lifecycle. Scheduler-owned run creation,
artifact/attempt linkage, input-manifest verification, run status/outcomes,
snapshot authority and semantic verifier decisions remain to be implemented.
`verify-evidence` continues to emit analysis_run_verified=false and does not
accept a fact. No graph/outbox projection is emitted merely by registering a
descriptor. P1.T07 and W1 remain open.

# Project Graph Agent

Rust terminal coding harness implementing [PLAN.md](PLAN.md).

W1-C development contract gate has passed its
[bounded audit](reports/w1-c-development-gate-audit-2026-09-11.md). W2 now has an
initial owned Linux process-fixture adapter. Full W1/W2 acceptance and native dispatch remain
gated; no full product package is complete.

## Implemented foundation

- `RpcTerminalReceiptRepository` stores one immutable terminal report per launch
  in V13, with exact registered spawn/run/artifact metadata and atomic event/outbox.
  Reads validate linkage in one transaction; identical replay is a no-op.
  Domain and wire retain uncertain RPCs even after exit zero. Supervisor terminal
  accounting exists. Trusted fixture `launch_bound` preserves its exact spawn
  through an opaque supervisor/terminal pair; legacy `launch` discards that binding.
  Bound terminal `prepare_receipt` derives output hashes and freezes descriptors;
  `publish` ingests/readback-verifies CAS bytes before recording the receipt.
  Staged failures retain the prepared report for retry; this is not an atomic
  filesystem/database transaction. Production verified identity, containment
  and crash recovery are still incomplete.
  See [ledger tests and limitations](reports/w2-rpc-terminal-ledger-source-review-2026-09-12.md).
- `RpcSpawnObservation` records a consumed launch's host-reported spawn result,
  not RPC completion or current process state. V12 stores one immutable report
  with atomic event/outbox; exact replay is a no-op and changed reports conflict.
  The trusted Linux fixture host records actual spawn PID or no-spawn reason.
  Recording errors retain the exact child and original spawn error where present;
  callers must reap/reconcile, never reset a claim or relaunch automatically.
  See [ledger source study](reports/w2-rpc-spawn-observation-source-review-2026-09-12.md)
  and [host ownership tests](reports/w2-rpc-spawn-recording-host-source-review-2026-09-12.md).
- `RpcLaunchQueryRepository` reads the full descriptor/task and optional claim
  time from one SQLite snapshot without registering or claiming. Historical
  claims survive cancellation/reopen; absence and presence both leave process
  liveness and retry authority unknown. Corrupt linkage/time is rejected, not
  repaired. See [source review](reports/w2-rpc-query-source-review-2026-09-12.md).
  `project-graph-agent --database harness.db rpc-launch LAUNCH_ID task.json`
  emits a brief, byte-bounded JSON observation (`--max-output-bytes`, default
  65536 including LF), without argv/env or retry permission. Complete TaskSpec
  is a caller-supplied filter, not authentication. Missing records do not prove
  an absent process; standard database open can still initialize/migrate.
  The unreleased v1 projection now includes `launch.spawn_observation`, null or
  `{observed_at_ms, disposition, process_id}`, read with the claim in one snapshot.
  PID and disposition are historical reports, not liveness, terminal success or
  attach/kill permission. Strict consumers of the previous output must account
  for this additional field. Corrupt reports cause an error, not silent absence.
  `launch.terminal_receipt` is also nullable; when present it contains
  `finished_at_ms`, `supervised_elapsed_ms`, versioned `completion` and
  `uncertain_request_count`. All launch/terminal/reference reads share one
  deferred transaction. This does not read CAS bytes or establish current
  process state, cleanup verification or retry authority. Method names and raw
  output are omitted. Strict consumers must accommodate this additional field.
  Application `verify_rpc_outputs` separately checks a host-selected exact launch,
  reads stored output bytes through the scoped reader with a total byte budget,
  and rechecks the ledger snapshot before returning fresh content observations.
  It does not repair missing bytes, grant retry authority or upgrade completion;
  absent stream descriptors remain absent. Reader deadlines remain host-owned.
  `project-graph-agent --database harness.db verify-rpc-outputs launch.json CAS_ROOT`
  exposes this check as bounded JSON. `--max-total-bytes` defaults to/caps at
  64 MiB; `--max-output-bytes` defaults to 65536 including LF (maximum 16 MiB).
  Launch JSON is capped at 8 MiB. Present streams report verified hash/length;
  missing descriptors are null, never fabricated empty verified streams.
  The supplied launch/root are caller expectations, not authorization. No raw
  output is printed or observation persisted. Failure has nonzero exit and no
  success JSON; byte limits do not impose filesystem I/O deadlines.
- Linux `TrustedRpcFixtureHost` composes host-selected exact description,
  executable/env/cwd checks, prepared initialization, SQLite one-shot claim,
  direct spawn and ownership-preserving supervisor attachment. Five integration
  tests cover actual RPC context/reap, reopen, preflight rejection, durable spawn
  failure and cancellation after claim. Only immutable owned fixtures: not a
  production approval resolver, containment or crash-recovery implementation.
  See [source and verification receipt](reports/w2-rpc-fixture-launch-source-review-2026-09-12.md).
- `RpcLaunchSpec` domain and strict versioned JSONL wire bind a process and
  connection description to origin task/lease, host/approval references and
  execution snapshot. Fifteen new domain/wire tests pass. References are claims,
  not approval or launch capability; host approval/spawn wiring remains open.
- `RpcLaunchRepository` + SQLite V11 register immutable descriptions and claim
  once against the current exact task lease. Unique origin-attempt/host-epoch
  bindings prevent alias IDs; event/outbox commit atomically. Twelve integration
  tests cover replay, fencing, contention, rollback and corrupt linkage; V10
  upgrade preserves history. Claim success alone does not authorize execution.
- Linux `ConnectionSupervisor` owns a trusted direct child and bounded I/O,
  checks timeout/cancel before data, reaps centrally and drains after shutdown.
  Three process tests cover normal RPC, stalled consumer cancellation/timeout
  and protocol failure. Deadlines require regular polling and start at attach;
  unresolved handles transfer on finish. No approved spawn factory, containment,
  automatic Drop reaper or cleanup authority; scope remains Unverifiable.
  A post-handshake blocked-input regression verifies stagnant partial delivery,
  cancellation/reap and preserved request identity/counters without retry.
- Linux `ConnectionSetup` validates initialization before spawn and attaches
  pipes from one owned child. Missing-pipe/setup errors return the child for
  explicit cleanup; attachment sends no bytes. This is not approved spawn.
- Linux `ConnectionIo` polls bounded stdin/stdout/stderr and emits at most one
  protocol event per poll. Raw output and uncertain pending requests survive
  stop/error; stderr can drain after stdout EOF. Three process tests cover
  JSONL plus non-newline stderr, overflow and early stop. Child lifecycle and
  cleanup supervision remain separate unfinished work.
  Actual-pipe boundary tests distinguish exact/oversized frames, invalid JSON
  and partial-frame EOF, retaining raw bytes and uncertain pending identity.
- Linux `ConnectionInput` joins the nonblocking writer with JSONL handshake and
  RPC correlation. ACK readiness follows complete pipe write; busy admission
  does not reserve another ID; failures/close retain uncertain pending metadata.
  Three owned-peer pipe tests pass. No full transport supervisor or native dispatch.
- Linux `NonblockingInput`: one byte-bounded frame, incremental pipe writes and
  explicit close retaining partial-delivery counters. BrokenPipe is sticky;
  no automatic retry or claim of peer acknowledgement. Three actual-process
  tests cover backpressure, binary/EOF and failure. Not yet a full RPC transport.
- Linux-only `graph-execution` trusted-fixture adapter: explicit argv/env/cwd and
  executable digest, closed stdin, bounded nonblocking binary streams, timeout,
  cancellation and direct-child reaping. Nine actual-process tests and three
  lifecycle unit tests pass, including simultaneous stdout/stderr pressure.
  Process timeout does not reclassify drainage after the child is reaped.
  Cleanup remains `Unverifiable`; unresolved child handles transfer to the caller.
  No sandbox, process-tree guarantee, native runtime, launch-claim wiring or
  verified receipt publication. Preflight/spawn are not deadline-bounded and
  path/hash checks are not race-proof. See the
  [source review](reports/w2-fixture-executor-source-review-2026-09-11.md).
- Typed source snapshot and task contracts.
- `publish_fixture_outputs` preflights both raw output descriptors, then reuses
  metadata-first ingestion with independent hash/length readback. A stderr
  failure retains stdout success for retry without rerunning the process.
  Three in-memory port tests cover publication/replay and partial failure.
  This helper alone is not launch attestation or receipt authority.
- Four Linux composition tests join the owned fixture, DirectoryArtifacts and
  SQLite plan/claim/receipt ledger. Reopening preserves receipt and prevents
  claim reuse; same-length blob corruption fails fresh hash verification.
  Binary exit, timeout, output truncation and spawn failure preserve distinct
  outcomes; empty stdout/stderr deduplicate without losing stream descriptors.
  Store/source remain test-only executor dependencies. This is not production
  dispatch, candidate application or process-tree cleanup proof; outcome stays
  Unknown when cleanup is Unverifiable.
- Linux descendant-held-pipe regression: the direct child exits while a finite
  descendant retains both streams. Runner returns Incomplete/Unverifiable at
  the drain deadline; an isolated test subreaper then reaps the exact descendant.
  Production descendant cleanup remains unimplemented. The ignored helper is
  invoked explicitly by its parent test, not silently skipped coverage.
- V10 one-shot execution launch claim atomically revalidates the registered plan
  and candidate metadata, then writes claim/event/outbox. Only the first committed
  claim returns true; restart and timeout never recycle it. Consumers must handle
  `execution_launch_claimed`. No OS process, approval or host authentication is
  implied; a crash after claim requires reconciliation, not automatic relaunch.
- Versioned ExecutionPlan and V9 registry pin binding/host/execution snapshot.
  First insertion atomically checks submitted ledger, policy and candidate metadata,
  rejecting registration after a receipt already exists. Existing-plan replay is
  historical, not a relaunch grant. Receipt writes with a plan must match its target.
  Legacy receipt-only diagnostics do not acquire fabricated plans on upgrade.
  This does not attest original lease expiry, authorize commands or prevent a
  host from launching twice; launch admission/lifecycle remains pending.
  Registry also validates candidate analysis-run linkage on read. Tests cover
  plan/receipt and plan/cancel races, corrupt plans, and an orphan candidate in
  an intentionally damaged fixture DB (delete guard and FK protections bypassed).
- V8 stores immutable execution receipt claims keyed by run ID and linked to
  the exact stored task. Identical replay is idempotent; changed claims conflict.
  Scoped reads reject corrupt identity/version/task linkage. Late diagnostics
  survive cancellation and restart without granting execution or integration.
  This is not pre-launch run admission or authenticated replay protection.
  Receipt tests cover eight independent writer connections and subprocess exit
  before/after commit. The crash helper is ignored as a standalone test and is
  explicitly run twice by its parent test, with bounded wait and exit checks.
  These tests do not simulate power loss or authenticate first-write ownership.
- `verify_receipt_outputs` checks host-selected binding/host/snapshot, independent
  total byte budget, exact registered descriptors and actual stream hashes/lengths,
  then rechecks the submitted candidate. Result retains original completion claims;
  correct output bytes cannot turn unknown cleanup into a verified execution.
  No candidate-byte verification, host authentication, replay protection or write.
- Versioned ExecutionReceipt binds check-run, claimed host, execution snapshot,
  wall timestamps, monotonic elapsed, retained stream descriptors and completion.
  Complete streams require artifacts, including empty output; missing output is
  explicit null. Output scope/run/kind/byte budgets are structurally checked.
  Wall-clock reversal is retained, not confused with negative elapsed time.
  Receipt parsing does not prove candidate application, host identity or bytes.
- CheckRunBinding associates task, origin lease, submission sequence, candidate,
  required policy and exact check command. Its validated shape is provenance data,
  not proof of current lease, registration, actual execution or permission.
- `reconcile_check_binding` compares that binding with the current submission,
  registered policy and full artifact descriptor, then rechecks the submission.
  Missing, mismatched or changed metadata fails without writing ledger events.
  This is an observation, not blob verification, original lease-expiry attestation,
  command approval, or protection against changes after return.
- Noninteractive CheckCommand contract binds argv/cwd, declared executable and
  environment digests, runtime/cleanup deadlines and stdout/stderr budgets.
  This descriptor is not execution approval; actual host binding remains pending.
  Application fingerprints use versioned length-delimited SHA-256 for command
  and explicit environment data, without reading ambient environment variables.
- Execution completion observations distinguish stop reason, child reaping,
  stdout/stderr completeness and process-scope cleanup. Versioned JSON decoding
  cannot create verified execution evidence; exit zero alone is insufficient.
- Pure required-check policy reports missing, duplicate, unexpected, non-passing,
  wrong-candidate and evidence-free observations. Reports remain untrusted claims;
  satisfying this policy alone never authorizes integration.
- Immutable task check-policy registration/read port backed by V7 migration.
  First registration requires the exact queued, never-leased task; identical
  replay survives restart and leasing. Older tasks receive no fabricated policy.
  This host-only port is not yet wired to dispatch or an agent-facing CLI.
  New registrations atomically emit `check_policy_registered` and outbox data;
  replay preserves original history. Legacy policy rows without events are not
  backfilled with invented timestamps. Consumers must support the new event kind.
- `PolicyLeaseRepository::lease_with_policy` checks exact task and registered
  policy within the lease transaction. Missing/mismatched policy cannot fall back
  to an ID-only lease. Both paths share fence/dependency/event logic; legacy CLI
  `lease` is still ungated and must not be used by the future native dispatcher.
- SQLite task/event store with atomic enqueue, dependency references, lease
  ownership and monotonically increasing fencing tokens.
- Version-controlled SQL migrations run by `refinery`; schema DDL lives in
  [`crates/store/migrations`](crates/store/migrations), not inline in Rust.
- Candidate submission is separate from acceptance: a submitted patch does
  not unlock downstream tasks or materialize graph facts.
- Application `verify_submitted_content` binds the complete expected task and
  registered artifact snapshot, verifies bounded content bytes, then rechecks the
  submission for changes during I/O. This read-only observation does not grant
  integration authority or eliminate the need for atomic commit-time checks.
- JSON CLI with init, enqueue, lease, submit and paginated events; legacy
  `integrate` is retained only to return an explicit unsupported error.
  All CLI JSON-file inputs use an 8 MiB byte limit before parsing, reading at
  most one extra sentinel byte. Oversize files fail without performing the
  requested registration/enqueue/submit (normal DB startup may still migrate).
  This does not cap DB-resident metadata, parser overhead, or blocking time for
  special files; artifact blob stdin retains its separate `--max-bytes` budget.
- `task ID` returns the validated stored spec and ledger state from one query,
  or `task: null`. This is a coordinator/database-owner view, not a scoped
  subagent endpoint or authorization boundary. It never grants a lease or proves
  agent liveness; expired leases remain `leased` until a real state transition.
  Reads do not emit task events, although normal CLI startup can migrate the DB.
  `task ID --scope TASK.json` filters by all project identity fields and graph
  version; mismatches return null and invalid scope fails without unscoped
  fallback. This application-level filter runs after validated lookup, not as
  database row-level security. Scope is caller supplied; the native host must
  authenticate/bind it and withhold unscoped tools/DB access from children.
  Task output defaults to a 64 KiB serialized JSON-line cap, configurable with
  `--max-output-bytes` (maximum 16 MiB). Overflow returns nonzero without writing
  partial JSON; newline/escapes/UTF-8 count toward the cap. This bounds output,
  not DB input/Value allocation or OS pipe failure atomicity. Other commands do
  not yet share this output cap; bounded context compilation remains pending.
- Immutable source citations with snapshot-scoped reads through
  `record-evidence citation.json` and `evidence ID task.json`. Citation JSON
  includes `schema_version`, `id`, `project`, `graph_version`, `path`,
  `content_sha256`, `start_line`, `end_line`, and `analysis_run`.
  The SHA-256 identifies the full source file; line ranges are inclusive.
- Event outbox with independently checkpointed, ordered consumers.
- `DirectoryArtifacts::ingest` stages bounded input, verifies length/hash,
  syncs and publishes without overwrite. Existing valid blobs replay; existing
  corrupt blobs fail without replacement. Application `ingest_artifact` registers
  metadata first, then writes and re-verifies the blob. Failed writes leave
  unverified metadata for identical-descriptor retry.
  CLI: `project-graph-agent --database state.db ingest-artifact artifact.json blobs < output.bin`.
  The run must already be registered and the private blob root must exist.
  Descriptor hash/length must describe the exact input; no automatic redaction
  or descriptor generation occurs. Piped/redirected stdin is required, with
  a default 64 MiB budget (`--max-bytes`). Success outputs metadata only.
  Requires a private host-managed root and hard-link support; validated on Linux.
- Streaming artifact content verification through `verify_artifact` and the
  read-only `DirectoryArtifacts` adapter: exact length/SHA-256, fixed 32 KiB
  buffer, no blob bytes in the result. Host-selected root/snapshot binding is
  caller supplied; verified content is not verified protection or execution.
  CLI: `verify-artifact ID TASK.json BLOB_ROOT --max-bytes 67108864`.
  Reads the SHA-256-named file and returns metadata only; by default no receipt
  is persisted. Add `--observation-id CHECK_ID` to save a successful check to V6.
  Use a new ID for each observation: an existing ID with different timestamp or
  content conflicts. Time is sampled from the host after verification, not supplied
  in JSON. A persistence error returns nonzero even if bytes were checked.
  `artifact-observation CHECK_ID TASK.json` reads historical metadata without
  opening blobs, returns null outside scope, and always reports current
  `content_verified: false`. It does not grant integration authority.
- Artifact domain/wire descriptor with snapshot, run, exact-byte SHA-256/length,
  retention class and producer-declared protection. This is metadata only;
  immutable metadata persistence requires a registered run in the same snapshot.
  Verified protection, durable ingestion receipts and consumer references remain pending.
  CLI `record-artifact ARTIFACT.json` requires matching run registration;
  `artifact ID TASK.json` returns scoped metadata or `artifact: null`.
  Both report `content_verified`, `protection_verified`, `execution_verified`
  as false. The task file supplies scope, not authorization.
- Immutable AnalysisRun registration through the application/store API,
  scoped by the full project snapshot and graph version. Registration is
  provenance metadata, not execution attestation. CLI: `record-analysis-run RUN.json`
  and `analysis-run ID TASK.json`; both return versioned JSON with
  `execution_verified: false`. Lookup returns `run: null` for missing or
  differently scoped registrations. The task file supplies scope, not authorization.
- `cancel TASK --requested-by ACTOR --reason REASON` records cancellation
  with atomic event/outbox, idempotent replay and stale-worker submission
  rejection. Its JSON explicitly reports `process_termination_confirmed: false`;
  the recorded actor is not an authorization grant.
- `verify-evidence ID task.json ROOT` verifies a recorded citation's full-file
  SHA-256 and line range, then emits only the requested UTF-8 source slice.
  Defaults are 8 MiB source input and 32 KiB slice output; use
  `--max-source-bytes` and `--max-slice-bytes` to set limits (slice <= source,
  source <= 64 MiB). Oversized slices fail rather than silently truncating.
- `analyze-compose ID task.json ROOT` reads a recorded **full-file** citation,
  verifies its bytes, and emits versioned deployment candidates without raw YAML.
  It reports services/volumes/networks and declared mounts/dependencies/network
  attachments, with source lines/hashes and unknowns for unsupported semantics.
  `--max-source-bytes` defaults to/caps at 1 MiB; `--max-output-bytes` defaults to
  256 KiB and caps at 16 MiB, including the final newline. Budget or verification
  failure produces nonzero exit with no partial JSON. Task JSON remains capped
  at 8 MiB. These byte limits are not allocation or filesystem deadline limits.
  Output is `candidate_only: true`, `persisted: false`: no derived citation or
  graph fact is written. Standard DB open may initialize/migrate. Root/snapshot
  binding is caller supplied; source verification does not verify the analysis
  run, a running deployment or authorize graph publication. No Docker is invoked.
  See [Compose CLI source review](reports/w9-compose-cli-source-review-2026-09-12.md).
- Compose CLI now passes through an immutable `DeploymentGraph` aggregate before
  encoding: every citation must share the source scope/hash/run, endpoint kinds
  must match the declared relationship, and conflicting citation/node/edge IDs
  are rejected. Per-batch caps are 10,000 nodes, 50,000 edges and 50,000 unknowns,
  with separate text-field caps; these are not total allocator limits. Empty
  graphs remain valid replacement inputs. This structural boundary alone is
  not a freshness proof or publication capability.
  See [deployment domain source review](reports/w9-deployment-domain-source-review-2026-09-12.md).
- `DeploymentRepository` persists declared graphs as relational header, node,
  edge and unknown rows through refinery migration V14. Replacement/invalidation
  uses a strict expected-generation compare-and-swap; generation zero means no
  previous owner, and every success increments. Owner includes the complete
  project snapshot, graph version, source path and adapter, but not adapter
  version. Empty successful graphs and invalidation tombstones are distinct.
  Citations remain immutable historical records. Reads use one transaction and
  reconstruct the validated aggregate; row counts detect missing records, not
  arbitrary coherent database tampering. Whole-project watcher and context/diagram
  integration remain open.
  See [deployment persistence source review](reports/w9-deployment-persistence-source-review-2026-09-12.md).
- `publish-compose ID task.json ROOT --expected-generation N` verifies the
  recorded full-file source, parses its declared graph and publishes with strict
  generation CAS. `analyze-compose` remains non-persistent. Source is capped at
  1 MiB; `--max-output-bytes` limits the mutation receipt (default 256 KiB,
  newline included), and rejection occurs before graph mutation.
- `deployment ID task.json` reads that citation's **owner scope**, not necessarily
  its old content hash: another successful generation may have replaced the
  graph. Returns JSON `null` if no graph owner exists, a snapshot with `graph:null`
  after invalidation, otherwise stored declared nodes/edges/citations. It reads
  no source bytes and explicitly reports historical/unverified status.
- `invalidate-compose ID task.json --expected-generation N` retracts that owner
  while preserving historical citations. It needs no source root and can run
  after source deletion. Failed publish preserves the previous historical graph;
  use explicit reindex/watch below for automatic source failure invalidation. Task/root
  bindings are caller supplied, not an untrusted worker's authorization token.
  DB commit can succeed even when writing stdout fails: inspect `deployment`
  before retrying, because strict CAS does not silently replay an old generation.
  These commands use standard DB open (may initialize/migrate), never Docker.
  See [deployment CLI source review](reports/w9-deployment-cli-source-review-2026-09-12.md).
- `reindex-compose ID task.json ROOT --analysis-run RUN --expected-generation N`
  captures new bytes at the registered citation's path; its old hash and line
  range are locator metadata, not the new content expectation. A new immutable
  full-file citation binds the captured bytes, complete caller scope and declared
  run label. Parsing is followed by a hash recheck before generation CAS. Valid
  source replaces graph rows; source/parser failure invalidates them and returns
  an explicit JSON report with a nonzero exit. This differs intentionally from
  `publish-compose`, whose rejected input leaves the old historical graph alone.
- `watch-compose` accepts the same arguments plus `--interval-ms` (default 1000,
  10–60000) and optional `--max-cycles` (1–1000000). It polls **one owner**,
  sequentially, carries the successful generation forward and emits one bounded
  JSON line per cycle. Unchanged graphs/tombstones do not increment generation.
  Source failures permit later recovery; CAS/DB/output errors stop the loop.
  Without a cycle cap it runs until interrupted. No background daemon, recursive
  watcher, unbounded queue or new filesystem watcher dependency is introduced.
  No-op and valid reports remain historical observations; byte rechecks are not
  an atomic filesystem+DB snapshot or a whole-worktree Git verification. Root
  binding and run labels remain caller supplied. OS I/O may outlast polling
  intervals; byte limits do not impose deadlines, and edits between polls may
  be missed. Task/run/output-budget errors do not invalidate graph state.
  See [reindex/watch source review](reports/w9-compose-reindex-source-review-2026-09-12.md).
- `deployment-context ID task.json --node NODE_ID` returns a bounded declared
  neighborhood without source bodies. Defaults: `--direction both`, `--depth 2`,
  `--max-nodes 40`, `--max-edges 80`, `--max-output-bytes 32768` including newline.
  Incoming traversal does not reverse the meaning of edges. Nodes/edges refer to
  deduplicated full citations through `evidence_id`; those IDs can be queried with
  `evidence`. Root source metadata and generation remain linked. Limits are
  explicit: budget/depth flags describe cut traversal; omitted counts also
  include unrelated/direction-excluded facts in the owner, and `unknown_count`
  counts all its unmodeled declarations. Output byte overflow returns an error,
  not broken JSON. Missing owner, tombstone or seed is an error. Context is
  historical and untrusted, not runtime proof or complete system architecture.
  This adapter slice is not the full multi-provider context router or Archify IR.
  Store currently reconstructs the entire owner before selection; output limits
  do not cap database fetch cost. Analysis-run labels are not verified by query.
  See [deployment context source review](reports/w9-deployment-context-source-review-2026-09-12.md).
- `analyze-sql ID task.json ROOT` analyzes an existing full-file citation as
  SQLite syntax. Defaults: 128 KiB source, 65536 output bytes including newline;
  `--max-source-bytes` accepts 1..131072 and `--max-output-bytes` 0..16777216.
  JSON contains the original evidence and ordered syntax observations, not SQL
  bodies. Partial citations, stale hashes and scope mismatches fail with empty
  stdout. Byte caps are not I/O deadlines. Output is untrusted/candidate-only:
  bytes/line range were checked during the command, but Git/root snapshot and
  analysis-run identity are not proven; relations are unresolved syntax and
  spans are partial parser hints. No graph publication or SQL execution occurs;
  normal store opening can initialize/migrate the harness database.
  Reproduce the Orders emitted-CLI check after building with
  `scripts/with-local-tools node scripts/orders-sql-cli-smoke.mjs target/debug/project-graph-agent NEW_RECEIPT.json`.
  It checks all 11 SQL files, source hashes, uncertainty flags and exact/one-under
  output caps using an isolated temporary ledger. Existing receipts are never
  overwritten; passing proves syntax expectations, not code→query graph binding.
- Reference-side automatic file discovery can be reproduced after compiling
  the local CodeGraph checkout with
  `scripts/with-local-tools node scripts/orders-file-discovery-smoke.mjs /path/to/codegraph NEW_RECEIPT.json`.
  It discovers read chains from Orders source without authored offsets, captures
  target identities, then compares all 11 SQL paths/hashes with ground truth.
  It writes only the new receipt, not a graph or SQL database. Coverage is
  recognized same-file fs imports; runtime identity and atomic snapshots remain
  unverified. This is a development probe, not a shipped harness command.
- The local CodeGraph checkout also exposes the same read-only inspection as
  `codegraph file-reads src/store.mjs --path /path/to/project`. It accepts
  JavaScript by default (or `--language typescript`) and emits bounded JSON;
  no index is required or created. Candidate bindings live once in
  `discovery.candidates`; every capture uses `candidateIndex` to refer back to
  it. Exit 0 means observed, 2 means a complete but incomplete observation,
  and 1 emits no JSON. This remains candidate evidence, not execution or graph
  publication.
- `graph_system::analyze_sqlite(&str)` is an offline SQLite-dialect syntax
  adapter backed by pinned Apache sqlparser 0.62.0. It returns a source hash,
  ordered statement operation categories and syntactic relation names, not SQL
  bodies. Limits: 128 KiB source, 2048 tokens (including whitespace), 256
  statements and parser recursion 32; errors return no partial report or SQL
  snippets. Tokenization allocation is bounded by the byte cap, not the later
  token cap. Unclassified statements remain `Other`. Relation names can refer
  to CTEs/views; read/write roles, physical tables and database instances are
  unresolved. Optional parser spans can be partial, so use source hash and
  ordinal as observation anchors, not spans as full-statement edit ranges.
  Syntax acceptance never sets `semantic_verified`; the adapter does not run
  SQL, persist facts or yet connect CodeGraph query-file references to tables.
- `deployment-diagram ID task.json --node NODE_ID` exports a JSON bundle:
  `diagram` is typed Archify architecture IR; `evidence_manifest` retains the
  selected context and complete citations. Bindings map each drawn component or
  connection to its exact manifest array entry. Defaults: both directions,
  depth 2, at most 12 nodes / 24 edges, 65536 output bytes including newline.
  Node/edge caps cannot exceed these overview limits. All edges are dashed IaC
  declarations, not traffic; Compose kinds use a neutral renderer category.
  Cards disclose historical state, omitted facts and owner-wide unknowns.
  Display labels shorten after 32 Unicode characters; full names and mount
  targets remain in the manifest. IDs are bundle-local, not revision-delta IDs.
  This command does not render, verify Git source links, create collapse rules,
  or complete the five-view compiler. Extract `diagram` for Archify and keep
  the bundle beside the artifact for evidence lookup. Do not pass the whole
  bundle as Archify IR or infer Git verification from a successful render.
  The explicit local integration gate below exercises actual validation,
  delivery, and last-good preservation on a rejected candidate; it does not
  install Archify, use a live model, or open a browser:

  ```sh
  scripts/with-local-tools cargo build --bin project-graph-agent --locked --offline
  scripts/with-local-tools node scripts/validate-deployment-diagram.mjs /path/to/archify/archify
  scripts/with-local-tools node scripts/validate-sql-link-diagram.mjs /path/to/archify/archify
  ```

  Each command prints an owned output directory containing bundle, IR, HTML and receipt.
  An optional second argument selects a **new**, non-existing output directory.
  Fixed grid placement is not general graph auto-layout; complex neighborhoods
  may fail Archify layout checks and require a future layout compiler.
  See [diagram source review](reports/w11-deployment-diagram-source-review-2026-09-12.md).

```sh
cargo test --workspace
cargo run --bin project-graph-agent -- --database /tmp/project-graph-example.db init
cargo run --bin project-graph-agent -- --database /tmp/project-graph-example.db events
```

Run the local foundation gate with `sh scripts/validate-foundation.sh`.
It checks declared Cargo dependency boundaries, the Node diagnostic tests,
and the Rust workspace tests. `node scripts/check-architecture.mjs` can run
separately: optional, renamed, target-specific, dev and build dependencies are
checked against an explicit per-package policy. New packages/dependencies need
review in that policy. This does not audit transitive dependencies, source-level
module imports, runtime capabilities or complete product acceptance.

This is implementation in progress. It does not yet run models, shell jobs,
graph extraction, browser tools or merge patches. The target is one Codex account
with native subagents inheriting the root model, not cross-account workers.
The existing `account_lane` wire field is legacy requested metadata, not an
implemented auth binding; migrate it compatibly to native session binding.
Lease reassignment does
not cancel an external process; the execution layer must reconcile side effects.
Evidence is recorded as unverified. The source verifier checks bytes, line
bounds and directory containment through `cap-std`; the checked text remains
untrusted input. The host supplies ROOT's project/snapshot binding, which is
explicitly labeled `caller_supplied`: this command does not compute the whole
worktree fingerprint or validate analysis-run identity/relationship semantics.
Successful source verification does not mutate an assertion or accept a task.
The legacy `integrate` command and production Store integration port reject
caller strings, including for submitted tasks. No success event or dependency
unlock occurs. Verified integration authority remains unimplemented: it must
bind candidate bytes, source snapshot, scope, checks and target head. Historical
integrated rows are not rewritten; transaction/readiness tests seed integration
through a private test-only fixture, not a production acceptance path.
The unsafe string-based assertion acceptance method has also been removed.

## Local storage direction

`SubmissionQueryRepository::submitted_candidate` reads task spec, owner/fence,
submission event sequence/time and unverified artifact reference in one SQLite
snapshot. Missing/duplicate or malformed submission bindings fail; non-submitted
tasks return none. This is host-only ledger observation, not an agent endpoint,
lease or integration authority. Artifact registration/content, source/check
receipts and target freshness still require verification before integration.

`graph_protocol::connection::Connection` composes framing, RPC correlation and
handshake behind one sans-I/O interface. It blocks ordinary requests until the
host confirms acknowledgement write, rejects repeat initialization, checks epoch
before consuming input, and yields one routed event at a time. Failures close the
logical connection and retain uncertain request metadata. The API returns unsent
bytes: an actual transport, permission checks, bounded queues, server-request
response policy and durable reconciliation are still required. Use this core
for transport integration rather than bypassing it through lower-level helpers.

`graph_protocol::handshake::Handshake` validates the matching initialize response
and orders preparation of `initialized` before host-confirmed write readiness.
Failed/malformed responses and explicit connection failure cannot become ready.
It projects user-agent/platform metadata and ignores server auth paths. This is
local sequencing only: the connection host must enforce it for every send, bind
the authenticated epoch and separately verify effective runtime capabilities.
No process transport is connected to this state machine yet.

`rpc::Message::encode_line` produces a complete bounded JSON line before any
transport write, validating RPC labels and including LF in the cap. CLI output
and RPC use the same `graph_protocol::output::json_line` implementation; it
serializes directly without first cloning payloads into another JSON Value.
Encoding does not send, flush, authorize or prove delivery. Direct serde calls
bypass the encode/decode policy; transport code must use the bounded methods.

`graph_protocol::correlation::PendingRequests` keeps bounded, connection-scoped
RPC metadata with monotonic integer IDs. It matches replies out of order, exposes
unmatched replies and passes through server requests/notifications. Ambiguous
timeouts retain slots; close stops new requests and exposes unresolved metadata
for host reconciliation. Late replies from the same epoch still match. Epochs
are host-supplied routing labels, not authentication. This table is in-memory;
it neither retries requests nor proves method-specific success.

`graph_protocol::rpc::Message::decode` classifies bounded Codex envelopes as
request, notification, success response or RPC error. It preserves numeric versus
string request IDs and unknown methods for explicit host handling. Mixed or
duplicate envelope fields fail decoding; errors do not echo raw payloads.
This is not a dispatcher: no approval replies, pending-request correlation,
authentication or automatic lifecycle changes are implemented here. Method
payloads/trace remain untrusted JSON and need separate validation.

`graph_protocol::native` provides a Deserialize-only lifecycle projection for
Codex app-server thread-close, thread-status and turn-start/completion messages.
It preserves validated IDs and distinct statuses without retaining transcript
payloads. Unknown methods/statuses and request-shaped envelopes are rejected;
this is not the complete message router. The host must bound frames before
parsing and authenticate their transport. No admission or task state changes
follow from parsing. Runtime-generation reconciliation is still required because
an unloaded thread can resume with the same ID. No native transport is wired yet.

`graph_protocol::framing::LineDecoder` incrementally emits one LF-delimited
byte frame at a time with an explicit consumed count. Its configurable 1-byte
to 16-MiB cap includes CR/LF; oversize and truncated EOF close the decoder.
CRLF is normalized only after a full frame arrives. This bounds the decoder's
retained bytes, not upstream read buffers, downstream queues or JSON allocation.
It performs no I/O; transport EOF is not thread/task completion.

`graph_application::SwarmAdmission` supplies host-owned in-memory child
reservation accounting: shared active/total/depth limits, host-derived dedup
keys, deadline/stop, and explicit terminal observation. Parent termination never
releases descendants. Completed reservations retain total-budget consumption
and dedup keys. It is not wired to native Codex yet: durable recovery, canonical
work-key derivation, authorized scope/dependency checks, retry policy and native
lifecycle evidence remain required. Creating separate gates would split budgets;
the host must own one shared gate per root and serialize every descendant call.
Reservations can bind one-to-one to native thread IDs. Exact repeated binds are
no-ops; replacement/cross-attempt reuse conflicts, including after completion.
Late binding after stop permits reconciliation, not dispatch. Unknown terminal
events do not release slots; the host must reconcile them after binding and
distinguish thread-final evidence from turn completion/idle. The gate pins the
host-supplied root ID, rejects root-as-child and validates each binding's parent
against the reserved lineage, including replay. An unbound parent requires
reconciliation first; a finished parent's retained identity still permits late
binding of an already-reserved child. These methods trust the host: local ID
checks do not authenticate native events, the root session or account identity.
`confirm_not_dispatched` releases only unbound reservations after host evidence
that no dispatch occurred; it rejects any bound thread. Lost responses cannot
be treated as no-dispatch. The prior generic attempt-terminal method was removed
after reviewing T3 Code's turn-completed/idle distinction.

Migration checks reject missing/future and divergent history explicitly.
Before journal configuration, nonzero SQLite `user_version` returns typed
`UnsupportedUserVersion`: legacy/foreign markers require explicit inspection,
not clearing the marker or fabricating migration history. A legacy importer is
not implemented. File-backed DELETE-journal fixtures retain identical bytes on
rejection; this is not a guarantee against SQLite recovery or concurrent writers.
File-backed tests cover a future V11 database and a V1 upgrade failing at V3:
pending V2 changes roll back, existing payloads survive, and an explicit fixture
conflict resolution permits retry. These tests do not simulate power loss or
prove byte-for-byte file immutability: opening may configure SQLite journal mode.

SQLite is the durable control plane: task/event ledger, leases, graph facts,
provenance and the transactional outbox. Zvec will be an optional, rebuildable
semantic read model for summaries, documents and context packs in W8; a vector
hit never creates or overrides a graph fact. The adapter will sit behind a
`VectorIndex` port because Zvec's Rust SDK links to its native engine and its
collection writer is process-exclusive.

## Delivery tracking

| Package | Status |
|---|---|
| W0 baseline | CodeGraph local-patch full suite: 4,629 passed, 0 failed, 11 skipped; source hash stable; platform/license/compatibility and agent benchmark pending |
| W1 protocol/store | Task store, event outbox, source evidence ledger and bounded citation verification implemented; semantic verifier, snapshot authority, assertion decisions and projectors pending |
| W2 execution | Pending |
| W3 native subagents, one account | Pending runtime adapter; architecture revised in PLAN.md |
| W4 graph/context | Pending baseline |
| W5–W13 | Pending; full scope remains in PLAN.md |

Reference snapshots are pinned in [repo-lock.json](repo-lock.json), and Rust
1.93.1 plus rustfmt/clippy are declared in [rust-toolchain.toml](rust-toolchain.toml).
Local tooling setup and baseline receipts are in
[the W0 report](reports/w0-codegraph-2026-09-11.md).
Local-patch full-suite results and fingerprinted probe evidence are in
[the W0 follow-up](reports/w0-store-owner-2026-09-11.md).

Next: investigate upstream extraction/flow failures, enforce context budgets,
finish snapshot/analysis-run verification, and connect executor/native subagents
once their prerequisite contracts are complete.

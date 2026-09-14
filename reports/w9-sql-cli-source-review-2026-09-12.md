# SQL CLI composition — before-code source gate

Read current outsource CodeGraph context registration/JSON/error handling and
complete cli-context-command.test.ts (SHA
8dd0540bdae8d465d099565af238bf6688612240ef58199e642657dae2107045).
Adopt actual emitted CLI boundary tests, not only in-process parser tests;
no daemon/account experiment. Unlike CodeGraph's general context, SQL output
is a bounded syntax report without source bodies or execution claims.

Read product compose.rs complete (SHA
c4ef7032729b925af5eb98340601a03c05bbe9bf7e7c08ee6fb2415257469990),
Compose full-source check, CLI argument/dispatch, tests fixture setup,
protocol deployment output types and output byte writer, input cap helper.
Reuse EvidenceRepository lookup in full project/graph scope, DirectorySource,
verify_source and json_line. A verified partial slice must not become a
full-file SQL report: require start_line=1 and parsed-text hash equal to the
full citation hash. Snapshot/root remains caller supplied and run ID unverified.

Contract: analyze-sql ID TASK_JSON ROOT, source cap 1..131072 default131072,
output cap 0..16777216 default65536 including newline. Existing store open may
initialize/migrate; analysis itself has no fact/ledger write or SQL execution.
Task JSON uses the existing 8MiB bound. Wire types live in protocol but must
not import graph-system; CLI maps parser observations to output-only types.
Return flags candidate_only/persisted/content_hash_and_lines_verified,
semantic_verified, relationship_verified and caller-supplied snapshot caveat;
relations remain syntactic unresolved, optional spans remain parser hints.

Skills rust-router/domain-cli/m06-error-handling guide boundaries, bounded
JSON stdout and redacted errors. m07-concurrency confirms no new async/thread
machinery is needed for this single-command bounded parse.

Acceptance: actual CLI success from stored full citation; exact output cap;
hash drift/deleted source/wrong project/partial citation rejection; parser
error redaction; unchanged legacy Compose command; full foundation and fmt.

## Focused implementation verification

Added output-only protocol/sql types and CLI analyze-sql dispatch/composition,
without protocol→system dependency, new crates or migrations. Cargo check
passed. Four emitted CLI tests passed (source fixture schema.sql, exact output
and source caps, repeated stable output, stale/missing/wrong snapshot, partial
citations and redacted invalid SQL/task JSON). The first attempt failed all
four at fixture setup because parent used --db; corrected helper to the actual
--database flag without changing CLI acceptance or weakening assertions.

Clippy for the binary exited 0; new SQL wire struct has one
struct_excessive_bools warning for its explicit independent verification flags.
No allow/lint suppression added, and no strict workspace-clean claim is made.

## Foundation and emitted Orders probe

Foundation receipt: `.harness/baselines/foundation-sql-cli-20260912-01.json`.
Terminal exit 0, 394 Rust passed, 0 failed, 3 ignored; 16 Node passed.
On resumption, independently recomputed all five recorded source hashes against
the working tree: all match. This is not a whole-worktree fingerprint.
Re-ran cargo fmt --all -- --check: exit 0.

Actual CLI probe receipt:
`.harness/baselines/orders-sql-cli-20260912-01.json`; 11 emitted reports,
13 statements. It preserves the full output reports and executable hash
7b5219512492892763bf8d64cdf14a1b163f6abdc5c9d103f512e814cb2ea600.
The authored Orders expectations are syntax test inputs, not graph or runtime
acceptance; product_acceptance remains false. Root/snapshot binding is caller
supplied, analysis-run identity unverified, relations syntactic and unresolved.

Next dependency: evidence-backed binding between source wrapper calls and query
files, followed by graph persistence/context/diagram integration. Do not infer
this binding from a matching string or promote relation names to physical
tables without scope/catalog resolution. W9 and the full product remain open.

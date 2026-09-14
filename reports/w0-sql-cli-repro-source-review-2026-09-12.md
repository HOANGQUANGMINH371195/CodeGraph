# Reproducible SQL CLI probe — before-code source gate

Read outsource CodeGraph scripts/agent-eval/probe-explore.mjs in full at
3ed73bc127323e63153bf6ec8354afa82ce36aaf; file SHA-256
27d65b3aa2986e61bb49b3bad8c24ec934c3c56d001be587f90cf459c9ffb058.
Adopt its real built-product boundary, not a replacement parser/scorer.
Avoid string-presence-only checks and swallowed cleanup errors as acceptance.
Read product scripts/orders-graph-smoke.mjs, validate-foundation.sh,
crates/cli/tests/sql.rs, fixture store.mjs and authored SQL expectations.

Implementation scope: committed, repeatable Node probe of the built SQL CLI.
Use independent temporary ledger/task inputs, never execute fixture SQL, check
source hashes before and after, compare exact citation/ordinal/operation and
syntactic relation lists, preserve uncertainty flags and complete emitted JSON.
Check exact output-byte limit and one-byte-under rejection via the binary.
Receipts are exclusive-create, subprocesses have timeout and output caps, and
cleanup targets only this invocation's mkdtemp directory. Missing executable
or failed child must produce nonzero outcome, never a passing empty report.
Build/source correspondence and full-product acceptance remain unverified.

This probe closes a reproducibility gap in the earlier one-off receipt. It does
not implement JS wrapper binding, SQL graph persistence or runtime semantics.

## Verification

- Added scripts/orders-sql-cli-smoke.mjs and README invocation.
- Actual target/debug/project-graph-agent: exit 0, 11 files, 13 statements,
  exact-cap and one-under rejection checked for each report. Binary and all
  recorded fixture/ground-truth hashes unchanged. Receipt:
  .harness/baselines/orders-sql-cli-repro-20260912-01.json (SHA-256
  12c62172964492b856fce986f107bac82494289a0ca283ecaf935cdd45bef59a).
- Missing binary: exit 1, passed=false, zero reports; negative receipt
  .harness/baselines/orders-sql-cli-repro-missing-20260912-01.json.
- Existing executable /usr/bin/false: child nonzero status produces exit 1,
  passed=false, zero reports; negative receipt
  .harness/baselines/orders-sql-cli-repro-nonzero-20260912-01.json.
- Reusing receipt path: independent spawnSync assertion verified exit 1 and
  exact original receipt bytes preserved.
- validate-foundation.sh completed exit 0 (exec session 93517);
  cargo fmt --all -- --check exit 0. No Rust production changes in this slice.

The script does not assert a complete SQL grammar or all graph acceptance gates.

## Parallel investigation handoff

Native Luna worker Jason (01a09687-80d5-7693-b1ff-5aa671c13503) was assigned
read-only wrapper-binding seam/negative-case research, not probe implementation.
No worker edits are authorized. Latest wait had no terminal result; retain the
same handle on resumption, do not infer completion or spawn duplicate research.

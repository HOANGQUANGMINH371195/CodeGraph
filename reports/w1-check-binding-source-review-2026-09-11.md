# W1-C check-run binding

Source gate: ready before implementation.

Re-read T3MP3ST clean `29824d5625ede419ac8cdae418c8f4c72c6270f7`, entire
`src/evidence/gate.ts` and `src/__tests__/parsers.test.ts:276–310`. Gate accepts
nonempty tool-tagged evidence; test materialises fixture output with target and
operator metadata. Adopt explicit provenance association; do not inherit its
metadata classification as proof of execution or correct harness snapshot.
Tests read, not run. No AGPL code copied.

Local TaskSpec/Lease/Artifact/RequiredChecks/CheckCommand and SubmittedCandidate
contracts inspected. Define immutable CheckRunBinding: unique run ID (host must
enforce persistence uniqueness), full task, origin lease, submission event sequence,
candidate descriptor, required policy, named check and command. Reject mismatched
lease task, invalid sequence/expiry, candidate project/graph mismatch and check
not in required policy. Preserve owner/fence and exact command as data.

Origin lease is provenance only: checks can happen after submission and expiry;
fresh execution authorization is separate. No constructor/JSON implies policy
registration, authentic lease, current submission, filesystem contents or host
permission. Execution worktree snapshot after applying candidate, output artifact
binding and observed host lifecycle must still be added to the complete receipt.

Tests planned: strict nested versioned roundtrip, all snapshot mismatch fields,
lease task/fence/expiry, check membership, submission sequence and authority-field
rejection. Skills: rust-router, m09-domain. No new dependency or runtime launch.

## Results

Added private-field domain binding and strict versioned protocol mapping. Three
tests cover full roundtrip, six project fields plus graph mismatch, lease task/
fence/expiry, sequence, check membership, required fields, unsupported nested
versions and unknown verified field. Origin expiry is preserved as provenance;
the constructor does not assert present authorization or authentic registration.

Foundation passed: 123 Rust + 11 Node tests, 6 crates / 30 direct dependencies.
No reference tests or processes executed. Ledger reconciliation, unique run
registration, execution worktree snapshot and complete output/host receipt remain.

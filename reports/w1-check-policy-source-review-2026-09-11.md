# W1 required-check policy source review

Source gate: ready, before implementation.

- Grit clean `0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe`, re-read
  `src/git/mod.rs:220–280`: rebase failure aborts and falls back to merge;
  merge exit success returns success. This is not check-suite verification.
- T3MP3ST clean `29824d5625ede419ac8cdae418c8f4c72c6270f7`, read entire
  `src/evidence/gate.ts` and `src/__tests__/parsers.test.ts:276–325`.
  The live gate accepts nonempty content tagged with a tool evidence type;
  tests construct findings from fixture scanner output. This proves metadata
  classification, not authenticity, execution identity or current snapshot.
  Upstream tests were read, not executed; no AGPL code copied/linked.
- Adopt explicit reasons and refusal without evidence. Do not equate evidence
  labels, prose or successful Git merge with verified execution/acceptance.
- Gap adaptation: a pure required-check policy evaluates observations against
  a specific artifact descriptor. Missing, duplicate, unexpected, non-passing,
  wrong-artifact and evidence-free observations remain explicit failures.
  Empty/duplicate/blank required-check configuration is invalid; no vacuous pass.
  This policy is NOT integration authority: observations remain caller claims
  until execution receipt/source/target verification and commit-time binding.
- Tests before use: malformed policy; each outcome; missing/duplicate/extra
  names; mismatched candidate; absent evidence; order-independent success.
  No executable checker or integration endpoint is being enabled here.
- Skills: rust-router, m09-domain. Keep policy pure and dependencies unchanged.

## Results

Implemented `crates/domain/src/checks.rs`: nonempty unique policy, explicit
outcomes/issues, deterministic exact-name assessment and full artifact equality.
Three tests cover configuration, all non-pass outcomes, wrong artifact identity,
blank evidence, missing/duplicate/unexpected names and ordering. Foundation:
102 Rust + 11 Node tests pass; 6 crates / 30 direct dependencies unchanged.
This is a pure policy foundation, not a wired acceptance endpoint or execution
receipt verifier. Host policy persistence/digest, task/attempt/target binding,
evidence resolution and transactional integration remain required.

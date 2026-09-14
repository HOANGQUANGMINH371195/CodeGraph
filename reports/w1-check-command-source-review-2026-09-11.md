# W1-C check command contract

Source gate: ready before implementation.

Re-read OpenDev clean `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
`crates/opendev-tools-impl/src/bash/foreground.rs:1–64`, `patterns.rs:1–60`
and `patterns_tests.rs:116–168`. Foreground runs sh -c, sets cwd, env_clear then
inherits filtered variables and enables unbuffered Python. Filtering excludes
known sensitive names/suffixes; tests cover those names and PATH preservation.
These assertions were read, not run (no process environment changed).

Adopt explicit execution inputs, caller timeout caps and environment handling.
Do not adopt implicit shell rewriting or name-based filtering as secret isolation.
Define a noninteractive CheckCommand: program+argv, portable relative cwd,
expected executable/environment SHA-256, bounded runtime/cleanup and separate
stdout/stderr budgets. Stdin is closed by this contract; interactive jobs remain
separate full-product work, not silently supported here.

Host must resolve/approve executable, enforce root/symlinks, construct environment
explicitly and verify actual digests before spawn. Declared digests are claims;
no authorization is created by deserializing. No actual env values in this type.
Fingerprint encoding and receipt task/policy binding remain follow-up work.

Tests planned: preserve empty/Unicode/metacharacter argv exactly, reject NUL,
invalid relative paths/digests/time limits, strict versioned JSON roundtrip.
Skills: rust-router, m09-domain. No new dependency or reference code copied.

## Results

Implemented validated private-field domain CheckCommand and strict versioned
wire mapping. No environment values stored. Debug reports only argument count;
serialization intentionally preserves argv and is not a redaction mechanism.
Three tests cover exact argv/Unicode/empty/metacharacter roundtrip, NUL/path/hash/
time/overflow rejection and mandatory fields/future schema/unknown authority field.
Zero-byte output budgets are valid; executor must enforce them, not imply no output.

Foundation passed: 118 Rust + 11 Node tests, 6 crates / 30 direct dependencies.
No executable invoked or environment changed. W1-C still needs canonical digest
encoding and task/lease/candidate/policy receipt binding; W2 must enforce real
filesystem, executable identity, environment, stdin and cleanup before acceptance.

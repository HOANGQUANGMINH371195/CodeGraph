# W1-C command/environment fingerprints

Source gate: ready before implementation.

OpenDev clean `d32c660e4eed1a8e988d1fd58da88e41ba641d08`: re-read
`crates/opendev-repl/src/skills.rs:44–56` (URL SHA-256 truncated to eight bytes for
cache naming), `bash/foreground.rs:40–60` and `bash/patterns_tests.rs:142–168`
under tools-impl (inherited filtered environment). Tests read, not run. Adopt
explicit byte hashing and environment construction, not truncated cache identity
or name filtering as an execution trust boundary. No code copied.

Gap: these references do not define structured execution fingerprints. Define
domain-separated full SHA-256 with u64 big-endian counts and UTF-8 byte-length
prefixes. Environment pairs sorted by exact UTF-8 key; invalid key/NUL rejected.
Host must supply the entire explicit environment, reject duplicate/case-colliding
names as required by the target OS, and prevent mutation between hash and spawn.
Fingerprint is not encryption/attestation; do not expose secret-derived hashes
as a protection mechanism. No process environment is read or modified.

Command encoding order: prefix `project-graph/check-command/v1\0`, program,
argument count + each arg, cwd, executable_sha256 text, environment_sha256 text,
timeout_ms, cleanup_timeout_ms, stdout_max_bytes, stderr_max_bytes. Strings are
u64-byte-length-prefixed. Integers/counts are u64 big-endian. Stdin closed and
explicit environment semantics are part of v1; changes require a new version.
Environment: prefix `project-graph/environment/v1\0`, pair count, then length-
prefixed key/value pairs in BTreeMap key order. No Unicode normalization.

Tests planned: independent Node crypto golden vectors, argv boundary collisions,
every command field changing the digest, environment ordering/value/key changes
and invalid names/NUL. Existing sha2 dependency reused. Skill: rust-router.

## Results

Added application fingerprint functions, no JSON/domain dependency changes.
Node crypto independently generated golden vectors using Buffer.writeBigUInt64BE
and UTF-8 byte lengths; Rust matched both:

- Command fixture (fixture; args empty and `a b`; cwd dot; digests a×64/b×64;
  limits 1000/500/1024/2048):
  `0e57be1f303c6d2dc11d6e046d043841cf56fb975328f56c2e004623876f0577`.
- Environment LANG=C.UTF-8, MODE=test:
  `4f6f1744f1db357d0d7952cd076058e6c28d5213d0a69b387c129a99142e6bd5`.

Two tests cover the vectors and planned mutation/boundary cases. Foundation:
120 Rust + 11 Node tests pass, 6 crates / 30 dependencies unchanged. UTF-8 inputs
are exact; Windows name aliases and non-UTF-8 environment support are host concerns,
not silently normalized here. No executable or actual environment verified.

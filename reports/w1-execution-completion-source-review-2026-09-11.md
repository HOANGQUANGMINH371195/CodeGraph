# W1-C execution completion contract

Source gate: ready before implementation.

Re-read OpenDev clean `d32c660e4eed1a8e988d1fd58da88e41ba641d08`,
`crates/opendev-tools-impl/src/bash/foreground.rs:108–172` and entire
`crates/opendev-mcp/src/transport/process_tests.rs`. Foreground ignores wait
results on cancellation/timeout and reader join timeout results; selected process
tests check enumeration/no-op, not successful reaping/drainage. Tests read only.

Adopt distinct stop reasons and separate child/output handling; avoid deriving
successful checks from exit zero alone. New pure report keeps child reaping,
stdout/stderr completeness and owned-process-scope cleanup independent. Missing
exit status, truncated/failed/incomplete streams and unverifiable cleanup cannot
produce reported Passed. Cancellation/timeout remain explicit even if exit zero
is observed later. No code copied, execution or sandbox started.

Versioned strict JSON report maps to domain observations only. It has no default
success, verified constructor, permission or integration authority. Host identity,
task/fence/snapshot/candidate/command/policy bindings remain separate required
receipt fields and are not claimed complete by this status component.

Tests planned: complete success, each incomplete dimension, all stop reasons,
strict wire schema, unknown variants and roundtrip. Skills: rust-router, m09-domain.

## Results

Added domain/protocol execution modules with explicit observations and conservative
reported-check projection. Two domain tests cover incomplete dimensions and stop
reasons; wire test covers mandatory fields, unknown values, future schema,
roundtrip and omitted numeric exit status. Initial negative test found tagged
unit variants ignored extra exit_code; changed wire variants to empty struct
variants so deny_unknown_fields applies, retaining the failing regression.

Foundation passes: 115 Rust + 11 Node tests, 6 crates / 30 direct dependencies.
No actual process or upstream tests run. This status component is not a complete
execution receipt: command/environment binding, task/fence/snapshot/candidate,
output artifact hashes, host evidence and verifier authority remain to implement.

# W1-C execution receipt source study

Source gate ready before implementation. OpenDev clean revision
`d32c660e4eed1a8e988d1fd58da88e41ba641d08`, bash/foreground.rs:25–175,
bash/helpers_tests.rs:1–65 under crates/opendev-tools-impl/src reviewed.
Readers collect line-based output, child polling separates timeout/cancel, but
reader join timeout results are ignored. Helper assertions cover presentation,
not authentic receipt/cleanup. Reference tests inspected, not executed.

Adopt separate stream and completion observations; avoid treating joined text
or child exit alone as proof. New independently written domain/wire receipt binds
CheckRunBinding, claimed host ID, execution ProjectRef, start/end wall timestamps,
monotonic elapsed duration, optional stdout/stderr descriptors and completion.
Execution worktree/head/fingerprint may differ after candidate application;
repository/config/ignore policy must match task. Outputs belong to execution
snapshot, task graph version and check run ID, have stream-specific kinds and
fit declared retained-byte budgets. Complete streams require descriptors (empty
output still has a descriptor); unavailable outputs remain explicit nulls.

Private validated fields, strict versioned wire. Tests will cover roundtrip,
scope/run/kind/size mismatch, missing complete output, time range and unknown
schema/authority. Skills rust-router, m09-domain, m05-type-driven.
No code copied, dependencies or migration changed. This is untrusted provenance,
not host authentication, actual candidate application, output byte verification,
deadline enforcement or integration authority. Runtime and registry remain open.

Validation: `sh scripts/validate-foundation.sh` exited 0, 129 Rust + 11 Node
tests passed. Three new tests cover roundtrip, execution snapshot differing from
source, missing complete output, read-failed output retained as Unknown, output
substitution across all snapshot fields/run/kind/graph/budget, negative wall
timestamps and elapsed overflow, explicit fields and unsupported/extra claims.
Wall-clock reversal is intentionally accepted: elapsed is independently measured,
not computed by subtracting wall timestamps. Format check and PLAN mirror checked.
W1-C remains open pending full contract/ledger coverage audit and receipt registry.

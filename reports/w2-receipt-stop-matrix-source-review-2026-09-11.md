# W2 actual receipt stop matrix

Source gate before code: OpenDev clean d32c660e4eed1a8e988d1fd58da88e41ba641d08,
foreground.rs:100–165 reread for stop/drain flow; bash test locations searched.
Existing product execution outcomes, composition and fixture tests reviewed.
Adopt preserving stop cause; upstream string result is not authoritative receipt.
No reference code copied or executed. Rust/CLI lifecycle/error skills already
read apply to this continuation.

Before patch: reuse actual SQLite/filesystem composition for binary exit,
timeout and output truncation, each with independent owned roots and run.
Verify exact completion survives receipt replay/reopen and fresh content check;
do not require blob_inserted twice for equal empty stream content (CAS dedup).
Timeout stays TimedOut; output limit stays Unknown with Unverifiable cleanup.
No production dispatch/approval or candidate application is introduced.

Follow-up before spawn-failure extension: read foreground.rs:65–78 spawn error
branch and bash/mod.rs:604–624 actual test assertions. The latter checks active
output avoids timeout, not a real timed-out receipt. This extra reference test
read occurred after the initial matrix patch, not retroactively before it.
Add a host-owned non-executable fixture copy to exercise SpawnFailed/NotSpawned,
empty output CAS dedup and immutable failure receipt without launching a child.

## Verified matrix

Four composition tests now exercise actual adapters: binary exit 23, 100ms
timeout of the owned finite sleep fixture, stdout truncation at 4096 bytes, and
spawn permission failure on a temporary non-executable fixture copy. Each records
an execution receipt, reopens SQLite/blob adapter, confirms exact receipt replay,
no second launch claim and fresh output verification without extra task events.
Equal empty streams deduplicate to one CAS blob while retaining two descriptors.
Nonempty stdout corruption specifically fails SHA-256 verification.

- Final `sh scripts/validate-foundation.sh`: 171 Rust + 12 Node pass, exit 0.
  Two standalone ignored helpers remain explicitly invoked by parent tests.
  Architecture unchanged: 7 packages, 39 declarations, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- Real-adapter tests make no snapshot/application/host-approval assertion.
  Timeout summary is TimedOut, truncated output Unknown, and spawn failure Failed
  with NotSpawned. Successful publication cannot promote cleanup uncertainty.

Independent cleanup source review was delegated read-only to native subagent
Franklin (inherited model), thread 01a093af-1783-7a11-b533-2140d358e653. Its result
was pending at this receipt's test run; parent review is now recorded in
`w2-cleanup-review-2026-09-11.md`, and the completed reviewer is closed.
Production cleanup/transport and W1-I acceptance remain unfinished.

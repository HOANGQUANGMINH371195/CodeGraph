# W2 drain lifecycle and simultaneous output

Source gate ready before edits. Reread clean OpenDev revision
`d32c660e4eed1a8e988d1fd58da88e41ba641d08`:
foreground.rs:110–180 separates child status from reader drain; helpers.rs:170–195
signals groups but does not establish reaping; bash/mod.rs:629–647 only asserts
launch success before group kill, not descendant disappearance. No upstream test
executed or code copied. Product AGENTS and m12-lifecycle read; prior Rust/CLI
skills apply to the same adapter. Product has no committed HEAD.

Adopt separate process and drain lifetimes. Current adapter checks runtime even
after child reaping, potentially reclassifying drain time as process timeout.
Before patch: gate runtime expiry on Unreaped; cancellation remains applicable
to the whole operation. Existing stop reasons remain sticky. Test decision
without timing races, plus real OS stream EOF with a retained writer handle.
Use finite owned fixture threads for simultaneous stdout/stderr floods and assert
exact raw bytes at both caps. No descendants launched by these tests: held-writer
coverage is a component test, not full process-tree acceptance. Leave that gate open.

## Verified results

- Three unit tests: runtime expiry cannot reclassify post-reap drain; existing
  stop reasons remain sticky and cancellation still applies during drain;
  retained UnixStream writer prevents EOF; missing reader reports ReadFailed.
  UnixStream is an OS socket pair, not a spawned descendant holding a pipe.
- Nine actual Linux process tests now pass. New simultaneous-pressure fixture
  has two joined writer threads, finite 512 KiB per stream, distinct raw bytes;
  exact caps both complete with exit zero and direct child reaped. Scope cleanup
  remains Unverifiable. No fixture creates subprocess descendants.
- `scripts/with-local-tools cargo test -p graph-execution --locked --offline`:
  3 unit + 9 integration tests pass, exit 0.
- `sh scripts/validate-foundation.sh`: 163 Rust + 11 Node pass; one standalone
  crash helper ignored in discovery but invoked twice by its parent test.
  Architecture: 7 packages / 36 declared dependencies, no errors.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- OpenDev remains clean. Source fingerprints after formatting (SHA-256):
  - src/linux.rs: `99d26bce9bd008631270b8e74f4d69e63342c47b1ab8b683fc6742c08147dab8`
  - fixtures/owned.rs: `ef5e7e5da453f162f364e475e6c72328c977d20eb8910c54c34d8d2a4f239ccd`
  - tests/fixture.rs: `0728e022f2193e5cfe45b60612e2610d403f921b1c3d029be67c753758ec00f9`

Remaining W2 gates unchanged: descendant/pipe-holder subprocess fixture and
scope ownership proof, deadline/cleanup failure paths, platform support, native
transport/backpressure, launch claim reconciliation and artifact/receipt wiring.
No complete W0–W13 package or native/untrusted execution authorization implied.

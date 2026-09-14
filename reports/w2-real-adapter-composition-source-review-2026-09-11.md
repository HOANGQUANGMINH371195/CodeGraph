# W2 owned process / filesystem / SQLite composition

Source gate ready before patch. Reread Grit clean
`0f3c9d04abe9525b1884f3a0ade890e54a6d9ffe` sqlite_store.rs:48–112,
448–480 (transaction and independent connection test setup), and OpenDev
truncation.rs:158–205 / truncation_tests.rs:70–83 at previously recorded local
revision. Adopt actual separate resource checks, not one in-memory fake for
everything; no source copied or reference tests run. Grit's TTL/refresh semantics
must not apply to one-shot launch claims. OpenDev text overflow is not binary
evidence. Existing domain, application ingestion, store test setup and directory
artifact adapter reviewed before composition.

Plan before code: add graph-store and graph-source as dev-only graph-execution
dependencies, explicitly allowlisted. Test owns temp SQLite/blob/working roots;
enqueue policy/task/candidate, register exact execution plan and claim once,
run only the existing owned binary, publish actual raw bytes, store receipt and
verify outputs. Reopen SQLite and directory, prove replay/no reclaim, corrupt
only the owned output blob and require fresh verification to fail. Use real wall
timestamps for execution and synthetic logical times for task ledger setup.
No candidate application or general host approval is asserted. Cleanup stays
Unverifiable, hence content verification cannot produce Passed. This is test
composition, not a production dispatch API. Skill m11-ecosystem keeps adapters
dev-only; no external dependency version or schema migration change.

## Verification

- `cargo test -p graph-execution --test composition --offline` through local
  tool wrapper: one test passed. Lockfile refreshed offline for the two existing
  workspace dev dependencies; no external crate version added.
- Final `sh scripts/validate-foundation.sh`: 167 Rust + 12 Node pass, exit 0;
  7 packages / 38 direct dependency declarations, no architecture errors.
  Existing crash helper is ignored in standalone discovery, invoked by parent.
- `scripts/with-local-tools cargo fmt --all -- --check`: exit 0.
- Actual owned binary returns exit 23, exact binary stdout/stderr and both EOF;
  direct child is reaped. Artifact publication reads back actual disk content.
  SQLite receipt survives connection/root reopening with identical content;
  one-shot claim and receipt replay return false without new events.
- Overwriting only this test's temporary stdout blob with same-length wrong
  bytes causes specifically HashMismatch; historical receipt remains unchanged.
  No reference files modified. OpenDev revision rechecked clean at
  `d32c660e4eed1a8e988d1fd58da88e41ba641d08`.
- Architecture regression proves store/source accepted only as execution dev
  dependencies, rejected as normal/build dependencies.

Limits: controlled test composition, not production host admission or native
dispatch. Project/analysis metadata and candidate bytes are fixtures; no actual
candidate application or repository snapshot attestation. No crash/restart of
the supervising OS process, concurrent relaunch or process-tree cleanup proof.
Cleanup remains Unverifiable and check outcome Unknown. Full W2/W1-I remain open.

# W2 Linux cleanup review: parent disposition

Native reviewer Franklin, inherited model, read-only thread
01a093af-1783-7a11-b533-2140d358e653 completed; parent closed it after review.
Reviewer made no edits and ran no tests/reference programs. Product has no HEAD.

Main independently read OpenSandbox initmode_linux.go:176–285 and
initmode_linux_test.go:145–177,221–240 at clean revision
eed301cca02b261256b2c5e5a23bb1a570c90160: registry-owned child uses pre-reap
barrier before consuming wait; “orphan” test starts an unregistered direct child,
not a double fork. Codex core/src/exec.rs:992–1138 at clean revision
818f1cca8ccf8899f0f4d59336baebaccf358eed separates leader completion and draining,
signals numeric process group on failed drain, and joins aborted output tasks.
OpenDev group signal/cleanup evidence was reread in prior W2 source receipts at
d32c660e4eed1a8e988d1fd58da88e41ba641d08. No reference code copied.

Accepted design constraints, not implemented capability:

- One supervisor owns registration, cancellation and consuming waits for each
  execution scope. Preserve ownership while finalization remains unresolved.
- Never treat a cached numeric PID/PGID after ownership expires as authority.
  Reuse hazard is a design inference, not a reproduced upstream bug here.
- Direct-child reap, stream EOF and scope containment require separate evidence.
  Existing fixture runner must remain Unverifiable for spawned scope cleanup.
- Investigate protected per-attempt cgroup placement plus stable process handles
  for production Linux scope; establish delegation/placement and failure behavior
  before implementation. Reviewer proposal is not proof host capabilities exist.
  Main kernel-document fetch did not complete and was cancelled; no kernel API
  claim is accepted from that fetch. Re-read authoritative API sources for that task.

Required next tests include double-fork/setsid, live descendants with closed pipes,
TERM resistance, concurrent-attempt isolation, pre-reap ordering, placement failure,
fork during teardown, supervisor crash and stale ownership rejection. These are
still unchecked W2 requirements, not reasons to enable native/untrusted dispatch.
No cgroup/process state or global Codex configuration was changed by this review.

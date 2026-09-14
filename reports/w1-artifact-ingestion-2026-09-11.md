# W1 — Staged blob ingestion API

DirectoryArtifacts::ingest accepts a descriptor, Read input and byte budget.
It checks snapshot/budget, creates an exclusive staging file (Unix mode 0600),
copies at most declared length+1 bytes, verifies length/SHA-256 from the staged
file, syncs it, then publishes via a same-directory no-replace hard link.
An existing valid blob yields false; an existing corrupt blob fails without
replacement. It removes its owned staging file and syncs the directory.
No whole-input Vec, new dependency or migration was added.

Resource/lifecycle skills informed cleanup ownership: a Drop guard attempts
removal on error, and explicit successful cleanup disarms it. Cleanup errors
on success propagate. Abrupt termination or failed cleanup can leave staging;
no automatic sweeping of other operations' files is performed.

The first test run found EBADF when syncing cap-std's directory handle (which
may use O_PATH on Linux). The implementation now reopens '.' within the same
directory capability before sync. The complete validation then passed:
`sh scripts/validate-foundation.sh`: 47 Rust + 10 Node tests, no failures/ignored;
architecture gate six packages/29 dependencies, format check passed.

Three new tests cover empty/binary/100k blobs, replay, preservation of corrupt
existing content, short/long/wrong-hash/read-error cleanup, preservation of an
unrelated staging sentinel, pre-copy byte budget, and four concurrent writers
publishing one blob with no final staging leaks. This is one concurrency
scenario, not exhaustive scheduling or injected-crash testing.

Requires a host-managed private per-snapshot directory with no hostile writer.
It does not enforce directory ownership, immutable filesystem attributes,
redaction or encryption. Hard links and directory fsync are required; only
Linux local-filesystem behavior was tested. No unsafe fallback copy/rename is
used on unsupported filesystems. Errors after publication can leave a valid
blob; retry verifies it. Blob publication and SQLite registration are not yet
coordinated. API does not require ledger run existence; the future application
ingestion use case must enforce that and reconcile orphan blobs. Crash recovery,
GC, live deadlines, CLI ingestion and durable receipts remain open.

## Application coordination

Added ArtifactWriter and ingest_artifact, composing existing ArtifactRepository
with blob publication and a separate read-back verification. The repository
contract enforces same-snapshot run linkage. Budget errors occur before metadata
registration; registration failures occur before consuming input or writing
blobs. Metadata commits first, so failed content work leaves an immutable,
unverified descriptor rather than a blob with no registered descriptor.
Identical-descriptor retry can complete publication after reopening SQLite.

ArtifactIngestion returns metadata_inserted/blob_inserted and verified content;
typed errors distinguish pre-registration errors from content failures with
metadata already registered. No durable success flag is added. This is not an
atomic distributed transaction, and metadata presence must never imply blob
availability. Private-root binding and compliant repository/reader adapters are
host obligations. Reconciliation after abrupt process death remains pending.

An integration test uses real SQLite and DirectoryArtifacts for missing run,
budget, hash failure, metadata preservation, reopen/retry/replay and conflict
rejection before input consumption. It lives in the CLI test target to reuse
existing adapter dependencies; it is not a CLI ingestion subprocess test.
An application unit test supplies a lying writer returning success and wrong
published bytes; read-back prevents producing a verified receipt.
`sh scripts/validate-foundation.sh` passed 49 Rust + 10 Node tests, zero failures
or ignored; architecture and format checks passed, no new dependencies.
Rust/domain skills informed the explicit partial-failure and retry contract.

## CLI pipe ingestion

Added `ingest-artifact ARTIFACT.json ROOT --max-bytes N`, default 64 MiB,
reading bytes from piped/redirected stdin. An interactive stdin is rejected.
The host supplies an existing private root and a descriptor for the exact
bytes. No automatic redaction, descriptor generation or execution success is
inferred. This is suitable for ingesting an already captured output file;
unknown-length live process output still requires capture/descriptor generation.
Non-terminating pipes can still block; byte budget is not a deadline.

Success returns versioned metadata-only JSON, metadata_inserted/blob_inserted,
content_verified true, protection_verified/execution_verified false,
snapshot_binding caller_supplied, content_is_untrusted true and receipt_persisted
false. Partial content failure prints a diagnostic noting retained metadata;
exit is nonzero with no success JSON. Root opening occurs before metadata
registration. The command currently opens the SQLite database before dispatch,
so even rejected interactive input may initialize the database.

A subprocess pipe test exercises missing run, budget-before-registration,
hash failure with retained metadata, retry in a new process, replay, byte-for-byte
blob readback, verify-artifact interoperability, short/long input, conflict and
unchanged metadata lookup trust. Help tests include ingestion without creating
a DB. Interactive TTY rejection is implemented but not covered by this pipe
test. Full foundation validation passed 50 Rust + 10 Node tests, zero failures
or ignored; format and architecture gate passed. CLI skill informed stdout/stderr
separation, pipe input and explicit trust flags. Durable receipt, crash recovery,
GC, deadlines and platform validation remain pending.
